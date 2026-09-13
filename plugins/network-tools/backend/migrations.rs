//! network-tools 数据库迁移。
//! scope = 插件 id（`network-tools`），version 在作用域内从 1 递增；已发布迁移不可修改，只能追加。

use crate::db::migration;

pub fn all() -> Vec<crate::db::Migration> {
    vec![
        migration(
            "network-tools",
            1,
            "network_tools_create_collections_requests_drafts",
            "CREATE TABLE IF NOT EXISTS network_tools_collections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                parent_id INTEGER,
                type TEXT NOT NULL DEFAULT 'folder',
                name TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );
            CREATE TABLE IF NOT EXISTS network_tools_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collection_id INTEGER,
                name TEXT NOT NULL,
                method TEXT NOT NULL DEFAULT 'GET',
                url TEXT NOT NULL DEFAULT '',
                query TEXT NOT NULL DEFAULT '[]',
                headers TEXT NOT NULL DEFAULT '[]',
                cookies TEXT NOT NULL DEFAULT '[]',
                body_type TEXT NOT NULL DEFAULT 'none',
                body_text TEXT NOT NULL DEFAULT '',
                body_lang TEXT NOT NULL DEFAULT 'json',
                form_fields TEXT NOT NULL DEFAULT '[]',
                multipart_fields TEXT NOT NULL DEFAULT '[]',
                binary_path TEXT NOT NULL DEFAULT '',
                auth_type TEXT NOT NULL DEFAULT 'none',
                auth_config TEXT NOT NULL DEFAULT '{}',
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );
            CREATE TABLE IF NOT EXISTS network_tools_drafts (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                payload TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );",
        ),
        migration(
            "network-tools",
            2,
            "network_tools_create_environments_vars",
            "CREATE TABLE IF NOT EXISTS network_tools_environments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );
            CREATE TABLE IF NOT EXISTS network_tools_env_vars (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                environment_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                is_secret INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0
            );",
        ),
        migration(
            "network-tools",
            3,
            "network_tools_create_history",
            "CREATE TABLE IF NOT EXISTS network_tools_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                request TEXT NOT NULL DEFAULT '{}',
                status INTEGER,
                ok INTEGER,
                elapsed_ms INTEGER,
                size_bytes INTEGER,
                response_headers TEXT NOT NULL DEFAULT '[]',
                response_body TEXT,
                body_truncated INTEGER NOT NULL DEFAULT 0,
                error TEXT,
                environment_id INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
            );",
        ),
    ]
}
