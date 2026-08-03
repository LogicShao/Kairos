//! AI 设置 WebDAV 加密同步：信封加密（envelope encryption）。
//!
//! # 设计决策
//!
//! - **信封结构**：payload（AI 设置 + API 密钥明文）用随机数据密钥 **DEK** 以 AES-256-GCM
//!   加密；DEK 再用 WebDAV 密码派生的 **KEK**（PBKDF2-HMAC-SHA256）包裹，随文件头存放。
//! - **改密码不丢数据**：DEK 跨设备恒定。改 WebDAV 密码只需重派生 KEK、重裹 DEK（几百字节），
//!   数据体不动。本地保留一份 `.ai_sync_dek` 副本，改密码时无需旧密码即可解开 payload。
//! - **恢复密钥** = DEK 的 hex，供「重装设备 + 密码已改」场景手工录入。
//! - **AEAD**：AES-256-GCM，AAD = `"kairos-ai-settings"`（域隔离，防跨用途替换）。
//! - **KDF 迭代次数随文件存储**（`kdf_iterations` 字段）：未来可安全提高默认值，旧包仍可解。
//! - **威胁模型**：与 `ai/crypto.rs` 同级——服务器只看到密文 + 盐，无密码解不出 KEK 进而
//!   解不出 DEK；不防本地 root（DEK 文件与应用同处 app_data_dir，本地 root 可同时取到）。

use std::fs;
use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::db::models::AiConfig;

/// PBKDF2-HMAC-SHA256 迭代次数（OWASP 2023 建议值）。
const KDF_ITERATIONS: u32 = 210_000;
/// PBKDF2 盐长度（字节）。
const SALT_LEN: usize = 16;
/// GCM nonce 长度（12 字节，标准推荐）。
const NONCE_LEN: usize = 12;
/// DEK 长度（AES-256 = 32 字节）。
const DEK_LEN: usize = 32;
/// 加密包格式版本。
const FORMAT_VERSION: u32 = 1;

/// AAD 域隔离常量：绑定「这是 AI 设置加密包」，防止密钥被其他用途复用/替换。
const AAD: &[u8] = b"kairos-ai-settings";

/// 本地 DEK 副本文件名（仿 `.ai_encryption_key`，Unix 下 0600）。
const DEK_FILE_NAME: &str = ".ai_sync_dek";

/// 远端加密包文件（`kairos-ai-settings.enc`）的 JSON 结构。所有二进制字段 hex 编码。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettingsBlob {
    pub format_version: u32,
    /// KDF 迭代次数（随文件存储，允许未来安全提高默认值）。
    pub kdf_iterations: u32,
    /// PBKDF2 盐（hex）。
    pub kdf_salt_hex: String,
    /// 包裹 DEK 的 GCM nonce（hex）。
    pub dek_wrap_nonce_hex: String,
    /// AES-256-GCM(KEK, DEK) 的密文 + tag（hex）。
    pub wrapped_dek_hex: String,
    /// 加密 payload 的 GCM nonce（hex）。
    pub payload_nonce_hex: String,
    /// AES-256-GCM(DEK, payload_json) 的密文 + tag（hex）。
    pub payload_cipher_hex: String,
}

/// 加密包内的 payload 明文。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncAiPayload {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    /// API 密钥明文（仅存在于加密包内；落库时再用本机 key 文件加密）。
    pub api_key: String,
    /// UTC ISO 8601 更新时间，LWW 合并依据。
    pub updated_at: String,
}

/// `decrypt_blob` 的返回结果。
#[derive(Debug, Clone)]
pub struct DecryptedBlob {
    pub payload: SyncAiPayload,
    /// 本次解密得到的有效 DEK（密码未变时来自解包，已变时来自本地副本）。
    pub dek: [u8; DEK_LEN],
    /// true = 远端 DEK 是用旧密码包裹的（密码已改），上传前需用当前密码重裹 DEK。
    pub needs_rewrap: bool,
}

/// LWW 合并结果。AI 设置为单例（id=1），整包胜负，不做字段级合并。
#[derive(Debug, Clone)]
pub enum MergeOutcome {
    /// 远端更新（或本地无数据），采纳远端。
    RemoteWins(SyncAiPayload),
    /// 本地更新或平局，保留本地。
    LocalWins,
}

// ─── 加解密 ────────────────────────────────────────────────────────────────

