//! 对称加密：AES / SM4 / DES / 3DES。
//!
//! - 模式：ECB / CBC / CTR；GCM（AEAD）仅 AES。
//! - 文本：一次性处理，支持 GCM。
//! - 文件：流式处理，仅 ECB / CBC / CTR。
//! - 口令模式：兼容 `openssl enc`（`Salted__` + EVP_BytesToKey(MD5)，或 `-pbkdf2`）。

use std::io::{Read, Write};

use aes::{Aes128, Aes192, Aes256};
use aes_gcm::aead::{Aead, Payload};
use aes_gcm::AesGcm;
use block_padding::Pkcs7;
use cipher::generic_array::GenericArray;
use cipher::{
    BlockCipher, BlockDecrypt, BlockDecryptMut, BlockEncrypt, BlockEncryptMut, KeyInit, KeyIvInit,
    StreamCipher,
};
use des::{Des, TdesEde3};
use sm4::Sm4;
use zeroize::Zeroizing;

use crate::error::{code, AppError};

use super::passphrase;
use super::progress::StreamObserver;

const CHUNK: usize = 1024 * 1024;

// ── 模式分派宏 ────────────────────────────────────────────────────

/// 依据模式执行一次性分组模式。
macro_rules! crypt_by_mode {
    ($c:ty, $ctr:ty, $mode:expr, $enc:expr, $key:expr, $iv:expr, $data:expr) => {{
        match $mode {
            "ecb" => ecb_crypt::<$c>($enc, $key, $data),
            "cbc" => cbc_crypt::<$c>($enc, $key, $iv, $data),
            "ctr" => ctr_crypt::<$ctr>($key, $iv, $data),
            _ => unreachable!(),
        }
    }};
}

/// 依据模式执行流式分组/CTR 处理（用于文件）。
macro_rules! stream_by_mode {
    ($c:ty, $ctr:ty, $mode:expr, $enc:expr, $key:expr, $iv:expr, $reader:expr, $writer:expr, $observer:expr, $bs:expr) => {{
        match $mode {
            "ecb" => {
                if $enc {
                    let cipher = ecb::Encryptor::<$c>::new_from_slice($key)
                        .map_err(|_| AppError::invalid_input("密钥长度不匹配"))?;
                    stream_encrypt_blocks(cipher, $reader, $writer, $observer, $bs)
                } else {
                    let cipher = ecb::Decryptor::<$c>::new_from_slice($key)
                        .map_err(|_| AppError::invalid_input("密钥长度不匹配"))?;
                    stream_decrypt_blocks(cipher, $reader, $writer, $observer, $bs)
                }
            }
            "cbc" => {
                if $enc {
                    let cipher = cbc::Encryptor::<$c>::new_from_slices($key, $iv)
                        .map_err(|_| AppError::invalid_input("密钥或 IV 长度不匹配"))?;
                    stream_encrypt_blocks(cipher, $reader, $writer, $observer, $bs)
                } else {
                    let cipher = cbc::Decryptor::<$c>::new_from_slices($key, $iv)
                        .map_err(|_| AppError::invalid_input("密钥或 IV 长度不匹配"))?;
                    stream_decrypt_blocks(cipher, $reader, $writer, $observer, $bs)
                }
            }
            "ctr" => {
                let cipher = <$ctr>::new_from_slices($key, $iv)
                    .map_err(|_| AppError::invalid_input("密钥或 IV 长度不匹配"))?;
                stream_ctr(cipher, $reader, $writer, $observer)
            }
            _ => unreachable!(),
        }
    }};
}

