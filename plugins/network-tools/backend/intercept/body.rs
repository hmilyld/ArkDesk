//! 体抓取：Tee 包装（边转发边复制，不破坏流式）+ 解压/文本化。

use std::io::Read;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use base64::Engine as _;
use hudsucker::hyper::body::{Body as HttpBody, Bytes, Frame, SizeHint};

use super::dto::BodyCapture;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// 抓取缓冲（原始字节，受上限约束）
#[derive(Default)]
pub struct BodySink {
    bytes: Vec<u8>,
    truncated: bool,
}

impl BodySink {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加数据，超过上限则截断（仍继续接收由调用方停止）
    pub fn push(&mut self, data: &[u8], limit: usize) {
        if self.truncated {
            return;
        }
        let remaining = limit.saturating_sub(self.bytes.len());
        if data.len() > remaining {
            self.bytes.extend_from_slice(&data[..remaining]);
            self.truncated = true;
        } else {
            self.bytes.extend_from_slice(data);
        }
    }
}

type EndCallback = Box<dyn FnOnce() + Send + Sync>;

/// 透传 body 的同时把数据复制到 [`BodySink`]；流结束时触发回调。
pub struct TeeBody<B> {
    inner: B,
    sink: Arc<Mutex<BodySink>>,
    limit: usize,
    on_end: Option<EndCallback>,
    ended: bool,
}

impl<B> TeeBody<B> {
    pub fn new(
        inner: B,
        sink: Arc<Mutex<BodySink>>,
        limit: usize,
        on_end: Option<EndCallback>,
    ) -> Self {
        Self {
            inner,
            sink,
            limit,
            on_end,
            ended: false,
        }
    }

    fn finish(&mut self) {
        if self.ended {
            return;
        }
        self.ended = true;
        if let Some(callback) = self.on_end.take() {
            callback();
        }
    }
}

impl<B> HttpBody for TeeBody<B>
where
    B: HttpBody<Data = Bytes> + Unpin,
    B::Error: Send + Sync + 'static,
{
    type Data = Bytes;
    type Error = B::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let result = Pin::new(&mut this.inner).poll_frame(cx);
        match &result {
            Poll::Ready(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    if let Ok(mut sink) = this.sink.lock() {
                        sink.push(data, this.limit);
                    }
                }
                // 已知长度时 hyper 读满即止、不会再 poll 到 None，需要此处判定结束
                if this.inner.is_end_stream() {
                    this.finish();
                }
            }
            Poll::Ready(Some(Err(_))) | Poll::Ready(None) => this.finish(),
            Poll::Pending => {}
        }
        result
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

/// 从体缓冲生成展示用 [`BodyCapture`]
pub fn capture_sink(
    sink: &Arc<Mutex<BodySink>>,
    content_type: Option<&str>,
    content_encoding: Option<&str>,
) -> BodyCapture {
    let (bytes, truncated) = match sink.lock() {
        Ok(guard) => (guard.bytes.clone(), guard.truncated),
        Err(_) => (Vec::new(), false),
    };
    capture(&bytes, content_type, content_encoding, truncated)
}

/// 粗略判断是否文本内容。
/// 说明：与框架 `src-tauri/src/http.rs` 的同名判断**有意分离**——插件不依赖框架内部实现，
/// 两者可各自演进（此处额外覆盖 svg）。
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
        || ct.contains("svg")
}

fn charset_of(content_type: &str) -> Option<String> {
    for part in content_type.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("charset=") {
            return Some(value.trim_matches('"').to_string());
        }
        let lower = part.to_ascii_lowercase();
        if let Some(index) = lower.find("charset=") {
            return Some(part[index + 8..].trim_matches('"').to_string());
        }
    }
    None
}

fn decompress(encoding: &str, data: &[u8]) -> Option<Vec<u8>> {
    match encoding.trim().to_ascii_lowercase().as_str() {
        "gzip" | "x-gzip" => read_all(flate2::read::GzDecoder::new(data)),
        "deflate" => read_all(flate2::read::ZlibDecoder::new(data))
            .or_else(|| read_all(flate2::read::DeflateDecoder::new(data))),
        "br" => {
            let mut out = Vec::new();
            brotli::BrotliDecompress(&mut &data[..], &mut out).ok().map(|_| out)
        }
        "zstd" => zstd::stream::decode_all(data).ok(),
        _ => None,
    }
}

fn read_all<R: Read>(mut reader: R) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    reader.read_to_end(&mut out).ok().map(|_| out)
}

/// 将原始字节转为可展示的 [`BodyCapture`]
pub fn capture(
    bytes: &[u8],
    content_type: Option<&str>,
    content_encoding: Option<&str>,
    truncated: bool,
) -> BodyCapture {
    let ct = content_type.unwrap_or("");
    let encoding = content_encoding.unwrap_or("").trim().to_ascii_lowercase();
    let mut decoded = false;
    let mut note: Option<String> = None;

    let display: Vec<u8> = if !encoding.is_empty() && encoding != "identity" {
        match decompress(&encoding, bytes) {
            Some(out) => {
                decoded = true;
                out
            }
            None => {
                note = Some(format!("未能解压（Content-Encoding: {encoding}），展示原始字节"));
                bytes.to_vec()
            }
        }
    } else {
        bytes.to_vec()
    };

    if truncated {
        let extra = "响应/请求体超过上限，已截断";
        note = Some(match note {
            Some(existing) => format!("{existing}；{extra}"),
            None => extra.to_string(),
        });
    }

    let textual = is_textual_content_type(ct) || std::str::from_utf8(&display).is_ok();
    if textual {
        let charset = charset_of(ct);
        let text = match charset
            .as_deref()
            .and_then(|label| encoding_rs::Encoding::for_label(label.as_bytes()))
        {
            Some(encoding) => encoding.decode(&display).0.into_owned(),
            None => match String::from_utf8(display.clone()) {
                Ok(value) => value,
                Err(_) => String::from_utf8_lossy(&display).into_owned(),
            },
        };
        BodyCapture {
            text: Some(text),
            base64: None,
            is_binary: false,
            size: bytes.len() as u64,
            truncated,
            content_type: content_type.map(|value| value.to_string()),
            decoded,
            note,
        }
    } else {
        BodyCapture {
            text: None,
            base64: Some(B64.encode(bytes)),
            is_binary: true,
            size: bytes.len() as u64,
            truncated,
            content_type: content_type.map(|value| value.to_string()),
            decoded,
            note,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sink_truncates() {
        let mut sink = BodySink::new();
        sink.push(b"hello", 3);
        assert_eq!(sink.bytes, b"hel");
        assert!(sink.truncated);
        sink.push(b"more", 3);
        assert_eq!(sink.bytes, b"hel");
    }

    #[test]
    fn capture_json_text() {
        let out = capture(br#"{"a":1}"#, Some("application/json"), None, false);
        assert!(!out.is_binary);
        assert_eq!(out.text.as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn capture_binary_base64() {
        let out = capture(&[0xff, 0x00, 0x01], Some("application/octet-stream"), None, false);
        assert!(out.is_binary);
        assert!(out.base64.is_some());
    }

    #[test]
    fn capture_decodes_gzip() {
        use flate2::write::GzEncoder;
        use std::io::Write;
        let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"hello gzip").unwrap();
        let compressed = encoder.finish().unwrap();
        let out = capture(&compressed, Some("text/plain"), Some("gzip"), false);
        assert!(out.decoded);
        assert_eq!(out.text.as_deref(), Some("hello gzip"));
    }
}
