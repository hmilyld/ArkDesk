//! text2video 图文视频工具：手动 / AI 文章 → 竖屏滚动视频。
//!
//! 命令（`text2video_` 前缀，构建期由 build.rs 扫描本文件自动登记）：
//! - `text2video_check_env`：环境检查（ffmpeg / 字体 / 输出目录）
//! - `text2video_generate_manual`：手动输入单条生成
//! - `text2video_generate_batch`：批量生成（草稿箱 / 多选）
//! - `text2video_ai_generate`：AI 写文章
//! - `text2video_draft_save` / `_list` / `_delete`：草稿箱
//! - `text2video_cancel`：取消当前生成
//! - `text2video_history` / `text2video_delete_history`：处理记录
//! - `text2video_open`：打开文件 / 在文件夹中显示

pub mod ai;
pub mod cleaner;
pub mod ffmpeg;
pub mod migrations;
pub mod models;
pub mod pipeline;
pub mod render;
pub mod sources;

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use serde_json::json;
use tauri::ipc::Channel;
use tauri::AppHandle;

use crate::db::SqlArgs;
use crate::error::{code, AppError};

use models::{
    AiArticle, AiConfig, AiGenerateRequest, Draft, DraftInput, EnvStatus, GenerateOptions,
    HistoryRow, ManualInput, ProgressMsg, RunSummary, Settings,
};
use render::fonts::FontKit;

/// 环境检查：ffmpeg/ffprobe 是否可用、字体是否就绪、有效输出目录
#[tauri::command]
pub async fn text2video_check_env(
    app: AppHandle,
    settings: Settings,
) -> Result<EnvStatus, AppError> {
    let mut ffmpeg_ok = false;
    let mut ffmpeg_path = String::new();
    let mut ffprobe_path = String::new();
    let mut message = String::new();

    match ffmpeg::locate_ffmpeg(&settings.ffmpeg_path) {
        Ok(path) => {
            ffmpeg_ok = true;
            ffprobe_path = ffmpeg::locate_ffprobe(&path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            ffmpeg_path = path.to_string_lossy().to_string();
        }
        Err(err) => message = err.to_string(),
    }

    let resource = pipeline::resource_dir(&app)?;
    let (fonts_ok, font_regular, font_bold) = match FontKit::load_cached(
        &resource,
        &settings.font_regular_path,
        &settings.font_bold_path,
    ) {
        Ok(fonts) => (true, fonts.regular_path.clone(), fonts.bold_path.clone()),
        Err(err) => {
            if message.is_empty() {
                message = err.to_string();
            }
            (false, String::new(), String::new())
        }
    };

    let output_dir = pipeline::resolve_output_dir(&app, &settings.output_dir)?
        .to_string_lossy()
        .to_string();

    Ok(EnvStatus {
        ffmpeg_ok,
        ffmpeg_path,
        ffprobe_path,
        fonts_ok,
        font_regular,
        font_bold,
        output_dir,
        message,
    })
}

/// 手动输入内容生成（单条）
#[tauri::command]
pub async fn text2video_generate_manual(
    app: AppHandle,
    options: GenerateOptions,
    input: ManualInput,
    channel: Channel<ProgressMsg>,
) -> Result<RunSummary, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::invalid_input("标题不能为空"));
    }
    if input.content.trim().is_empty() {
        return Err(AppError::invalid_input("正文不能为空"));
    }
    if pipeline::RUNNING.swap(true, Ordering::SeqCst) {
        return Err(AppError::invalid_input("已有生成任务正在运行"));
    }
    crate::tasks::begin("text2video");

    let result = pipeline::run(&app, options, channel, vec![input]).await;

    match &result {
        Ok(_) => crate::tasks::emit_done(&app, "text2video"),
        Err(err) => crate::tasks::emit_error(&app, "text2video", &err.to_string()),
    }
    crate::tasks::end("text2video");
    pipeline::RUNNING.store(false, Ordering::SeqCst);
    result
}

/// 批量生成（草稿箱多选）
#[tauri::command]
pub async fn text2video_generate_batch(
    app: AppHandle,
    options: GenerateOptions,
    inputs: Vec<ManualInput>,
    channel: Channel<ProgressMsg>,
) -> Result<RunSummary, AppError> {
    if inputs.is_empty() {
        return Err(AppError::invalid_input("未选择任何条目"));
    }
    if inputs
        .iter()
        .any(|i| i.title.trim().is_empty() || i.content.trim().is_empty())
    {
        return Err(AppError::invalid_input("存在标题或正文为空的条目"));
    }
    if pipeline::RUNNING.swap(true, Ordering::SeqCst) {
        return Err(AppError::invalid_input("已有生成任务正在运行"));
    }
    crate::tasks::begin("text2video");

    let result = pipeline::run(&app, options, channel, inputs).await;

    match &result {
        Ok(_) => crate::tasks::emit_done(&app, "text2video"),
        Err(err) => crate::tasks::emit_error(&app, "text2video", &err.to_string()),
    }
    crate::tasks::end("text2video");
    pipeline::RUNNING.store(false, Ordering::SeqCst);
    result
}

/// AI 写文章（配置来自全局设置的 AI 分区）
#[tauri::command]
pub async fn text2video_ai_generate(
    config: AiConfig,
    request: AiGenerateRequest,
) -> Result<AiArticle, AppError> {
    ai::generate(&config, &request).await
}

