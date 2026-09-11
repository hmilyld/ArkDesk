//! 后台任务框架（框架能力）。
//!
//! 提供按 task id 的取消令牌与统一的进度事件（`task://progress` / `task://done` /
//! `task://error`），前端经 `core/tasks` 消费。
//!
//! 用法（Rust 侧）：
//!   let cancel = tasks::begin("my-task");      // 注册并取得取消标志
//!   if cancel.load(Ordering::Relaxed) { ... }
//!   tasks::emit_progress(app, "my-task", done, total, message);
//!   tasks::end("my-task");
//! 前端 `task_cancel` 命令可取消指定任务。

// 框架对外 API：base 可能没有消费方（插件接入后使用），故允许暂时未使用
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::events;

static TASKS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();

fn tasks() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 注册任务并返回取消标志（重复 id 会取消旧任务后再注册）
pub fn begin(id: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut map) = tasks().lock() {
        if let Some(previous) = map.insert(id.to_string(), flag.clone()) {
            previous.store(true, Ordering::Relaxed);
        }
    }
    flag
}

/// 取任务的取消标志（未注册返回 None）
pub fn flag(id: &str) -> Option<Arc<AtomicBool>> {
    tasks().lock().ok().and_then(|map| map.get(id).cloned())
}

/// 任务是否已取消（未注册视为未取消）
#[allow(dead_code)]
pub fn is_cancelled(id: &str) -> bool {
    flag(id).is_some_and(|flag| flag.load(Ordering::Relaxed))
}

/// 结束任务（清理注册；若已注册则置为取消以通知仍在等待的循环）
pub fn end(id: &str) {
    if let Ok(mut map) = tasks().lock() {
        if let Some(flag) = map.remove(id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

/// 取消任务
pub fn cancel(id: &str) {
    if let Some(flag) = flag(id) {
        flag.store(true, Ordering::Relaxed);
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskProgress {
    task_id: String,
    done: u64,
    total: Option<u64>,
    message: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskRef {
    task_id: String,
}

/// 回传任务进度
pub fn emit_progress<R: Runtime>(
    app: &AppHandle<R>,
    task_id: &str,
    done: u64,
    total: Option<u64>,
    message: Option<String>,
) {
    let _ = app.emit(
        events::TASK_PROGRESS,
        TaskProgress {
            task_id: task_id.to_string(),
            done,
            total,
            message,
        },
    );
}

/// 回传任务完成
pub fn emit_done<R: Runtime>(app: &AppHandle<R>, task_id: &str) {
    let _ = app.emit(
        events::TASK_DONE,
        TaskRef {
            task_id: task_id.to_string(),
        },
    );
}

/// 回传任务失败
pub fn emit_error<R: Runtime>(app: &AppHandle<R>, task_id: &str, message: &str) {
    let _ = app.emit(
        events::TASK_ERROR,
        serde_json::json!({ "taskId": task_id, "message": message }),
    );
}

/// 取消指定任务（前端调用）
#[tauri::command]
pub fn task_cancel(task_id: String) {
    log::info!("取消任务: {task_id}");
    cancel(&task_id);
}
