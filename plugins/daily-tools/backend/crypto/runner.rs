//! 命令层公共支撑：文件任务的进度/取消、对称参数组装、操作解析。

use crate::error::{code, AppError};

use super::progress::{CallbackObserver, NoopObserver, StreamObserver};
use super::symmetric::SymParams;

/// 取文件大小（用于进度百分比），不可用时返回 `None`。
pub fn file_size(path: &str) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

/// 运行带进度与取消的文件任务（框架 `crate::tasks`）。
///
/// - `task_id` 为空时不注册任务，仅执行；
/// - `total` 为输入文件字节数；进度经 `task://progress` 回传；
/// - `job` 在阻塞线程执行，取消时观察者返回错误，调用方负责清理部分输出。
pub async fn run_file_task<F, T>(
    app: tauri::AppHandle,
    task_id: Option<String>,
    total: Option<u64>,
    job: F,
) -> Result<T, AppError>
where
    F: FnOnce(&mut dyn StreamObserver) -> Result<T, AppError> + Send + 'static,
    T: Send + 'static,
{
    let task_id = task_id.filter(|id| !id.trim().is_empty());
    let cancel = task_id.as_deref().map(crate::tasks::begin);
    let app_for_job = app.clone();
    let cancel_for_job = cancel.clone();
    let id_for_job = task_id.clone();

    let joined = tokio::task::spawn_blocking(move || -> Result<T, AppError> {
        let mut emit = |done: u64, total: u64| {
            if let Some(id) = id_for_job.as_deref() {
                crate::tasks::emit_progress(&app_for_job, id, done, Some(total), None);
            }
        };
        match (cancel_for_job.as_deref(), total) {
            (Some(flag), Some(total)) => {
                let mut observer = CallbackObserver::new(flag, total, &mut emit);
                job(&mut observer)
            }
            _ => job(&mut NoopObserver),
        }
    })
    .await;

    if let Some(id) = task_id.as_deref() {
        let cancelled = cancel
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed));
        match &joined {
            Ok(_) if cancelled => crate::tasks::emit_done(&app, id),
            Ok(Ok(_)) => crate::tasks::emit_done(&app, id),
            Ok(Err(err)) => crate::tasks::emit_error(&app, id, &err.to_string()),
            Err(_) => crate::tasks::emit_error(&app, id, "任务执行失败"),
        }
        crate::tasks::end(id);
    }

    joined.map_err(|e| AppError::custom(code::PLUGIN_ERROR, format!("任务执行失败: {e}")))?
}

/// 解析操作字符串（encrypt / decrypt，兼容中文）。
pub fn parse_operation(operation: &str) -> Result<bool, AppError> {
    match super::normalize(operation).as_str() {
        "encrypt" | "enc" | "加密" => Ok(true),
        "decrypt" | "dec" | "解密" => Ok(false),
        other => Err(AppError::invalid_input(format!("未知操作: {other}"))),
    }
}

/// `build_sym_params` 的输入（前端可空字段）。
pub struct SymParamInput {
    pub algorithm: String,
    pub mode: String,
    pub key: Option<String>,
    pub key_encoding: Option<String>,
    pub iv: Option<String>,
    pub iv_encoding: Option<String>,
    pub passphrase: Option<String>,
    pub kdf: Option<String>,
    pub salt: Option<String>,
    pub salt_encoding: Option<String>,
    pub rounds: Option<u32>,
    pub aad: Option<String>,
    pub aad_encoding: Option<String>,
}

/// 将请求字段解析为对称加解密参数。
pub fn build_sym_params(input: SymParamInput) -> Result<SymParams, AppError> {
    let SymParamInput {
        algorithm,
        mode,
        key,
        key_encoding,
        iv,
        iv_encoding,
        passphrase,
        kdf,
        salt,
        salt_encoding,
        rounds,
        aad,
        aad_encoding,
    } = input;
    let key_bytes = super::parse_bytes(
        key.as_deref().unwrap_or(""),
        key_encoding.as_deref().unwrap_or("hex"),
    )?;
    let iv_bytes = super::parse_bytes(
        iv.as_deref().unwrap_or(""),
        iv_encoding.as_deref().unwrap_or("hex"),
    )?;
    let salt_bytes = match salt {
        Some(salt) => Some(super::parse_bytes(
            &salt,
            salt_encoding.as_deref().unwrap_or("hex"),
        )?),
        None => None,
    };
    let aad_bytes = super::parse_bytes(
        aad.as_deref().unwrap_or(""),
        aad_encoding.as_deref().unwrap_or("utf8"),
    )?;
    Ok(SymParams {
        algorithm,
        mode,
        key: key_bytes,
        iv: iv_bytes,
        passphrase: passphrase.map(|value| value.into_bytes()),
        kdf: kdf.unwrap_or_else(|| "evp".to_string()),
        salt: salt_bytes,
        rounds: rounds.unwrap_or(0),
        aad: aad_bytes,
    })
}
