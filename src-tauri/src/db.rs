//! SQLite 数据层（自建 sqlx 通道）。
//!
//! - 连接池直连 `app_data_dir/arkdesk.db`（WAL、外键约束、lazy 连接）
//! - 自建迁移器：与 tauri-plugin-sql 的 `_sqlx_migrations` 表结构完全兼容
//!   （历史库无损沿用；plugin-sql 已退役，本模块为唯一数据通道）
//! - 对前端暴露两个通用命令：`db_query_values`（按列序返回值数组，供 Drizzle
//!   proxy 驱动消费）与 `db_execute`（返回 rowsAffected / lastInsertId）

use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteRow,
};
use sqlx::{Column, Decode, Row, Sqlite, TypeInfo, ValueRef};
use tauri::Manager;

use crate::error::{code, AppError};

const DB_FILENAME: &str = "arkdesk.db";

/// 作用域迁移记录表 DDL：以 (scope, version) 为主键，框架与各插件各自从 1 编号，
/// 独立演进、互不撞号。
const PLUGIN_MIGRATIONS_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS plugin_migrations (
    scope TEXT NOT NULL,
    version INTEGER NOT NULL,
    description TEXT NOT NULL,
    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN NOT NULL,
    checksum BLOB NOT NULL,
    execution_time BIGINT NOT NULL,
    PRIMARY KEY (scope, version)
);";

/// 迁移 SQL 的轻量校验和（FNV-1a 64 位），写入 checksum 列。
/// 历史库（plugin-sql 写入）的 checksum 为哈希字节，本实现不参与其语义校验，
/// 仅保证新写入的迁移记录有值；已发布迁移不可修改是团队约定而非运行时强制。
fn migration_checksum(sql: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in sql.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

static DB_PATH: OnceLock<String> = OnceLock::new();
static POOL: OnceLock<SqlitePool> = OnceLock::new();

/// setup 阶段调用：解析并固定数据库文件路径（池 lazy 初始化）
pub fn init(app: &tauri::AppHandle) {
    let dir = app.path().app_data_dir().expect("无法解析应用数据目录");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        panic!("创建应用数据目录失败: {err}");
    }
    let path = dir.join(DB_FILENAME);
    DB_PATH
        .set(path.to_string_lossy().to_string())
        .expect("数据库路径重复初始化");
}

fn pool() -> &'static SqlitePool {
    POOL.get_or_init(|| {
        let path = DB_PATH.get().expect("数据库路径未初始化（应先调用 init）");
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        SqlitePoolOptions::new()
            .max_connections(5)
            .connect_lazy_with(options)
    })
}

/// 迁移定义（各插件在 backend/migrations.rs 中声明，构建期自动聚合）。
/// `scope` 为作用域（通常 = 插件 id），version 在每个作用域内独立递增。
#[derive(Debug)]
pub struct Migration {
    pub scope: &'static str,
    pub version: i64,
    pub description: &'static str,
    pub sql: &'static str,
}

/// 便捷构造
pub const fn migration(
    scope: &'static str,
    version: i64,
    description: &'static str,
    sql: &'static str,
) -> Migration {
    Migration {
        scope,
        version,
        description,
        sql,
    }
}

/// 旧库桥接项：把历史全局编号迁移映射到新作用域编号（仅登记、不重复执行）。
/// 由各插件 `plugin.json` 的 `legacyMigrations` 声明，构建期聚合。
#[derive(Debug)]
pub struct LegacyAdoption {
    pub scope: &'static str,
    /// (旧全局版本, 新作用域内版本)
    pub versions: &'static [(i64, i64)],
}

/// 执行迁移：幂等，按 (scope, version) 只应用未记录的迁移（每个独立事务）。
pub async fn migrate() -> Result<(), AppError> {
    migrate_pool(pool()).await
}

