//! 统一错误定义。
//!
//! 所有 `#[tauri::command]` 返回 `Result<T, AppError>`。
//! 序列化形态与前端 `core/errors` 的 `AppError` 对齐：`{ code, message, details? }`。
//!
//! 新增错误码时，同步在 `code` 模块与前端 `core/errors` 的 `ErrorCode` 登记。
//! 命令一律返回 `Result<T, AppError>`，前端经 `core/errors` 统一转换与提示。

use serde::{ser::SerializeStruct, Serialize, Serializer};
use thiserror::Error;

/// 错误码常量，与前端 ErrorCode 保持一致
#[allow(dead_code)]
pub mod code {
    pub const UNKNOWN: &str = "UNKNOWN";
    pub const INVALID_INPUT: &str = "INVALID_INPUT";
    pub const IO_ERROR: &str = "IO_ERROR";
    pub const DB_ERROR: &str = "DB_ERROR";
    pub const IPC_ERROR: &str = "IPC_ERROR";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const PLUGIN_ERROR: &str = "PLUGIN_ERROR";
    pub const HTTP_ERROR: &str = "HTTP_ERROR";
    pub const UPDATE_ERROR: &str = "UPDATE_ERROR";
}

#[derive(Debug, Error)]
pub enum AppError {
    /// 业务自定义错误（工具插件常用）
    #[error("{message}")]
    Custom { code: String, message: String },

    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl AppError {
    pub fn custom(code: &str, message: impl Into<String>) -> Self {
        AppError::Custom {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::custom(code::INVALID_INPUT, message)
    }

    #[allow(dead_code)]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::custom(code::NOT_FOUND, message)
    }

    /// (code, message) 二元组
    fn parts(&self) -> (&str, String) {
        match self {
            AppError::Custom { code, message } => (code, message.clone()),
            AppError::Tauri(err) => (code::IPC_ERROR, err.to_string()),
            AppError::Io(err) => (code::IO_ERROR, err.to_string()),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (code, message) = self.parts();
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", code)?;
        state.serialize_field("message", &message)?;
        state.end()
    }
}
