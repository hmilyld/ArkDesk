//! 非对称：RSA 与 SM2（加解密、签名/验签、密钥对）。
//!
//! - RSA：OpenSSL/标准兼容，同时支持 PKCS#8/SPKI 与 PKCS#1 PEM，OAEP/PKCS#1v15、PSS/PKCS#1v15。
//! - SM2：基于 `gmcrypto-core`（constant-time），支持 ASN.1 DER 与 raw C1C3C2，PKCS#8/SPKI PEM。

use rand_core::OsRng;
use rsa::pkcs1::{
    DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey,
};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::{Oaep, Pkcs1v15Encrypt, Pkcs1v15Sign, Pss, RsaPrivateKey, RsaPublicKey};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};

use crate::error::{code, AppError};

// ── RSA ──────────────────────────────────────────────────────────

/// 生成 RSA 密钥对 PEM。`format` = `pkcs8`（默认）或 `pkcs1`。
pub fn rsa_generate(bits: usize, format: &str) -> Result<(String, String), AppError> {
    let bits = bits.clamp(1024, 8192);
    let mut rng = OsRng;
    let private = RsaPrivateKey::new(&mut rng, bits)
        .map_err(|e| AppError::custom(code::UNKNOWN, format!("RSA 密钥生成失败: {e}")))?;
    let public = RsaPublicKey::from(&private);

    if super::normalize(format) == "pkcs1" {
        let private_pem = private
            .to_pkcs1_pem(LineEnding::LF)
            .map_err(|e| AppError::custom(code::UNKNOWN, format!("导出私钥失败: {e}")))?
            .to_string();
        let public_pem = public
            .to_pkcs1_pem(LineEnding::LF)
            .map_err(|e| AppError::custom(code::UNKNOWN, format!("导出公钥失败: {e}")))?;
        Ok((private_pem, public_pem))
    } else {
        let private_pem = private
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| AppError::custom(code::UNKNOWN, format!("导出私钥失败: {e}")))?
            .to_string();
        let public_pem = public
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| AppError::custom(code::UNKNOWN, format!("导出公钥失败: {e}")))?;
        Ok((private_pem, public_pem))
    }
}

fn parse_rsa_private(pem: &str) -> Result<RsaPrivateKey, AppError> {
    if pem.trim().is_empty() {
        return Err(AppError::invalid_input(
            "请先填写或生成 RSA 私钥（PEM 格式）",
        ));
    }
    if pem.contains("BEGIN RSA PRIVATE KEY") {
        RsaPrivateKey::from_pkcs1_pem(pem)
            .map_err(|e| AppError::invalid_input(format!("RSA 私钥解析失败: {e}")))
    } else {
        RsaPrivateKey::from_pkcs8_pem(pem)
            .map_err(|e| AppError::invalid_input(format!("RSA 私钥解析失败: {e}")))
    }
}

fn parse_rsa_public(pem: &str) -> Result<RsaPublicKey, AppError> {
    if pem.trim().is_empty() {
        return Err(AppError::invalid_input(
            "请先填写或生成 RSA 公钥（PEM 格式）",
        ));
    }
    if pem.contains("BEGIN RSA PUBLIC KEY") {
        RsaPublicKey::from_pkcs1_pem(pem)
            .map_err(|e| AppError::invalid_input(format!("RSA 公钥解析失败: {e}")))
    } else {
        RsaPublicKey::from_public_key_pem(pem)
            .map_err(|e| AppError::invalid_input(format!("RSA 公钥解析失败: {e}")))
    }
}

fn rsa_digest(hash: &str, data: &[u8]) -> Result<Vec<u8>, AppError> {
    use digest::Digest;
    Ok(match super::normalize(hash).as_str() {
        "sha1" => Sha1::digest(data).to_vec(),
        "sha256" | "" => Sha256::digest(data).to_vec(),
        "sha384" => Sha384::digest(data).to_vec(),
        "sha512" => Sha512::digest(data).to_vec(),
        other => return Err(AppError::invalid_input(format!("不支持的哈希: {other}"))),
    })
}