/// AES-GCM 一次性加解密（AEAD，密文尾部含 16 字节 tag）。
macro_rules! gcm_crypt {
    ($aes:ty, $enc:expr, $key:expr, $nonce:expr, $data:expr, $aad:expr) => {{
        if $nonce.len() != 12 {
            return Err(AppError::invalid_input("GCM nonce 必须为 12 字节"));
        }
        type Cipher = AesGcm<$aes, aes_gcm::aead::consts::U12>;
        let cipher = <Cipher as aes_gcm::aead::KeyInit>::new_from_slice($key)
            .map_err(|_| AppError::invalid_input("AES-GCM 密钥长度不匹配"))?;
        let payload = Payload {
            msg: $data,
            aad: $aad,
        };
        let nonce = GenericArray::from_slice($nonce);
        let result = if $enc {
            cipher.encrypt(nonce, payload)
        } else {
            cipher.decrypt(nonce, payload)
        };
        result.map_err(|_| AppError::invalid_input("GCM 校验失败：密钥、nonce、AAD 或密文不匹配"))
    }};
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Alg {
    Aes128,
    Aes192,
    Aes256,
    Sm4,
    Des,
    Tdes,
}

impl Alg {
    /// 依据算法名与（可选）密钥长度确定算法。
    pub fn parse(algorithm: &str, key_len: Option<usize>) -> Result<Self, AppError> {
        let name = super::normalize(algorithm);
        let alg = match name.as_str() {
            "aes" => match key_len {
                Some(16) => Alg::Aes128,
                Some(24) => Alg::Aes192,
                Some(32) => Alg::Aes256,
                Some(n) => {
                    return Err(AppError::invalid_input(format!(
                        "AES 密钥长度应为 16/24/32 字节，当前 {n}"
                    )))
                }
                None => Alg::Aes256,
            },
            "aes128" => Alg::Aes128,
            "aes192" => Alg::Aes192,
            "aes256" => Alg::Aes256,
            "sm4" => Alg::Sm4,
            "des" => Alg::Des,
            "3des" | "des3" | "tripledes" | "tdes" => Alg::Tdes,
            other => return Err(AppError::invalid_input(format!("不支持的对称算法: {other}"))),
        };

        if let Some(len) = key_len {
            let expected = alg.key_len();
            if len != expected {
                return Err(AppError::invalid_input(format!(
                    "{} 密钥长度应为 {} 字节，当前 {len}",
                    alg.name(),
                    expected
                )));
            }
        }
        Ok(alg)
    }

    pub fn name(self) -> &'static str {
        match self {
            Alg::Aes128 => "AES-128",
            Alg::Aes192 => "AES-192",
            Alg::Aes256 => "AES-256",
            Alg::Sm4 => "SM4",
            Alg::Des => "DES",
            Alg::Tdes => "3DES",
        }
    }

    pub fn key_len(self) -> usize {
        match self {
            Alg::Aes128 => 16,
            Alg::Aes192 => 24,
            Alg::Aes256 => 32,
            Alg::Sm4 => 16,
            Alg::Des => 8,
            Alg::Tdes => 24,
        }
    }

    pub fn block_size(self) -> usize {
        match self {
            Alg::Des | Alg::Tdes => 8,
            _ => 16,
        }
    }
}

/// 归一化后的对称参数。
pub struct SymParams {
    pub algorithm: String,
    pub mode: String,
    pub key: Vec<u8>,
    pub iv: Vec<u8>,
    pub passphrase: Option<Vec<u8>>,
    /// `evp`（默认，OpenSSL EVP_BytesToKey/MD5）或 `pbkdf2`
    pub kdf: String,
    pub salt: Option<Vec<u8>>,
    pub rounds: u32,
    pub aad: Vec<u8>,
}

impl SymParams {
    fn alg(&self) -> Result<Alg, AppError> {
        let key_len = if self.passphrase.is_some() {
            None
        } else {
            Some(self.key.len())
        };
        Alg::parse(&self.algorithm, key_len)
    }

    fn mode(&self) -> Result<&'static str, AppError> {
        match super::normalize(&self.mode).as_str() {
            "ecb" => Ok("ecb"),
            "cbc" => Ok("cbc"),
            "ctr" => Ok("ctr"),
            "gcm" => Ok("gcm"),
            other => Err(AppError::invalid_input(format!("不支持的加密模式: {other}"))),
        }
    }
}

