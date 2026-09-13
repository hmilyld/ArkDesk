//! 一键系统代理（Fiddler 模式）：设置/还原系统 HTTP(S) 代理指向本地拦截端口。
//!
//! - macOS：`networksetup`（需管理员，经 osascript 弹框授权）
//! - Windows：HKCU Internet Settings 注册表（免管理员）
//! - 设置前会备份原代理配置，还原时按备份恢复；备份文件在 app_data 下

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::error::{code, AppError};

use super::dto::SystemProxyStatus;

#[derive(Serialize, Deserialize, Default)]
struct ProxyCfg {
    enabled: bool,
    server: String,
    port: u16,
}

#[derive(Serialize, Deserialize)]
struct ServiceBackup {
    service: String,
    web: ProxyCfg,
    secure: ProxyCfg,
}

/// 原系统代理配置备份（用于还原）
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
enum Backup {
    Macos { services: Vec<ServiceBackup> },
    Windows { enable: u32, server: String, override_list: String },
}

fn backup_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("无法解析应用数据目录: {err}")))?;
    Ok(base.join("network-tools").join("system-proxy.json"))
}

fn write_backup(path: &Path, backup: &Backup) -> Result<(), AppError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(backup)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("备份序列化失败: {err}")))?;
    std::fs::write(path, text)?;
    Ok(())
}

fn read_backup(path: &Path) -> Result<Backup, AppError> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("备份解析失败: {err}")))
}

/// 执行外部命令并返回 stdout
fn run(program: &str, args: &[&str]) -> Result<String, AppError> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("执行 {program} 失败: {err}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::custom(
            code::IO_ERROR,
            format!("{program} 执行失败: {stderr}"),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn is_supported() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows"))
}

/// 设置系统代理指向 127.0.0.1:port（首次会备份原配置）
pub async fn enable(app: &tauri::AppHandle, port: u16) -> Result<SystemProxyStatus, AppError> {
    if !is_supported() {
        return Err(AppError::custom(code::UNKNOWN, "当前平台不支持自动设置系统代理"));
    }
    let path = backup_path(app)?;
    tokio::task::spawn_blocking(move || {
        if !path.exists() {
            let backup = snapshot()?;
            write_backup(&path, &backup)?;
        }
        apply(port)
    })
    .await
    .map_err(|err| AppError::custom(code::IO_ERROR, format!("系统代理任务失败: {err}")))??;
    Ok(status(app))
}

/// 还原系统代理（按备份恢复），并删除备份
pub async fn disable(app: &tauri::AppHandle) -> Result<SystemProxyStatus, AppError> {
    let path = backup_path(app)?;
    let existed = path.exists();
    if existed {
        let path_for_task = path.clone();
        tokio::task::spawn_blocking(move || {
            let backup = read_backup(&path_for_task)?;
            restore(&backup)?;
            let _ = std::fs::remove_file(&path_for_task);
            Ok::<(), AppError>(())
        })
        .await
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("系统代理任务失败: {err}")))??;
    }
    Ok(status(app))
}

pub fn status(app: &tauri::AppHandle) -> SystemProxyStatus {
    let enabled = backup_path(app).map(|path| path.exists()).unwrap_or(false);
    SystemProxyStatus {
        supported: is_supported(),
        enabled,
        detail: if !is_supported() {
            "当前平台不支持自动设置".to_string()
        } else if enabled {
            "系统代理已指向本地拦截端口".to_string()
        } else {
            "未托管系统代理".to_string()
        },
    }
}

// ─────────────────────────── macOS ───────────────────────────

#[cfg(target_os = "macos")]
fn macos_services() -> Result<Vec<String>, AppError> {
    let output = run("/usr/sbin/networksetup", &["-listallnetworkservices"])?;
    Ok(output
        .lines()
        .skip(1)
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('*'))
        .map(|line| line.to_string())
        .collect())
}

#[cfg(target_os = "macos")]
fn macos_get_proxy(service: &str, secure: bool) -> ProxyCfg {
    let flag = if secure {
        "-getsecurewebproxy"
    } else {
        "-getwebproxy"
    };
    let mut cfg = ProxyCfg::default();
    if let Ok(output) = run("/usr/sbin/networksetup", &[flag, service]) {
        for line in output.lines() {
            if let Some((key, value)) = line.split_once(':') {
                match key.trim() {
                    "Enabled" => cfg.enabled = value.trim().eq_ignore_ascii_case("yes"),
                    "Server" => cfg.server = value.trim().to_string(),
                    "Port" => cfg.port = value.trim().parse().unwrap_or(0),
                    _ => {}
                }
            }
        }
    }
    cfg
}

#[cfg(target_os = "macos")]
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// 以管理员权限执行 shell（弹系统授权框）
#[cfg(target_os = "macos")]
fn macos_run_elevated(shell: &str) -> Result<(), AppError> {
    let escaped = shell.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("do shell script \"{escaped}\" with administrator privileges");
    run("/usr/bin/osascript", &["-e", &script]).map(|_| ())
}

