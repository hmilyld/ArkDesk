//! 事件名常量（与前端 `src/core/events` 保持同步）。
//!
//! 统一前缀：`app://`（应用级）、`task://`（后台任务）、`http://`（HTTP）、
//! `updater://`（在线更新）。新增事件时两端同步登记。

pub const APP_OPEN: &str = "app://open";
pub const APP_MENU: &str = "app://menu";
pub const TASK_PROGRESS: &str = "task://progress";
pub const TASK_DONE: &str = "task://done";
pub const TASK_ERROR: &str = "task://error";
pub const HTTP_DOWNLOAD_PROGRESS: &str = "http://download-progress";
pub const UPDATER_PROGRESS: &str = "updater://progress";
pub const UPDATER_INSTALLED: &str = "updater://installed";
