//! 通用 HTTP 客户端：框架级的网页采集 / API 调用 / 文件下载能力。
//!
//! - 全局 Client（可重建）：统一 UA、超时、重定向、cookie 会话、可选代理
//! - `http_request` 通用命令：覆盖 GET/POST 等常见采集请求
//! - `http_download` 流式下载：分块写盘 + `http://download-progress` 事件
//! - `http_set_proxy` 由前端设置驱动（代理变更时重建 Client）
//! - 前端统一经 `core/http` 使用，禁止在 webview 内直接 fetch 跨域

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::error::{code, AppError};
use crate::events;

const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_REDIRECTS: usize = 10;
/// 响应体大小上限（10 MB）
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

/// 当前代理设置（`None` = 直连）
static PROXY: OnceLock<RwLock<Option<String>>> = OnceLock::new();

fn build_client(proxy: Option<&str>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("PocketArk/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_millis(DEFAULT_TIMEOUT_MS))
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .cookie_store(true);
    if let Some(proxy) = proxy.filter(|value| !value.is_empty()) {
        match reqwest::Proxy::all(proxy) {
            Ok(configured) => builder = builder.proxy(configured),
            Err(err) => log::warn!("代理配置无效（已忽略）: {err}"),
        }
    }
    builder.build().expect("HTTP 客户端初始化失败")
}

fn proxy_setting() -> Option<String> {
    PROXY
        .get_or_init(|| RwLock::new(None))
        .read()
        .ok()
        .and_then(|guard| guard.clone())
}

/// 带代理缓存的 Client：代理变化时重建
fn client() -> reqwest::Client {
    static CELL: OnceLock<RwLock<(String, reqwest::Client)>> = OnceLock::new();
    let proxy = proxy_setting().unwrap_or_default();
    let cell = CELL.get_or_init(|| RwLock::new((String::new(), build_client(None))));

    if let Ok(guard) = cell.read() {
        if guard.0 == proxy {
            return guard.1.clone();
        }
    }
    if let Ok(mut guard) = cell.write() {
        if guard.0 != proxy {
            let built = build_client(Some(&proxy));
            *guard = (proxy, built);
        }
        return guard.1.clone();
    }
    build_client(None)
}

fn http_err(message: impl Into<String>) -> AppError {
    AppError::custom(code::HTTP_ERROR, message)
}

/// 设置 HTTP 代理（空字符串/null = 直连）；由前端设置驱动
#[tauri::command]
pub fn http_set_proxy(proxy: Option<String>) -> Result<(), AppError> {
    let normalized = proxy.filter(|value| !value.trim().is_empty());
    if let Ok(mut guard) = PROXY.get_or_init(|| RwLock::new(None)).write() {
        *guard = normalized;
    }
    Ok(())
}

/// http_request 命令参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestArgs {
    pub method: Option<String>,
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub query: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub timeout_ms: Option<u64>,
}

/// http_request 命令响应
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponsePayload {
    pub status: u16,
    pub ok: bool,
    pub final_url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub elapsed_ms: u64,
}

