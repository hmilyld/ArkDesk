//! hello-world 示例插件的数据库迁移定义。
//! scope = 插件 id；version 在作用域内从 1 递增（各插件互不干扰）。

use crate::db::migration;

pub fn all() -> Vec<crate::db::Migration> {
    vec![
        migration(
            "hello-world",
            1,
            "hello_world_create_notes",
            "CREATE TABLE IF NOT EXISTS hello_notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );",
        ),
        migration(
            "hello-world",
            2,
            "hello_world_create_tasks",
            "CREATE TABLE IF NOT EXISTS hello_tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            priority TEXT NOT NULL DEFAULT 'medium',
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
        INSERT INTO hello_tasks (title, status, priority) VALUES
            ('调研 SQLite 迁移方案', 'done', 'high'),
            ('搭建插件系统骨架', 'done', 'high'),
            ('编写示例工具集', 'in_progress', 'medium'),
            ('缩放体验优化', 'pending', 'low');",
        ),
    ]
}
