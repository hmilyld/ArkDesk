//! 本地 MITM 拦截引擎（hudsucker）。
//!
//! - 仅绑定 `127.0.0.1`，手动启动/停止，常驻直到手动停止
//! - HTTP/1.1 连接内请求-响应由 hudsucker 同一 handler 实例配对
//! - 流量存内存环形缓冲；体用 Tee 抓取（不破坏流式）
//! - WebSocket 只读记录（独立于 HTTP flow）

pub mod body;
pub mod ca;
pub mod dto;
pub mod handler;
pub mod state;
pub mod system_proxy;

use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use hudsucker::certificate_authority::RcgenAuthority;
use hudsucker::Proxy;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot};

use crate::error::{code, AppError};

use dto::{
    CaInfo, FlowRecord, FlowSummary, ProxyConfig, ProxyStatus, SystemProxyStatus, WsRecord,
    WsSummary,
};
use handler::InterceptHandler;
use state::{Outbox, SharedState};

static SHARED: OnceLock<Arc<SharedState>> = OnceLock::new();
static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

struct Running {
    port: u16,
    shutdown: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

/// 全局共享状态
pub fn shared() -> Arc<SharedState> {
    SHARED
        .get_or_init(|| Arc::new(SharedState::new()))
        .clone()
}

fn is_running() -> bool {
    RUNNING
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

/// 启动代理（幂等：已运行直接返回状态）
pub async fn start(
    app: AppHandle,
    port: u16,
    config: ProxyConfig,
) -> Result<ProxyStatus, AppError> {
    if port == 0 {
        return Err(AppError::invalid_input("端口不能为 0"));
    }

    let state = shared();
    if let Ok(mut guard) = state.config.lock() {
        *guard = config;
    }
    if is_running() {
        return Ok(status(&app));
    }

    let material = ca::ensure(&app)?;
    let provider = hudsucker::rustls::crypto::ring::default_provider();
    let authority = RcgenAuthority::new(material.issuer, 1000, provider.clone());

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|err| AppError::custom(code::HTTP_ERROR, format!("端口 {port} 绑定失败：{err}")))?;

    let (outbox_tx, outbox_rx) = mpsc::unbounded_channel::<Outbox>();
    state.set_outbox(Some(outbox_tx));
    spawn_emitter(app.clone(), outbox_rx);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let handler = InterceptHandler::new(state.clone());

    let proxy = Proxy::builder()
        .with_listener(listener)
        .with_ca(authority)
        .with_rustls_connector(provider)
        .with_http_handler(handler.clone())
        .with_websocket_handler(handler)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .build()
        .map_err(|err| AppError::custom(code::HTTP_ERROR, format!("代理构建失败：{err}")))?;

    let task = tokio::spawn(async move {
        if let Err(err) = proxy.start().await {
            log::error!("网络拦截代理退出：{err}");
        }
    });

    if let Ok(mut guard) = RUNNING.lock() {
        *guard = Some(Running {
            port,
            shutdown: Some(shutdown_tx),
            task,
        });
    }
    log::info!("网络拦截代理已启动：127.0.0.1:{port}");
    Ok(status(&app))
}

/// 停止代理（幂等）；若托管了系统代理则一并还原
pub async fn stop(app: &AppHandle) -> ProxyStatus {
    let running = RUNNING.lock().ok().and_then(|mut guard| guard.take());
    if let Some(mut running) = running {
        if let Some(tx) = running.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = tokio::time::timeout(Duration::from_secs(5), running.task).await;
        shared().set_outbox(None);
        log::info!("网络拦截代理已停止");
    }
    if system_proxy::status(app).enabled {
        if let Err(err) = system_proxy::disable(app).await {
            log::warn!("还原系统代理失败：{err}");
        }
    }
    status(app)
}

/// 当前状态
pub fn status(app: &AppHandle) -> ProxyStatus {
    let (running, port) = RUNNING
        .lock()
        .map(|guard| match guard.as_ref() {
            Some(current) => (true, Some(current.port)),
            None => (false, None),
        })
        .unwrap_or((false, None));
    let state = shared();
    ProxyStatus {
        running,
        port,
        flow_count: state.flow_count(),
        ws_count: state.ws_count(),
        ca: ca::info(app),
    }
}

/// 热更新配置
pub fn configure(config: ProxyConfig) {
    let state = shared();
    if let Ok(mut guard) = state.config.lock() {
        *guard = config;
    };
}

pub fn flows_list() -> Vec<FlowSummary> {
    shared().flows_list()
}

pub fn flow_get(id: u64) -> Result<FlowRecord, AppError> {
    shared()
        .flow_get(id)
        .ok_or_else(|| AppError::not_found(format!("流量 {id} 不存在或已被淘汰")))
}

pub fn flows_clear() {
    shared().clear_flows();
}

pub fn ws_list() -> Vec<WsSummary> {
    shared().ws_list()
}

pub fn ws_get(id: u64) -> Result<WsRecord, AppError> {
    shared()
        .ws_get(id)
        .ok_or_else(|| AppError::not_found(format!("WebSocket {id} 不存在")))
}

pub fn ws_clear() {
    shared().clear_ws();
}

/// 把捕获的二进制体写入缓存目录并返回路径（供重放 / 另存）
pub fn flow_body_temp(app: &AppHandle, id: u64, side: &str) -> Result<String, AppError> {
    use base64::Engine as _;
    use tauri::Manager;

    let record = flow_get(id)?;
    let body = if side == "req" {
        record.req_body
    } else {
        record.res_body
    };
    let body = body.ok_or_else(|| AppError::not_found("该流量没有可导出的请求体"))?;
    let encoded = body
        .base64
        .ok_or_else(|| AppError::invalid_input("该请求体不是二进制内容"))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("base64 解码失败: {err}")))?;
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("无法解析缓存目录: {err}")))?
        .join("network-tools");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("flow-{id}-{side}.bin"));
    std::fs::write(&path, bytes)?;
    Ok(path.to_string_lossy().to_string())
}