/// 文本一次性加解密。`enc = true` 为加密。
pub fn text_crypt(enc: bool, p: &SymParams, data: &[u8]) -> Result<Vec<u8>, AppError> {
    let alg = p.alg()?;
    let mode = p.mode()?;

    if p.passphrase.is_some() {
        if mode == "gcm" {
            return Err(AppError::invalid_input(
                "GCM 不支持口令模式，请使用原始密钥（建议 12 字节随机 nonce）",
            ));
        }
        if enc {
            let salt = match &p.salt {
                Some(salt) => passphrase::ensure_salt(salt)?,
                None => passphrase::random_salt()?,
            };
            let password = p.passphrase.as_deref().unwrap_or_default();
            let (key, iv) = passphrase::derive(alg, mode, password, &p.kdf, p.rounds, &salt)?;
            let body = crypt_blocks(alg, mode, enc, &key[..], &iv[..], data, &p.aad)?;
            let mut out = Vec::with_capacity(16 + body.len());
            out.extend_from_slice(passphrase::SALT_MAGIC);
            out.extend_from_slice(&salt);
            out.extend_from_slice(&body);
            Ok(out)
        } else {
            let (salt, body) = passphrase::unwrap_salted(data)?;
            let salt = salt.to_vec();
            let password = p.passphrase.as_deref().unwrap_or_default();
            let (key, iv) = passphrase::derive(alg, mode, password, &p.kdf, p.rounds, &salt)?;
            crypt_blocks(alg, mode, enc, &key[..], &iv[..], body, &p.aad)
        }
    } else {
        check_iv(alg, mode, &p.iv)?;
        crypt_blocks(alg, mode, enc, &p.key, &p.iv, data, &p.aad)
    }
}

