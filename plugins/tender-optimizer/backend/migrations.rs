//! 投标报价插件的数据库迁移定义。
//! scope = 插件 id；version 在作用域内从 1 递增（各插件互不干扰）。

use crate::db::migration;

pub fn all() -> Vec<crate::db::Migration> {
    vec![
        migration(
            "tender-optimizer",
            1,
            "tender_create_companies",
            "CREATE TABLE IF NOT EXISTS tender_companies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            round1_price REAL NOT NULL,
            price_limit REAL NOT NULL,
            company_type TEXT NOT NULL DEFAULT 'U',
            behavior TEXT,
            note TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0
        );",
        ),
        migration(
            "tender-optimizer",
            2,
            "tender_create_scenarios",
            "CREATE TABLE IF NOT EXISTS tender_scenarios (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            reduction_type TEXT NOT NULL DEFAULT 'percent',
            reduction_value REAL NOT NULL,
            participation_rate REAL NOT NULL DEFAULT 1,
            std_dev REAL NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            note TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0
        );",
        ),
        migration(
            "tender-optimizer",
            3,
            "tender_create_params",
            "CREATE TABLE IF NOT EXISTS tender_params (
            id INTEGER PRIMARY KEY,
            w1 REAL NOT NULL DEFAULT -0.15,
            w2 REAL NOT NULL DEFAULT 0.1,
            c REAL NOT NULL DEFAULT 0,
            n1 REAL NOT NULL DEFAULT 1,
            n2 REAL NOT NULL DEFAULT 0.5,
            aggressive_factor REAL NOT NULL DEFAULT 1.3,
            normal_factor REAL NOT NULL DEFAULT 1,
            conservative_factor REAL NOT NULL DEFAULT 0.5,
            min_aux_price_diff REAL NOT NULL DEFAULT 0.02,
            num_simulations INTEGER NOT NULL DEFAULT 100
        );
        INSERT OR IGNORE INTO tender_params (id) VALUES (1);",
        ),
        migration(
            "tender-optimizer",
            4,
            "tender_create_history",
            "CREATE TABLE IF NOT EXISTS tender_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            num_companies INTEGER NOT NULL,
            num_scenarios INTEGER NOT NULL,
            num_simulations INTEGER NOT NULL,
            best_index INTEGER NOT NULL DEFAULT 0,
            best_score REAL,
            best_scenario TEXT,
            target_price REAL,
            base_price REAL,
            companies_json TEXT NOT NULL,
            scenarios_json TEXT NOT NULL,
            params_json TEXT,
            results_json TEXT NOT NULL
        );",
        ),
        migration(
            "tender-optimizer",
            5,
            "tender_add_plan_name",
            "ALTER TABLE tender_params ADD COLUMN plan_name TEXT NOT NULL DEFAULT '';
            ALTER TABLE tender_history ADD COLUMN plan_name TEXT NOT NULL DEFAULT '';",
        ),
    ]
}
