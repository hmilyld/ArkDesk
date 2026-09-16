//! 流式操作的进度上报与取消。
//!
//! 文件哈希 / 编码 / 加解密在分块循环中调用 [`StreamObserver::advance`]：
//! - 上报已处理字节数（`CallbackObserver` 会做去抖，避免事件过密）；
//! - 若取消标记已置位则返回 [`cancelled`] 错误，调用方据此中止并清理输出。

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::error::AppError;

/// 取消错误统一消息（前端据此提示“已取消”而非报错）。
pub const CANCELLED_MESSAGE: &str = "操作已取消";

/// 构造取消错误。
pub fn cancelled() -> AppError {
    AppError::custom("CANCELLED", CANCELLED_MESSAGE)
}

/// 流式操作观察者。
pub trait StreamObserver {
    /// 已处理 `bytes` 字节；返回 `Err` 表示已取消。
    fn advance(&mut self, bytes: u64) -> Result<(), AppError>;
}

/// 无进度、不可取消（测试或无需进度时使用）。
pub struct NoopObserver;

impl StreamObserver for NoopObserver {
    fn advance(&mut self, _bytes: u64) -> Result<(), AppError> {
        Ok(())
    }
}

/// 基于取消标记 + 进度回调的实现。
///
/// 进度按 80ms 去抖；当累计达到 `total` 时必定上报一次。
pub struct CallbackObserver<'a> {
    cancel: &'a AtomicBool,
    total: u64,
    done: u64,
    last: Instant,
    emit: &'a mut dyn FnMut(u64, u64),
}

impl<'a> CallbackObserver<'a> {
    pub fn new(cancel: &'a AtomicBool, total: u64, emit: &'a mut dyn FnMut(u64, u64)) -> Self {
        Self {
            cancel,
            total,
            done: 0,
            last: Instant::now(),
            emit,
        }
    }
}

impl StreamObserver for CallbackObserver<'_> {
    fn advance(&mut self, bytes: u64) -> Result<(), AppError> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(cancelled());
        }
        self.done += bytes;
        let now = Instant::now();
        let reached_total = self.total > 0 && self.done >= self.total;
        if reached_total || now.duration_since(self.last) >= Duration::from_millis(80) {
            (self.emit)(self.done, self.total);
            self.last = now;
        }
        Ok(())
    }
}