/// 迁移实现（连接池可注入，便于测试）。
async fn migrate_pool(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::raw_sql(PLUGIN_MIGRATIONS_TABLE_SQL)
        .execute(pool)
        .await
        .map_err(db_err("迁移表初始化"))?;

    adopt_legacy_migrations(pool).await?;

    let applied: HashSet<(String, i64)> =
        sqlx::query_as::<_, (String, i64)>("SELECT scope, version FROM plugin_migrations")
            .fetch_all(pool)
            .await
            .map_err(db_err("读取迁移记录"))?
            .into_iter()
            .collect();

    for migration in crate::plugins::collect_migrations() {
        if applied.contains(&(migration.scope.to_string(), migration.version)) {
            continue;
        }
        log::info!(
            "应用迁移 [{}] v{}: {}",
            migration.scope,
            migration.version,
            migration.description
        );
        let started = Instant::now();

        let mut tx = pool.begin().await.map_err(db_err("开启迁移事务"))?;
        sqlx::raw_sql(migration.sql)
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                AppError::custom(
                    code::DB_ERROR,
                    format!(
                        "迁移 [{}] v{} 失败: {err}",
                        migration.scope, migration.version
                    ),
                )
            })?;
        sqlx::query(
            "INSERT INTO plugin_migrations (scope, version, description, success, checksum, execution_time)
             VALUES (?, ?, ?, 1, ?, ?)",
        )
        .bind(migration.scope)
        .bind(migration.version)
        .bind(migration.description)
        .bind(migration_checksum(migration.sql).to_be_bytes().to_vec())
        .bind(started.elapsed().as_millis() as i64)
        .execute(&mut *tx)
        .await
        .map_err(db_err("记录迁移版本"))?;
        tx.commit().await.map_err(db_err("提交迁移事务"))?;
    }

    Ok(())
}

/// 旧库桥接：若存在历史 `_sqlx_migrations`（tauri-plugin-sql 遗留），
/// 依据各插件声明的映射把「已应用」项登记进 `plugin_migrations`，避免重复执行。
async fn adopt_legacy_migrations(pool: &SqlitePool) -> Result<(), AppError> {
    let legacy_exists: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(pool)
    .await
    .map_err(db_err("检测旧迁移表"))?;
    if legacy_exists.is_none() {
        return Ok(());
    }

    let old_versions: HashSet<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations")
        .fetch_all(pool)
        .await
        .map_err(db_err("读取旧迁移记录"))?
        .into_iter()
        .collect();

    for adoption in crate::plugins::legacy_adoptions() {
        for (old, new) in adoption.versions {
            if !old_versions.contains(old) {
                continue;
            }
            sqlx::query(
                "INSERT OR IGNORE INTO plugin_migrations (scope, version, description, success, checksum, execution_time)
                 VALUES (?, ?, 'adopted-from-_sqlx_migrations', 1, ?, 0)",
            )
            .bind(adoption.scope)
            .bind(new)
            .bind(Vec::<u8>::new())
            .execute(pool)
            .await
            .map_err(db_err("桥接旧迁移记录"))?;
        }
    }
    Ok(())
}

fn db_err(operation: &'static str) -> impl Fn(sqlx::Error) -> AppError {
    move |err| AppError::custom(code::DB_ERROR, format!("{operation}失败: {err}"))
}

// ─────────────────────────── 通用执行命令 ───────────────────────────

/// SQL 命令参数（前端统一经 ipc 传入）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlArgs {
    pub sql: String,
    #[serde(default)]
    pub params: Vec<serde_json::Value>,
}

/// 查询结果：列名 + 按列序的值数组（Drizzle proxy 驱动直接消费）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuesResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// 执行结果
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteResult {
    pub rows_affected: u64,
    pub last_insert_id: i64,
}

/// 绑定参数：serde_json::Value → sqlx 原生类型
/// （Array/Object 不支持直接绑定，调用方需自行序列化为 JSON 文本）
fn bind_value<'q>(
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    value: &'q serde_json::Value,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    match value {
        serde_json::Value::Null => query.bind(None::<i64>),
        serde_json::Value::Bool(v) => query.bind(*v),
        serde_json::Value::Number(n) if n.is_i64() => query.bind(n.as_i64()),
        serde_json::Value::Number(n) => query.bind(n.as_f64()),
        serde_json::Value::String(s) => query.bind(s.as_str()),
        other => query.bind(other.to_string()),
    }
}

