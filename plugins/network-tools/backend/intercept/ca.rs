//! 根证书生成、加载、导出与信息。

use std::path::{Path, PathBuf};

use hudsucker::rcgen::{
    date_time_ymd, BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, Issuer,
    KeyPair, KeyUsagePurpose,
};
use sha2::{Digest, Sha256};
use tauri::Manager;

use crate::error::{code, AppError};

use super::dto::CaInfo;

const CA_DIR: &str = "network-tools/ca";
const KEY_FILE: &str = "ca.key";
const CERT_FILE: &str = "ca.crt";
const META_FILE: &str = "ca.meta.json";
/// 根证书有效期（生成时的固定值，与 meta 保持一致）
const CA_NOT_AFTER_TEXT: &str = "2035-01-01";

/// 已就绪的 CA 材料
pub struct CaMaterial {
    pub issuer: Issuer<'static, KeyPair>,
}

fn ca_dir(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("无法解析应用数据目录: {err}")))?;
    Ok(base.join(CA_DIR))
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(":")
}

/// 写入私钥（Unix 下 0600）
fn write_private(path: &Path, contents: &str) -> Result<(), AppError> {
    std::fs::write(path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

/// 确保 CA 存在并返回材料
pub fn ensure(app: &tauri::AppHandle) -> Result<CaMaterial, AppError> {
    let dir = ca_dir(app)?;
    std::fs::create_dir_all(&dir)?;
    let key_path = dir.join(KEY_FILE);
    let cert_path = dir.join(CERT_FILE);

    if key_path.exists() && cert_path.exists() {
        let key_pem = std::fs::read_to_string(&key_path)?;
        let cert_pem = std::fs::read_to_string(&cert_path)?;
        let key = KeyPair::from_pem(&key_pem)
            .map_err(|err| AppError::custom(code::IO_ERROR, format!("CA 私钥解析失败: {err}")))?;
        let issuer = Issuer::from_ca_cert_pem(&cert_pem, key)
            .map_err(|err| AppError::custom(code::IO_ERROR, format!("CA 证书解析失败: {err}")))?;
        return Ok(CaMaterial { issuer });
    }

    let key = KeyPair::generate()
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("CA 私钥生成失败: {err}")))?;
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.not_before = date_time_ymd(2020, 1, 1);
    params.not_after = date_time_ymd(2035, 1, 1); // 与 CA_NOT_AFTER_TEXT 对齐
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "ArkDesk Network Tools CA");
    dn.push(DnType::OrganizationName, "ArkDesk");
    params.distinguished_name = dn;

    let cert = params
        .self_signed(&key)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("CA 证书生成失败: {err}")))?;
    let cert_pem = cert.pem();
    let key_pem = key.serialize_pem();

    std::fs::write(&cert_path, &cert_pem)?;
    write_private(&key_path, &key_pem)?;
    let meta = serde_json::json!({
        "notAfter": CA_NOT_AFTER_TEXT,
        "fingerprint": sha256_hex(cert_pem.as_bytes()),
    });
    let meta_text = serde_json::to_string_pretty(&meta)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("CA 元数据序列化失败: {err}")))?;
    std::fs::write(dir.join(META_FILE), meta_text)?;

    let key = KeyPair::from_pem(&key_pem)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("CA 私钥解析失败: {err}")))?;
    let issuer = Issuer::from_ca_cert_pem(&cert_pem, key)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("CA 证书解析失败: {err}")))?;
    Ok(CaMaterial { issuer })
}

/// 读取 CA 信息
pub fn info(app: &tauri::AppHandle) -> CaInfo {
    let Ok(dir) = ca_dir(app) else {
        return CaInfo::default();
    };
    let cert_path = dir.join(CERT_FILE);
    if !cert_path.exists() {
        return CaInfo::default();
    }
    let mut fingerprint = std::fs::read_to_string(&cert_path)
        .ok()
        .map(|pem| sha256_hex(pem.as_bytes()));
    let mut not_after = None;
    if let Ok(meta_text) = std::fs::read_to_string(dir.join(META_FILE)) {
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_text) {
            fingerprint = meta
                .get("fingerprint")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .or(fingerprint);
            not_after = meta
                .get("notAfter")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
        }
    }
    CaInfo {
        exists: true,
        fingerprint,
        not_after,
        cert_path: Some(cert_path.to_string_lossy().to_string()),
    }
}

/// 导出证书（PEM）到指定路径
pub fn export(app: &tauri::AppHandle, path: &str) -> Result<(), AppError> {
    let dir = ca_dir(app)?;
    let cert_path = dir.join(CERT_FILE);
    if !cert_path.exists() {
        return Err(AppError::not_found("CA 证书尚未生成，请先启动一次代理"));
    }
    let pem = std::fs::read_to_string(&cert_path)?;
    std::fs::write(path, pem)?;
    Ok(())
}

/// 删除并重新生成 CA（调用方需确保代理已停止）
pub fn regenerate(app: &tauri::AppHandle) -> Result<CaInfo, AppError> {
    let dir = ca_dir(app)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    ensure(app)?;
    Ok(info(app))
}
