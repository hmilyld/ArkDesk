//! text2video 插件的数据库迁移定义。
//! scope = 插件 id；version 在作用域内从 1 递增（各插件互不干扰）。

use crate::db::migration;

pub fn all() -> Vec<crate::db::Migration> {
    vec![
        migration(
            "text2video",
            1,
            "text2video_create_processed",
            "CREATE TABLE IF NOT EXISTS text2video_processed (
            ref_id TEXT PRIMARY KEY,
            kind TEXT,
            title TEXT,
            status TEXT,
            detail TEXT,
            video TEXT,
            meta TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );",
        ),
        migration(
            "text2video",
            2,
            "text2video_create_drafts",
            "CREATE TABLE IF NOT EXISTS text2video_drafts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            author TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'manual',
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );",
        ),
    ]
}
