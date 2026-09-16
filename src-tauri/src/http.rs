//! 通用 HTTP 客户端：框架级的网页采集 / API 调用 / 文件下载能力。
//!
//! - 全局 Client（可重建）：统一 UA、超时、重定向、cookie 会话、可选代理
//! - `http_request` 通用命令：覆盖 GET/POST 等常见采集请求
//! - `http_download` 流式下载：分块写盘 + `http://download-progress` 事件
//! - `http_set_proxy` 由前端设置驱动（代理变更时重建 Client）
//! - 前端统一经 `core/http` 使用，禁止在 webview 内直接 fetch 跨域

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

use base64::Engine as _;
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

/// 默认 User-Agent：用 crate 名（= `Cargo.toml` 的 package name，`pnpm scaffold` 会改名），
/// 避免把某个 fork 的品牌硬编码进框架
const DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

fn build_client(proxy: Option<&str>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
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

// ─────────────────────────── http_send（高级通用传输） ───────────────────────────
//
// 与 `http_request` 并列的进阶原语：可参数化的重定向 / SSL / 超时 / 代理 / cookie 模式、
// multipart 上传、二进制响应、保序且保留重复的响应头、可取消。供接口调试等工具复用。

/// 名称-值对（保序，允许重复键）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NameValue {
    pub name: String,
    pub value: String,
}

/// 响应头条目（保序，保留重复项，如多条 Set-Cookie）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
}

/// `http_send` 请求体（按 `type` 打标签）
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum HttpSendBody {
    /// 原始文本体（JSON / XML / HTML / 纯文本）
    Raw {
        text: String,
        #[serde(default)]
        content_type: Option<String>,
    },
    /// application/x-www-form-urlencoded
    Form { fields: Vec<NameValue> },
    /// multipart/form-data（文本字段 + 文件）
    Multipart { parts: Vec<MultipartPart> },
    /// 二进制文件体（从磁盘读取）
    Binary {
        path: String,
        #[serde(default)]
        content_type: Option<String>,
    },
}

/// multipart 单个 part
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultipartPart {
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
}

/// `http_send` 命令参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpSendArgs {
    #[serde(default)]
    pub method: Option<String>,
    pub url: String,
    #[serde(default)]
    pub headers: Option<Vec<NameValue>>,
    #[serde(default)]
    pub query: Option<Vec<NameValue>>,
    /// 原始 Cookie 头（可选便捷项）
    #[serde(default)]
    pub cookies: Option<String>,
    #[serde(default)]
    pub body: Option<HttpSendBody>,
    /// `follow`（默认）| `manual`
    #[serde(default)]
    pub redirect: Option<String>,
    #[serde(default)]
    pub verify_ssl: Option<bool>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub user_agent: Option<String>,
    /// 代理覆盖：`None` = 沿用框架全局（`http_set_proxy`）；`""` = 直连；其他 = 覆盖
    #[serde(default)]
    pub proxy: Option<String>,
    /// `none`（默认，不自动管理）| `jar`（共享 cookie jar）
    #[serde(default)]
    pub cookie_mode: Option<String>,
    #[serde(default)]
    pub max_body_bytes: Option<u64>,
    /// 取消任务 id（配合框架 `task_cancel`）
    #[serde(default)]
    pub task_id: Option<String>,
}

/// `http_send` 命令响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpSendResult {
    pub status: u16,
    pub status_text: String,
    pub ok: bool,
    pub final_url: String,
    pub headers: Vec<HeaderEntry>,
    pub content_type: Option<String>,
    pub body_text: Option<String>,
    pub body_base64: Option<String>,
    pub is_binary: bool,
    pub size_bytes: u64,
    pub content_length: Option<u64>,
    pub elapsed_ms: u64,
    pub truncated: bool,
    pub redirect_location: Option<String>,
}

