//! ffmpeg / ffprobe 定位与执行（依赖系统安装；设置可指定路径）。
//!
//! macOS 上 Finder 启动的 .app 不继承 shell PATH，故除 PATH 外还显式探测
//! Homebrew / MacPorts 等常见目录。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::error::{code, AppError};

/// 取消标志专用错误码（前端据此区分「用户取消」与真正失败）
pub const CANCELLED: &str = "CANCELLED";

fn exe_name(base: &str) -> String {
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn common_dirs() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        ["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"]
            .iter()
            .map(PathBuf::from)
            .collect()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(exe_name(name));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// 定位 ffmpeg：设置路径优先 → PATH → 常见目录
pub fn locate_ffmpeg(configured: &str) -> Result<PathBuf, AppError> {
    if !configured.trim().is_empty() {
        let path = PathBuf::from(configured.trim());
        if path.is_file() {
            return Ok(path);
        }
        return Err(AppError::custom(
            code::NOT_FOUND,
            format!("设置的 ffmpeg 不存在: {}", path.display()),
        ));
    }
    if let Some(path) = find_in_path("ffmpeg") {
        return Ok(path);
    }
    for dir in common_dirs() {
        let candidate = dir.join(exe_name("ffmpeg"));
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(AppError::custom(
        code::NOT_FOUND,
        "未找到 ffmpeg：请在设置中指定路径，或安装系统 ffmpeg",
    ))
}

/// 定位 ffprobe：优先与 ffmpeg 同目录
pub fn locate_ffprobe(ffmpeg: &Path) -> Result<PathBuf, AppError> {
    if let Some(parent) = ffmpeg.parent() {
        let candidate = parent.join(exe_name("ffprobe"));
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    if let Some(path) = find_in_path("ffprobe") {
        return Ok(path);
    }
    for dir in common_dirs() {
        let candidate = dir.join(exe_name("ffprobe"));
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(AppError::custom(
        code::NOT_FOUND,
        "未找到 ffprobe（应与 ffmpeg 同目录，或安装系统 ffprobe）",
    ))
}

/// 运行命令并等待；期间轮询取消标志，命中则 kill 子进程。
pub fn run_cmd(cmd: &mut Command, cancel: &AtomicBool) -> Result<(), AppError> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("命令启动失败: {err}")))?;

    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::custom(CANCELLED, "已取消"));
        }
        match child
            .try_wait()
            .map_err(|err| AppError::custom(code::IO_ERROR, format!("等待命令失败: {err}")))?
        {
            Some(_) => {
                let output = child.wait_with_output().map_err(|err| {
                    AppError::custom(code::IO_ERROR, format!("读取命令输出失败: {err}"))
                })?;
                if output.status.success() {
                    return Ok(());
                }
                let stderr = String::from_utf8_lossy(&output.stderr);
                let lines: Vec<&str> = stderr.lines().collect();
                let tail = lines[lines.len().saturating_sub(8)..].join("\n");
                return Err(AppError::custom(
                    code::PLUGIN_ERROR,
                    format!("ffmpeg 执行失败:\n{tail}"),
                ));
            }
            None => std::thread::sleep(Duration::from_millis(150)),
        }
    }
}
