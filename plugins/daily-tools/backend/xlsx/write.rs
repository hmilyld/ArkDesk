//! Excel 写出：rust_xlsxwriter（表头加粗 + 冻结首行 + 自动列宽）。

use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};

use crate::error::{code, AppError};

use super::dto::{CellDto, SheetData};

/// 默认日期时间格式（与前端「自定义格式」一致）
const DEFAULT_DATETIME_FORMAT: &str = "yyyy-mm-dd hh:mm:ss";
const DEFAULT_DATE_FORMAT: &str = "yyyy-mm-dd";

/// 写出工作簿：每个 `SheetData` 一个工作表
pub fn write_workbook(path: &str, sheets: &[SheetData], style: bool) -> Result<(), AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("保存路径不能为空"));
    }
    if !path.to_ascii_lowercase().ends_with(".xlsx") {
        return Err(AppError::invalid_input("仅支持 .xlsx 文件"));
    }
    if sheets.is_empty() {
        return Err(AppError::invalid_input("没有可写出的数据"));
    }

    let mut used_names: Vec<String> = Vec::new();
    for (index, sheet) in sheets.iter().enumerate() {
        let name = sheet_name(sheet, index)?;
        if used_names.contains(&name) {
            return Err(AppError::invalid_input(format!("工作表名重复：「{name}」")));
        }
        used_names.push(name);
    }

    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold();
    let date_format = Format::new().set_num_format(DEFAULT_DATE_FORMAT);
    let datetime_format = Format::new().set_num_format(DEFAULT_DATETIME_FORMAT);

    for (index, sheet) in sheets.iter().enumerate() {
        let name = used_names[index].clone();
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(&name)
            .map_err(|err| AppError::invalid_input(format!("工作表名无效：{err}")))?;

        for (col, column) in sheet.columns.iter().enumerate() {
            let col = col as u16;
            if style {
                worksheet
                    .write_string_with_format(0, col, column, &header_format)
                    .map_err(write_error)?;
            } else {
                worksheet.write_string(0, col, column).map_err(write_error)?;
            }
        }

        for (row, cells) in sheet.rows.iter().enumerate() {
            let row = row as u32 + 1;
            for (col, cell) in cells.iter().enumerate() {
                write_cell(
                    worksheet,
                    row,
                    col as u16,
                    cell,
                    &date_format,
                    &datetime_format,
                )?;
            }
        }

        if style {
            let _ = worksheet.set_freeze_panes(1, 0);
            let _ = worksheet.autofit();
        }
    }

    workbook
        .save(path)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("保存工作簿失败: {err}")))?;
    log::info!("表格已导出: {path}");
    Ok(())
}

fn sheet_name(sheet: &SheetData, index: usize) -> Result<String, AppError> {
    let name = sheet.name.trim();
    let name = if name.is_empty() {
        format!("Sheet{}", index + 1)
    } else {
        name.to_string()
    };
    if name.chars().count() > 31 {
        return Err(AppError::invalid_input(format!(
            "工作表名「{name}」超过 31 个字符"
        )));
    }
    if name.contains(['[', ']', ':', '*', '?', '/', '\\']) {
        return Err(AppError::invalid_input(format!(
            "工作表名「{name}」含有非法字符（[]:*?/\\）"
        )));
    }
    Ok(name)
}

fn write_cell(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    cell: &CellDto,
    date_format: &Format,
    datetime_format: &Format,
) -> Result<(), AppError> {
    match cell.kind.as_str() {
        "n" => {
            if let Some(value) = cell.value.as_f64() {
                worksheet.write_number(row, col, value).map_err(write_error)?;
            } else {
                write_text(worksheet, row, col, cell)?;
            }
        }
        "b" => {
            if let Some(value) = cell.value.as_bool() {
                worksheet.write_boolean(row, col, value).map_err(write_error)?;
            } else {
                write_text(worksheet, row, col, cell)?;
            }
        }
        "d" => match as_excel_datetime(cell) {
            Some(datetime) => {
                let custom = cell
                    .format
                    .as_ref()
                    .filter(|pattern| !pattern.trim().is_empty())
                    .map(|pattern| Format::new().set_num_format(pattern));
                let selected = match &custom {
                    Some(format) => format,
                    None if datetime.is_date_only => date_format,
                    None => datetime_format,
                };
                worksheet
                    .write_datetime_with_format(row, col, &datetime.value, selected)
                    .map_err(write_error)?;
            }
            None => write_text(worksheet, row, col, cell)?,
        },
        // 字符串与 Excel 错误值一律按文本写出：绝不使用 write_formula，天然无公式注入
        _ => write_text(worksheet, row, col, cell)?,
    }
    Ok(())
}

fn write_text(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    cell: &CellDto,
) -> Result<(), AppError> {
    let text = plain_text(cell);
    if text.is_empty() {
        return Ok(());
    }
    worksheet.write_string(row, col, &text).map_err(write_error)?;
    Ok(())
}

struct Datetime {
    value: ExcelDateTime,
    /// 分秒为 0，可用纯日期格式
    is_date_only: bool,
}

fn as_excel_datetime(cell: &CellDto) -> Option<Datetime> {
    if let Some(text) = cell.value.as_str() {
        if let Some(datetime) = from_iso(text) {
            return Some(datetime);
        }
    }
    let serial = cell.serial?;
    let value = ExcelDateTime::from_serial_datetime(serial).ok()?;
    Some(Datetime {
        value,
        is_date_only: serial.fract() == 0.0,
    })
}

/// 解析 ISO 8601（`YYYY-MM-DD[THH:MM[:SS[.mmm]]]`）为 Excel 日期
fn from_iso(text: &str) -> Option<Datetime> {
    let text = text.trim().trim_end_matches('Z');
    if let Ok(datetime) = NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S%.f"))
    {
        return build_datetime(
            datetime.year(),
            datetime.month(),
            datetime.day(),
            datetime.hour(),
            datetime.minute(),
            datetime.second(),
            datetime.and_utc().timestamp_subsec_millis(),
            false,
        );
    }

    let date = NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()?;
    build_datetime(date.year(), date.month(), date.day(), 0, 0, 0, 0, true)
}

#[allow(clippy::too_many_arguments)]
fn build_datetime(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    milli: u32,
    date_only: bool,
) -> Option<Datetime> {
    let value = ExcelDateTime::from_ymd(year as u16, month as u8, day as u8)
        .ok()?
        .and_hms_milli(hour as u16, minute as u8, second as u8, milli as u16)
        .ok()?;
    Some(Datetime {
        value,
        is_date_only: date_only,
    })
}

fn plain_text(cell: &CellDto) -> String {
    match &cell.value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        other => other.to_string(),
    }
}

fn write_error(err: rust_xlsxwriter::XlsxError) -> AppError {
    AppError::custom(code::IO_ERROR, format!("写入单元格失败: {err}"))
}