/// 按配置构建 Client（`http_send` 专用，与采集用全局 Client 隔离）
fn build_client_with(
    proxy: Option<&str>,
    cookie_jar: bool,
    verify_ssl: bool,
    follow_redirects: bool,
    timeout: Duration,
    user_agent: &str,
) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .user_agent(user_agent.to_string())
        .timeout(timeout)
        .cookie_store(cookie_jar);
    builder = if follow_redirects {
        builder.redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
    } else {
        builder.redirect(reqwest::redirect::Policy::none())
    };
    if !verify_ssl {
        builder = builder.danger_accept_invalid_certs(true);
    }
    if let Some(proxy) = proxy.filter(|value| !value.is_empty()) {
        match reqwest::Proxy::all(proxy) {
            Ok(configured) => builder = builder.proxy(configured),
            Err(err) => log::warn!("代理配置无效（已忽略）: {err}"),
        }
    }
    builder.build().expect("HTTP 客户端初始化失败")
}

/// 按配置签名缓存 Client：既避免重复建连池，又让 `cookie_mode=jar` 能跨请求保留 cookie
static SEND_CLIENTS: OnceLock<Mutex<HashMap<String, reqwest::Client>>> = OnceLock::new();

fn send_client(signature: &str, build: impl FnOnce() -> reqwest::Client) -> reqwest::Client {
    let cache = SEND_CLIENTS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(map) = cache.lock() {
        if let Some(client) = map.get(signature) {
            return client.clone();
        }
    }
    let client = build();
    if let Ok(mut map) = cache.lock() {
        if map.len() >= 32 {
            map.clear();
        }
        map.insert(signature.to_string(), client.clone());
    }
    client
}

/// 任务结束时清理注册（Drop 保证任意提前返回都会调用）
struct TaskGuard(Option<String>);

impl Drop for TaskGuard {
    fn drop(&mut self) {
        if let Some(id) = &self.0 {
            crate::tasks::end(id);
        }
    }
}

/// 粗略判断响应是否文本（决定走文本还是 base64）
fn is_textual_content_type(content_type: &str) -> bool {
    let ct = content_type.to_ascii_lowercase();
    ct.starts_with("text/")
        || ct.contains("json")
        || ct.contains("xml")
        || ct.contains("javascript")
        || ct.contains("ecmascript")
        || ct.contains("x-www-form-urlencoded")
        || ct.contains("graphql")
        || ct.contains("yaml")
        || ct.contains("csv")
}

