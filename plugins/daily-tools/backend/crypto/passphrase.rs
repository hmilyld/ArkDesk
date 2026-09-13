//! OpenSSL `enc` 口令模式：`Salted__` 头与密钥派生。
//!
//! - 默认 `EVP_BytesToKey(MD5)`（兼容 `openssl enc`）；
//! - `kdf = "pbkdf2"` 时使用 PBKDF2-HMAC-SHA256（兼容 `openssl enc -pbkdf2`）。

use digest::Digest;
use zeroize::Zeroizing;

use crate::error::{code, AppError};

use super::symmetric::Alg;

/// `openssl enc` 带盐输出的魔数头。
pub const SALT_MAGIC: &[u8; 8] = b"Salted__";

/// 敏感字节（自动清零）。
pub type Secret = Zeroizing<Vec<u8>>;
/// 派生得到的 key 与 iv。
pub type DerivedKeyIv = (Secret, Secret);

/// 校验盐长度（OpenSSL 固定 8 字节）。
pub fn ensure_salt(salt: &[u8]) -> Result<Vec<u8>, AppError> {
    if salt.len() != 8 {
        return Err(AppError::invalid_input("salt 应为 8 字节"));
    }
    Ok(salt.to_vec())
}

/// 生成随机 8 字节盐。
pub fn random_salt() -> Result<Vec<u8>, AppError> {
    let mut salt = vec![0u8; 8];
    getrandom::fill(&mut salt)
        .map_err(|e| AppError::custom(code::UNKNOWN, format!("随机数失败: {e}")))?;
    Ok(salt)
}

/// 从 `Salted__ || salt || 密文` 中拆出盐与密文体。
pub fn unwrap_salted(data: &[u8]) -> Result<(&[u8], &[u8]), AppError> {
    if data.len() < 16 || &data[..8] != SALT_MAGIC {
        return Err(AppError::invalid_input("口令模式密文缺少 Salted__ 头"));
    }
    Ok((&data[8..16], &data[16..]))
}

/// EVP_BytesToKey(MD5) 派生 `key || iv`。
fn evp_bytes_to_key(password: &[u8], salt: &[u8], key_len: usize, iv_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(key_len + iv_len);
    let mut prev: Vec<u8> = Vec::new();
    while out.len() < key_len + iv_len {
        let mut hasher = md5::Md5::new();
        if !prev.is_empty() {
            hasher.update(&prev);
        }
        hasher.update(password);
        hasher.update(salt);
        let digest = hasher.finalize().to_vec();
        out.extend_from_slice(&digest);
        prev = digest;
    }
    out.truncate(key_len + iv_len);
    out
}

/// 口令派生 key 与 iv。`mode = "ecb"` 时 iv 为空。
pub fn derive(
    alg: Alg,
    mode: &str,
    password: &[u8],
    kdf: &str,
    rounds: u32,
    salt: &[u8],
) -> Result<DerivedKeyIv, AppError> {
    let key_len = alg.key_len();
    let iv_len = if mode == "ecb" { 0 } else { alg.block_size() };

    let material = if kdf.eq_ignore_ascii_case("pbkdf2") {
        let rounds = if rounds == 0 { 10000 } else { rounds };
        let mut buf = vec![0u8; key_len + iv_len];
        pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password, salt, rounds, &mut buf);
        Zeroizing::new(buf)
    } else {
        Zeroizing::new(evp_bytes_to_key(password, salt, key_len, iv_len))
    };

    let key = Zeroizing::new(material[..key_len].to_vec());
    let iv = Zeroizing::new(material[key_len..].to_vec());
    Ok((key, iv))
}
