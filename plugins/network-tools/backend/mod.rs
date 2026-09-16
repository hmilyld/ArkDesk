//! network-tools 插件后端。
//!
//! - HTTP 传输走框架 `http_send`；数据经 `@/core/db`
//! - 「请求拦截」（http-interceptor）由 `intercept/` 提供：本地 MITM 代理
//! - 命令须定义在本文件（构建期扫描 `#[tauri::command]` 自动登记），一律 `network_tools_` 前缀

pub mod intercept;
pub mod migrations;

use crate::error::AppError;

use intercept::dto::{CaInfo, FlowRecord, FlowSummary, ProxyConfig, ProxyStatus, WsRecord, WsSummary};

// ─────────────────────────── 代理生命周期 ───────────────────────────

#[tauri::command]
pub async fn network_tools_proxy_start(
    app: tauri::AppHandle,
    port: u16,
    config: ProxyConfig,
) -> Result<ProxyStatus, AppError> {
    intercept::start(app, port, config).await
}

#[tauri::command]
pub async fn network_tools_proxy_stop(app: tauri::AppHandle) -> Result<ProxyStatus, AppError> {
    Ok(intercept::stop(&app).await)
}

#[tauri::command]
pub fn network_tools_proxy_status(app: tauri::AppHandle) -> Result<ProxyStatus, AppError> {
    Ok(intercept::status(&app))
}

#[tauri::command]
pub fn network_tools_proxy_configure(config: ProxyConfig) -> Result<(), AppError> {
    intercept::configure(config);
    Ok(())
}

// ─────────────────────────── 流量 ───────────────────────────

#[tauri::command]
pub fn network_tools_flows_list() -> Result<Vec<FlowSummary>, AppError> {
    Ok(intercept::flows_list())
}

#[tauri::command]
pub fn network_tools_flow_get(id: u64) -> Result<FlowRecord, AppError> {
    intercept::flow_get(id)
}

#[tauri::command]
pub fn network_tools_flows_clear() -> Result<(), AppError> {
    intercept::flows_clear();
    Ok(())
}

#[tauri::command]
pub fn network_tools_ws_list() -> Result<Vec<WsSummary>, AppError> {
    Ok(intercept::ws_list())
}

#[tauri::command]
pub fn network_tools_ws_get(id: u64) -> Result<WsRecord, AppError> {
    intercept::ws_get(id)
}

#[tauri::command]
pub fn network_tools_ws_clear() -> Result<(), AppError> {
    intercept::ws_clear();
    Ok(())
}

#[tauri::command]
pub fn network_tools_flow_body_temp(
    app: tauri::AppHandle,
    id: u64,
    side: String,
) -> Result<String, AppError> {
    intercept::flow_body_temp(&app, id, &side)
}

// ─────────────────────────── 系统代理 ───────────────────────────

#[tauri::command]
pub async fn network_tools_system_proxy_enable(
    app: tauri::AppHandle,
    port: u16,
) -> Result<intercept::dto::SystemProxyStatus, AppError> {
    intercept::system_proxy_enable(&app, port).await
}

#[tauri::command]
pub async fn network_tools_system_proxy_disable(
    app: tauri::AppHandle,
) -> Result<intercept::dto::SystemProxyStatus, AppError> {
    intercept::system_proxy_disable(&app).await
}

#[tauri::command]
pub fn network_tools_system_proxy_status(
    app: tauri::AppHandle,
) -> Result<intercept::dto::SystemProxyStatus, AppError> {
    Ok(intercept::system_proxy_status(&app))
}

// ─────────────────────────── 根证书 ───────────────────────────

#[tauri::command]
pub fn network_tools_ca_info(app: tauri::AppHandle) -> Result<CaInfo, AppError> {
    Ok(intercept::ca_info(&app))
}

#[tauri::command]
pub fn network_tools_ca_export(app: tauri::AppHandle, path: String) -> Result<(), AppError> {
    intercept::ca_export(&app, &path)
}

#[tauri::command]
pub fn network_tools_ca_regenerate(app: tauri::AppHandle) -> Result<CaInfo, AppError> {
    intercept::ca_regenerate(&app)
}
