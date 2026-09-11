//! 模板插件后端（示例）。复制插件目录后按需改写：
//! - 命令函数名 = 前端调用名（本插件 id 为 template，故用 `template_` 前缀防冲突），
//!   一律返回 `Result<T, AppError>`。
//! - 命令须定义在 `backend/mod.rs`（构建期扫描此文件自动登记，无需任何手动注册）。
//! - 有数据库表时在 `backend/migrations.rs` 实现 `all()`，并在 `plugin.json` 按需声明
//!   `legacyMigrations`（旧全局版本 → 本作用域内版本）。无数据库需求可删除该文件与本行。

pub mod migrations;

use crate::error::AppError;

/// 示例命令：前端 `await ipc('template_hello', { name })`
#[tauri::command]
pub fn template_hello(name: String) -> Result<String, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::invalid_input("名字不能为空"));
    }
    Ok(format!("Hello, {name}!"))
}