/// 文件流式加解密（ECB / CBC / CTR）。失败（含取消）时删除未完成的输出文件。
pub fn file_crypt(
    enc: bool,
    p: &SymParams,
    input: &str,
    output: &str,
    observer: &mut dyn StreamObserver,
) -> Result<(), AppError> {
    let alg = p.alg()?;
    let mode = p.mode()?;
    if mode == "gcm" {
        return Err(AppError::invalid_input(
            "GCM 不支持文件（无法流式），请改用 CBC/CTR",
        ));
    }

    let result = (|| -> Result<(), AppError> {
        let mut reader = std::io::BufReader::with_capacity(
            CHUNK,
            std::fs::File::open(input)
                .map_err(|e| AppError::custom(code::IO_ERROR, format!("打开输入文件失败: {e}")))?,
        );
        let mut writer = std::io::BufWriter::with_capacity(
            CHUNK,
            std::fs::File::create(output)
                .map_err(|e| AppError::custom(code::IO_ERROR, format!("创建输出文件失败: {e}")))?,
        );

        let key: Zeroizing<Vec<u8>>;
        let iv: Zeroizing<Vec<u8>>;
        if p.passphrase.is_some() {
            let password = p.passphrase.as_deref().unwrap_or_default();
            if enc {
                let salt = match &p.salt {
                    Some(salt) => passphrase::ensure_salt(salt)?,
                    None => passphrase::random_salt()?,
                };
                let derived = passphrase::derive(alg, mode, password, &p.kdf, p.rounds, &salt)?;
                key = derived.0;
                iv = derived.1;
                writer.write_all(passphrase::SALT_MAGIC).map_err(io_err)?;
                writer.write_all(&salt).map_err(io_err)?;
            } else {
                let mut header = [0u8; 16];
                reader
                    .read_exact(&mut header)
                    .map_err(|_| AppError::invalid_input("口令模式密文缺少 Salted__ 头"))?;
                if &header[..8] != passphrase::SALT_MAGIC {
                    return Err(AppError::invalid_input("口令模式密文缺少 Salted__ 头"));
                }
                let derived =
                    passphrase::derive(alg, mode, password, &p.kdf, p.rounds, &header[8..])?;
                key = derived.0;
                iv = derived.1;
            }
        } else {
            check_iv(alg, mode, &p.iv)?;
            key = Zeroizing::new(p.key.clone());
            iv = Zeroizing::new(p.iv.clone());
        }

        stream_crypt(
            alg,
            mode,
            enc,
            (&key[..], &iv[..]),
            &mut reader,
            &mut writer,
            observer,
        )?;
        writer.flush().map_err(io_err)
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(output);
    }
    result
}

/// 生成随机密钥与 IV。GCM 模式返回 12 字节 nonce，其余为分组大小。
pub fn generate(algorithm: &str, mode: &str) -> Result<(Vec<u8>, Vec<u8>), AppError> {
    let alg = Alg::parse(algorithm, None)?;
    let mut key = vec![0u8; alg.key_len()];
    getrandom::fill(&mut key)
        .map_err(|e| AppError::custom(code::UNKNOWN, format!("随机数失败: {e}")))?;
    let iv_len = if super::normalize(mode) == "gcm" {
        12
    } else {
        alg.block_size()
    };
    let mut iv = vec![0u8; iv_len];
    getrandom::fill(&mut iv)
        .map_err(|e| AppError::custom(code::UNKNOWN, format!("随机数失败: {e}")))?;
    Ok((key, iv))
}

// ── 一次性分组/AEAD ───────────────────────────────────────────────

fn crypt_blocks(
    alg: Alg,
    mode: &str,
    enc: bool,
    key: &[u8],
    iv: &[u8],
    data: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, AppError> {
    if mode == "gcm" {
        return match alg {
            Alg::Aes128 => gcm_crypt!(Aes128, enc, key, iv, data, aad),
            Alg::Aes192 => gcm_crypt!(Aes192, enc, key, iv, data, aad),
            Alg::Aes256 => gcm_crypt!(Aes256, enc, key, iv, data, aad),
            _ => Err(AppError::invalid_input("GCM 仅支持 AES")),
        };
    }
    match alg {
        Alg::Aes128 => crypt_by_mode!(Aes128, ctr::Ctr128BE<Aes128>, mode, enc, key, iv, data),
        Alg::Aes192 => crypt_by_mode!(Aes192, ctr::Ctr128BE<Aes192>, mode, enc, key, iv, data),
        Alg::Aes256 => crypt_by_mode!(Aes256, ctr::Ctr128BE<Aes256>, mode, enc, key, iv, data),
        Alg::Sm4 => crypt_by_mode!(Sm4, ctr::Ctr128BE<Sm4>, mode, enc, key, iv, data),
        Alg::Des => crypt_by_mode!(Des, ctr::Ctr64BE<Des>, mode, enc, key, iv, data),
        Alg::Tdes => crypt_by_mode!(TdesEde3, ctr::Ctr64BE<TdesEde3>, mode, enc, key, iv, data),
    }
}

fn ecb_crypt<C>(enc: bool, key: &[u8], data: &[u8]) -> Result<Vec<u8>, AppError>
where
    C: BlockCipher + BlockEncrypt + BlockDecrypt + KeyInit,
{
    if enc {
        let cipher = ecb::Encryptor::<C>::new_from_slice(key)
            .map_err(|_| AppError::invalid_input("密钥长度不匹配"))?;
        Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data))
    } else {
        let cipher = ecb::Decryptor::<C>::new_from_slice(key)
            .map_err(|_| AppError::invalid_input("密钥长度不匹配"))?;
        cipher
            .decrypt_padded_vec_mut::<Pkcs7>(data)
            .map_err(|_| AppError::invalid_input("解密失败：密文长度或填充不合法"))
    }
}