/// 由 WebDAV 密码 + 盐派生 KEK（AES-256 密钥）。
fn derive_kek_with_iterations(password: &str, salt: &[u8], iterations: u32) -> [u8; DEK_LEN] {
    let mut kek = [0u8; DEK_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut kek);
    kek
}

/// 加密 AI 设置 payload 为远端加密包 JSON。DEK 由调用方持有（本地副本或新生成）。
pub fn encrypt_to_blob(
    password: &str,
    payload: &SyncAiPayload,
    dek: [u8; DEK_LEN],
) -> Result<String, String> {
    encrypt_to_blob_with_iterations(password, KDF_ITERATIONS, payload, dek)
}

/// 指定迭代次数的加密入口（单测用低迭代值加速；生产走默认迭代）。
fn encrypt_to_blob_with_iterations(
    password: &str,
    iterations: u32,
    payload: &SyncAiPayload,
    dek: [u8; DEK_LEN],
) -> Result<String, String> {
    let salt = random_bytes(SALT_LEN);
    let kek = derive_kek_with_iterations(password, &salt, iterations);

    let dek_wrap_nonce = random_bytes(NONCE_LEN);
    let wrapped_dek = seal(&kek, &dek_wrap_nonce, &dek, AAD)?;

    let payload_json =
        serde_json::to_string(payload).map_err(|e| format!("AI 设置 payload 序列化失败: {e}"))?;
    let payload_nonce = random_bytes(NONCE_LEN);
    let payload_cipher = seal(&dek, &payload_nonce, payload_json.as_bytes(), AAD)?;

    let blob = AiSettingsBlob {
        format_version: FORMAT_VERSION,
        kdf_iterations: iterations,
        kdf_salt_hex: hex::encode(salt),
        dek_wrap_nonce_hex: hex::encode(dek_wrap_nonce),
        wrapped_dek_hex: hex::encode(wrapped_dek),
        payload_nonce_hex: hex::encode(payload_nonce),
        payload_cipher_hex: hex::encode(payload_cipher),
    };

    serde_json::to_string(&blob).map_err(|e| format!("AI 设置加密包序列化失败: {e}"))
}

/// 解密远端加密包。迭代次数从包内读取，兼容未来 KDF 参数变更。
///
/// 流程：当前密码派生 KEK 尝试解包裹 → 成功则拿到 DEK；失败（密码已改）回退到本地 DEK
/// 副本（此时 `needs_rewrap=true`）；两者都失败返回可恢复错误，提示粘贴恢复密钥。
pub fn decrypt_blob(
    password: &str,
    blob_json: &str,
    local_dek: Option<[u8; DEK_LEN]>,
) -> Result<DecryptedBlob, String> {
    let blob: AiSettingsBlob = serde_json::from_str(blob_json)
        .map_err(|e| format!("AI 设置加密包解析失败: {e}"))?;
    if blob.format_version != FORMAT_VERSION {
        return Err(format!(
            "AI 设置加密包版本不受支持: {}",
            blob.format_version
        ));
    }

    let salt = hex::decode(&blob.kdf_salt_hex).map_err(|_| "加密包盐解码失败".to_string())?;
    let kek = derive_kek_with_iterations(password, &salt, blob.kdf_iterations);
    let wrap_nonce = decode_nonce(&blob.dek_wrap_nonce_hex)?;
    let wrapped_dek = hex::decode(&blob.wrapped_dek_hex).map_err(|_| "包裹 DEK 解码失败".to_string())?;

    // 当前密码解包裹成功 → DEK 直接从包内获得，无需本地副本。
    let (dek, needs_rewrap) = match open(&kek, &wrap_nonce, &wrapped_dek, AAD) {
        Ok(dek_bytes) => {
            let dek: [u8; DEK_LEN] = dek_bytes
                .try_into()
                .map_err(|_| "加密包 DEK 长度异常".to_string())?;
            (dek, false)
        }
        Err(_) => match local_dek {
            Some(dek) => (dek, true),
            None => {
                return Err(
                    "无法解密 AI 设置：WebDAV 密码不匹配，且本机没有可用的恢复密钥。\
                     请在 AI 设置页粘贴恢复密钥后重试"
                        .to_string(),
                )
            }
        },
    };

    let payload_nonce = decode_nonce(&blob.payload_nonce_hex)?;
    let payload_cipher =
        hex::decode(&blob.payload_cipher_hex).map_err(|_| "payload 密文解码失败".to_string())?;
    let payload_json = open(&dek, &payload_nonce, &payload_cipher, AAD)
        .map_err(|_| "AI 设置加密包校验失败：密文可能已损坏或恢复密钥不正确".to_string())?;
    let payload: SyncAiPayload = serde_json::from_slice(&payload_json)
        .map_err(|e| format!("AI 设置 payload 解析失败: {e}"))?;

    Ok(DecryptedBlob {
        payload,
        dek,
        needs_rewrap,
    })
}

