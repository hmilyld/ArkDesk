//! 生成流水线：清洗 → 渲染 → 封面 → 记录，含进度上报与取消。

use regex::Regex;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use crate::db::SqlArgs;
use crate::error::{code, AppError};
use crate::plugins::text2video::cleaner::{draft_to_content, CleanOptions, Skip};
use crate::plugins::text2video::ffmpeg::{self, CANCELLED};
use crate::plugins::text2video::models::{
    ContentDraft, GenerateOptions, ManualInput, MetaFiles, ProgressMsg, RunResult, RunSummary,
    VideoMeta,
};
use crate::plugins::text2video::render::fonts::FontKit;
use crate::plugins::text2video::render::scroll::render_scroll_video;
use crate::plugins::text2video::sources;

/// 生成进行中标记（单实例守卫）
pub static RUNNING: AtomicBool = AtomicBool::new(false);

fn is_cancelled(err: &AppError) -> bool {
    matches!(err, AppError::Custom { code, .. } if code == CANCELLED)
}

/// 解析资源目录（含 `fonts/` 子目录的父目录）。
///
/// - dev：`src-tauri/local-resources`
/// - prod：Tauri 会保留 `local-resources/fonts/*` 的原始结构，即
///   `$RESOURCE/local-resources/fonts`；同时兼容资源被扁平化到 `$RESOURCE/fonts`
///   的情形（探测后择一）。
pub fn resource_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    if cfg!(debug_assertions) {
        let mut path = std::env::current_dir()
            .map_err(|err| AppError::custom(code::IO_ERROR, format!("获取当前目录失败: {err}")))?;
        path.push("local-resources");
        Ok(path)
    } else {
        let base = app
            .path()
            .resource_dir()
            .map_err(|err| AppError::custom(code::IO_ERROR, format!("获取资源目录失败: {err}")))?;
        let nested = base.join("local-resources");
        if nested.join("fonts").is_dir() {
            Ok(nested)
        } else {
            Ok(base)
        }
    }
}

/// 解析输出目录：设置优先，否则系统下载目录
pub fn resolve_output_dir(app: &AppHandle, configured: &str) -> Result<PathBuf, AppError> {
    if !configured.trim().is_empty() {
        return Ok(PathBuf::from(configured.trim()));
    }
    let downloads = app
        .path()
        .download_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("无法解析下载目录: {err}")))?;
    Ok(downloads)
}

fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn slugify(text: &str, max_len: usize) -> String {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r#"[\\/:*?"<>|\s]+"#).expect("slug 正则"));
    let cleaned = re.replace_all(text, "");
    let truncated: String = cleaned.chars().take(max_len).collect();
    if truncated.is_empty() {
        "untitled".to_string()
    } else {
        truncated
    }
}

/// 目录名安全化（非字母数字/下划线/点/连字符 → 下划线）
fn sanitize_dir_name(raw: &str) -> String {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[^\w.-]").expect("目录名正则"))
        .replace_all(raw, "_")
        .to_string()
}

/// 生成建议标题（移植自 Python `suggested_title`）
pub fn suggested_title(title: &str) -> String {
    static PREFIX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static TRAILING: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let prefix = PREFIX.get_or_init(|| {
        Regex::new(r"^(如何看待|如何评价|怎么看待|怎样看待|怎么评价|怎样评价)").expect("前缀正则")
    });
    let trailing = TRAILING.get_or_init(|| Regex::new(r"[?？!！。]+$").expect("尾正则"));
    let t = title.trim();
    let is_q = t.ends_with('?') || t.ends_with('？');
    let stripped = prefix.replace(t, "").trim().to_string();
    if stripped.is_empty() {
        return title.trim().to_string();
    }
    let stripped = trailing.replace(&stripped, "").to_string();
    let mut out = if is_q {
        format!("{stripped}？")
    } else {
        stripped
    };
    if out.chars().count() > 32 {
        let head: String = out.chars().take(31).collect();
        out = format!("{head}…");
    }
    out
}

fn pick_bgm(dir: &str) -> Option<PathBuf> {
    if dir.trim().is_empty() {
        return None;
    }
    let mut tracks: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .map(|e| {
                    matches!(
                        e.to_string_lossy().to_lowercase().as_str(),
                        "mp3" | "m4a" | "wav" | "flac" | "ogg"
                    )
                })
                .unwrap_or(false)
        })
        .collect();
    if tracks.is_empty() {
        return None;
    }
    use rand::seq::SliceRandom;
    tracks.shuffle(&mut rand::rng());
    tracks
        .into_iter()
        .next()
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
}