fn cbc_crypt<C>(enc: bool, key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, AppError>
where
    C: BlockCipher + BlockEncrypt + BlockDecrypt + KeyInit,
{
    if enc {
        let cipher = cbc::Encryptor::<C>::new_from_slices(key, iv)
            .map_err(|_| AppError::invalid_input("密钥或 IV 长度不匹配"))?;
        Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data))
    } else {
        let cipher = cbc::Decryptor::<C>::new_from_slices(key, iv)
            .map_err(|_| AppError::invalid_input("密钥或 IV 长度不匹配"))?;
        cipher
            .decrypt_padded_vec_mut::<Pkcs7>(data)
            .map_err(|_| AppError::invalid_input("解密失败：密文长度或填充不合法"))
    }
}

fn ctr_crypt<S>(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, AppError>
where
    S: KeyIvInit + StreamCipher,
{
    let mut cipher = S::new_from_slices(key, iv)
        .map_err(|_| AppError::invalid_input("密钥或 IV 长度不匹配"))?;
    let mut buf = data.to_vec();
    cipher.apply_keystream(&mut buf);
    Ok(buf)
}

// ── 流式（文件） ──────────────────────────────────────────────────

fn stream_crypt(
    alg: Alg,
    mode: &str,
    enc: bool,
    key_iv: (&[u8], &[u8]),
    reader: &mut impl Read,
    writer: &mut impl Write,
    observer: &mut dyn StreamObserver,
) -> Result<(), AppError> {
    let (key, iv) = key_iv;
    let bs = alg.block_size();
    match alg {
        Alg::Aes128 => stream_by_mode!(Aes128, ctr::Ctr128BE<Aes128>, mode, enc, key, iv, reader, writer, observer, bs),
        Alg::Aes192 => stream_by_mode!(Aes192, ctr::Ctr128BE<Aes192>, mode, enc, key, iv, reader, writer, observer, bs),
        Alg::Aes256 => stream_by_mode!(Aes256, ctr::Ctr128BE<Aes256>, mode, enc, key, iv, reader, writer, observer, bs),
        Alg::Sm4 => stream_by_mode!(Sm4, ctr::Ctr128BE<Sm4>, mode, enc, key, iv, reader, writer, observer, bs),
        Alg::Des => stream_by_mode!(Des, ctr::Ctr64BE<Des>, mode, enc, key, iv, reader, writer, observer, bs),
        Alg::Tdes => stream_by_mode!(TdesEde3, ctr::Ctr64BE<TdesEde3>, mode, enc, key, iv, reader, writer, observer, bs),
    }
}

fn stream_encrypt_blocks<M, R, W>(
    mut cipher: M,
    reader: &mut R,
    writer: &mut W,
    observer: &mut dyn StreamObserver,
    bs: usize,
) -> Result<(), AppError>
where
    M: BlockEncryptMut,
    R: Read,
    W: Write,
{
    let mut carry: Vec<u8> = Vec::with_capacity(bs * 2);
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = reader.read(&mut buf).map_err(io_err)?;
        if n == 0 {
            break;
        }
        observer.advance(n as u64)?;
        carry.extend_from_slice(&buf[..n]);
        let full = carry.len() / bs * bs;
        let mut offset = 0;
        while offset < full {
            let block = GenericArray::from_mut_slice(&mut carry[offset..offset + bs]);
            cipher.encrypt_block_mut(block);
            offset += bs;
        }
        if full > 0 {
            writer.write_all(&carry[..full]).map_err(io_err)?;
            carry.drain(..full);
        }
    }
    let pad = bs - (carry.len() % bs);
    carry.extend(std::iter::repeat_n(pad as u8, pad));
    let mut offset = 0;
    while offset < carry.len() {
        let block = GenericArray::from_mut_slice(&mut carry[offset..offset + bs]);
        cipher.encrypt_block_mut(block);
        offset += bs;
    }
    writer.write_all(&carry).map_err(io_err)?;
    Ok(())
}