/// 保存草稿（有 id 更新，无 id 新增），返回草稿 id
#[tauri::command]
pub async fn text2video_draft_save(draft: DraftInput) -> Result<i64, AppError> {
    if draft.title.trim().is_empty() {
        return Err(AppError::invalid_input("草稿标题不能为空"));
    }
    if draft.content.trim().is_empty() {
        return Err(AppError::invalid_input("草稿正文不能为空"));
    }
    let author = draft.author.trim().to_string();
    let source = if draft.source.trim().is_empty() {
        "manual".to_string()
    } else {
        draft.source.trim().to_string()
    };

    match draft.id {
        Some(id) => {
            let result = crate::db::db_execute(SqlArgs {
                sql: "UPDATE text2video_drafts SET title = $1, author = $2, content = $3,
                      source = $4, updated_at = datetime('now', 'localtime') WHERE id = $5"
                    .to_string(),
                params: vec![
                    json!(draft.title.trim()),
                    json!(author),
                    json!(draft.content),
                    json!(source),
                    json!(id),
                ],
            })
            .await?;
            if result.rows_affected == 0 {
                return Err(AppError::custom(
                    code::NOT_FOUND,
                    format!("草稿不存在: {id}"),
                ));
            }
            Ok(id)
        }
        None => {
            let result = crate::db::db_execute(SqlArgs {
                sql: "INSERT INTO text2video_drafts (title, author, content, source)
                      VALUES ($1, $2, $3, $4)"
                    .to_string(),
                params: vec![
                    json!(draft.title.trim()),
                    json!(author),
                    json!(draft.content),
                    json!(source),
                ],
            })
            .await?;
            Ok(result.last_insert_id)
        }
    }
}

/// 草稿列表（按更新时间倒序）
#[tauri::command]
pub async fn text2video_draft_list() -> Result<Vec<Draft>, AppError> {
    let result = crate::db::db_query_values(SqlArgs {
        sql: "SELECT id, title, author, content, source, created_at, updated_at
              FROM text2video_drafts ORDER BY updated_at DESC"
            .to_string(),
        params: vec![],
    })
    .await?;

    let index_of = |name: &str| result.columns.iter().position(|c| c == name);
    let (i_id, i_title, i_author, i_content, i_source, i_created, i_updated) = (
        index_of("id"),
        index_of("title"),
        index_of("author"),
        index_of("content"),
        index_of("source"),
        index_of("created_at"),
        index_of("updated_at"),
    );
    let get_str = |row: &[serde_json::Value], idx: Option<usize>| -> String {
        idx.and_then(|i| row.get(i))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };

    Ok(result
        .rows
        .iter()
        .map(|row| Draft {
            id: i_id
                .and_then(|i| row.get(i))
                .and_then(|v| v.as_i64())
                .unwrap_or_default(),
            title: get_str(row, i_title),
            author: get_str(row, i_author),
            content: get_str(row, i_content),
            source: get_str(row, i_source),
            created_at: get_str(row, i_created),
            updated_at: get_str(row, i_updated),
        })
        .collect())
}

/// 删除草稿
#[tauri::command]
pub async fn text2video_draft_delete(id: i64) -> Result<(), AppError> {
    crate::db::db_execute(SqlArgs {
        sql: "DELETE FROM text2video_drafts WHERE id = $1".to_string(),
        params: vec![json!(id)],
    })
    .await?;
    Ok(())
}

/// 取消当前生成（阶段边界生效，并终止 ffmpeg 子进程）
#[tauri::command]
pub fn text2video_cancel() {
    crate::tasks::cancel("text2video");
}

/// 处理记录
#[tauri::command]
pub async fn text2video_history(limit: usize) -> Result<Vec<HistoryRow>, AppError> {
    let result = crate::db::db_query_values(SqlArgs {
        sql: "SELECT ref_id, kind, title, status, detail, video, created_at
              FROM text2video_processed ORDER BY created_at DESC LIMIT $1"
            .to_string(),
        params: vec![json!(limit as i64)],
    })
    .await?;

    let index_of = |name: &str| result.columns.iter().position(|c| c == name);
    let (i_ref, i_kind, i_title, i_status, i_detail, i_video, i_created) = (
        index_of("ref_id"),
        index_of("kind"),
        index_of("title"),
        index_of("status"),
        index_of("detail"),
        index_of("video"),
        index_of("created_at"),
    );

    let get = |row: &[serde_json::Value], idx: Option<usize>| -> String {
        idx.and_then(|i| row.get(i))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };

    Ok(result
        .rows
        .iter()
        .map(|row| HistoryRow {
            ref_id: get(row, i_ref),
            kind: get(row, i_kind),
            title: get(row, i_title),
            status: get(row, i_status),
            detail: get(row, i_detail),
            video: get(row, i_video),
            created_at: get(row, i_created),
        })
        .collect())
}

/// 删除一条处理记录
#[tauri::command]
pub async fn text2video_delete_history(ref_id: String) -> Result<(), AppError> {
    crate::db::db_execute(SqlArgs {
        sql: "DELETE FROM text2video_processed WHERE ref_id = $1".to_string(),
        params: vec![json!(ref_id)],
    })
    .await?;
    Ok(())
}

/// 打开文件 / 在文件夹中显示（Rust 侧执行，规避前端 opener scope 限制）
#[tauri::command]
pub fn text2video_open(path: String, reveal: bool) -> Result<(), AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("路径不能为空"));
    }
    let target = PathBuf::from(path.trim());
    if !target.exists() {
        return Err(AppError::custom(
            code::NOT_FOUND,
            format!("路径不存在: {}", target.display()),
        ));
    }
    let result = if reveal && !target.is_dir() {
        opener::reveal(&target)
    } else {
        opener::open(&target)
    };
    result.map_err(|err| AppError::custom(code::IO_ERROR, format!("打开失败: {err}")))
}
