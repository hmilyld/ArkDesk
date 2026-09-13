//! 哈希 / HMAC 实现。文本与文件共用同一套 `Hasher` 以支持流式更新。

use crate::error::AppError;
use digest::Digest;

use md5::Md5;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};
use sm3::Sm3;

/// 支持增量更新的哈希器（含 CRC32）。
pub enum Hasher {
    Md5(Md5),
    Sha1(Sha1),
    Sha224(Sha224),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
    Sha3_224(Sha3_224),
    Sha3_256(Sha3_256),
    Sha3_384(Sha3_384),
    Sha3_512(Sha3_512),
    Sm3(Sm3),
    Crc32(crc32fast::Hasher),
}

impl Hasher {
    pub fn new(algorithm: &str) -> Result<Self, AppError> {
        Ok(match super::normalize(algorithm).as_str() {
            "md5" => Self::Md5(Md5::new()),
            "sha1" => Self::Sha1(Sha1::new()),
            "sha224" => Self::Sha224(Sha224::new()),
            "sha256" => Self::Sha256(Sha256::new()),
            "sha384" => Self::Sha384(Sha384::new()),
            "sha512" => Self::Sha512(Sha512::new()),
            "sha3224" => Self::Sha3_224(Sha3_224::new()),
            "sha3" | "sha3256" => Self::Sha3_256(Sha3_256::new()),
            "sha3384" => Self::Sha3_384(Sha3_384::new()),
            "sha3512" => Self::Sha3_512(Sha3_512::new()),
            "sm3" => Self::Sm3(Sm3::new()),
            "crc32" => Self::Crc32(crc32fast::Hasher::new()),
            other => return Err(AppError::invalid_input(format!("不支持的哈希算法: {other}"))),
        })
    }

    pub fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Md5(h) => h.update(bytes),
            Self::Sha1(h) => h.update(bytes),
            Self::Sha224(h) => h.update(bytes),
            Self::Sha256(h) => h.update(bytes),
            Self::Sha384(h) => h.update(bytes),
            Self::Sha512(h) => h.update(bytes),
            Self::Sha3_224(h) => h.update(bytes),
            Self::Sha3_256(h) => h.update(bytes),
            Self::Sha3_384(h) => h.update(bytes),
            Self::Sha3_512(h) => h.update(bytes),
            Self::Sm3(h) => h.update(bytes),
            Self::Crc32(h) => h.update(bytes),
        }
    }

    pub fn finalize(self) -> Vec<u8> {
        match self {
            Self::Md5(h) => h.finalize().to_vec(),
            Self::Sha1(h) => h.finalize().to_vec(),
            Self::Sha224(h) => h.finalize().to_vec(),
            Self::Sha256(h) => h.finalize().to_vec(),
            Self::Sha384(h) => h.finalize().to_vec(),
            Self::Sha512(h) => h.finalize().to_vec(),
            Self::Sha3_224(h) => h.finalize().to_vec(),
            Self::Sha3_256(h) => h.finalize().to_vec(),
            Self::Sha3_384(h) => h.finalize().to_vec(),
            Self::Sha3_512(h) => h.finalize().to_vec(),
            Self::Sm3(h) => h.finalize().to_vec(),
            Self::Crc32(h) => h.finalize().to_be_bytes().to_vec(),
        }
    }
}

/// 一次性哈希。
pub fn digest(algorithm: &str, bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    let mut hasher = Hasher::new(algorithm)?;
    hasher.update(bytes);
    Ok(hasher.finalize())
}

/// HMAC（CRC32 不属于 HMAC 范畴）。
pub fn hmac(algorithm: &str, key: &[u8], bytes: &[u8]) -> Result<Vec<u8>, AppError> {
    use hmac::{Hmac, Mac};

    macro_rules! run {
        ($t:ty) => {{
            let mut mac = <Hmac<$t>>::new_from_slice(key)
                .map_err(|_| AppError::invalid_input("HMAC 密钥无效"))?;
            mac.update(bytes);
            Ok(mac.finalize().into_bytes().to_vec())
        }};
    }

    match super::normalize(algorithm).as_str() {
        "md5" => run!(Md5),
        "sha1" => run!(Sha1),
        "sha224" => run!(Sha224),
        "sha256" => run!(Sha256),
        "sha384" => run!(Sha384),
        "sha512" => run!(Sha512),
        "sha3224" => run!(Sha3_224),
        "sha3" | "sha3256" => run!(Sha3_256),
        "sha3384" => run!(Sha3_384),
        "sha3512" => run!(Sha3_512),
        "sm3" => run!(Sm3),
        other => Err(AppError::invalid_input(format!(
            "HMAC 不支持该算法: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        data_encoding::HEXLOWER.encode(bytes)
    }

    #[test]
    fn digest_known_vectors() {
        assert_eq!(
            hex(&digest("md5", b"abc").unwrap()),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hex(&digest("sha1", b"abc").unwrap()),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(&digest("sha256", b"abc").unwrap()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&digest("sha3-256", b"abc").unwrap()),
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
        );
        assert_eq!(
            hex(&digest("sm3", b"abc").unwrap()),
            "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0"
        );
        assert_eq!(
            hex(&digest("crc32", b"123456789").unwrap()),
            "cbf43926"
        );
    }

    #[test]
    fn hmac_rfc4231_case2() {
        let mac = hmac("sha256", b"Jefe", b"what do ya want for nothing?").unwrap();
        assert_eq!(
            hex(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn unknown_algorithm_errors() {
        assert!(digest("sha999", b"x").is_err());
    }
}