/// 写入处理记录；失败仅告警，不中断本次生成
async fn mark(
    ref_id: &str,
    kind: &str,
    title: &str,
    status: &str,
    detail: &str,
    video: &str,
    meta: &str,
) {
    let result = crate::db::db_execute(SqlArgs {
        sql: "INSERT INTO text2video_processed
              (ref_id, kind, title, status, detail, video, meta, created_at)
              VALUES ($1, $2, $3, $4, $5, $6, $7, datetime('now', 'localtime'))
              ON CONFLICT(ref_id) DO UPDATE SET
              status = excluded.status, detail = excluded.detail,
              video = excluded.video, meta = excluded.meta"
            .to_string(),
        params: vec![
            json!(ref_id),
            json!(kind),
            json!(title),
            json!(status),
            json!(detail),
            json!(video),
            json!(meta),
        ],
    })
    .await;
    if let Err(err) = result {
        log::warn!("写入处理记录失败 ({ref_id}, {status}): {err}");
    }
}

/// 运行一次生成（手动 / AI 输入，逐条；每条一个视频）。
pub async fn run(
    app: &AppHandle,
    options: GenerateOptions,
    channel: Channel<ProgressMsg>,
    inputs: Vec<ManualInput>,
) -> Result<RunSummary, AppError> {
    if inputs.is_empty() {
        return Err(AppError::invalid_input("没有可生成的内容"));
    }
    let settings = options.settings.clone();
    let template = if options.template.is_empty() {
        "scroll".to_string()
    } else {
        options.template.clone()
    };
    if template != "scroll" {
        return Err(AppError::invalid_input(format!(
            "当前版本仅支持滚动模板（scroll），收到: {template}"
        )));
    }

    let output_dir = resolve_output_dir(app, &settings.output_dir)?;
    std::fs::create_dir_all(&output_dir)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("创建输出目录失败: {err}")))?;

    let ffmpeg = ffmpeg::locate_ffmpeg(&settings.ffmpeg_path)?;
    let resource = resource_dir(app)?;
    let fonts = FontKit::load_cached(
        &resource,
        &settings.font_regular_path,
        &settings.font_bold_path,
    )?;

    // 取消标志交由框架任务注册表管理（id = text2video）
    let cancel = crate::tasks::flag("text2video").unwrap_or_default();

    let send = |stage: &str, current: usize, total: usize, message: String| {
        crate::tasks::emit_progress(
            app,
            "text2video",
            current as u64,
            Some(total as u64),
            Some(message.clone()),
        );
        let _ = channel.send(ProgressMsg {
            stage: stage.to_string(),
            current,
            total,
            message,
        });
    };

    let planned: Vec<ContentDraft> = inputs
        .iter()
        .enumerate()
        .map(|(seq, input)| sources::manual_draft(input, seq))
        .collect();
    let total = planned.len();

    let clean_opts = CleanOptions {
        min_chars: settings.min_chars,
        max_chars: settings.max_chars,
        min_sentence: settings.min_sentence_chars,
        max_sentence: settings.max_sentence_chars,
        min_block: settings.min_block_chars,
        banned_keywords: settings.banned_keywords.clone(),
    };

    let mut results: Vec<RunResult> = Vec::new();
    let mut rendered = 0usize;
    let mut cancelled = false;

    for (index, draft) in planned.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }

        let label = if draft.title.is_empty() {
            draft.ref_id.clone()
        } else {
            draft.title.clone()
        };
        send(
            "cleaning",
            index + 1,
            total,
            format!("处理正文: {}", truncate(&label, 40)),
        );

        let content = match draft_to_content(draft, &clean_opts) {
            Ok(content) => content,
            Err(Skip { reason, detail }) => {
                mark(
                    &draft.ref_id,
                    &draft.source_type,
                    &label,
                    &reason,
                    &detail,
                    "",
                    "",
                )
                .await;
                send(
                    "skip",
                    index + 1,
                    total,
                    format!("跳过 [{reason}] {detail}"),
                );
                results.push(RunResult {
                    ref_id: draft.ref_id.clone(),
                    title: label.clone(),
                    status: reason,
                    detail,
                    video: String::new(),
                    cover: String::new(),
                });
                continue;
            }
        };

        let theme = match settings.theme_mode.as_str() {
            "dark" => "dark".to_string(),
            "light" => "light".to_string(),
            _ => {
                use rand::RngExt;
                if rand::rng().random::<bool>() {
                    "dark".to_string()
                } else {
                    "light".to_string()
                }
            }
        };
        use rand::RngExt;
        let bg_index = rand::rng().random_range(0..100usize);
        let bgm = pick_bgm(&settings.bgm_dir);

        let date = chrono::Local::now().format("%Y%m%d").to_string();
        let ref_short: String = draft.ref_id.chars().take(20).collect();
        let raw_name = format!(
            "{date}_{}_{}_{index}",
            slugify(&content.title, 24),
            ref_short
        );
        let safe_name = sanitize_dir_name(&raw_name);
        let workdir = output_dir.join(safe_name);

        send(
            "rendering",
            index + 1,
            total,
            format!("渲染视频: {}", truncate(&label, 40)),
        );

        let content_for_render = content.clone();
        let settings_for_render = settings.clone();
        let fonts_for_render = fonts.clone();
        let ffmpeg_for_render = ffmpeg.clone();
        let workdir_for_render = workdir.clone();
        let theme_for_render = theme.clone();
        let cancel_for_render = cancel.clone();

        let render_result = tauri::async_runtime::spawn_blocking(move || {
            render_scroll_video(
                &content_for_render,
                &settings_for_render,
                &fonts_for_render,
                &ffmpeg_for_render,
                &workdir_for_render,
                bg_index,
                &theme_for_render,
                bgm.as_deref(),
                &cancel_for_render,
            )
        })
        .await
        .map_err(|err| AppError::custom(code::UNKNOWN, format!("渲染任务失败: {err}")))?;

        let (video, cover, duration) = match render_result {
            Ok(value) => value,
            Err(err) if is_cancelled(&err) => {
                cancelled = true;
                break;
            }
            Err(err) => {
                let detail = err.to_string();
                mark(
                    &draft.ref_id,
                    &draft.source_type,
                    &label,
                    "render_failed",
                    &detail,
                    "",
                    "",
                )
                .await;
                send("error", index + 1, total, format!("渲染失败: {detail}"));
                results.push(RunResult {
                    ref_id: draft.ref_id.clone(),
                    title: label.clone(),
                    status: "render_failed".into(),
                    detail,
                    video: String::new(),
                    cover: String::new(),
                });
                continue;
            }
        };

        // 清理工作目录（仅保留产物）
        cleanup_workdir(&workdir);

        let meta = VideoMeta {
            raw_title: content.title.clone(),
            suggested_title: suggested_title(&content.title),
            original_url: content.source_url.clone(),
            author: content.author.clone(),
            voteups: content.voteups,
            source_type: content.source_type.clone(),
            template: template.clone(),
            duration_sec: (duration * 10.0).round() / 10.0,
            char_count: content.char_count(),
            created_at: now_iso(),
            files: MetaFiles {
                video: video.to_string_lossy().to_string(),
                cover: cover.to_string_lossy().to_string(),
            },
        };
        let meta_json = serde_json::to_string_pretty(&meta).unwrap_or_default();
        let meta_path = workdir.join("meta.json");
        if let Err(err) = std::fs::write(&meta_path, &meta_json) {
            log::warn!("meta.json 写入失败: {err}");
        }

        let video_str = video.to_string_lossy().to_string();
        mark(
            &draft.ref_id,
            &draft.source_type,
            &label,
            "done",
            "",
            &video_str,
            &meta_json,
        )
        .await;

        rendered += 1;
        send("done", index + 1, total, format!("完成 → {video_str}"));
        results.push(RunResult {
            ref_id: draft.ref_id.clone(),
            title: label,
            status: "done".into(),
            detail: String::new(),
            video: video_str,
            cover: cover.to_string_lossy().to_string(),
        });
    }

    if settings.open_after_render && rendered > 0 {
        let _ = opener::open(&output_dir);
    }

    Ok(RunSummary {
        rendered,
        cancelled,
        output_dir: output_dir.to_string_lossy().to_string(),
        results,
    })
}

fn truncate(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        text.to_string()
    } else {
        chars[..max].iter().collect()
    }
}

fn cleanup_workdir(workdir: &Path) {
    let keep = ["video.mp4", "cover.jpg", "meta.json"];
    let Ok(entries) = std::fs::read_dir(workdir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !keep.contains(&name.as_ref()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