#[cfg(target_os = "macos")]
fn snapshot() -> Result<Backup, AppError> {
    let services = macos_services()?
        .into_iter()
        .map(|service| ServiceBackup {
            web: macos_get_proxy(&service, false),
            secure: macos_get_proxy(&service, true),
            service,
        })
        .collect();
    Ok(Backup::Macos { services })
}

#[cfg(target_os = "macos")]
fn apply(port: u16) -> Result<(), AppError> {
    let services = macos_services()?;
    if services.is_empty() {
        return Err(AppError::custom(code::IO_ERROR, "未找到可用网络服务"));
    }
    let mut script = String::new();
    for service in &services {
        let svc = shell_quote(service);
        script.push_str(&format!(
            "/usr/sbin/networksetup -setwebproxy {svc} 127.0.0.1 {port}; \
             /usr/sbin/networksetup -setsecurewebproxy {svc} 127.0.0.1 {port}; \
             /usr/sbin/networksetup -setwebproxystate {svc} on; \
             /usr/sbin/networksetup -setsecurewebproxystate {svc} on; "
        ));
    }
    macos_run_elevated(&script)
}

#[cfg(target_os = "macos")]
fn restore(backup: &Backup) -> Result<(), AppError> {
    let Backup::Macos { services } = backup else {
        return Ok(());
    };
    if services.is_empty() {
        return Ok(());
    }
    let mut script = String::new();
    for entry in services {
        let svc = shell_quote(&entry.service);
        if entry.web.enabled && !entry.web.server.is_empty() {
            script.push_str(&format!(
                "/usr/sbin/networksetup -setwebproxy {svc} {} {}; /usr/sbin/networksetup -setwebproxystate {svc} on; ",
                shell_quote(&entry.web.server),
                entry.web.port
            ));
        } else {
            script.push_str(&format!(
                "/usr/sbin/networksetup -setwebproxystate {svc} off; "
            ));
        }
        if entry.secure.enabled && !entry.secure.server.is_empty() {
            script.push_str(&format!(
                "/usr/sbin/networksetup -setsecurewebproxy {svc} {} {}; /usr/sbin/networksetup -setsecurewebproxystate {svc} on; ",
                shell_quote(&entry.secure.server),
                entry.secure.port
            ));
        } else {
            script.push_str(&format!(
                "/usr/sbin/networksetup -setsecurewebproxystate {svc} off; "
            ));
        }
    }
    macos_run_elevated(&script)
}

// ─────────────────────────── Windows ───────────────────────────

#[cfg(target_os = "windows")]
const WIN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings";

#[cfg(target_os = "windows")]
fn win_query(name: &str) -> Option<String> {
    let output = run("reg", &["query", WIN_KEY, "/v", name]).ok()?;
    for line in output.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(name) {
            continue;
        }
        let rest = trimmed[name.len()..].trim();
        for kind in ["REG_SZ", "REG_DWORD", "REG_EXPAND_SZ"] {
            if let Some(index) = rest.find(kind) {
                return Some(rest[index + kind.len()..].trim().to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn win_set(name: &str, kind: &str, value: &str) -> Result<(), AppError> {
    run("reg", &["add", WIN_KEY, "/v", name, "/t", kind, "/d", value, "/f"]).map(|_| ())
}

#[cfg(target_os = "windows")]
fn snapshot() -> Result<Backup, AppError> {
    Ok(Backup::Windows {
        enable: win_query("ProxyEnable")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0),
        server: win_query("ProxyServer").unwrap_or_default(),
        override_list: win_query("ProxyOverride").unwrap_or_default(),
    })
}

#[cfg(target_os = "windows")]
fn apply(port: u16) -> Result<(), AppError> {
    win_set("ProxyEnable", "REG_DWORD", "1")?;
    win_set("ProxyServer", "REG_SZ", &format!("127.0.0.1:{port}"))?;
    win_set("ProxyOverride", "REG_SZ", "<local>")?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn restore(backup: &Backup) -> Result<(), AppError> {
    let Backup::Windows {
        enable,
        server,
        override_list,
    } = backup
    else {
        return Ok(());
    };
    win_set("ProxyEnable", "REG_DWORD", &enable.to_string())?;
    if !server.is_empty() {
        win_set("ProxyServer", "REG_SZ", server)?;
    }
    if !override_list.is_empty() {
        win_set("ProxyOverride", "REG_SZ", override_list)?;
    }
    Ok(())
}

// ─────────────────────────── 其他平台 ───────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn snapshot() -> Result<Backup, AppError> {
    Err(AppError::custom(code::UNKNOWN, "当前平台不支持自动设置系统代理"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn apply(_port: u16) -> Result<(), AppError> {
    Err(AppError::custom(code::UNKNOWN, "当前平台不支持自动设置系统代理"))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn restore(_backup: &Backup) -> Result<(), AppError> {
    Ok(())
}
