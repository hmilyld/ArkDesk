//! 可选：数据库迁移。
//! scope = 插件 id；version 在作用域内从 1 递增（各插件互不干扰）。
//! 无数据库需求时可删除本文件，并移除 mod.rs 中的 `pub mod migrations;`。

pub fn all() -> Vec<crate::db::Migration> {
    // 示例（建表 DDL 的列默认值需与 frontend/schema.ts 对齐）：
    // vec![crate::db::migration(
    //     "template",
    //     1,
    //     "template_create_items",
    //     "CREATE TABLE IF NOT EXISTS template_items (
    //         id INTEGER PRIMARY KEY AUTOINCREMENT,
    //         title TEXT NOT NULL
    //     );",
    // )]
    vec![]
}
