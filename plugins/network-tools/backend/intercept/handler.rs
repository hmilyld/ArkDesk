//! MITM 处理器：HTTP 请求/响应（Tee 抓体 + 同实例配对）与 WebSocket 只读记录。

use std::sync::{Arc, Mutex};
use std::time::Instant;

use hudsucker::hyper::body::{Body as HttpBodyTrait, Bytes};
use hudsucker::hyper::{Method, Request, Response, StatusCode};
use hudsucker::hyper_util::client::legacy::Error as ClientError;
use hudsucker::{
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use http_body_util::combinators::BoxBody;
use hudsucker::tokio_tungstenite::tungstenite::Message;

use super::body::{capture_sink, BodySink, TeeBody};
use super::dto::{now_ms, FlowRecord, FlowSummary, HeaderPair, ProxyConfig, WsFrame};
use super::state::{Outbox, SharedState};

/// 每个请求由 hudsucker 克隆一份 handler，故 `pending` 天然与本次请求-响应配对。
#[derive(Clone)]
pub struct InterceptHandler {
    state: Arc<SharedState>,
    pending: Option<PendingFlow>,
}

#[derive(Clone)]
struct PendingFlow {
    summary: FlowSummary,
    client_addr: String,
    req_headers: Vec<HeaderPair>,
    req_sink: Arc<Mutex<BodySink>>,
    started: Instant,
}

struct FinalizeCtx {
    state: Arc<SharedState>,
    summary: FlowSummary,
    client_addr: String,
    req_headers: Vec<HeaderPair>,
    res_headers: Vec<HeaderPair>,
    req_sink: Arc<Mutex<BodySink>>,
    res_sink: Option<Arc<Mutex<BodySink>>>,
}

impl InterceptHandler {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self {
            state,
            pending: None,
        }
    }
}

fn header_pairs(headers: &hudsucker::hyper::HeaderMap) -> Vec<HeaderPair> {
    headers
        .iter()
        .map(|(name, value)| HeaderPair {
            name: name.to_string(),
            value: value.to_str().unwrap_or_default().to_string(),
        })
        .collect()
}

fn find_header(headers: &[HeaderPair], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.clone())
}

/// 单次抓体上限（字节）
fn body_limit(config: &ProxyConfig) -> usize {
    (config.max_body_kb as usize).saturating_mul(1024)
}

fn is_websocket_upgrade(req: &Request<Body>) -> bool {
    req.headers()
        .get("upgrade")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}

fn finalize(ctx: FinalizeCtx) {
    let req_ct = find_header(&ctx.req_headers, "content-type");
    let req_ce = find_header(&ctx.req_headers, "content-encoding");
    let req_body = capture_sink(&ctx.req_sink, req_ct.as_deref(), req_ce.as_deref());
    let res_body = ctx.res_sink.as_ref().map(|sink| {
        let ct = find_header(&ctx.res_headers, "content-type");
        let ce = find_header(&ctx.res_headers, "content-encoding");
        capture_sink(sink, ct.as_deref(), ce.as_deref())
    });

    let mut summary = ctx.summary;
    summary.req_size = req_body.size;
    summary.res_size = res_body.as_ref().map(|body| body.size).unwrap_or(0);

    let record = FlowRecord {
        summary: summary.clone(),
        client_addr: ctx.client_addr,
        req_headers: ctx.req_headers,
        res_headers: ctx.res_headers,
        req_body: if req_body.size > 0 { Some(req_body) } else { None },
        res_body: res_body.filter(|body| body.size > 0),
    };
    let emitted = ctx.state.push_flow(record);
    ctx.state.emit(Outbox::Flow(emitted));
}

impl HttpHandler for InterceptHandler {
    async fn handle_request(
        &mut self,
        ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        // CONNECT 隧道本身不记录（其内部解密后的请求会再次进入本处理器）
        if req.method() == Method::CONNECT {
            return req.into();
        }

        let method = req.method().clone();
        let uri = req.uri().clone();
        let url = uri.to_string();
        let scheme = uri.scheme_str().unwrap_or("http").to_string();
        let host = uri.host().unwrap_or_default().to_string();
        let client_addr = ctx.client_addr.to_string();
        let req_headers = header_pairs(req.headers());
        let is_upgrade = is_websocket_upgrade(&req);
        let cfg = self
            .state
            .config
            .lock()
            .map(|config| config.clone())
            .unwrap_or_default();
        let limit = body_limit(&cfg);
        let req_sink = Arc::new(Mutex::new(BodySink::new()));

        let summary = FlowSummary {
            id: self.state.next_id(),
            started_at: now_ms(),
            method: method.to_string(),
            scheme,
            host,
            url,
            status: if is_upgrade { Some(101) } else { None },
            duration_ms: if is_upgrade { Some(0) } else { None },
            req_size: 0,
            res_size: 0,
            error: None,
            is_upgrade,
            content_type: None,
        };

        // WebSocket 升级请求不会走 handle_response，立即落库
        if is_upgrade {
            let record = FlowRecord {
                summary: summary.clone(),
                client_addr,
                req_headers,
                res_headers: Vec::new(),
                req_body: None,
                res_body: None,
            };
            let emitted = self.state.push_flow(record);
            self.state.emit(Outbox::Flow(emitted));
            return req.into();
        }

        self.pending = Some(PendingFlow {
            summary,
            client_addr,
            req_headers,
            req_sink: req_sink.clone(),
            started: Instant::now(),
        });

        if cfg.record_bodies {
            let (parts, body) = req.into_parts();
            let tee = TeeBody::new(body, req_sink, limit, None);
            let boxed: BoxBody<Bytes, hudsucker::Error> = BoxBody::new(tee);
            RequestOrResponse::Request(Request::from_parts(parts, Body::from(boxed)))
        } else {
            req.into()
        }
    }

