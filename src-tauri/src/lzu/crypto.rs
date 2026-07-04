//! AES-CBC-128 hex 加解密，兼容 FasterLZU 实现。
//!
//! # 协议参数
//!
//! - 算法：AES-CBC-128
//! - Key：16 字节 UTF-8
//! - IV：与 Key 相同（兼容 LZU 后端）
//! - Padding：`\0` 补足到 16 字节倍数（非 PKCS7）
//! - 输出：小写 hex 字符串
//!
//! # 安全注意
//!
//! IV 与 Key 相同是弱配置，但必须与 LZU 后端兼容。不要在其他场景复用此实现。

use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hex;
use std::path::PathBuf;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

const AES_KEY_ENV: &str = "KAIROS_LZU_AES_KEY";

/// 使用 `\0` 将数据填充到 16 字节倍数。
fn zero_pad(data: &[u8]) -> Vec<u8> {
    let block_size = 16;
    let padded_len = data.len() + (block_size - (data.len() % block_size)) % block_size;
    let mut padded = vec![0u8; padded_len];
    padded[..data.len()].copy_from_slice(data);
    padded
}

fn configured_key() -> Result<[u8; 16], crate::lzu::error::LzuError> {
    let value = read_configured_key()
        .map_err(|_| crate::lzu::error::LzuError::Config(format!("缺少 {AES_KEY_ENV} 环境变量")))?;
    key_from_utf8(&value)
}

fn read_configured_key() -> Result<String, std::env::VarError> {
    match std::env::var(AES_KEY_ENV) {
        Ok(value) => Ok(value),
        Err(err) => {
            load_local_env_file();
            std::env::var(AES_KEY_ENV).map_err(|_| err)
        }
    }
}

fn load_local_env_file() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env.local");
    if path.exists() {
        if let Err(err) = dotenvy::from_path(&path) {
            log::warn!("读取 src-tauri/.env.local 失败: {err}");
        }
    }
}

fn key_from_utf8(value: &str) -> Result<[u8; 16], crate::lzu::error::LzuError> {
    let bytes = value.as_bytes();
    if bytes.len() != 16 {
        return Err(crate::lzu::error::LzuError::Config(format!(
            "{AES_KEY_ENV} 必须是 16 字节 UTF-8 字符串，当前为 {} 字节",
            bytes.len()
        )));
    }

    let mut key = [0u8; 16];
    key.copy_from_slice(bytes);
    Ok(key)
}

/// AES-CBC-128 hex 加密。
///
/// 1. UTF-8 编码明文
/// 2. `\0` 填充到 16 字节倍数
/// 3. AES-CBC 加密（IV = Key）
/// 4. 输出小写 hex 字符串
pub fn encrypt(plaintext: &str) -> Result<String, crate::lzu::error::LzuError> {
    encrypt_with_key(plaintext, &configured_key()?)
}

fn encrypt_with_key(
    plaintext: &str,
    key: &[u8; 16],
) -> Result<String, crate::lzu::error::LzuError> {
    let padded = zero_pad(plaintext.as_bytes());
    let mut buf = vec![0u8; padded.len()];

    let cipher = Aes128CbcEnc::new(key.into(), key.into());
    let encrypted = cipher
        .encrypt_padded_b2b_mut::<NoPadding>(&padded, &mut buf)
        .map_err(|e| crate::lzu::error::LzuError::Crypto(format!("AES encrypt failed: {e}")))?;

    Ok(hex::encode(encrypted))
}

/// AES-CBC-128 hex 解密。
///
/// 1. hex 解码
/// 2. AES-CBC 解密（IV = Key）
/// 3. 去除尾部 `\0` 字节
/// 4. UTF-8 解码
pub fn decrypt(encrypted_hex: &str) -> Result<String, crate::lzu::error::LzuError> {
    decrypt_with_key(encrypted_hex, &configured_key()?)
}

fn decrypt_with_key(
    encrypted_hex: &str,
    key: &[u8; 16],
) -> Result<String, crate::lzu::error::LzuError> {
    let encrypted_bytes = hex::decode(encrypted_hex)
        .map_err(|e| crate::lzu::error::LzuError::Crypto(format!("hex decode failed: {e}")))?;

    let mut buf = vec![0u8; encrypted_bytes.len()];

    let cipher = Aes128CbcDec::new(key.into(), key.into());
    let decrypted = cipher
        .decrypt_padded_b2b_mut::<NoPadding>(&encrypted_bytes, &mut buf)
        .map_err(|e| crate::lzu::error::LzuError::Crypto(format!("AES decrypt failed: {e}")))?;

    // 去除尾部 \0 字节
    let valid_len = decrypted
        .iter()
        .rposition(|&b| b != 0)
        .map(|pos| pos + 1)
        .unwrap_or(0);

    let trimmed = &decrypted[..valid_len];
    let result = String::from_utf8(trimmed.to_vec())
        .map_err(|e| crate::lzu::error::LzuError::Crypto(format!("UTF-8 decode failed: {e}")))?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEY: &[u8; 16] = b"1234567890abcdef";

    fn assert_roundtrip(plaintext: &str) {
        let encrypted = encrypt_with_key(plaintext, TEST_KEY).unwrap();
        let decrypted = decrypt_with_key(&encrypted, TEST_KEY).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_key_from_utf8_requires_16_bytes() {
        assert!(key_from_utf8("1234567890abcdef").is_ok());
        assert!(key_from_utf8("short").is_err());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        assert_roundtrip(r#"{"app_os":2,"name":"testuser","pwd":"testpass"}"#);
    }

    #[test]
    fn test_encrypt_decrypt_empty_string() {
        assert_roundtrip("");
    }

    #[test]
    fn test_encrypt_decrypt_unicode() {
        assert_roundtrip("你好，世界！");
    }

    #[test]
    fn test_encrypt_decrypt_exact_block_size() {
        assert_roundtrip("1234567890123456");
    }

    #[test]
    fn test_encrypt_decrypt_exact_block_size_2() {
        assert_roundtrip("12345678901234561234567890123456");
    }

    #[test]
    fn test_decrypt_invalid_hex() {
        let result = decrypt_with_key("not-hex-string", TEST_KEY);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_invalid_ciphertext() {
        let result = decrypt_with_key("000102030405060708090a0b0c0d0e0f", TEST_KEY);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_output_is_lowercase_hex() {
        let plaintext = "hello";
        let encrypted = encrypt_with_key(plaintext, TEST_KEY).unwrap();
        // 验证只包含小写 hex 字符
        assert!(encrypted
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }
}