// ─── 本地 DEK 副本管理 ─────────────────────────────────────────────────────

fn dek_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join(DEK_FILE_NAME)
}

/// 读取本地 DEK 副本；不存在返回 `None`（首次使用或已被删除）。
pub fn load_dek(app_data_dir: &Path) -> Result<Option<[u8; DEK_LEN]>, String> {
    let path = dek_path(app_data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|e| format!("读取 AI 同步 DEK 失败（{}）: {e}", path.display()))?;
    let dek: [u8; DEK_LEN] = bytes
        .try_into()
        .map_err(|_| "AI 同步 DEK 文件长度异常，请删除该文件后重新同步".to_string())?;
    Ok(Some(dek))
}

/// 读取本地 DEK；不存在则生成并落盘（首次启用同步时调用）。
pub fn load_or_create_dek(app_data_dir: &Path) -> Result<[u8; DEK_LEN], String> {
    if let Some(dek) = load_dek(app_data_dir)? {
        return Ok(dek);
    }
    let dek = random_bytes(DEK_LEN)
        .try_into()
        .expect("DEK_LEN 与 random_bytes 输出长度一致");
    save_dek(app_data_dir, &dek)?;
    Ok(dek)
}

/// 写入本地 DEK 副本（Unix 下 0600，仿 `.ai_encryption_key`）。
pub fn save_dek(app_data_dir: &Path, dek: &[u8; DEK_LEN]) -> Result<(), String> {
    let path = dek_path(app_data_dir);
    fs::write(&path, dek).map_err(|e| format!("写入 AI 同步 DEK 失败（{}）: {e}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("设置 AI 同步 DEK 权限失败: {e}"))?;
    }

    Ok(())
}

// ─── 恢复密钥（DEK 的 hex 表示） ───────────────────────────────────────────

/// DEK → 恢复密钥 hex。
pub fn recovery_key_hex(dek: &[u8; DEK_LEN]) -> String {
    hex::encode(dek)
}

/// 恢复密钥 hex → DEK。校验长度（32 字节 = 64 位 hex）。
pub fn recovery_key_from_hex(input: &str) -> Result<[u8; DEK_LEN], String> {
    let bytes = hex::decode(input.trim())
        .map_err(|_| "恢复密钥格式无效：应为 64 位十六进制字符".to_string())?;
    bytes
        .try_into()
        .map_err(|_| "恢复密钥长度无效：应为 32 字节".to_string())
}

// ─── payload 构建与 LWW 合并 ───────────────────────────────────────────────

/// 从本地 AI 配置构建 payload。api_key 为解密后的明文（仅进入加密包，不过桥前端）。
pub fn payload_from_config(config: &AiConfig, api_key_plaintext: &str) -> SyncAiPayload {
    SyncAiPayload {
        enabled: config.enabled,
        base_url: config.base_url.clone(),
        model: config.model.clone(),
        api_key: api_key_plaintext.to_string(),
        updated_at: config.updated_at.clone(),
    }
}

/// LWW 合并：远端 `updated_at` 更新则整包采纳远端；否则保留本地。平局保本地（收敛）。
pub fn merge_payload(local: Option<&SyncAiPayload>, remote: &SyncAiPayload) -> MergeOutcome {
    match local {
        None => MergeOutcome::RemoteWins(remote.clone()),
        Some(local) => {
            if remote.updated_at > local.updated_at {
                MergeOutcome::RemoteWins(remote.clone())
            } else {
                MergeOutcome::LocalWins
            }
        }
    }
}

// ─── 底层工具 ──────────────────────────────────────────────────────────────

/// AES-256-GCM 加密（带 AAD）。
fn seal(key: &[u8; DEK_LEN], nonce: &[u8], msg: &[u8], aad: &[u8]) -> Result<Vec<u8>, String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|_| "初始化 AES-256-GCM 失败".to_string())?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .encrypt(nonce, Payload { msg, aad })
        .map_err(|_| "AES-256-GCM 加密失败".to_string())
}