    async fn handle_response(&mut self, _ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        let Some(pending) = self.pending.take() else {
            return res;
        };

        let status = res.status().as_u16();
        let res_headers = header_pairs(res.headers());
        let cfg = self
            .state
            .config
            .lock()
            .map(|config| config.clone())
            .unwrap_or_default();
        let limit = body_limit(&cfg);

        let mut summary = pending.summary.clone();
        summary.status = Some(status);
        summary.duration_ms = Some(pending.started.elapsed().as_millis() as u64);
        summary.content_type = find_header(&res_headers, "content-type");

        // 空体（204/HEAD/304 等）不会触发 Tee 结束回调，直接落库
        let empty_body = HttpBodyTrait::is_end_stream(res.body());
        if !cfg.record_bodies || empty_body {
            finalize(FinalizeCtx {
                state: self.state.clone(),
                summary,
                client_addr: pending.client_addr,
                req_headers: pending.req_headers,
                res_headers,
                req_sink: pending.req_sink,
                res_sink: None,
            });
            return res;
        }

        let res_sink = Arc::new(Mutex::new(BodySink::new()));
        let finalize_ctx = FinalizeCtx {
            state: self.state.clone(),
            summary,
            client_addr: pending.client_addr,
            req_headers: pending.req_headers,
            res_headers,
            req_sink: pending.req_sink,
            res_sink: Some(res_sink.clone()),
        };
        let on_end: Box<dyn FnOnce() + Send + Sync> = Box::new(move || finalize(finalize_ctx));
        let (parts, body) = res.into_parts();
        let tee = TeeBody::new(body, res_sink, limit, Some(on_end));
        let boxed: BoxBody<Bytes, hudsucker::Error> = BoxBody::new(tee);
        Response::from_parts(parts, Body::from(boxed))
    }

    async fn handle_error(&mut self, _ctx: &HttpContext, err: ClientError) -> Response<Body> {
        let message = err.to_string();
        if let Some(pending) = self.pending.take() {
            let mut summary = pending.summary.clone();
            summary.error = Some(message.clone());
            summary.duration_ms = Some(pending.started.elapsed().as_millis() as u64);
            finalize(FinalizeCtx {
                state: self.state.clone(),
                summary,
                client_addr: pending.client_addr,
                req_headers: pending.req_headers,
                res_headers: Vec::new(),
                req_sink: pending.req_sink,
                res_sink: None,
            });
        }
        Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from(message))
            .expect("构建 502 响应失败")
    }
}

impl WebSocketHandler for InterceptHandler {
    async fn handle_message(
        &mut self,
        ctx: &WebSocketContext,
        msg: Message,
    ) -> Option<Message> {
        let (dir, client_addr, uri) = match ctx {
            WebSocketContext::ClientToServer { src, dst, .. } => {
                ("up", src.to_string(), dst.clone())
            }
            WebSocketContext::ServerToClient { src, dst, .. } => {
                ("down", dst.to_string(), src.clone())
            }
        };
        let url = uri.to_string();
        let host = uri.host().unwrap_or_default().to_string();
        let (opcode, text) = match &msg {
            Message::Text(value) => ("text", Some(value.to_string())),
            Message::Binary(value) => ("binary", Some(format!("<{} bytes>", value.len()))),
            Message::Ping(_) => ("ping", None),
            Message::Pong(_) => ("pong", None),
            Message::Close(_) => ("close", None),
            Message::Frame(_) => ("frame", None),
        };
        let frame = WsFrame {
            dir: dir.to_string(),
            opcode: opcode.to_string(),
            len: msg.len(),
            text,
            at: now_ms(),
        };
        let max = self
            .state
            .config
            .lock()
            .map(|config| config.max_ws_frames)
            .unwrap_or(500);
        let summary = self
            .state
            .ws_push(format!("{client_addr}|{url}"), host, url, frame, max);
        self.state.emit(Outbox::Ws(summary));
        Some(msg)
    }
}
