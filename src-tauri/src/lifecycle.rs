//! 应用生命周期钩子（框架级 SPI 契约，base 不含任何业务实现）。
//!
//! base 只定义「在哪些节点回调、契约是什么、失败怎么办」；fork 若需要
//! （如启动前校验注册、退出前保存数据），在 `src-tauri/local/lifecycle.rs` 实现
//! [`AppLifecycle`] 即可被构建期自动发现并调用，**无需任何登记**；不提供实现时
//! 全部为 no-op，行为与未引入本模块时完全一致。
//!
//! 四个节点：
//! - [`AppLifecycle::before_start`]：主窗口显示**之前**（异步；返回 Hold 则不显示）
//! - [`AppLifecycle::after_start`]：主窗口显示之后
//! - [`AppLifecycle::before_exit`]：退出流程开始、进程结束之前（可取消退出）
//! - [`AppLifecycle::after_exit`]：进程即将结束（同步且限时，异步任务会被直接杀掉）
//!
//! 契约约束：
//! - 钩子运行在独立异步任务中，**不得阻塞主线程**（内部 `block_on` 会让启动假死）；
//! - 报错 / panic / 超时按各自声明的 [`FailPolicy`] 处理，不会把应用带崩；
//! - `before_start` 返回 [`StartupFlow::Hold`] 后进程继续运行但不显示主窗口，
//!   fork 须自行处理后调用 [`resume_startup`] 继续启动（否则只能从托盘退出）。

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::error::AppError;

/// `before_start` 的最长等待时间，超时按失败策略处理
pub const BEFORE_START_TIMEOUT: Duration = Duration::from_secs(10);
/// `before_exit` 的最长等待时间，超时按失败策略处理
pub const BEFORE_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
/// `after_exit` 的最长等待时间（同步钩子，超时后不再等待，进程即退）
pub const AFTER_EXIT_TIMEOUT: Duration = Duration::from_millis(200);

/// 启动阶段的允许动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupFlow {
    /// 继续启动：显示主窗口并回调 `after_start`
    Continue,
    /// 挂起：不显示主窗口（fork 自行处理，之后调 [`resume_startup`] 继续）
    Hold,
    /// 退出进程（由 fork 的实现返回）
    #[allow(dead_code)]
    Quit,
}

/// 退出阶段的允许动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitFlow {
    /// 放行退出
    Continue,
    /// 取消本次退出
    Cancel,
}

/// 钩子失败（返回 Err / panic / 超时）时的兜底策略，由实现自行声明
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailPolicy {
    /// 记日志后按默认值放行（启动则不拦、退出则放行）
    Skip,
    /// 启动视为 [`StartupFlow::Hold`]；退出视为 [`ExitFlow::Cancel`]
    Halt,
}

/// 钩子返回的 future（对象安全的异步方法，避免引入 `async_trait` 依赖）
pub type HookFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// 生命周期钩子契约：全部方法都有默认实现（默认即是 no-op）。
///
/// fork 实现位于 `src-tauri/local/lifecycle.rs`，须导出 `pub struct Hooks;`：
///
/// ```ignore
/// pub struct Hooks;
///
/// impl crate::lifecycle::AppLifecycle for Hooks {
///     fn before_start(&self, app: &tauri::AppHandle) -> crate::lifecycle::HookFuture<'_, Result<crate::lifecycle::StartupFlow, crate::error::AppError>> {
///         Box::pin(async move {
///             if crate::licensing::verify(app).await? {
///                 Ok(crate::lifecycle::StartupFlow::Continue)
///             } else {
///                 Ok(crate::lifecycle::StartupFlow::Hold) // 自建注册窗口，通过后调 resume_startup
///             }
///         })
///     }
/// }
/// ```
pub trait AppLifecycle: Send + Sync + 'static {
    /// 主窗口显示之前调用（异步，可拦截）
    fn before_start(&self, _app: &AppHandle) -> HookFuture<'_, Result<StartupFlow, AppError>> {
        Box::pin(async { Ok(StartupFlow::Continue) })
    }

    /// 主窗口显示之后调用（仅通知，失败不影响运行）
    fn after_start(&self, _app: &AppHandle) {}

    /// 退出流程开始前调用（异步，可取消退出）
    fn before_exit(&self, _app: &AppHandle) -> HookFuture<'_, Result<ExitFlow, AppError>> {
        Box::pin(async { Ok(ExitFlow::Continue) })
    }

    /// 进程即将结束时调用：**必须同步且快速**（异步任务会被直接杀掉）
    fn after_exit(&self, _app: &AppHandle) {}

    /// `before_start` 失败时的兜底（默认 Halt = 不显示主窗口）
    fn on_before_start_fail(&self) -> FailPolicy {
        FailPolicy::Halt
    }

    /// `before_exit` 失败时的兜底（默认 Skip = 绝不把用户卡在退不出去）
    fn on_before_exit_fail(&self) -> FailPolicy {
        FailPolicy::Skip
    }
}

/// 启动门禁状态：置位后不显示主窗口，直到 [`resume_startup`] 被调用
static STARTUP_HELD: AtomicBool = AtomicBool::new(false);

/// 启动是否处于挂起状态（`Hold` 或启动钩子失败按 Halt 处理）
pub fn is_startup_held() -> bool {
    STARTUP_HELD.load(Ordering::Relaxed)
}

/// 设置挂起状态（内部状态机）
fn set_startup_held(held: bool) {
    STARTUP_HELD.store(held, Ordering::Relaxed);
}

/// 钩子失败原因（仅用于内部日志与失败策略分派）
enum HookError {
    Failed,
    TimedOut,
    Panicked,
}