/// 通用 HTTP 请求命令（网页采集 / API 调用的统一入口）
#[tauri::command]
pub async fn http_request(args: HttpRequestArgs) -> Result<HttpResponsePayload, AppError> {
    let method_text = args.method.unwrap_or_else(|| "GET".to_string());
    let method = reqwest::Method::from_bytes(method_text.to_uppercase().as_bytes())
        .map_err(|_| AppError::invalid_input(format!("非法 HTTP 方法: {method_text}")))?;

    if args.url.trim().is_empty() {
        return Err(AppError::invalid_input("URL 不能为空"));
    }

    let timeout = Duration::from_millis(args.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
    let mut request = client().request(method, &args.url).timeout(timeout);

    if let Some(headers) = &args.headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }
    if let Some(query) = &args.query {
        request = request.query(query);
    }
    if let Some(body) = &args.body {
        request = request.body(body.clone());
    }

    let started = Instant::now();
    let response = request
        .send()
        .await
        .map_err(|err| http_err(err.to_string()))?;

    let status = response.status().as_u16();
    let final_url = response.url().to_string();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(key, value)| {
            (
                key.to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    // 流式读取，超限立即中止
    let mut response = response;
    let mut body_bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| http_err(err.to_string()))?
    {
        if body_bytes.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(http_err(format!(
                "响应体超过大小上限（{MAX_BODY_BYTES} 字节）"
            )));
        }
        body_bytes.extend_from_slice(&chunk);
    }

    Ok(HttpResponsePayload {
        status,
        ok: (200..300).contains(&status),
        final_url,
        headers,
        body: String::from_utf8_lossy(&body_bytes).to_string(),
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

/// http_download 命令参数
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpDownloadArgs {
    pub url: String,
    /// 保存路径（由前端经系统对话框选择）
    pub path: String,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    url: String,
    downloaded: u64,
    total: Option<u64>,
}

/// 流式下载到文件：分块写盘并回传进度，返回保存路径
#[tauri::command]
pub async fn http_download(app: AppHandle, args: HttpDownloadArgs) -> Result<String, AppError> {
    if args.url.trim().is_empty() {
        return Err(AppError::invalid_input("URL 不能为空"));
    }
    if args.path.trim().is_empty() {
        return Err(AppError::invalid_input("保存路径不能为空"));
    }

    let mut request = client().get(&args.url);
    if let Some(headers) = &args.headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }
    let response = request
        .send()
        .await
        .map_err(|err| http_err(err.to_string()))?;
    if !response.status().is_success() {
        return Err(http_err(format!("下载失败：HTTP {}", response.status())));
    }

    let total = response.content_length();
    let mut response = response;

    // 先写临时文件，成功后原子重命名，避免半成品文件
    let target = std::path::Path::new(&args.path);
    if let Some(dir) = target.parent() {
        if !dir.as_os_str().is_empty() {
            tokio::fs::create_dir_all(dir)
                .await
                .map_err(|err| http_err(format!("创建目录失败: {err}")))?;
        }
    }
    let part_path = format!("{}.part", args.path);
    let mut file = tokio::fs::File::create(&part_path)
        .await
        .map_err(|err| http_err(format!("创建文件失败: {err}")))?;
    let mut downloaded: u64 = 0;
    let mut last_emitted: u64 = 0;

    loop {
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(err) => {
                drop(file);
                let _ = tokio::fs::remove_file(&part_path).await;
                return Err(http_err(format!("读取响应失败: {err}")));
            }
        };
        if let Err(err) = file.write_all(&chunk).await {
            drop(file);
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(http_err(format!("写入文件失败: {err}")));
        }
        downloaded += chunk.len() as u64;
        // 每 256KB 回传一次进度
        if downloaded - last_emitted >= 256 * 1024 {
            last_emitted = downloaded;
            let _ = app.emit(
                events::HTTP_DOWNLOAD_PROGRESS,
                DownloadProgress {
                    url: args.url.clone(),
                    downloaded,
                    total,
                },
            );
        }
    }
    if let Err(err) = file.flush().await {
        drop(file);
        let _ = tokio::fs::remove_file(&part_path).await;
        return Err(http_err(format!("刷新文件失败: {err}")));
    }
    drop(file);
    tokio::fs::rename(&part_path, &args.path)
        .await
        .map_err(|err| http_err(format!("保存文件失败: {err}")))?;

    let _ = app.emit(
        events::HTTP_DOWNLOAD_PROGRESS,
        DownloadProgress {
            url: args.url.clone(),
            downloaded,
            total,
        },
    );
    log::info!("下载完成: {} -> {}", args.url, args.path);
    Ok(args.path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "依赖 httpbin.org 外部网络，手动运行：cargo test -- --ignored"]
    async fn http_request_smoke() {
        let args = HttpRequestArgs {
            method: Some("GET".to_string()),
            url: "https://httpbin.org/json".to_string(),
            headers: None,
            query: None,
            body: None,
            timeout_ms: Some(15000),
        };
        let resp = http_request(args).await.expect("HTTP 请求失败");
        assert_eq!(resp.status, 200);
        assert!(resp.ok);
        assert!(resp.body.contains("slideshow"));
    }
}
