//! 加解密命令的请求/响应 DTO。
//!
//! `#[tauri::command]` 必须定义在 `backend/mod.rs`（构建期只扫描该文件），
//! 故这里只放数据结构。字段统一 `camelCase`，与前端调用参数对齐。

use serde::{Deserialize, Serialize};

/// 对称加解密（文本）请求。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymmetricRequest {
    /// encrypt / decrypt
    pub operation: String,
    pub algorithm: String,
    /// ecb / cbc / ctr / gcm
    pub mode: String,
    #[serde(default)]
    pub data: String,
    pub data_encoding: Option<String>,
    pub output: Option<String>,
    pub key: Option<String>,
    pub key_encoding: Option<String>,
    pub iv: Option<String>,
    pub iv_encoding: Option<String>,
    pub passphrase: Option<String>,
    /// evp / pbkdf2
    pub kdf: Option<String>,
    pub salt: Option<String>,
    pub salt_encoding: Option<String>,
    pub rounds: Option<u32>,
    pub aad: Option<String>,
    pub aad_encoding: Option<String>,
}

/// 对称加解密（文件）请求。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymmetricFileRequest {
    pub operation: String,
    pub algorithm: String,
    pub mode: String,
    #[serde(default)]
    pub input_path: String,
    #[serde(default)]
    pub output_path: String,
    pub key: Option<String>,
    pub key_encoding: Option<String>,
    pub iv: Option<String>,
    pub iv_encoding: Option<String>,
    pub passphrase: Option<String>,
    pub kdf: Option<String>,
    pub salt: Option<String>,
    pub salt_encoding: Option<String>,
    pub rounds: Option<u32>,
}

/// 随机密钥与 IV。
#[derive(Serialize)]
pub struct SymmetricKey {
    pub key: String,
    pub iv: String,
}

/// RSA / SM2 密钥对（PEM）。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPair {
    pub private_key: String,
    pub public_key: String,
}

/// 非对称操作请求（RSA / SM2 共用）。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsymRequest {
    /// 公钥或私钥 PEM（依操作而定）；缺失按空串处理，交由业务层给出友好提示
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub data: String,
    pub data_encoding: Option<String>,
    pub output: Option<String>,
    /// RSA：oaep / oaep-sha1 / pkcs1v15；签名时 pss / pkcs1v15
    pub padding: Option<String>,
    pub hash: Option<String>,
    /// SM2：asn1 / raw
    pub format: Option<String>,
    pub signature: Option<String>,
    pub signature_encoding: Option<String>,
    /// SM2 签名者 ID
    pub id: Option<String>,
}

/// 口令派生请求。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfRequest {
    pub password: String,
    pub salt: String,
    pub salt_encoding: Option<String>,
    /// pbkdf2 / scrypt / argon2
    pub algorithm: String,
    pub length: usize,
    pub output: Option<String>,
    pub rounds: Option<u32>,
    pub log_n: Option<u8>,
    pub r: Option<u32>,
    pub p: Option<u32>,
    pub memory_kib: Option<u32>,
    pub iterations: Option<u32>,
    pub parallelism: Option<u32>,
}