/// BLOB → base64（无损传输；前端需自行解码，避免二进制被文本编码损坏）
fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn row_to_value(row: &SqliteRow, index: usize) -> serde_json::Value {
    // 按「存储类型」分发（try_get_raw 的 type_info 走 sqlite3_value_type，是运行时实际
    // 类型而非列声明类型）：SQL NULL 恒为 null；表达式列（COUNT(*)、pragma_table_info
    // 等表值函数，sqlx 报 "NULL" 声明类型）与 SQLite 动态类型（INTEGER 列存文本、
    // TEXT 列存数字等）都能按实际存储值无损提取。
    let value = match row.try_get_raw(index) {
        Ok(value) => value,
        Err(_) => return serde_json::Value::Null,
    };
    if value.is_null() {
        return serde_json::Value::Null;
    }
    match value.type_info().name() {
        "INTEGER" => <i64 as Decode<Sqlite>>::decode(value).map(serde_json::Value::from),
        "REAL" => <f64 as Decode<Sqlite>>::decode(value).map(serde_json::Value::from),
        "TEXT" => <String as Decode<Sqlite>>::decode(value).map(serde_json::Value::from),
        "BLOB" => <Vec<u8> as Decode<Sqlite>>::decode(value)
            .map(|bytes| serde_json::Value::from(base64_encode(&bytes))),
        _ => Ok(serde_json::Value::Null),
    }
    .unwrap_or(serde_json::Value::Null)
}

/// 查询命令：返回列名 + 按列序的值数组（无类型映射，供 Drizzle 消费）
#[tauri::command]
pub async fn db_query_values(args: SqlArgs) -> Result<ValuesResult, AppError> {
    let mut query = sqlx::query(&args.sql);
    for param in &args.params {
        query = bind_value(query, param);
    }
    let rows: Vec<SqliteRow> = query.fetch_all(pool()).await.map_err(db_err("查询"))?;

    let columns: Vec<String> = rows
        .first()
        .map(|row| {
            row.columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect()
        })
        .unwrap_or_default();
    let data = rows
        .iter()
        .map(|row| {
            (0..row.columns().len())
                .map(|index| row_to_value(row, index))
                .collect()
        })
        .collect();

    Ok(ValuesResult {
        columns,
        rows: data,
    })
}

/// 执行命令：INSERT / UPDATE / DELETE / DDL
#[tauri::command]
pub async fn db_execute(args: SqlArgs) -> Result<ExecuteResult, AppError> {
    let mut query = sqlx::query(&args.sql);
    for param in &args.params {
        query = bind_value(query, param);
    }
    let result = query.execute(pool()).await.map_err(db_err("执行"))?;

    Ok(ExecuteResult {
        rows_affected: result.rows_affected(),
        last_insert_id: result.last_insert_rowid(),
    })
}

/// 事务中的单条语句
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlStatement {
    pub sql: String,
    #[serde(default)]
    pub params: Vec<serde_json::Value>,
}

/// 事务命令参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlTransactionArgs {
    pub statements: Vec<SqlStatement>,
}

/// 事务命令：在单个事务中执行多条语句（任一失败则整体回滚）。
/// 解决连接池下裸 BEGIN/COMMIT 不保证同连接的问题。
#[tauri::command]
pub async fn db_transaction(args: SqlTransactionArgs) -> Result<ExecuteResult, AppError> {
    if args.statements.is_empty() {
        return Err(AppError::invalid_input("事务语句不能为空"));
    }

    let mut tx = pool().begin().await.map_err(db_err("开启事务"))?;
    let mut rows_affected: u64 = 0;
    let mut last_insert_id: i64 = 0;

    for statement in &args.statements {
        let mut query = sqlx::query(&statement.sql);
        for param in &statement.params {
            query = bind_value(query, param);
        }
        let result = query
            .execute(&mut *tx)
            .await
            .map_err(db_err("执行事务语句"))?;
        rows_affected += result.rows_affected();
        last_insert_id = result.last_insert_rowid();
    }

    tx.commit().await.map_err(db_err("提交事务"))?;
    Ok(ExecuteResult {
        rows_affected,
        last_insert_id,
    })
}

