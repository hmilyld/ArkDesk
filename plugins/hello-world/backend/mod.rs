//! hello-world 示例工具：演示「前端 → 命令 → SQLite」完整链路。
//!
//! 本模块同时是「插件如何扩展 Rust 命令」的教学样板：
//! 1. 命令函数名 = 前端调用名（`hello_world_` 前缀防冲突）
//! 2. 一律返回 `Result<T, AppError>`，错误经 ipc 封装在前端统一处理
//! 3. 登记：构建期由 `src-tauri/build.rs` 扫描本文件自动登记，无需手动注册

pub mod migrations;

use crate::db::SqlArgs;
use crate::error::{code, AppError};
use serde_json::Value;
use tauri::Manager;

/// 默认问候模板。前端 shared.ts 里有同值副本用于展示兜底，
/// 此处为最终回退（前端留空时生效），两者需同步修改。
const DEFAULT_TEMPLATE: &str = "你好，{name}！ArkDesk 插件链路已打通。";

/// 问候命令：演示参数校验（错误路径）、可选参数（设置页自定义模板）与返回值
/// 命令名 = 函数名，须与前端 ipc("hello_world_greet") 一致（<tool_id>_ 前缀约定）
#[tauri::command]
pub fn hello_world_greet(name: String, template: Option<String>) -> Result<String, AppError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::invalid_input("名字不能为空"));
    }

    let text = template
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_TEMPLATE.to_string());

    Ok(text.replace("{name}", name))
}

// ── CSV 导出 / 导入 ────────────────────────────────────────────
// 演示「插件自有 Rust 命令 + 文件 IO + 参数化写入」的完整组合，
// 解析不引入 csv crate：手写引号状态机（约 30 行，教学价值更高）。

/// CSV 字段转义：含逗号 / 引号 / 换行的字段用引号包裹，内部引号翻倍
fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// CSV 行解析（引号状态机）：返回字段列表；空行返回 None
fn parse_csv_row(line: &str) -> Option<Vec<String>> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    if line.trim().is_empty() {
        return None;
    }

    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        current.push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                    }
                }
                _ => current.push(c),
            }
        } else {
            match c {
                '"' => in_quotes = true,
                ',' => {
                    fields.push(std::mem::take(&mut current));
                }
                _ => current.push(c),
            }
        }
    }
    fields.push(current);
    Some(fields)
}

/// 查询全部任务（复用框架 db_query_values 通用通道，参数化 SQL）
async fn query_tasks() -> Result<Vec<Vec<Value>>, AppError> {
    let result = crate::db::db_query_values(SqlArgs {
        sql: "SELECT id, title, status, priority, created_at FROM hello_tasks ORDER BY id"
            .to_string(),
        params: vec![],
    })
    .await?;
    Ok(result.rows)
}

/// 导出全部任务为 CSV：写入 `appDataDir/exports/hello-tasks-<时间戳>.csv`，返回文件路径
#[tauri::command]
pub async fn hello_world_export_csv(app: tauri::AppHandle) -> Result<String, AppError> {
    let rows = query_tasks().await?;

    let mut csv = String::from("id,title,status,priority,created_at\n");
    for row in rows {
        let fields: Vec<String> = row
            .iter()
            .map(|value| {
                csv_escape(&match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => String::new(),
                })
            })
            .collect();
        csv.push_str(&fields.join(","));
        csv.push('\n');
    }

    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("解析数据目录失败: {err}")))?
        .join("exports");
    std::fs::create_dir_all(&dir)?;

    let path = dir.join(format!(
        "hello-tasks-{}.csv",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    ));
    std::fs::write(&path, csv)?;

    log::info!("任务已导出: {}", path.display());
    Ok(path.to_string_lossy().to_string())
}

/// 从 CSV 导入任务（文件路径由前端 plugin-dialog 选择）。
/// 表头需含 title/status/priority 列（多余列忽略），返回导入条数。
#[tauri::command]
pub async fn hello_world_import_csv(path: String) -> Result<u64, AppError> {
    if !path.to_ascii_lowercase().ends_with(".csv") {
        return Err(AppError::invalid_input("仅支持 .csv 文件"));
    }
    let content = std::fs::read_to_string(&path)?;

    let mut rows = content.lines().filter_map(parse_csv_row);
    let header = rows
        .next()
        .ok_or_else(|| AppError::invalid_input("CSV 为空"))?;
    let index = |name: &str| {
        header
            .iter()
            .position(|col| col.trim().eq_ignore_ascii_case(name))
    };
    let (title_idx, status_idx, priority_idx) = (
        index("title").ok_or_else(|| AppError::invalid_input("缺少 title 列"))?,
        index("status"),
        index("priority"),
    );

    let valid_status = ["pending", "in_progress", "done"];
    let valid_priority = ["high", "medium", "low"];
    let mut imported: u64 = 0;

    for row in rows {
        let get = |idx: Option<usize>| {
            idx.and_then(|i| row.get(i).map(|v| v.trim().to_string()))
                .unwrap_or_default()
        };
        let title = get(Some(title_idx));
        if title.is_empty() {
            continue;
        }
        let status = get(status_idx);
        let status = if valid_status.contains(&status.as_str()) {
            status
        } else {
            "pending".to_string()
        };
        let priority = get(priority_idx);
        let priority = if valid_priority.contains(&priority.as_str()) {
            priority
        } else {
            "medium".to_string()
        };

        let result = crate::db::db_execute(SqlArgs {
            sql: "INSERT INTO hello_tasks (title, status, priority) VALUES ($1, $2, $3)"
                .to_string(),
            params: vec![
                Value::String(title),
                Value::String(status),
                Value::String(priority),
            ],
        })
        .await?;
        imported += result.rows_affected;
    }

    log::info!("CSV 导入完成: {imported} 条");
    Ok(imported)
}