// ─────────────────────────── 系统代理 ───────────────────────────

pub async fn system_proxy_enable(
    app: &AppHandle,
    port: u16,
) -> Result<SystemProxyStatus, AppError> {
    system_proxy::enable(app, port).await
}

pub async fn system_proxy_disable(app: &AppHandle) -> Result<SystemProxyStatus, AppError> {
    system_proxy::disable(app).await
}

pub fn system_proxy_status(app: &AppHandle) -> SystemProxyStatus {
    system_proxy::status(app)
}

pub fn ca_info(app: &AppHandle) -> CaInfo {
    ca::info(app)
}

pub fn ca_export(app: &AppHandle, path: &str) -> Result<(), AppError> {
    ca::export(app, path)
}

pub fn ca_regenerate(app: &AppHandle) -> Result<CaInfo, AppError> {
    if is_running() {
        return Err(AppError::invalid_input("请先停止代理再重新生成证书"));
    }
    ca::regenerate(app)
}

/// 批量事件发送（约 200ms 合并一次，避免 IPC 洪泛）
fn spawn_emitter(app: AppHandle, mut rx: mpsc::UnboundedReceiver<Outbox>) {
    tokio::spawn(async move {
        loop {
            let Some(first) = rx.recv().await else {
                break;
            };
            let mut flows: Vec<FlowSummary> = Vec::new();
            let mut ws: Vec<WsSummary> = Vec::new();
            match first {
                Outbox::Flow(flow) => flows.push(flow),
                Outbox::Ws(frame) => ws.push(frame),
            }
            let deadline = tokio::time::Instant::now() + Duration::from_millis(200);
            loop {
                match tokio::time::timeout_at(deadline, rx.recv()).await {
                    Ok(Some(Outbox::Flow(flow))) => flows.push(flow),
                    Ok(Some(Outbox::Ws(frame))) => ws.push(frame),
                    Ok(None) | Err(_) => break,
                }
            }
            if !flows.is_empty() {
                let _ = app.emit("network-tools://flow", &flows);
            }
            if !ws.is_empty() {
                let _ = app.emit("network-tools://ws", &ws);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use hudsucker::rcgen::{
        BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair,
    };
    use std::net::SocketAddr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::task::JoinHandle;

    /// 测试用根证书：返回 CA 与证书 PEM
    fn test_authority() -> (RcgenAuthority, String) {
        let key = KeyPair::generate().expect("key");
        let mut params = CertificateParams::default();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "network-tools test ca");
        params.distinguished_name = dn;
        let cert = params.self_signed(&key).expect("cert");
        let pem = cert.pem();
        let issuer = Issuer::from_ca_cert_pem(&pem, key).expect("issuer");
        (
            RcgenAuthority::new(
                issuer,
                10,
                hudsucker::rustls::crypto::ring::default_provider(),
            ),
            pem,
        )
    }

    /// 独立共享状态（事件出口无消费者，测试不关心）
    fn test_state() -> Arc<SharedState> {
        let state = Arc::new(SharedState::new());
        let (tx, _rx) = mpsc::unbounded_channel();
        state.set_outbox(Some(tx));
        state
    }

    /// 启动监听回环的测试代理：返回 (地址, CA PEM, 任务)
    async fn spawn_test_proxy(state: Arc<SharedState>) -> (SocketAddr, String, JoinHandle<()>) {
        let handler = InterceptHandler::new(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("proxy listener");
        let addr = listener.local_addr().expect("proxy addr");
        let (authority, pem) = test_authority();
        let proxy = Proxy::builder()
            .with_listener(listener)
            .with_ca(authority)
            .with_rustls_connector(hudsucker::rustls::crypto::ring::default_provider())
            .with_http_handler(handler.clone())
            .with_websocket_handler(handler)
            .with_graceful_shutdown(std::future::pending::<()>())
            .build()
            .expect("build proxy");
        let task = tokio::spawn(async move {
            let _ = proxy.start().await;
        });
        (addr, pem, task)
    }

    /// 启动返回固定响应的上游 HTTP 服务，返回其地址
    async fn spawn_upstream(response: &'static str) -> SocketAddr {
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("upstream");
        let addr = upstream.local_addr().expect("upstream addr");
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = upstream.accept().await {
                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    let _ = sock.read(&mut buffer).await;
                    let _ = sock.write_all(response.as_bytes()).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        addr
    }

    /// 等待至少 n 条流量被记录（响应体流结束后才落库）
    async fn wait_for_flows(state: &SharedState, n: usize) {
        for _ in 0..100 {
            if state.flow_count() >= n {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// 经代理的测试客户端（可选注入 CA 以信任 HTTPS MITM）
    fn test_client(proxy_addr: SocketAddr, ca_pem: Option<&str>) -> reqwest::Client {
        let mut builder = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(format!("http://{proxy_addr}")).expect("proxy"));
        if let Some(pem) = ca_pem {
            let cert = reqwest::Certificate::from_pem(pem.as_bytes()).expect("root cert");
            builder = builder.add_root_certificate(cert);
        }
        builder.build().expect("client")
    }

    /// 本地回环：经代理发起 HTTP 请求，验证捕获与文本化。
    #[tokio::test]
    #[ignore = "本地端到端冒烟，手动运行：cargo test -- --ignored proxy_captures"]
    async fn proxy_captures_http_flow() {
        let upstream = spawn_upstream(
            "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nhello",
        )
        .await;
        let state = test_state();
        let (proxy_addr, _pem, task) = spawn_test_proxy(state.clone()).await;

        let client = test_client(proxy_addr, None);
        let response = client
            .get(format!("http://{upstream}/ping"))
            .send()
            .await
            .expect("send");
        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(response.text().await.expect("body"), "hello");

        wait_for_flows(&state, 1).await;
        let flows = state.flows_list();
        assert_eq!(flows.len(), 1, "应捕获 1 条流量");
        assert_eq!(flows[0].status, Some(200));
        let record = state.flow_get(flows[0].id).expect("record");
        assert_eq!(record.res_body.and_then(|body| body.text), Some("hello".to_string()));

        task.abort();
    }

    /// 分块传输（chunked）响应体也应被完整抓取。
    #[tokio::test]
    #[ignore = "本地端到端冒烟，手动运行：cargo test -- --ignored proxy_captures_chunked"]
    async fn proxy_captures_chunked_body() {
        let upstream = spawn_upstream(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
        )
        .await;
        let state = test_state();
        let (proxy_addr, _pem, task) = spawn_test_proxy(state.clone()).await;

        let client = test_client(proxy_addr, None);
        let body = client
            .get(format!("http://{upstream}/chunked"))
            .send()
            .await
            .expect("send")
            .text()
            .await
            .expect("body");
        assert_eq!(body, "hello");

        wait_for_flows(&state, 1).await;
        let flows = state.flows_list();
        assert_eq!(flows.len(), 1, "应捕获 1 条流量");
        let record = state.flow_get(flows[0].id).expect("record");
        assert_eq!(record.res_body.and_then(|body| body.text), Some("hello".to_string()));

        task.abort();
    }

    /// 307 重定向及其后的目标请求都应被捕获。
    #[tokio::test]
    #[ignore = "本地端到端冒烟，手动运行：cargo test -- --ignored proxy_captures_redirect"]
    async fn proxy_captures_redirect_chain() {
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("upstream");
        let upstream_addr = upstream.local_addr().expect("upstream addr");
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = upstream.accept().await {
                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    let n = sock.read(&mut buffer).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buffer[..n]).to_string();
                    let response = if request.contains("/a ") {
                        "HTTP/1.1 307 Temporary Redirect\r\nLocation: /b\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    } else {
                        "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\ndone"
                    };
                    let _ = sock.write_all(response.as_bytes()).await;
                    let _ = sock.shutdown().await;
                });
            }
        });

        let state = test_state();
        let (proxy_addr, _pem, task) = spawn_test_proxy(state.clone()).await;
        let client = test_client(proxy_addr, None);
        let body = client
            .get(format!("http://{upstream_addr}/a"))
            .send()
            .await
            .expect("send")
            .text()
            .await
            .expect("body");
        assert_eq!(body, "done");

        wait_for_flows(&state, 2).await;
        let flows = state.flows_list();
        assert!(flows.len() >= 2, "应捕获重定向链路的至少 2 条流量，实际 {}", flows.len());
        assert!(flows.iter().any(|flow| flow.status == Some(307)), "应包含 307");

        task.abort();
    }

    /// 依赖外网：验证 HTTPS MITM（用测试 CA 作为客户端信任根）。
    #[tokio::test]
    #[ignore = "依赖外网 example.com，手动运行：cargo test -- --ignored proxy_intercepts_https"]
    async fn proxy_intercepts_https_flow() {
        let state = test_state();
        let (proxy_addr, pem, task) = spawn_test_proxy(state.clone()).await;

        let client = test_client(proxy_addr, Some(&pem));
        let response = client
            .get("https://example.com/")
            .send()
            .await
            .expect("send");
        assert_eq!(response.status().as_u16(), 200);

        wait_for_flows(&state, 1).await;
        let flows = state.flows_list();
        let flow = flows
            .iter()
            .find(|flow| flow.host == "example.com")
            .expect("example.com flow");
        assert_eq!(flow.scheme, "https");
        assert_eq!(flow.status, Some(200));

        task.abort();
    }
}
