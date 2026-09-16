//! Excel（.xlsx）读写：calamine 解析、rust_xlsxwriter 写出。

pub mod dto;
mod read;
mod write;

pub use dto::SheetData;

/// 列出工作表名
pub fn list_sheets(path: &str) -> Result<Vec<String>, crate::error::AppError> {
    read::list_sheets(path)
}

/// 读取工作表为表格
pub fn read_sheet(
    path: &str,
    sheet: Option<&str>,
    has_header: bool,
    fill_merged: bool,
    trim_empty: bool,
) -> Result<SheetData, crate::error::AppError> {
    read::read_sheet(path, sheet, has_header, fill_merged, trim_empty)
}

/// 写出工作簿
pub fn write_workbook(
    path: &str,
    sheets: &[SheetData],
    style: bool,
) -> Result<(), crate::error::AppError> {
    write::write_workbook(path, sheets, style)
}

#[cfg(test)]
mod tests;