/// 备份数据库：WAL 检查点后复制到指定路径，返回该路径
#[tauri::command]
pub async fn db_backup(path: String) -> Result<String, AppError> {
    let db_path = DB_PATH
        .get()
        .ok_or_else(|| AppError::custom(code::DB_ERROR, "数据库路径未初始化"))?;

    if std::path::Path::new(&path).exists() {
        return Err(AppError::invalid_input("备份目标已存在，请另选路径"));
    }

    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(pool())
        .await
        .map_err(db_err("WAL 检查点"))?;

    let src = db_path.clone();
    let dst = path.clone();
    tokio::task::spawn_blocking(move || std::fs::copy(src, dst))
        .await
        .map_err(|err| AppError::custom(code::DB_ERROR, format!("备份任务失败: {err}")))?
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("复制数据库失败: {err}")))?;

    log::info!("数据库已备份: {path}");
    Ok(path)
}

/// 从备份恢复数据库：覆盖当前库并重启应用（重启前关闭连接池）
#[tauri::command]
pub async fn db_restore(app: tauri::AppHandle, path: String) -> Result<(), AppError> {
    let db_path = DB_PATH
        .get()
        .ok_or_else(|| AppError::custom(code::DB_ERROR, "数据库路径未初始化"))?;
    if !std::path::Path::new(&path).exists() {
        return Err(AppError::not_found(format!("备份文件不存在: {path}")));
    }

    // 拒绝「备份文件 = 当前数据库」以避免自我覆盖损坏
    let src = std::fs::canonicalize(&path)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("解析备份路径失败: {err}")))?;
    let dst = std::fs::canonicalize(db_path).unwrap_or_else(|_| std::path::PathBuf::from(db_path));
    if src == dst {
        return Err(AppError::invalid_input("备份文件不能是当前数据库自身"));
    }

    pool().clone().close().await;
    std::fs::copy(&src, &dst)?;
    let _ = std::fs::remove_file(format!("{db_path}-wal"));
    let _ = std::fs::remove_file(format!("{db_path}-shm"));
    log::warn!("数据库已恢复，应用即将重启");
    app.restart();
}

