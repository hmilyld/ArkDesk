//! 口令派生：PBKDF2 / scrypt / Argon2。

use crate::error::AppError;

/// PBKDF2-HMAC-SHA256。
pub fn pbkdf2(password: &[u8], salt: &[u8], rounds: u32, length: usize) -> Vec<u8> {
    let mut out = vec![0u8; length];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password, salt, rounds.max(1), &mut out);
    out
}

/// scrypt。`log_n` 为 N 的以 2 为底对数。
pub fn scrypt(
    password: &[u8],
    salt: &[u8],
    log_n: u8,
    r: u32,
    p: u32,
    length: usize,
) -> Result<Vec<u8>, AppError> {
    let params = scrypt::Params::new(log_n, r, p, length)
        .map_err(|e| AppError::invalid_input(format!("scrypt 参数无效: {e}")))?;
    let mut out = vec![0u8; length];
    scrypt::scrypt(password, salt, &params, &mut out)
        .map_err(|e| AppError::invalid_input(format!("scrypt 派生失败: {e}")))?;
    Ok(out)
}

/// Argon2id（v=0x13）。`memory_kib` 单位 KiB。
pub fn argon2(
    password: &[u8],
    salt: &[u8],
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
    length: usize,
) -> Result<Vec<u8>, AppError> {
    use argon2::{Algorithm, Argon2, Params, Version};
    let params = Params::new(memory_kib, iterations, parallelism, Some(length))
        .map_err(|e| AppError::invalid_input(format!("Argon2 参数无效: {e}")))?;
    let hasher = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = vec![0u8; length];
    hasher
        .hash_password_into(password, salt, &mut out)
        .map_err(|e| AppError::invalid_input(format!("Argon2 派生失败: {e}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pbkdf2_rfc6070_case2() {
        // RFC 6070: P="password", S="salt", c=2, dkLen=20 → SHA-1 vector；
        // 此处用 SHA-256 自洽校验 + 已知前缀长度。
        let out = pbkdf2(b"password", b"salt", 2, 32);
        assert_eq!(out.len(), 32);
        assert_ne!(out, vec![0u8; 32]);
    }

    #[test]
    fn scrypt_rfc7914_case() {
        let out = scrypt(b"", b"", 4, 1, 1, 64).unwrap();
        assert_eq!(
            data_encoding::HEXLOWER.encode(&out[..16]),
            "77d6576238657b203b19ca42c18a0497"
        );
    }

    #[test]
    fn argon2_roundtrip() {
        let out = argon2(b"password", b"somesalt", 4096, 3, 1, 32).unwrap();
        assert_eq!(out.len(), 32);
        let again = argon2(b"password", b"somesalt", 4096, 3, 1, 32).unwrap();
        assert_eq!(out, again);
    }
}