/// application/x-www-form-urlencoded 单字段编码（unreserved 直出，空格转 `+`）
fn urlencode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// 将键值对编码为 application/x-www-form-urlencoded（跳过空名）
fn encode_form(fields: &[NameValue]) -> String {
    fields
        .iter()
        .filter(|field| !field.name.is_empty())
        .map(|field| {
            format!(
                "{}={}",
                urlencode_component(&field.name),
                urlencode_component(&field.value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// 组装 multipart 表单（文本字段 + 文件）
async fn build_multipart(parts: Vec<MultipartPart>) -> Result<reqwest::multipart::Form, AppError> {
    let mut form = reqwest::multipart::Form::new();
    for part in parts {
        if let Some(path) = part.file_path.filter(|value| !value.is_empty()) {
            let bytes = tokio::fs::read(&path)
                .await
                .map_err(|err| http_err(format!("读取上传文件失败（{path}）: {err}")))?;
            let file_name = part
                .file_name
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| {
                    std::path::Path::new(&path)
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_default()
                });
            let mut piece = reqwest::multipart::Part::bytes(bytes).file_name(file_name);
            if let Some(content_type) = part.content_type.filter(|value| !value.is_empty()) {
                piece = piece
                    .mime_str(&content_type)
                    .map_err(|err| AppError::invalid_input(format!("非法 Content-Type: {err}")))?;
            }
            form = form.part(part.name, piece);
        } else {
            form = form.text(part.name, part.value.unwrap_or_default());
        }
    }
    Ok(form)
}

/// 高级通用 HTTP 请求命令：参数化重定向 / SSL / 超时 / 代理 / cookie，
/// 支持 multipart、二进制响应、保序重复响应头与取消。
#[tauri::command]
pub async fn http_send(args: HttpSendArgs) -> Result<HttpSendResult, AppError> {
    let method_text = args.method.clone().unwrap_or_else(|| "GET".to_string());
    let method = reqwest::Method::from_bytes(method_text.to_uppercase().as_bytes())
        .map_err(|_| AppError::invalid_input(format!("非法 HTTP 方法: {method_text}")))?;
    if args.url.trim().is_empty() {
        return Err(AppError::invalid_input("URL 不能为空"));
    }

    let timeout_ms = args.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let follow_redirects = args.redirect.as_deref() != Some("manual");
    let verify_ssl = args.verify_ssl.unwrap_or(true);
    let cookie_jar = args.cookie_mode.as_deref() == Some("jar");
    let user_agent = args
        .user_agent
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());

    // 代理优先级：显式覆盖（""=直连） > 框架全局设置
    let proxy = match args.proxy.as_deref() {
        Some("") => None,
        Some(value) => Some(value.to_string()),
        None => proxy_setting(),
    };
    let signature = format!(
        "{}|{}|{}|{}|{}|{}",
        proxy.as_deref().unwrap_or(""),
        cookie_jar,
        verify_ssl,
        follow_redirects,
        timeout_ms,
        user_agent
    );
    let proxy_for_build = proxy.clone();
    let client = send_client(&signature, move || {
        build_client_with(
            proxy_for_build.as_deref(),
            cookie_jar,
            verify_ssl,
            follow_redirects,
            Duration::from_millis(timeout_ms),
            &user_agent,
        )
    });

    let mut request = client
        .request(method, &args.url)
        .timeout(Duration::from_millis(timeout_ms));

    if let Some(query) = &args.query {
        let pairs: Vec<(String, String)> = query
            .iter()
            .filter(|entry| !entry.name.is_empty())
            .map(|entry| (entry.name.clone(), entry.value.clone()))
            .collect();
        if !pairs.is_empty() {
            request = request.query(&pairs);
        }
    }

    // reqwest 的 RequestBuilder::header 是 **append** 语义：为避免 body 推导的
    // Content-Type 与用户显式头重复，这里先判断用户是否已带 Content-Type，
    // 仅在缺失时才补 body 推导值。
    let user_has_content_type = args.headers.as_ref().is_some_and(|headers| {
        headers
            .iter()
            .any(|h| h.name.eq_ignore_ascii_case("content-type"))
    });
    let cookies_present = args
        .cookies
        .as_deref()
        .is_some_and(|value| !value.is_empty());

    let mut is_multipart = false;
    if let Some(body) = &args.body {
        match body {
            HttpSendBody::Raw { text, content_type } => {
                if !user_has_content_type {
                    if let Some(ct) = content_type.as_deref().filter(|value| !value.is_empty()) {
                        request = request.header(reqwest::header::CONTENT_TYPE, ct);
                    }
                }
                request = request.body(text.clone());
            }
            HttpSendBody::Form { fields } => {
                if !user_has_content_type {
                    request = request.header(
                        reqwest::header::CONTENT_TYPE,
                        "application/x-www-form-urlencoded",
                    );
                }
                request = request.body(encode_form(fields));
            }
            HttpSendBody::Multipart { parts } => {
                is_multipart = true;
                request = request.multipart(build_multipart(parts.clone()).await?);
            }
            HttpSendBody::Binary { path, content_type } => {
                let bytes = tokio::fs::read(path)
                    .await
                    .map_err(|err| http_err(format!("读取请求文件失败（{path}）: {err}")))?;
                if !user_has_content_type {
                    if let Some(ct) = content_type.as_deref().filter(|value| !value.is_empty()) {
                        request = request.header(reqwest::header::CONTENT_TYPE, ct);
                    }
                }
                request = request.body(bytes);
            }
        }
    }

    if let Some(headers) = &args.headers {
        for entry in headers {
            if entry.name.is_empty() {
                continue;
            }
            // multipart 的 Content-Type 含 boundary，由 reqwest 生成，忽略用户覆盖
            if is_multipart && entry.name.eq_ignore_ascii_case("content-type") {
                log::warn!("multipart 请求体忽略自定义 Content-Type（boundary 由客户端生成）");
                continue;
            }
            // `cookies` 参数优先：避免与之重复的 Cookie 头
            if cookies_present && entry.name.eq_ignore_ascii_case("cookie") {
                continue;
            }
            request = request.header(&entry.name, &entry.value);
        }
    }
    if let Some(cookies) = args.cookies.as_deref().filter(|value| !value.is_empty()) {
        request = request.header(reqwest::header::COOKIE, cookies);
    }

    // 取消令牌：注册后经轮询检测，send 与读取响应阶段均可中止
    let cancel_flag = args.task_id.as_ref().map(|id| crate::tasks::begin(id));
    let _task_guard = TaskGuard(args.task_id.clone());

    let started = Instant::now();
    let response = if let Some(flag) = &cancel_flag {
        let send_future = request.send();
        tokio::pin!(send_future);
        loop {
            tokio::select! {
                result = &mut send_future => break result,
                _ = tokio::time::sleep(Duration::from_millis(150)) => {
                    if flag.load(Ordering::Relaxed) {
                        return Err(http_err("请求已取消"));
                    }
                }
            }
        }
    } else {
        request.send().await
    }
    .map_err(|err| http_err(err.to_string()))?;

    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or_default()
        .to_string();
    let final_url = response.url().to_string();
    let content_length = response.content_length();

    // 保序 + 保留重复项（如多条 Set-Cookie）
    let mut headers: Vec<HeaderEntry> = Vec::new();
    for (name, value) in response.headers().iter() {
        headers.push(HeaderEntry {
            name: name.to_string(),
            value: value.to_str().unwrap_or_default().to_string(),
        });
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let redirect_location = if (300..400).contains(&status) {
        response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string())
    } else {
        None
    };

    let cap = args
        .max_body_bytes
        .map(|value| value as usize)
        .unwrap_or(MAX_BODY_BYTES);

    let mut response = response;
    let mut body_bytes: Vec<u8> = Vec::new();
    let mut truncated = false;
    loop {
        if let Some(flag) = &cancel_flag {
            if flag.load(Ordering::Relaxed) {
                return Err(http_err("请求已取消"));
            }
        }
        let chunk = response
            .chunk()
            .await
            .map_err(|err| http_err(err.to_string()))?;
        let Some(chunk) = chunk else { break };
        let remaining = cap.saturating_sub(body_bytes.len());
        if chunk.len() > remaining {
            body_bytes.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        body_bytes.extend_from_slice(&chunk);
        if body_bytes.len() >= cap {
            truncated = true;
            break;
        }
    }

    let size_bytes = body_bytes.len() as u64;
    let textual = content_type
        .as_deref()
        .map(is_textual_content_type)
        .unwrap_or(false);
    let (body_text, body_base64, is_binary) = if textual {
        (
            Some(String::from_utf8_lossy(&body_bytes).to_string()),
            None,
            false,
        )
    } else {
        match std::str::from_utf8(&body_bytes) {
            Ok(text) => (Some(text.to_string()), None, false),
            Err(_) => (
                None,
                Some(base64::engine::general_purpose::STANDARD.encode(&body_bytes)),
                true,
            ),
        }
    };

    Ok(HttpSendResult {
        status,
        status_text,
        ok: (200..300).contains(&status),
        final_url,
        headers,
        content_type,
        body_text,
        body_base64,
        is_binary,
        size_bytes,
        content_length,
        elapsed_ms: started.elapsed().as_millis() as u64,
        truncated,
        redirect_location,
    })
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

    #[tokio::test]
    #[ignore = "依赖 httpbin.org 外部网络，手动运行：cargo test -- --ignored"]
    async fn http_send_smoke() {
        let args = HttpSendArgs {
            method: Some("POST".to_string()),
            url: "https://httpbin.org/post".to_string(),
            headers: Some(vec![NameValue {
                name: "X-Test".to_string(),
                value: "1".to_string(),
            }]),
            query: Some(vec![NameValue {
                name: "q".to_string(),
                value: "ark".to_string(),
            }]),
            cookies: Some("session=abc".to_string()),
            body: Some(HttpSendBody::Raw {
                text: "{\"hello\":\"world\"}".to_string(),
                content_type: Some("application/json".to_string()),
            }),
            redirect: None,
            verify_ssl: None,
            timeout_ms: Some(15000),
            user_agent: None,
            proxy: Some(String::new()),
            cookie_mode: None,
            max_body_bytes: None,
            task_id: None,
        };
        let resp = http_send(args).await.expect("HTTP 请求失败");
        assert_eq!(resp.status, 200);
        assert!(resp.ok);
        assert!(resp.body_text.unwrap_or_default().contains("ark"));
    }
}