/// RSA 加密（公钥）。
pub fn rsa_encrypt(
    public_pem: &str,
    data: &[u8],
    padding: &str,
    hash: &str,
) -> Result<Vec<u8>, AppError> {
    let key = parse_rsa_public(public_pem)?;
    let mut rng = OsRng;
    let result = match super::normalize(padding).as_str() {
        "pkcs1v15" | "pkcs1" => key.encrypt(&mut rng, Pkcs1v15Encrypt, data),
        "oaepsha1" => key.encrypt(&mut rng, Oaep::new::<Sha1>(), data),
        "oaepsha384" => key.encrypt(&mut rng, Oaep::new::<Sha384>(), data),
        "oaepsha512" => key.encrypt(&mut rng, Oaep::new::<Sha512>(), data),
        // 通用 OAEP：哈希由 UI 的「哈希」选择决定
        "oaep" | "" => match super::normalize(hash).as_str() {
            "sha1" => key.encrypt(&mut rng, Oaep::new::<Sha1>(), data),
            "sha384" => key.encrypt(&mut rng, Oaep::new::<Sha384>(), data),
            "sha512" => key.encrypt(&mut rng, Oaep::new::<Sha512>(), data),
            _ => key.encrypt(&mut rng, Oaep::new::<Sha256>(), data),
        },
        other => {
            return Err(AppError::invalid_input(format!(
                "不支持的 RSA 填充: {other}"
            )));
        }
    };
    result.map_err(|e| AppError::invalid_input(format!("RSA 加密失败: {e}")))
}

/// RSA 解密（私钥）。
pub fn rsa_decrypt(
    private_pem: &str,
    data: &[u8],
    padding: &str,
    hash: &str,
) -> Result<Vec<u8>, AppError> {
    let key = parse_rsa_private(private_pem)?;
    let result = match super::normalize(padding).as_str() {
        "pkcs1v15" | "pkcs1" => key.decrypt(Pkcs1v15Encrypt, data),
        "oaepsha1" => key.decrypt(Oaep::new::<Sha1>(), data),
        "oaepsha384" => key.decrypt(Oaep::new::<Sha384>(), data),
        "oaepsha512" => key.decrypt(Oaep::new::<Sha512>(), data),
        // 通用 OAEP：哈希由 UI 的「哈希」选择决定
        "oaep" | "" => match super::normalize(hash).as_str() {
            "sha1" => key.decrypt(Oaep::new::<Sha1>(), data),
            "sha384" => key.decrypt(Oaep::new::<Sha384>(), data),
            "sha512" => key.decrypt(Oaep::new::<Sha512>(), data),
            _ => key.decrypt(Oaep::new::<Sha256>(), data),
        },
        other => {
            return Err(AppError::invalid_input(format!(
                "不支持的 RSA 填充: {other}"
            )));
        }
    };
    result.map_err(|e| AppError::invalid_input(format!("RSA 解密失败: {e}")))
}

/// RSA 签名（私钥）。`padding` = `pss` 或 `pkcs1v15`。
pub fn rsa_sign(
    private_pem: &str,
    data: &[u8],
    padding: &str,
    hash: &str,
) -> Result<Vec<u8>, AppError> {
    let key = parse_rsa_private(private_pem)?;
    let digest = rsa_digest(hash, data)?;
    let mut rng = OsRng;
    let pss = super::normalize(padding) == "pss";
    let result = if pss {
        match super::normalize(hash).as_str() {
            "sha1" => key.sign_with_rng(&mut rng, Pss::new::<Sha1>(), &digest),
            "sha384" => key.sign_with_rng(&mut rng, Pss::new::<Sha384>(), &digest),
            "sha512" => key.sign_with_rng(&mut rng, Pss::new::<Sha512>(), &digest),
            _ => key.sign_with_rng(&mut rng, Pss::new::<Sha256>(), &digest),
        }
    } else {
        match super::normalize(hash).as_str() {
            "sha1" => key.sign(Pkcs1v15Sign::new::<Sha1>(), &digest),
            "sha384" => key.sign(Pkcs1v15Sign::new::<Sha384>(), &digest),
            "sha512" => key.sign(Pkcs1v15Sign::new::<Sha512>(), &digest),
            _ => key.sign(Pkcs1v15Sign::new::<Sha256>(), &digest),
        }
    };
    result.map_err(|e| AppError::invalid_input(format!("RSA 签名失败: {e}")))
}

