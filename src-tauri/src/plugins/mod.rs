//! 插件后端聚合（构建期自动生成，请勿手改本文件内容）。
//!
//! `build.rs` 扫描仓库根 `plugins/*/backend/mod.rs`，解析 `#[tauri::command]`
//! 生成命令注册与迁移聚合，写入 `$OUT_DIR/plugin_registry.rs`，此处 include。
//! 新增插件只需在 `plugins/<id>/backend/` 下写命令，无需任何登记。

include!(concat!(env!("OUT_DIR"), "/plugin_registry.rs"));