fn stream_decrypt_blocks<M, R, W>(
    mut cipher: M,
    reader: &mut R,
    writer: &mut W,
    observer: &mut dyn StreamObserver,
    bs: usize,
) -> Result<(), AppError>
where
    M: BlockDecryptMut,
    R: Read,
    W: Write,
{
    let mut carry: Vec<u8> = Vec::with_capacity(bs * 2);
    let mut pending: Option<Vec<u8>> = None;
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = reader.read(&mut buf).map_err(io_err)?;
        if n == 0 {
            break;
        }
        observer.advance(n as u64)?;
        carry.extend_from_slice(&buf[..n]);
        while carry.len() >= bs {
            let block: Vec<u8> = carry.drain(..bs).collect();
            if let Some(mut prev) = pending.take() {
                let b = GenericArray::from_mut_slice(&mut prev);
                cipher.decrypt_block_mut(b);
                writer.write_all(&prev).map_err(io_err)?;
            }
            pending = Some(block);
        }
    }
    if !carry.is_empty() {
        return Err(AppError::invalid_input("密文长度不是块大小的整数倍"));
    }
    let mut last = pending.ok_or_else(|| AppError::invalid_input("密文为空"))?;
    let b = GenericArray::from_mut_slice(&mut last);
    cipher.decrypt_block_mut(b);
    let pad = *last.last().unwrap_or(&0) as usize;
    if pad == 0 || pad > bs || pad > last.len() {
        return Err(AppError::invalid_input(
            "解密失败：填充不合法（口令或密钥可能错误）",
        ));
    }
    last.truncate(last.len() - pad);
    writer.write_all(&last).map_err(io_err)?;
    Ok(())
}

fn stream_ctr<S, R, W>(
    mut cipher: S,
    reader: &mut R,
    writer: &mut W,
    observer: &mut dyn StreamObserver,
) -> Result<(), AppError>
where
    S: StreamCipher,
    R: Read,
    W: Write,
{
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = reader.read(&mut buf).map_err(io_err)?;
        if n == 0 {
            break;
        }
        observer.advance(n as u64)?;
        cipher.apply_keystream(&mut buf[..n]);
        writer.write_all(&buf[..n]).map_err(io_err)?;
    }
    Ok(())
}

// ── 辅助 ──────────────────────────────────────────────────────────

fn io_err(e: std::io::Error) -> AppError {
    AppError::custom(code::IO_ERROR, format!("文件读写失败: {e}"))
}

