//! daily-tools 常用工具插件：文件转换（anytomd + lopdf）与图片 OCR（PaddleOCR）。

use crate::error::{code, AppError};
use serde::Serialize;
use std::path::Path;
use std::sync::OnceLock;
use tauri::Manager;

// ── 返回类型 ──────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ConvertResult {
    pub markdown: String,
}

#[derive(Serialize)]
pub struct OcrResult {
    pub text: String,
    pub confidence: f32,
}

// ── OCR 引擎单例 ─────────────────────────────────────────────────

static OCR_ENGINE: OnceLock<ocr_rs::OcrEngine> = OnceLock::new();

fn get_or_init_ocr_engine(app: &tauri::AppHandle) -> Result<&'static ocr_rs::OcrEngine, AppError> {
    if let Some(engine) = OCR_ENGINE.get() {
        return Ok(engine);
    }

    // dev 模式下资源在 src-tauri/local-resources/，prod 模式下在 resource_dir()
    let resource_dir = if cfg!(debug_assertions) {
        let mut path = std::env::current_dir()
            .map_err(|e| AppError::custom(code::IO_ERROR, format!("获取当前目录失败: {e}")))?;
        // tauri dev 的工作目录是 src-tauri/，只需拼 local-resources/
        path.push("local-resources");
        path
    } else {
        let base = app
            .path()
            .resource_dir()
            .map_err(|e| AppError::custom(code::IO_ERROR, format!("获取资源目录失败: {e}")))?;
        // 打包后保留 local-resources/ 前缀；若被扁平化则退回 base
        let nested = base.join("local-resources");
        if nested.join("ocr-models").is_dir() {
            nested
        } else {
            base
        }
    };

    let det = resource_dir.join("ocr-models/PP-OCRv6_small_det.mnn");
    let rec = resource_dir.join("ocr-models/PP-OCRv6_small_rec.mnn");
    let charset = resource_dir.join("ocr-models/ppocr_keys_v6_small.txt");

    log::debug!("OCR 资源目录: {}", resource_dir.display());
    log::debug!("OCR 模型: det={}, exists={}", det.display(), det.exists());
    log::debug!("OCR 模型: rec={}, exists={}", rec.display(), rec.exists());
    log::debug!(
        "OCR 模型: charset={}, exists={}",
        charset.display(),
        charset.exists()
    );

    let engine = ocr_rs::OcrEngine::new(
        det.to_str()
            .ok_or_else(|| AppError::custom(code::IO_ERROR, "det 模型路径包含非法字符"))?,
        rec.to_str()
            .ok_or_else(|| AppError::custom(code::IO_ERROR, "rec 模型路径包含非法字符"))?,
        charset
            .to_str()
            .ok_or_else(|| AppError::custom(code::IO_ERROR, "charset 路径包含非法字符"))?,
        None,
    )
    .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("OCR 引擎初始化失败: {e}")))?;

    // 尝试设置值（并发场景下可能已被其他线程设置）
    let _ = OCR_ENGINE.set(engine);
    Ok(OCR_ENGINE.get().unwrap())
}

// ── 文件转换 ──────────────────────────────────────────────────────

/// PDF 文本提取（lopdf）
fn convert_pdf(path: &str) -> Result<String, AppError> {
    let doc = lopdf::Document::load(path)
        .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("PDF 加载失败: {e}")))?;

    let pages = doc.get_pages();
    if pages.is_empty() {
        return Ok(String::new());
    }

    let page_numbers: Vec<u32> = pages.keys().copied().collect();
    let text = doc
        .extract_text(&page_numbers)
        .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("PDF 文本提取失败: {e}")))?;

    Ok(text)
}

/// 文件转换命令：根据扩展名分发到 anytomd 或 lopdf
#[tauri::command]
pub async fn daily_tools_convert_file(path: String) -> Result<ConvertResult, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("文件路径不能为空"));
    }

    let ext = Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let markdown = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        match ext.as_str() {
            "pdf" => convert_pdf(&path),
            _ => {
                let result = anytomd::convert_file(&path, &anytomd::ConversionOptions::default())
                    .map_err(|e| {
                    AppError::custom(code::PLUGIN_ERROR, format!("文件转换失败: {e}"))
                })?;
                Ok(result.markdown)
            }
        }
    })
    .await
    .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("任务执行失败: {e}")))??;

    Ok(ConvertResult { markdown })
}

// ── 图片 OCR ──────────────────────────────────────────────────────

/// 图片 OCR 识别命令：加载图片 → PaddleOCR → 返回纯文本
#[tauri::command]
pub async fn daily_tools_ocr_image(
    path: String,
    app: tauri::AppHandle,
) -> Result<OcrResult, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("图片路径不能为空"));
    }

    let engine = get_or_init_ocr_engine(&app)?;

    let results =
        tokio::task::spawn_blocking(move || -> Result<Vec<ocr_rs::OcrResult_>, AppError> {
            let img = image::open(&path)
                .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("图片加载失败: {e}")))?;
            engine
                .recognize(&img)
                .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("OCR 识别失败: {e}")))
        })
        .await
        .map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("任务执行失败: {e}")))??;

    if results.is_empty() {
        return Ok(OcrResult {
            text: String::new(),
            confidence: 0.0,
        });
    }

    let text: String = results
        .iter()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let confidence = results.iter().map(|r| r.confidence).sum::<f32>() / results.len() as f32;

    Ok(OcrResult { text, confidence })
}

/// 保存 Markdown 文件
#[tauri::command]
pub async fn daily_tools_save_markdown(path: String, content: String) -> Result<(), AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("保存路径不能为空"));
    }
    std::fs::write(&path, content)?;
    log::info!("Markdown 已保存: {path}");
    Ok(())
}
