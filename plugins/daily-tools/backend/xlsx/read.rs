//! Excel 读取：calamine → 类型化单元格（含合并填充、尾部空行列裁剪与上限保护）。

use calamine::{open_workbook, Data, DataType, Xlsx};
use calamine::Reader;

use crate::error::{code, AppError};

use super::dto::{CellDto, SheetData};

/// 单表行数上限（超出直接报错，避免界面卡死）
pub const MAX_ROWS: usize = 200_000;
/// 单表列数上限
pub const MAX_COLS: usize = 2_048;

/// 路径校验：非空、`.xlsx`、存在
pub fn validate_path(path: &str) -> Result<(), AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("文件路径不能为空"));
    }
    if !path.to_ascii_lowercase().ends_with(".xlsx") {
        return Err(AppError::invalid_input("仅支持 .xlsx 文件"));
    }
    if !std::path::Path::new(path).is_file() {
        return Err(AppError::not_found(format!("文件不存在: {path}")));
    }
    Ok(())
}

/// 列出工作簿中的工作表名
pub fn list_sheets(path: &str) -> Result<Vec<String>, AppError> {
    validate_path(path)?;
    let workbook: Xlsx<_> = open_workbook(path)
        .map_err(|err| AppError::custom(code::PLUGIN_ERROR, format!("打开工作簿失败: {err}")))?;
    Ok(workbook.sheet_names())
}

/// 读取单个工作表
pub fn read_sheet(
    path: &str,
    sheet: Option<&str>,
    has_header: bool,
    fill_merged: bool,
    trim_empty: bool,
) -> Result<SheetData, AppError> {
    validate_path(path)?;
    let mut workbook: Xlsx<_> = open_workbook(path)
        .map_err(|err| AppError::custom(code::PLUGIN_ERROR, format!("打开工作簿失败: {err}")))?;

    let names = workbook.sheet_names();
    let name = match sheet.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => value.to_string(),
        None => names
            .first()
            .cloned()
            .ok_or_else(|| AppError::invalid_input("工作簿中没有工作表"))?,
    };
    if !names.iter().any(|item| item == &name) {
        return Err(AppError::invalid_input(format!("找不到工作表「{name}」")));
    }

    let range = workbook
        .worksheet_range(&name)
        .map_err(|err| AppError::custom(code::PLUGIN_ERROR, format!("读取工作表失败: {err}")))?;

    let (height, width) = data_size(&range, trim_empty);

    if height > MAX_ROWS {
        return Err(AppError::invalid_input(format!(
            "工作表「{name}」共 {height} 行，超过单表上限 {MAX_ROWS} 行"
        )));
    }
    if width > MAX_COLS {
        return Err(AppError::invalid_input(format!(
            "工作表「{name}」共 {width} 列，超过单表上限 {MAX_COLS} 列"
        )));
    }

    let mut cells: Vec<Vec<CellDto>> = (0..height)
        .map(|row| (0..width).map(|col| cell_from_data(range.get((row, col)))).collect())
        .collect();

    if fill_merged {
        fill_merged_cells(&mut workbook, &name, &mut cells, height, width);
    }

    let (columns, rows) = if has_header && height > 0 {
        (
            (0..width).map(|col| cell_text(&cells[0][col])).collect(),
            cells.split_off(1),
        )
    } else {
        (
            (0..width).map(|col| format!("column{}", col + 1)).collect(),
            cells,
        )
    };

    Ok(SheetData { name, columns, rows })
}

/// 合并单元格填充：把区域左上角的值复制到区域内其余单元格
fn fill_merged_cells(
    workbook: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    name: &str,
    cells: &mut [Vec<CellDto>],
    height: usize,
    width: usize,
) {
    let Ok(regions) = workbook.merge_cells_by_sheet_name(name) else {
        return;
    };

    for dims in regions {
        let (start_row, start_col) = (dims.start.0 as usize, dims.start.1 as usize);
        let (end_row, end_col) = (dims.end.0 as usize, dims.end.1 as usize);
        if start_row >= height || start_col >= width {
            continue;
        }
        let source = cells[start_row][start_col].clone();
        let last_row = end_row.min(height.saturating_sub(1));
        let last_col = end_col.min(width.saturating_sub(1));
        for (row, line) in cells.iter_mut().enumerate().take(last_row + 1).skip(start_row) {
            for (col, target) in line.iter_mut().enumerate().take(last_col + 1).skip(start_col) {
                if row == start_row && col == start_col {
                    continue;
                }
                *target = source.clone();
            }
        }
    }
}

/// 有效区域尺寸；`trim_empty` 时裁掉尾部全空的行与列
pub(crate) fn data_size(range: &calamine::Range<Data>, trim_empty: bool) -> (usize, usize) {
    let (mut height, mut width) = range.get_size();
    if !trim_empty {
        return (height, width);
    }
    while height > 0 && (0..width).all(|col| is_empty(range, height - 1, col)) {
        height -= 1;
    }
    while width > 0 && (0..height).all(|row| is_empty(range, row, width - 1)) {
        width -= 1;
    }
    (height, width)
}

/// 空单元格或空字符串都视为“空”，用于尾部裁剪
fn is_empty(range: &calamine::Range<Data>, row: usize, col: usize) -> bool {
    match range.get((row, col)) {
        None => true,
        Some(Data::String(value)) => value.is_empty(),
        Some(value) => DataType::is_empty(value),
    }
}

fn cell_from_data(data: Option<&Data>) -> CellDto {
    match data {
        None | Some(Data::Empty) => CellDto::empty(),
        Some(Data::String(value)) => CellDto::text(value.clone()),
        Some(Data::Int(value)) => CellDto::number(*value as f64),
        Some(Data::Float(value)) => CellDto::number(*value),
        Some(Data::Bool(value)) => CellDto::boolean(*value),
        Some(Data::Error(err)) => CellDto::error(err.to_string()),
        Some(Data::DateTimeIso(value)) => CellDto::datetime(value.clone(), 0.0),
        Some(Data::DurationIso(value)) => CellDto::text(value.clone()),
        Some(Data::DateTime(value)) => {
            // 时长（[hh]:mm:ss 等）不当日期处理
            if value.is_duration() {
                return CellDto::number(value.as_f64());
            }
            let (year, month, day, hour, minute, second, milli) = value.to_ymd_hms_milli();
            CellDto::datetime(iso_of(year, month, day, hour, minute, second, milli), value.as_f64())
        }
    }
}

fn iso_of(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8, milli: u16) -> String {
    if hour == 0 && minute == 0 && second == 0 && milli == 0 {
        return format!("{year:04}-{month:02}-{day:02}");
    }
    if milli == 0 {
        return format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}");
    }
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}")
}

/// 单元格 → 文本（表头、非表头格式的降级都用它）
fn cell_text(cell: &CellDto) -> String {
    match &cell.value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        other => other.to_string(),
    }
}