/// AES-256-GCM 解密（带 AAD，GCM tag 校验失败即返回 Err）。
fn open(key: &[u8; DEK_LEN], nonce: &[u8], ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|_| "初始化 AES-256-GCM 失败".to_string())?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, Payload {
            msg: ciphertext,
            aad,
        })
        .map_err(|_| "AES-256-GCM 解密失败".to_string())
}

/// 从 hex 解码并校验 GCM nonce 长度。
fn decode_nonce(hex_str: &str) -> Result<Vec<u8>, String> {
    let bytes =
        hex::decode(hex_str).map_err(|_| "加密包 nonce 解码失败".to_string())?;
    if bytes.len() != NONCE_LEN {
        return Err("加密包 nonce 长度无效".to_string());
    }
    Ok(bytes)
}

/// CSPRNG 随机字节。复用 uuid v4（底层 getrandom），避免引入新依赖。
fn random_bytes(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        out.extend_from_slice(&uuid::Uuid::new_v4().into_bytes());
    }
    out.truncate(len);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PASSWORD: &str = "webdav-secret";
    const TEST_ITERATIONS: u32 = 1_000;

    fn sample_payload(updated_at: &str) -> SyncAiPayload {
        SyncAiPayload {
            enabled: true,
            base_url: "https://api.deepseek.com".to_string(),
            model: "deepseek-v4-flash".to_string(),
            api_key: "sk-test-123".to_string(),
            updated_at: updated_at.to_string(),
        }
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let payload = sample_payload("2026-08-01T00:00:00Z");
        let dek = random_bytes(DEK_LEN).try_into().unwrap();

        let blob = encrypt_to_blob_with_iterations(TEST_PASSWORD, TEST_ITERATIONS, &payload, dek)
            .expect("encrypt");
        let decrypted = decrypt_blob(TEST_PASSWORD, &blob, None).expect("decrypt");

        assert!(!decrypted.needs_rewrap);
        assert_eq!(decrypted.dek, dek);
        assert_eq!(decrypted.payload.enabled, payload.enabled);
        assert_eq!(decrypted.payload.base_url, payload.base_url);
        assert_eq!(decrypted.payload.model, payload.model);
        assert_eq!(decrypted.payload.api_key, payload.api_key);
        assert_eq!(decrypted.payload.updated_at, payload.updated_at);
    }

    #[test]
    fn test_blob_is_not_plaintext() {
        let payload = sample_payload("2026-08-01T00:00:00Z");
        let dek = random_bytes(DEK_LEN).try_into().unwrap();

        let blob = encrypt_to_blob_with_iterations(TEST_PASSWORD, TEST_ITERATIONS, &payload, dek)
            .expect("encrypt");
        // 密钥明文、base_url、模型名都不应出现在密文包里。
        assert!(!blob.contains("sk-test-123"));
        assert!(!blob.contains("api.deepseek.com"));
        assert!(!blob.contains("deepseek-v4-flash"));
    }

    #[test]
    fn test_decrypt_wrong_password_without_local_dek_fails() {
        let payload = sample_payload("2026-08-01T00:00:00Z");
        let dek = random_bytes(DEK_LEN).try_into().unwrap();
        let blob = encrypt_to_blob_with_iterations(TEST_PASSWORD, TEST_ITERATIONS, &payload, dek)
            .expect("encrypt");

        let err = decrypt_blob("wrong-password", &blob, None).expect_err("应解密失败");
        assert!(err.contains("恢复密钥"), "错误信息应提示恢复密钥兜底");
    }

    #[test]
    fn test_decrypt_wrong_password_with_local_dek_falls_back() {
        let payload = sample_payload("2026-08-01T00:00:00Z");
        let dek = random_bytes(DEK_LEN).try_into().unwrap();
        let blob = encrypt_to_blob_with_iterations(TEST_PASSWORD, TEST_ITERATIONS, &payload, dek)
            .expect("encrypt");

        // 密码已改（新密码解不开包裹），但有本地 DEK 副本 → 应回退成功并标记需要重裹。
        let decrypted = decrypt_blob("new-password", &blob, Some(dek)).expect("fallback decrypt");
        assert!(decrypted.needs_rewrap);
        assert_eq!(decrypted.payload.api_key, "sk-test-123");
    }

    #[test]
    fn test_decrypt_wrong_password_and_wrong_local_dek_fails() {
        let payload = sample_payload("2026-08-01T00:00:00Z");
        let dek = random_bytes(DEK_LEN).try_into().unwrap();
        let wrong_dek = random_bytes(DEK_LEN).try_into().unwrap();
        let blob = encrypt_to_blob_with_iterations(TEST_PASSWORD, TEST_ITERATIONS, &payload, dek)
            .expect("encrypt");

        // 密码错 + 恢复密钥错 → GCM 校验失败。
        let err = decrypt_blob("new-password", &blob, Some(wrong_dek))
            .expect_err("错误恢复密钥应解密失败");
        assert!(err.contains("校验失败"), "错误信息应指出校验失败");
    }

    #[test]
    fn test_tampered_payload_fails_auth() {
        let payload = sample_payload("2026-08-01T00:00:00Z");
        let dek = random_bytes(DEK_LEN).try_into().unwrap();
        let blob = encrypt_to_blob_with_iterations(TEST_PASSWORD, TEST_ITERATIONS, &payload, dek)
            .expect("encrypt");

        let mut parsed: AiSettingsBlob = serde_json::from_str(&blob).expect("parse");
        // 翻转 payload 密文的一个 hex 字符。
        let flip = |hex_str: &str| {
            let mut bytes = hex::decode(hex_str).expect("hex");
            bytes[0] ^= 0x01;
            hex::encode(bytes)
        };
        parsed.payload_cipher_hex = flip(&parsed.payload_cipher_hex);
        let tampered = serde_json::to_string(&parsed).expect("serialize");

        assert!(decrypt_blob(TEST_PASSWORD, &tampered, Some(dek)).is_err());
    }

    #[test]
    fn test_merge_remote_newer_wins() {
        let local = sample_payload("2026-08-01T00:00:00Z");
        let mut remote = sample_payload("2026-08-02T00:00:00Z");
        remote.base_url = "https://new.example.com".to_string();

        match merge_payload(Some(&local), &remote) {
            MergeOutcome::RemoteWins(p) => {
                assert_eq!(p.base_url, "https://new.example.com");
            }
            MergeOutcome::LocalWins => panic!("远端更新应胜出"),
        }
    }

    #[test]
    fn test_merge_local_newer_or_tie_keeps_local() {
        // 本地较新。
        let local = sample_payload("2026-08-02T00:00:00Z");
        let remote = sample_payload("2026-08-01T00:00:00Z");
        assert!(matches!(
            merge_payload(Some(&local), &remote),
            MergeOutcome::LocalWins
        ));

        // 平局（同 updated_at）保本地，保证跨设备收敛。
        let tie_remote = sample_payload("2026-08-02T00:00:00Z");
        assert!(matches!(
            merge_payload(Some(&local), &tie_remote),
            MergeOutcome::LocalWins
        ));
    }

    #[test]
    fn test_merge_no_local_takes_remote() {
        let remote = sample_payload("2026-08-01T00:00:00Z");
        assert!(matches!(
            merge_payload(None, &remote),
            MergeOutcome::RemoteWins(_)
        ));
    }

    #[test]
    fn test_recovery_key_roundtrip() {
        let dek = random_bytes(DEK_LEN).try_into().unwrap();
        let hex_key = recovery_key_hex(&dek);
        assert_eq!(hex_key.len(), 64);
        assert_eq!(recovery_key_from_hex(&hex_key).expect("parse"), dek);
        assert!(recovery_key_from_hex("zzzz").is_err());
        assert!(recovery_key_from_hex("abcd").is_err());
    }

    #[test]
    fn test_dek_file_roundtrip() {
        let dir = test_dir();
        let dek = random_bytes(DEK_LEN).try_into().unwrap();

        assert!(load_dek(&dir).expect("missing").is_none());
        save_dek(&dir, &dek).expect("save");
        assert_eq!(load_dek(&dir).expect("load").unwrap(), dek);
        assert_eq!(load_or_create_dek(&dir).expect("load-or-create"), dek);
    }

    fn test_dir() -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let base = std::env::temp_dir().join(format!(
            "kairos-ai-settings-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&base).expect("Failed to create temp dir");
        let dir = base.join(COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst).to_string());
        fs::create_dir_all(&dir).expect("Failed to create dir");
        dir
    }
}
