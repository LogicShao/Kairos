//! API key AES-128-CBC 加解密与本地随机 key 文件管理。
//!
//! # 设计决策
//!
//! - **不复用** `lzu/crypto.rs`（其 IV=Key 弱配置，文件头注释明确禁止复用）。
//! - 密钥：16 随机字节写入 `app_data_dir/.ai_encryption_key`，首次启动生成；Unix 下 chmod 600。
//! - 加密：AES-128-CBC + **每次随机 16 字节 IV** + PKCS7 padding（复用 `aes`/`cbc`，零新依赖）。
//! - 存储格式：`hex(iv) || ":" || hex(cipher)`。
//! - 威胁模型：防护目标是**远端/WebDAV/静态单文件泄露**，不防本地 root（key 与 db 同处
//!   app_data_dir 时 root 可同时取到）。

use std::fs;
use std::path::Path;

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

/// key 文件名（放在 app_data_dir，随用户数据目录而非仓库）。
const KEY_FILE_NAME: &str = ".ai_encryption_key";

fn key_path(app_data_dir: &Path) -> std::path::PathBuf {
    app_data_dir.join(KEY_FILE_NAME)
}

/// 首次启动生成 16 随机字节 key 文件并返回；已存在则读取。Unix 下 chmod 600。
pub fn ensure_key_file(app_data_dir: &Path) -> Result<[u8; 16], String> {
    let path = key_path(app_data_dir);
    if path.exists() {
        return load_key(app_data_dir);
    }

    // uuid v4 底层走 CSPRNG（getrandom），16 字节熵足以作 AES key。
    let key = uuid::Uuid::new_v4().into_bytes();
    fs::write(&path, key)
        .map_err(|e| format!("写入 AI key 文件失败（{}）: {e}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("设置 AI key 文件权限失败: {e}"))?;
    }

    Ok(key)
}

/// 读取已存在的 key 文件；缺失视为未初始化，返回错误提示重新配置。
pub fn load_key(app_data_dir: &Path) -> Result<[u8; 16], String> {
    let path = key_path(app_data_dir);
    let bytes =
        fs::read(&path).map_err(|e| format!("读取 AI key 文件失败（{}）: {e}", path.display()))?;
    let arr: [u8; 16] = bytes.try_into().map_err(|_| {
        format!(
            "AI key 文件长度异常（{}），请删除后重新配置",
            path.display()
        )
    })?;
    Ok(arr)
}

/// AES-128-CBC + 随机 IV + PKCS7 加密。输出 `hex(iv):hex(cipher)`。
pub fn encrypt_api_key(plaintext: &str, key: &[u8; 16]) -> Result<String, String> {
    let iv = uuid::Uuid::new_v4().into_bytes();
    let cipher = Aes128CbcEnc::new_from_slices(key, &iv)
        .map_err(|e| format!("初始化 AES 加密器失败: {e}"))?;
    let mut buf = vec![0u8; plaintext.len() + 16];
    let ciphertext = cipher
        .encrypt_padded_b2b_mut::<Pkcs7>(plaintext.as_bytes(), &mut buf)
        .map_err(|e| format!("AES 加密失败: {e}"))?;
    Ok(format!("{}:{}", hex::encode(iv), hex::encode(ciphertext)))
}

/// 解密 `hex(iv):hex(cipher)` 密文。
pub fn decrypt_api_key(encrypted: &str, key: &[u8; 16]) -> Result<String, String> {
    let (iv_hex, cipher_hex) = encrypted
        .split_once(':')
        .ok_or_else(|| "AI key 密文格式无效（缺少 ':' 分隔）".to_string())?;
    let iv = hex::decode(iv_hex).map_err(|e| format!("AI key IV 解码失败: {e}"))?;
    let ciphertext = hex::decode(cipher_hex).map_err(|e| format!("AI key 密文解码失败: {e}"))?;

    let cipher = Aes128CbcDec::new_from_slices(key, &iv)
        .map_err(|e| format!("初始化 AES 解密器失败: {e}"))?;
    let mut buf = ciphertext.clone();
    let plaintext = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| "AI key 解密失败（key 文件或密文可能已损坏）".to_string())?;

    String::from_utf8(plaintext.to_vec()).map_err(|e| format!("AI key 明文解码失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 每次调用返回一个独立临时子目录；base 带进程 ID 隔离多次运行残留。
    fn test_dir() -> std::path::PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let base =
            std::env::temp_dir().join(format!("kairos-ai-crypto-test-{}", std::process::id()));
        fs::create_dir_all(&base).expect("Failed to create temp dir");
        let dir = base.join(COUNTER.fetch_add(1, Ordering::SeqCst).to_string());
        fs::create_dir_all(&dir).expect("Failed to create test dir");
        dir
    }

    #[test]
    fn test_ensure_key_file_creates_and_reloads() {
        let dir = test_dir();
        fs::create_dir_all(&dir).expect("Failed to create dir");

        let key1 = ensure_key_file(&dir).expect("Failed to ensure key");
        let key2 = ensure_key_file(&dir).expect("Failed to ensure key again");
        assert_eq!(key1, key2, "再次调用应复用同一 key 文件");
        assert!(dir.join(KEY_FILE_NAME).exists());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let dir = test_dir();
        fs::create_dir_all(&dir).expect("Failed to create dir");
        let key = ensure_key_file(&dir).expect("Failed to ensure key");

        let plaintext = "sk-1234567890abcdef";
        let encrypted = encrypt_api_key(plaintext, &key).expect("Failed to encrypt");
        assert!(encrypted.contains(':'), "应包含 iv:cipher 分隔");
        let decrypted = decrypt_api_key(&encrypted, &key).expect("Failed to decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_random_iv_produces_distinct_ciphertext() {
        let dir = test_dir();
        fs::create_dir_all(&dir).expect("Failed to create dir");
        let key = ensure_key_file(&dir).expect("Failed to ensure key");

        let e1 = encrypt_api_key("same-plain", &key).expect("Failed to encrypt 1");
        let e2 = encrypt_api_key("same-plain", &key).expect("Failed to encrypt 2");
        assert_ne!(e1, e2, "随机 IV 下同一明文两次密文应不同");
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let dir = test_dir();
        fs::create_dir_all(&dir).expect("Failed to create dir");
        let key = ensure_key_file(&dir).expect("Failed to ensure key");
        let other_key = uuid::Uuid::new_v4().into_bytes();

        let encrypted = encrypt_api_key("secret", &key).expect("Failed to encrypt");
        assert!(decrypt_api_key(&encrypted, &other_key).is_err());
    }

    #[test]
    fn test_decrypt_malformed_fails() {
        let dir = test_dir();
        fs::create_dir_all(&dir).expect("Failed to create dir");
        let key = ensure_key_file(&dir).expect("Failed to ensure key");

        assert!(decrypt_api_key("not-a-valid-format", &key).is_err());
        assert!(decrypt_api_key("zzzz:zzzz", &key).is_err());
    }

    #[test]
    fn test_load_key_missing_fails() {
        let dir = test_dir();
        fs::create_dir_all(&dir).expect("Failed to create dir");
        assert!(load_key(&dir).is_err(), "无 key 文件时应报错");
    }
}
