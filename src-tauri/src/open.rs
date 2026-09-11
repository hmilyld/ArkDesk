//! 统一的「打开内容」入口：把 CLI 参数、深链接、二次启动参数收敛为 `app://open`。
//!
//! - 启动早期（webview 未就绪）：参数先入 pending 队列，前端经 `take_pending_open` 取走；
//! - 运行时（二次启动 / 深链接）：直接 emit 事件给已就绪的前端。

use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Runtime};

use crate::events;

static PENDING_OPEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[derive(Clone, Serialize)]
pub struct OpenPayload {
    pub paths: Vec<String>,
    /// 来源：cli / deep-link / drag-drop
    pub source: String,
}

/// 过滤参数：忽略选项（- 开头），保留存在的文件路径或含 `://` 的 URL
pub fn normalize_args(args: impl IntoIterator<Item = String>) -> Vec<String> {
    args.into_iter()
        .filter(|arg| !arg.starts_with('-'))
        .filter(|arg| std::path::Path::new(arg).exists() || arg.contains("://"))
        .collect()
}

/// 启动阶段：存入 pending 队列（webview 尚未就绪）
pub fn stash_pending(paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    if let Ok(mut pending) = PENDING_OPEN.lock() {
        pending.extend(paths);
    }
}

/// 运行阶段：直接向已就绪的前端发送
pub fn emit_open<R: Runtime>(app: &AppHandle<R>, paths: Vec<String>, source: &str) {
    if paths.is_empty() {
        return;
    }
    let _ = app.emit_to(
        "main",
        events::APP_OPEN,
        OpenPayload {
            paths,
            source: source.to_string(),
        },
    );
}

/// 前端就绪后一次性取走启动参数
#[tauri::command]
pub fn take_pending_open() -> Vec<String> {
    PENDING_OPEN
        .lock()
        .map(|mut pending| std::mem::take(&mut *pending))
        .unwrap_or_default()
}
