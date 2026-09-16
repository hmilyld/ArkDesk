//! Excel（.xlsx）读写 DTO。
//!
//! 前端 `frontend/table/model.ts` 的 `Cell` / `Table` 与此处一一对应（camelCase）。

use serde::{Deserialize, Serialize};

/// 单元格值（`null` 表示空单元格）
pub type CellValue = serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellDto {
    /// 类型：s 字符串 / n 数字 / b 布尔 / d 日期 / e Excel 错误值
    #[serde(rename = "t")]
    pub kind: String,
    /// 值
    #[serde(rename = "v", default)]
    pub value: CellValue,
    /// 日期原始序列号（Excel 1900 日期系统）
    #[serde(rename = "n", skip_serializing_if = "Option::is_none", default)]
    pub serial: Option<f64>,
    /// 自定义数字格式（写出时使用）
    #[serde(rename = "f", skip_serializing_if = "Option::is_none", default)]
    pub format: Option<String>,
}

impl CellDto {
    pub fn empty() -> Self {
        Self {
            kind: "s".to_string(),
            value: CellValue::Null,
            serial: None,
            format: None,
        }
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self {
            kind: "s".to_string(),
            value: CellValue::String(value.into()),
            serial: None,
            format: None,
        }
    }

    pub fn number(value: f64) -> Self {
        Self {
            kind: "n".to_string(),
            value: serde_json::json!(value),
            serial: None,
            format: None,
        }
    }

    pub fn boolean(value: bool) -> Self {
        Self {
            kind: "b".to_string(),
            value: CellValue::Bool(value),
            serial: None,
            format: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            kind: "e".to_string(),
            value: CellValue::String(message.into()),
            serial: None,
            format: None,
        }
    }

    pub fn datetime(iso: impl Into<String>, serial: f64) -> Self {
        Self {
            kind: "d".to_string(),
            value: CellValue::String(iso.into()),
            serial: Some(serial),
            format: None,
        }
    }
}

/// 工作表（读出的表格 / 待写出的表格）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetData {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<CellDto>>,
}