/// RSA 验签（公钥）。
pub fn rsa_verify(
    public_pem: &str,
    data: &[u8],
    signature: &[u8],
    padding: &str,
    hash: &str,
) -> Result<bool, AppError> {
    let key = parse_rsa_public(public_pem)?;
    let digest = rsa_digest(hash, data)?;
    let pss = super::normalize(padding) == "pss";
    let ok = if pss {
        match super::normalize(hash).as_str() {
            "sha1" => key.verify(Pss::new::<Sha1>(), &digest, signature),
            "sha384" => key.verify(Pss::new::<Sha384>(), &digest, signature),
            "sha512" => key.verify(Pss::new::<Sha512>(), &digest, signature),
            _ => key.verify(Pss::new::<Sha256>(), &digest, signature),
        }
    } else {
        match super::normalize(hash).as_str() {
            "sha1" => key.verify(Pkcs1v15Sign::new::<Sha1>(), &digest, signature),
            "sha384" => key.verify(Pkcs1v15Sign::new::<Sha384>(), &digest, signature),
            "sha512" => key.verify(Pkcs1v15Sign::new::<Sha512>(), &digest, signature),
            _ => key.verify(Pkcs1v15Sign::new::<Sha256>(), &digest, signature),
        }
    };
    Ok(ok.is_ok())
}

// ── SM2 ──────────────────────────────────────────────────────────

const SM2_PRIVATE_LABELS: [&str; 2] = ["PRIVATE KEY", "SM2 PRIVATE KEY"];
const SM2_PUBLIC_LABELS: [&str; 2] = ["PUBLIC KEY", "SM2 PUBLIC KEY"];

/// 生成 SM2 密钥对 PEM（PKCS#8 / SPKI）。
pub fn sm2_generate() -> Result<(String, String), AppError> {
    for _ in 0..64 {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes)
            .map_err(|e| AppError::custom(code::UNKNOWN, format!("随机数失败: {e}")))?;
        if let Some(key) = gmcrypto_core::sm2::Sm2PrivateKey::from_bytes_be(&bytes).into_option() {
            let public = key.public_key();
            let private_pem = gmcrypto_core::pem::encode(
                "PRIVATE KEY",
                &gmcrypto_core::pkcs8::encode(&key),
            );
            let public_pem =
                gmcrypto_core::pem::encode("PUBLIC KEY", &gmcrypto_core::spki::encode(&public));
            return Ok((private_pem, public_pem));
        }
    }
    Err(AppError::custom(code::UNKNOWN, "SM2 密钥生成失败"))
}

fn parse_sm2_private(pem: &str) -> Result<gmcrypto_core::sm2::Sm2PrivateKey, AppError> {
    if pem.trim().is_empty() {
        return Err(AppError::invalid_input(
            "请先填写或生成 SM2 私钥（PEM 格式）",
        ));
    }
    for label in SM2_PRIVATE_LABELS {
        if let Ok(der) = gmcrypto_core::pem::decode(pem, label) {
            if let Ok(key) = gmcrypto_core::pkcs8::decode(&der) {
                return Ok(key);
            }
        }
    }
    Err(AppError::invalid_input("SM2 私钥解析失败"))
}

fn parse_sm2_public(pem: &str) -> Result<gmcrypto_core::sm2::Sm2PublicKey, AppError> {
    if pem.trim().is_empty() {
        return Err(AppError::invalid_input(
            "请先填写或生成 SM2 公钥（PEM 格式）",
        ));
    }
    for label in SM2_PUBLIC_LABELS {
        if let Ok(der) = gmcrypto_core::pem::decode(pem, label) {
            if let Some(key) = gmcrypto_core::spki::decode(&der) {
                return Ok(key);
            }
        }
    }
    Err(AppError::invalid_input("SM2 公钥解析失败"))
}

/// SM2 加密。`format` = `asn1`（默认，DER）或 `raw`（C1C3C2）。
pub fn sm2_encrypt(public_pem: &str, data: &[u8], format: &str) -> Result<Vec<u8>, AppError> {
    let key = parse_sm2_public(public_pem)?;
    let mut rng = getrandom::SysRng;
    let der = gmcrypto_core::sm2::encrypt(&key, data, &mut rng)
        .map_err(|e| AppError::invalid_input(format!("SM2 加密失败: {e:?}")))?;
    if super::normalize(format) == "raw" {
        let ciphertext = gmcrypto_core::asn1::ciphertext::decode(&der)
            .ok_or_else(|| AppError::custom(code::UNKNOWN, "SM2 密文编码失败"))?;
        Ok(gmcrypto_core::sm2::raw_ciphertext::encode_c1c3c2(
            &ciphertext,
        ))
    } else {
        Ok(der)
    }
}

