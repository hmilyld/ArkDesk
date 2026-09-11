//! 应用在线更新（框架级能力）。
//!
//! 采用官方 `tauri-plugin-updater`：签名公钥固化在 `tauri.conf.json`
//! （信任根不可被界面篡改），更新服务器地址由前端设置传入并在运行时覆盖端点。
//! 仅 HTTPS：非 https 地址会在 `endpoints()` 校验阶段被拒绝。
//!
//! 命令：`updater_check`（仅检查）→ `updater_install`（下载并安装，期间经
//! `updater://progress` 事件回传进度）→ `updater_restart`（macOS/Linux 安装后重启）。

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

use crate::error::{self, AppError};
use crate::events;

/// 检查结果（`None` 表示无可用更新）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// 服务器公告的新版本号
    pub version: String,
    /// 当前安装版本号
    pub current_version: String,
    /// 更新说明（服务器 `notes` 字段，Markdown）
    pub notes: Option<String>,
    /// 发布日期（RFC 3339）
    pub date: Option<String>,
}

/// 下载进度事件负载
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    downloaded: usize,
    total: Option<u64>,
}

fn update_error(message: impl Into<String>) -> AppError {
    AppError::custom(error::code::UPDATE_ERROR, message)
}

/// 规范化更新端点：
/// - 非空校验
/// - 强制 HTTPS（官方插件安全默认，此处提前给出可读错误）
/// - 未显式指向清单且未使用 Tauri 模板占位符时，自动补 `/latest.json`
fn build_endpoint(raw: &str) -> Result<tauri::Url, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("未配置更新服务器地址"));
    }

    let mut normalized = trimmed.trim_end_matches('/').to_string();
    if !normalized.contains(".json") && !normalized.contains("{{") {
        normalized.push_str("/latest.json");
    }

    let url = normalized
        .parse::<tauri::Url>()
        .map_err(|err| AppError::invalid_input(format!("更新服务器地址非法: {err}")))?;

    if url.scheme() != "https" {
        return Err(AppError::invalid_input("更新服务器地址必须使用 HTTPS"));
    }

    Ok(url)
}

/// 使用运行时端点构建 updater（公钥继承 tauri.conf.json 内置配置）
fn build_updater(
    app: &AppHandle,
    endpoint: &str,
) -> Result<tauri_plugin_updater::Updater, AppError> {
    let url = build_endpoint(endpoint)?;
    app.updater_builder()
        .endpoints(vec![url])
        .map_err(|err| update_error(format!("更新端点无效: {err}")))?
        .build()
        .map_err(|err| update_error(format!("初始化更新器失败: {err}")))
}

/// 检查更新（不下载）
#[tauri::command]
pub async fn updater_check(
    app: AppHandle,
    endpoint: String,
) -> Result<Option<UpdateInfo>, AppError> {
    let updater = build_updater(&app, &endpoint)?;
    let update = updater
        .check()
        .await
        .map_err(|err| update_error(format!("检查更新失败: {err}")))?;

    Ok(update.map(|update| UpdateInfo {
        version: update.version,
        current_version: update.current_version,
        notes: update.body,
        date: update.date.map(|date| date.to_string()),
    }))
}

/// 下载并安装更新（进度经 `updater://progress` 事件回传）
#[tauri::command]
pub async fn updater_install(app: AppHandle, endpoint: String) -> Result<(), AppError> {
    let updater = build_updater(&app, &endpoint)?;
    let update = updater
        .check()
        .await
        .map_err(|err| update_error(format!("检查更新失败: {err}")))?
        .ok_or_else(|| update_error("当前已是最新版本"))?;

    let progress_app = app.clone();
    update
        .download_and_install(
            move |downloaded, total| {
                let _ = progress_app.emit(
                    events::UPDATER_PROGRESS,
                    ProgressPayload { downloaded, total },
                );
            },
            || {},
        )
        .await
        .map_err(|err| update_error(format!("更新下载或安装失败: {err}")))?;

    let _ = app.emit(events::UPDATER_INSTALLED, ());
    Ok(())
}

/// 重启应用以运行新版本（Windows 由安装器自行处理，其余平台安装后调用）
#[tauri::command]
pub fn updater_restart(app: AppHandle) -> Result<(), AppError> {
    app.restart()
}