// 取 fork 提供的实现：构建期由 build.rs 生成（无实现时为 None）
include!(concat!(env!("OUT_DIR"), "/local_lifecycle.rs"));

/// 启动钩子失败时的兜底动作
fn fallback_start(policy: FailPolicy) -> StartupFlow {
    match policy {
        FailPolicy::Skip => StartupFlow::Continue,
        FailPolicy::Halt => StartupFlow::Hold,
    }
}

/// 退出钩子失败时的兜底动作
fn fallback_exit(policy: FailPolicy) -> ExitFlow {
    match policy {
        FailPolicy::Skip => ExitFlow::Continue,
        FailPolicy::Halt => ExitFlow::Cancel,
    }
}

/// 挂起启动：不显示主窗口，等待 fork 调用 [`resume_startup`]
pub fn hold_startup() {
    set_startup_held(true);
    log::info!("启动已挂起（before_start 返回 Hold），等待 fork 调用 resume_startup");
}

/// 完成启动：显示主窗口并回调 `after_start`（`Continue` 与 [`resume_startup`] 共用）
pub fn complete_startup(app: &AppHandle) {
    set_startup_held(false);
    reveal_main_window(app);
    run_after_start(app);
}

/// 继续被挂起的启动（由 fork 在自身流程完成后调用）
#[allow(dead_code)]
pub fn resume_startup(app: &AppHandle) {
    if !is_startup_held() {
        log::warn!("当前未处于启动挂起状态，resume_startup 已忽略");
        return;
    }
    log::info!("启动挂起解除，继续启动流程");
    complete_startup(app);
}

/// 显示并聚焦主窗口（挂起状态下拒绝显示，防止托盘/单实例/Dock 旁路门禁）
pub fn reveal_main_window(app: &AppHandle) {
    if is_startup_held() {
        log::debug!("启动挂起中，忽略显示主窗口请求");
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// 运行 `before_start` 并归一化结果（未通过 / 报错 / panic / 超时）
pub async fn run_before_start(app: &AppHandle) -> StartupFlow {
    let Some(hook) = local_impl() else {
        return StartupFlow::Continue;
    };
    let handle = app.clone();
    match invoke("before_start", BEFORE_START_TIMEOUT, move || async move {
        hook.before_start(&handle).await
    })
    .await
    {
        Ok(flow) => flow,
        Err(_) => fallback_start(hook.on_before_start_fail()),
    }
}

/// 运行 `before_exit` 并归一化结果
pub async fn run_before_exit(app: &AppHandle) -> ExitFlow {
    let Some(hook) = local_impl() else {
        return ExitFlow::Continue;
    };
    let handle = app.clone();
    match invoke("before_exit", BEFORE_EXIT_TIMEOUT, move || async move {
        hook.before_exit(&handle).await
    })
    .await
    {
        Ok(flow) => flow,
        Err(_) => fallback_exit(hook.on_before_exit_fail()),
    }
}

/// 运行 `after_start`（同步钩子，panic 不能带崩启动流程）
pub fn run_after_start(app: &AppHandle) {
    let Some(hook) = local_impl() else {
        return;
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hook.after_start(app)));
    if result.is_err() {
        log::error!("after_start 钩子 panic");
    }
}

/// 运行 `after_exit`：独立线程执行 + 限时等待（进程即将结束，不能无限等）
pub fn run_after_exit(app: &AppHandle) {
    let Some(hook) = local_impl() else {
        return;
    };
    let handle = app.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("after-exit-hook".into())
        .spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hook.after_exit(&handle)));
            if result.is_err() {
                log::error!("after_exit 钩子 panic");
            }
            let _ = sender.send(());
        });
    if let Err(err) = spawned {
        log::error!("after_exit 钩子线程创建失败: {err}");
        return;
    }
    if receiver.recv_timeout(AFTER_EXIT_TIMEOUT).is_err() {
        log::warn!("after_exit 钩子超时（{AFTER_EXIT_TIMEOUT:?}），不再等待");
    }
}

/// 在独立任务中带超时执行异步钩子：Err / 超时 / panic 统一归一化并记日志
async fn invoke<T, F, Fut>(label: &'static str, timeout: Duration, call: F) -> Result<T, HookError>
where
    T: Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, AppError>> + Send + 'static,
{
    let task = tauri::async_runtime::spawn(async move {
        match tokio::time::timeout(timeout, call()).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(err)) => {
                log::error!("{label} 钩子返回错误: {err}");
                Err(HookError::Failed)
            }
            Err(_) => {
                log::error!("{label} 钩子超时（{timeout:?}）");
                Err(HookError::TimedOut)
            }
        }
    });

    match task.await {
        Ok(result) => result,
        Err(_) => {
            log::error!("{label} 钩子 panic");
            Err(HookError::Panicked)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Defaults;

    impl AppLifecycle for Defaults {}

    #[test]
    fn default_fail_policy_is_strict_start_lenient_exit() {
        let hook = Defaults;
        assert_eq!(hook.on_before_start_fail(), FailPolicy::Halt);
        assert_eq!(hook.on_before_exit_fail(), FailPolicy::Skip);
    }

    #[test]
    fn fail_policy_maps_to_flow_action() {
        assert_eq!(fallback_start(FailPolicy::Skip), StartupFlow::Continue);
        assert_eq!(fallback_start(FailPolicy::Halt), StartupFlow::Hold);
        assert_eq!(fallback_exit(FailPolicy::Skip), ExitFlow::Continue);
        assert_eq!(fallback_exit(FailPolicy::Halt), ExitFlow::Cancel);
    }

    #[test]
    fn startup_held_flag_toggles() {
        assert!(!is_startup_held());
        set_startup_held(true);
        assert!(is_startup_held());
        set_startup_held(false);
        assert!(!is_startup_held());
    }
}
