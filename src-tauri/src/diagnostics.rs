//! 诊断报告导出（框架级）。
//!
//! 生成一个纯文本报告：环境信息（应用/版本/平台/目录/时间）+ 最近日志片段。
//! 便于用户在反馈时附上。路径由前端经系统对话框取得。

use tauri::Manager;

use crate::error::AppError;

/// 日志附件大小上限（取最新的部分）
const MAX_LOG_BYTES: usize = 512 * 1024;

fn collect_info(app: &tauri::AppHandle) -> String {
    let pkg = app.package_info();
    let data_dir = app
        .path()
        .app_data_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let log_dir = app
        .path()
        .app_log_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    format!(
        "应用: {}\n版本: {}\n平台: {}/{}\n生成时间: {}\n数据目录: {}\n日志目录: {}\n",
        pkg.name,
        pkg.version,
        std::env::consts::OS,
        std::env::consts::ARCH,
        chrono::Local::now().to_rfc3339(),
        data_dir,
        log_dir,
    )
}

fn collect_recent_logs(app: &tauri::AppHandle) -> String {
    let Ok(log_dir) = app.path().app_log_dir() else {
        return String::new();
    };
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&log_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    // 按修改时间倒序（最新优先），避免依赖文件名排序
    files.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    files.reverse();

    let mut body = String::new();
    let mut budget = MAX_LOG_BYTES;
    // 取最新的文件优先
    for file in files.into_iter() {
        if budget == 0 {
            break;
        }
        let Ok(bytes) = std::fs::read(&file) else {
            continue;
        };
        let take = bytes.len().min(budget);
        let slice = &bytes[bytes.len() - take..];
        body.push_str(&format!(
            "\n----- {} -----\n",
            file.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        body.push_str(&String::from_utf8_lossy(slice));
        budget -= take;
    }
    body
}

/// 导出诊断报告到指定路径，返回该路径
#[tauri::command]
pub async fn diagnostics_export(app: tauri::AppHandle, path: String) -> Result<String, AppError> {
    let mut report = collect_info(&app);
    report.push_str("\n===== 最近日志 =====\n");
    report.push_str(&collect_recent_logs(&app));
    tokio::fs::write(&path, report).await?;
    log::info!("诊断报告已导出: {path}");
    Ok(path)
}
