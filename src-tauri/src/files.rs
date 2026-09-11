//! 通用文件读写命令（框架级）。
//!
//! 供「设置导入/导出」「诊断导出」及插件处理用户选择的文件使用。
//! 路径由前端经系统对话框取得；CSP 严格、命令不加载远程内容的边界不变。
//! 二进制内容以 base64 经 IPC 传输（前端自行解码为 Uint8Array / data URL）。

use base64::Engine;

use crate::error::AppError;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// 写入文本文件（自动创建父目录）
#[tauri::command]
pub async fn file_write_text(path: String, contents: String) -> Result<(), AppError> {
    let target = std::path::Path::new(&path);
    if let Some(dir) = target.parent() {
        if !dir.as_os_str().is_empty() {
            tokio::fs::create_dir_all(dir).await?;
        }
    }
    tokio::fs::write(target, contents).await?;
    Ok(())
}

/// 读取文本文件
#[tauri::command]
pub async fn file_read_text(path: String) -> Result<String, AppError> {
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::not_found(format!("文件不存在: {path}")));
    }
    Ok(tokio::fs::read_to_string(path).await?)
}

/// 读取文件为 base64 字符串（二进制）
#[tauri::command]
pub async fn file_read_bytes(path: String) -> Result<String, AppError> {
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::not_found(format!("文件不存在: {path}")));
    }
    let bytes = tokio::fs::read(path).await?;
    Ok(B64.encode(bytes))
}

/// 将 base64 内容写入文件（自动创建父目录）
#[tauri::command]
pub async fn file_write_bytes(path: String, contents: String) -> Result<(), AppError> {
    let bytes = B64
        .decode(contents.as_bytes())
        .map_err(|err| AppError::invalid_input(format!("base64 解码失败: {err}")))?;
    let target = std::path::Path::new(&path);
    if let Some(dir) = target.parent() {
        if !dir.as_os_str().is_empty() {
            tokio::fs::create_dir_all(dir).await?;
        }
    }
    tokio::fs::write(target, bytes).await?;
    Ok(())
}