/// SM2 解密。`format` = `asn1`（默认）或 `raw`。
pub fn sm2_decrypt(private_pem: &str, data: &[u8], format: &str) -> Result<Vec<u8>, AppError> {
    let key = parse_sm2_private(private_pem)?;
    let der = if super::normalize(format) == "raw" {
        let ciphertext = gmcrypto_core::sm2::raw_ciphertext::decode_c1c3c2(data)
            .or_else(|| gmcrypto_core::sm2::raw_ciphertext::decode_c1c2c3_legacy(data))
            .ok_or_else(|| AppError::invalid_input("SM2 raw 密文解析失败"))?;
        gmcrypto_core::asn1::ciphertext::encode(&ciphertext)
    } else {
        data.to_vec()
    };
    gmcrypto_core::sm2::decrypt(&key, &der)
        .map_err(|e| AppError::invalid_input(format!("SM2 解密失败: {e:?}")))
}

/// SM2 签名（DER）。`id` 为空时使用默认签名者 ID。
pub fn sm2_sign(private_pem: &str, id: &str, data: &[u8]) -> Result<Vec<u8>, AppError> {
    let key = parse_sm2_private(private_pem)?;
    let id_bytes: &[u8] = if id.is_empty() {
        gmcrypto_core::sm2::DEFAULT_SIGNER_ID
    } else {
        id.as_bytes()
    };
    let mut rng = getrandom::SysRng;
    gmcrypto_core::sm2::sign_with_id(&key, id_bytes, data, &mut rng)
        .map_err(|e| AppError::invalid_input(format!("SM2 签名失败: {e:?}")))
}

/// SM2 验签。
pub fn sm2_verify(
    public_pem: &str,
    id: &str,
    data: &[u8],
    signature: &[u8],
) -> Result<bool, AppError> {
    let key = parse_sm2_public(public_pem)?;
    let id_bytes: &[u8] = if id.is_empty() {
        gmcrypto_core::sm2::DEFAULT_SIGNER_ID
    } else {
        id.as_bytes()
    };
    Ok(gmcrypto_core::sm2::verify_with_id(
        &key, id_bytes, data, signature,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsa_sign_verify_roundtrip() {
        let (private, public) = rsa_generate(2048, "pkcs8").unwrap();
        assert!(private.contains("BEGIN PRIVATE KEY"));
        let sig = rsa_sign(&private, b"hello", "pss", "sha256").unwrap();
        assert!(rsa_verify(&public, b"hello", &sig, "pss", "sha256").unwrap());
        assert!(!rsa_verify(&public, b"tampered", &sig, "pss", "sha256").unwrap());
    }

    #[test]
    fn rsa_pkcs1_encrypt_decrypt_roundtrip() {
        let (private, public) = rsa_generate(2048, "pkcs1").unwrap();
        assert!(private.contains("BEGIN RSA PRIVATE KEY"));
        let ct = rsa_encrypt(&public, b"secret", "oaep", "sha256").unwrap();
        assert_eq!(rsa_decrypt(&private, &ct, "oaep", "sha256").unwrap(), b"secret");
    }

    #[test]
    fn empty_key_gives_friendly_error() {
        let err = rsa_encrypt("", b"x", "oaep", "sha256").unwrap_err();
        assert!(err.to_string().contains("RSA 公钥"), "{err}");
        let err = sm2_sign("", "", b"x").unwrap_err();
        assert!(err.to_string().contains("SM2 私钥"), "{err}");
    }

    #[test]
    fn sm2_sign_verify_and_encrypt_roundtrip() {
        let (private, public) = sm2_generate().unwrap();
        let sig = sm2_sign(&private, "", b"attack at dawn").unwrap();
        assert!(sm2_verify(&public, "", b"attack at dawn", &sig).unwrap());
        assert!(!sm2_verify(&public, "", b"attack at dusk", &sig).unwrap());

        let ct = sm2_encrypt(&public, b"\x01\x02\x03", "asn1").unwrap();
        assert_eq!(sm2_decrypt(&private, &ct, "asn1").unwrap(), vec![1, 2, 3]);

        let raw = sm2_encrypt(&public, b"raw-mode", "raw").unwrap();
        assert_eq!(sm2_decrypt(&private, &raw, "raw").unwrap(), b"raw-mode");
    }
}