fn check_iv(alg: Alg, mode: &str, iv: &[u8]) -> Result<(), AppError> {
    match mode {
        "ecb" => Ok(()),
        "gcm" => {
            if iv.len() != 12 {
                return Err(AppError::invalid_input("GCM nonce 应为 12 字节"));
            }
            Ok(())
        }
        _ => {
            let bs = alg.block_size();
            if iv.len() != bs {
                return Err(AppError::invalid_input(format!(
                    "{} 模式 IV 应为 {} 字节",
                    mode.to_uppercase(),
                    bs
                )));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::progress::NoopObserver;
    use super::*;

    fn h(s: &str) -> Vec<u8> {
        data_encoding::HEXLOWER_PERMISSIVE.decode(s.as_bytes()).unwrap()
    }

    fn x(b: &[u8]) -> String {
        data_encoding::HEXLOWER.encode(b)
    }

    #[test]
    fn aes128_ecb_fips197() {
        let key = h("000102030405060708090a0b0c0d0e0f");
        let pt = h("00112233445566778899aabbccddeeff");
        let ct = crypt_blocks(Alg::Aes128, "ecb", true, &key, &[], &pt, &[]).unwrap();
        assert_eq!(x(&ct[..16]), "69c4e0d86a7b0430d8cdb78070b4c55a");
        let back = crypt_blocks(Alg::Aes128, "ecb", false, &key, &[], &ct, &[]).unwrap();
        assert_eq!(back, pt);
    }

    #[test]
    fn aes128_cbc_sp800_38a() {
        let key = h("2b7e151628aed2a6abf7158809cf4f3c");
        let iv = h("000102030405060708090a0b0c0d0e0f");
        let pt = h("6bc1bee22e409f96e93d7e117393172a");
        let ct = crypt_blocks(Alg::Aes128, "cbc", true, &key, &iv, &pt, &[]).unwrap();
        assert_eq!(x(&ct[..16]), "7649abac8119b246cee98e9b12e9197d");
    }

    #[test]
    fn aes128_ctr_sp800_38a() {
        let key = h("2b7e151628aed2a6abf7158809cf4f3c");
        let iv = h("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
        let pt = h("6bc1bee22e409f96e93d7e117393172a");
        let ct = crypt_blocks(Alg::Aes128, "ctr", true, &key, &iv, &pt, &[]).unwrap();
        assert_eq!(x(&ct), "874d6191b620e3261bef6864990db6ce");
    }

    #[test]
    fn sm4_ecb_gbt32907() {
        let key = h("0123456789abcdeffedcba9876543210");
        let pt = h("0123456789abcdeffedcba9876543210");
        let ct = crypt_blocks(Alg::Sm4, "ecb", true, &key, &[], &pt, &[]).unwrap();
        assert_eq!(x(&ct[..16]), "681edf34d206965e86b3e94f536e4246");
    }

    #[test]
    fn aes128_gcm_nist_empty() {
        let key = h("00000000000000000000000000000000");
        let nonce = h("000000000000000000000000");
        let ct = crypt_blocks(Alg::Aes128, "gcm", true, &key, &nonce, &[], &[]).unwrap();
        assert_eq!(x(&ct), "58e2fccefa7e3061367f1d57a4e7455a");
        let back = crypt_blocks(Alg::Aes128, "gcm", false, &key, &nonce, &ct, &[]).unwrap();
        assert!(back.is_empty());
    }

    #[test]
    fn gcm_rejects_wrong_nonce_len() {
        let key = h("00000000000000000000000000000000");
        let nonce = h("00112233445566778899aabbccddeeff"); // 16 字节
        assert!(crypt_blocks(Alg::Aes128, "gcm", true, &key, &nonce, &[], &[]).is_err());
    }

    #[test]
    fn passphrase_roundtrip_and_openssl_format() {
        let p = SymParams {
            algorithm: "aes256".into(),
            mode: "cbc".into(),
            key: vec![],
            iv: vec![],
            passphrase: Some(b"secret".to_vec()),
            kdf: "evp".into(),
            salt: None,
            rounds: 0,
            aad: vec![],
        };
        let ct = text_crypt(true, &p, "你好, ArkDesk".as_bytes()).unwrap();
        assert_eq!(&ct[..8], super::passphrase::SALT_MAGIC);
        let pt = text_crypt(false, &p, &ct).unwrap();
        assert_eq!(pt, "你好, ArkDesk".as_bytes());
    }

    #[test]
    fn file_cbc_roundtrip() {
        let dir = std::env::temp_dir();
        let src = dir.join(format!("arkdesk_sym_src_{}.bin", std::process::id()));
        let enc = dir.join(format!("arkdesk_sym_enc_{}.bin", std::process::id()));
        let dec = dir.join(format!("arkdesk_sym_dec_{}.bin", std::process::id()));
        let data: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&src, &data).unwrap();

        let key = h("2b7e151628aed2a6abf7158809cf4f3c");
        let iv = h("000102030405060708090a0b0c0d0e0f");
        let p = SymParams {
            algorithm: "aes128".into(),
            mode: "cbc".into(),
            key: key.clone(),
            iv: iv.clone(),
            passphrase: None,
            kdf: "evp".into(),
            salt: None,
            rounds: 0,
            aad: vec![],
        };
        file_crypt(
            true,
            &p,
            src.to_str().unwrap(),
            enc.to_str().unwrap(),
            &mut NoopObserver,
        )
        .unwrap();
        file_crypt(
            false,
            &p,
            enc.to_str().unwrap(),
            dec.to_str().unwrap(),
            &mut NoopObserver,
        )
        .unwrap();
        assert_eq!(std::fs::read(&dec).unwrap(), data);

        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&enc);
        let _ = std::fs::remove_file(&dec);
    }
}