/// 重置数据库：删除库文件并重启应用（重启前关闭连接池）
#[tauri::command]
pub async fn db_reset(app: tauri::AppHandle) -> Result<(), AppError> {
    let db_path = DB_PATH
        .get()
        .ok_or_else(|| AppError::custom(code::DB_ERROR, "数据库路径未初始化"))?;
    pool().clone().close().await;
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_file(format!("{db_path}-wal"));
    let _ = std::fs::remove_file(format!("{db_path}-shm"));
    log::warn!("数据库已重置，应用即将重启");
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrate_is_idempotent() {
        // 使用临时库验证：新库建表 + 应用全部迁移；重复执行不再产生记录
        let temp_dir = std::env::temp_dir().join(format!("arkdesk-test-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        DB_PATH
            .set(temp_dir.join(DB_FILENAME).to_string_lossy().to_string())
            .unwrap();

        migrate().await.expect("首次迁移失败");
        migrate().await.expect("重复迁移失败");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_migrations")
            .fetch_one(pool())
            .await
            .unwrap();
        let total = crate::plugins::collect_migrations().len() as i64;
        assert_eq!(count, total, "迁移记录数应等于定义数（幂等）");

        // 每个 (scope, version) 唯一（作用域内版本独立）
        let distinct: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT scope || ':' || version) FROM plugin_migrations",
        )
        .fetch_one(pool())
        .await
        .unwrap();
        assert_eq!(distinct, total, "作用域内版本应唯一");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    /// 旧库桥接：`_sqlx_migrations` 中的历史版本应被登记进 `plugin_migrations`，
    /// 且不重复执行 DDL（全新临时库中业务表不应被创建）。
    #[tokio::test]
    async fn legacy_migrations_are_adopted_without_rerun() {
        let temp_dir =
            std::env::temp_dir().join(format!("arkdesk-legacy-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(temp_dir.join("legacy.db"))
                .create_if_missing(true),
        )
        .await
        .unwrap();

        // 造出历史库：_sqlx_migrations 记录全局版本 1..=9
        sqlx::raw_sql(
            "CREATE TABLE _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            );
            INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
            VALUES
              (1,'a',1,x'00',0),(2,'b',1,x'00',0),(3,'c',1,x'00',0),(4,'d',1,x'00',0),
              (5,'e',1,x'00',0),(6,'f',1,x'00',0),(7,'g',1,x'00',0),(8,'h',1,x'00',0),
              (9,'i',1,x'00',0);",
        )
        .execute(&pool)
        .await
        .unwrap();

        migrate_pool(&pool).await.expect("旧库迁移失败");

        // 全部作用域迁移均被「桥接登记」，而非执行
        let total = crate::plugins::collect_migrations().len() as i64;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_migrations")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, total, "旧库全部迁移应被桥接登记");

        // 桥接不执行 DDL：全新库中不应出现业务表
        let tender_table: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='tender_companies'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(tender_table.is_none(), "桥接不应执行建表 DDL");

        pool.close().await;
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    /// row_to_value 提取语义回归：独立临时库（不触碰全局 DB_PATH/pool，避免与其他测试竞争）。
    /// 覆盖三点：SQL NULL 保持 null；表达式列（COUNT(*)，sqlx 报 "NULL" 类型）按实际值提取；
    /// 动态类型按存储值提取（INTEGER 列存文本、TEXT 列存数字、TIMESTAMP/BOOLEAN 声明类型）。
    #[tokio::test]
    async fn row_to_value_extracts_values() {
        let temp_dir = std::env::temp_dir().join(format!("arkdesk-r2v-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(temp_dir.join("row-to-value.db"))
                .create_if_missing(true),
        )
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE r2v (
                i INTEGER, t TEXT, r REAL, b BLOB,
                ts TIMESTAMP, bo BOOLEAN, i_notnull INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // 全 NULL 行：所有 nullable 列应保持 null（而非 0/""），NOT NULL 列正常提取
        sqlx::query("INSERT INTO r2v (i, t, r, b, ts, bo, i_notnull) VALUES (NULL, NULL, NULL, NULL, NULL, NULL, 7)")
            .execute(&pool)
            .await
            .unwrap();
        let null_row = sqlx::query("SELECT i, t, r, b, ts, bo, i_notnull FROM r2v")
            .fetch_one(&pool)
            .await
            .unwrap();
        let values: Vec<serde_json::Value> = (0..7).map(|i| row_to_value(&null_row, i)).collect();
        assert_eq!(
            values,
            vec![
                serde_json::Value::Null,
                serde_json::Value::Null,
                serde_json::Value::Null,
                serde_json::Value::Null,
                serde_json::Value::Null,
                serde_json::Value::Null,
                serde_json::Value::from(7),
            ]
        );

        // 表达式列无声明类型（sqlx 报 "NULL"）：COUNT(*) 应提取为数字
        let count_row = sqlx::query("SELECT COUNT(*) AS n FROM r2v")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row_to_value(&count_row, 0), serde_json::Value::from(1));

        // 动态类型：按实际存储值提取；TIMESTAMP/BOOLEAN 声明类型不在静态分支内。
        // 注意 TEXT 亲和列 INSERT 数字会被 SQLite 自动转为文本存储（存 123 → 读出 "123"），
        // INTEGER 亲和列存非数字文本则保持文本存储。
        sqlx::query("INSERT INTO r2v (i, t, r, b, ts, bo, i_notnull) VALUES ('abc', 123, 1.5, x'00', '2026-01-01 00:00:00', 1, 8)")
            .execute(&pool)
            .await
            .unwrap();
        let dyn_row = sqlx::query("SELECT i, t, r, b, ts, bo, i_notnull FROM r2v WHERE i = 'abc'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let dyn_values: Vec<serde_json::Value> =
            (0..7).map(|i| row_to_value(&dyn_row, i)).collect();
        assert_eq!(dyn_values[0], serde_json::Value::from("abc"));
        assert_eq!(dyn_values[1], serde_json::Value::from("123"));
        assert_eq!(dyn_values[2], serde_json::Value::from(1.5));
        assert!(matches!(dyn_values[3], serde_json::Value::String(_)));
        assert_eq!(
            dyn_values[4],
            serde_json::Value::from("2026-01-01 00:00:00")
        );
        assert_eq!(dyn_values[5], serde_json::Value::from(1));

        pool.close().await;
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
