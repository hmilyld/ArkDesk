//! 请求拦截器 DTO（serde camelCase，与前端字段对齐）。

use serde::{Deserialize, Serialize};

/// 头部键值对（保序，保留重复项）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderPair {
    pub name: String,
    pub value: String,
}

/// 代理运行配置（前端可热更新）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    #[serde(default = "default_true")]
    pub record_bodies: bool,
    #[serde(default = "default_max_body_kb")]
    pub max_body_kb: u64,
    #[serde(default = "default_max_flows")]
    pub max_flows: usize,
    #[serde(default = "default_max_ws_frames")]
    pub max_ws_frames: usize,
}

fn default_true() -> bool {
    true
}
fn default_max_body_kb() -> u64 {
    1024
}
fn default_max_flows() -> usize {
    300
}
fn default_max_ws_frames() -> usize {
    500
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            record_bodies: true,
            max_body_kb: default_max_body_kb(),
            max_flows: default_max_flows(),
            max_ws_frames: default_max_ws_frames(),
        }
    }
}

/// 根证书信息
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaInfo {
    pub exists: bool,
    pub fingerprint: Option<String>,
    pub not_after: Option<String>,
    pub cert_path: Option<String>,
}

/// 系统代理托管状态
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemProxyStatus {
    pub supported: bool,
    pub enabled: bool,
    pub detail: String,
}

/// 代理状态
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub flow_count: usize,
    pub ws_count: usize,
    pub ca: CaInfo,
}

/// 流量摘要（列表用，不含头部/体）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowSummary {
    pub id: u64,
    pub started_at: i64,
    pub method: String,
    pub scheme: String,
    pub host: String,
    pub url: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub req_size: u64,
    pub res_size: u64,
    pub error: Option<String>,
    pub is_upgrade: bool,
    /// 响应 Content-Type（用于前端资源类型分类）
    pub content_type: Option<String>,
}

/// 体抓取结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyCapture {
    pub text: Option<String>,
    pub base64: Option<String>,
    pub is_binary: bool,
    pub size: u64,
    pub truncated: bool,
    pub content_type: Option<String>,
    /// 是否已按 Content-Encoding 解压
    pub decoded: bool,
    pub note: Option<String>,
}

/// 完整流量记录（详情用）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowRecord {
    pub summary: FlowSummary,
    pub client_addr: String,
    pub req_headers: Vec<HeaderPair>,
    pub res_headers: Vec<HeaderPair>,
    pub req_body: Option<BodyCapture>,
    pub res_body: Option<BodyCapture>,
}

/// WebSocket 帧
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsFrame {
    /// "up"（客户端→服务端）| "down"
    pub dir: String,
    pub opcode: String,
    pub len: usize,
    pub text: Option<String>,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsSummary {
    pub id: u64,
    pub host: String,
    pub url: String,
    pub frame_count: usize,
    pub started_at: i64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsRecord {
    pub summary: WsSummary,
    pub frames: Vec<WsFrame>,
}

/// 当前毫秒时间戳
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
