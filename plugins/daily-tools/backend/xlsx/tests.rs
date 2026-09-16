//! xlsx 读写回归测试：用临时文件写出后读回。

use std::path::PathBuf;

use calamine::{Cell, Data, Range};
use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};

use super::dto::{CellDto, SheetData};
use super::read::data_size;
use super::{list_sheets, read_sheet, write_workbook};

fn temp_path(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("arkdesk-xlsx-{tag}-{}-{nanos}.xlsx", std::process::id()))
}

fn text(value: &str) -> CellDto {
    CellDto::text(value)
}

fn sample_sheet() -> SheetData {
    SheetData {
        name: "数据".to_string(),
        columns: vec!["名称".to_string(), "数量".to_string(), "启用".to_string(), "日期".to_string()],
        rows: vec![
            vec![
                text("甲"),
                CellDto::number(12.5),
                CellDto::boolean(true),
                CellDto {
                    kind: "d".to_string(),
                    value: serde_json::json!("2024-03-05T08:30:00"),
                    serial: None,
                    format: None,
                },
            ],
            vec![text("乙"), CellDto::number(1.0), CellDto::boolean(false), CellDto::empty()],
        ],
    }
}

#[test]
fn write_then_read_roundtrip() {
    let path = temp_path("roundtrip");
    let sheet = sample_sheet();
    write_workbook(path.to_str().unwrap(), std::slice::from_ref(&sheet), true).unwrap();

    let read = read_sheet(path.to_str().unwrap(), None, true, false, true).unwrap();
    assert_eq!(read.name, "数据");
    assert_eq!(read.columns, vec!["名称", "数量", "启用", "日期"]);
    assert_eq!(read.rows.len(), 2);

    let first = &read.rows[0];
    assert_eq!(first[0].kind, "s");
    assert_eq!(first[0].value, serde_json::json!("甲"));
    assert_eq!(first[1].kind, "n");
    assert_eq!(first[1].value.as_f64(), Some(12.5));
    assert_eq!(first[2].kind, "b");
    assert_eq!(first[2].value.as_bool(), Some(true));
    // 日期：类型化为 d，ISO 与原始序列号同时保留
    assert_eq!(first[3].kind, "d");
    assert!(first[3].serial.is_some());
    assert!(first[3]
        .value
        .as_str()
        .unwrap()
        .starts_with("2024-03-05T08:30:00"));

    assert_eq!(read.rows[1][3].value, serde_json::Value::Null);

    let _ = std::fs::remove_file(path);
}

#[test]
fn read_without_header_generates_column_names() {
    let path = temp_path("no-header");
    write_workbook(path.to_str().unwrap(), &[sample_sheet()], false).unwrap();

    let read = read_sheet(path.to_str().unwrap(), Some("数据"), false, false, true).unwrap();
    assert_eq!(read.columns, vec!["column1", "column2", "column3", "column4"]);
    // 无表头时首行也是数据
    assert_eq!(read.rows.len(), 3);

    let sheets = list_sheets(path.to_str().unwrap()).unwrap();
    assert_eq!(sheets, vec!["数据"]);

    let missing = read_sheet(path.to_str().unwrap(), Some("不存在"), true, false, true);
    assert!(missing.is_err());

    let _ = std::fs::remove_file(path);
}

#[test]
fn trim_empty_trims_trailing_blank_rows_and_columns() {
    // 用稀疏 Range 直接验证裁剪逻辑（calamine 不会把“带格式的空单元格”算进区域）
    let range = Range::from_sparse(vec![
        Cell::new((0, 0), Data::String("A".to_string())),
        Cell::new((1, 0), Data::String("1".to_string())),
        Cell::new((1, 5), Data::Empty),
        Cell::new((9, 0), Data::String(String::new())),
    ]);

    assert_eq!(data_size(&range, false), (10, 6));
    assert_eq!(data_size(&range, true), (2, 1));
}

#[test]
fn read_from_file_respects_trim_flag() {
    let path = temp_path("trim-file");
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.write_string(0, 0, "A").unwrap();
    worksheet.write_string(1, 0, "1").unwrap();
    workbook.save(path.to_str().unwrap()).unwrap();

    let kept = read_sheet(path.to_str().unwrap(), None, true, false, false).unwrap();
    assert_eq!(kept.columns, vec!["A"]);
    assert_eq!(kept.rows.len(), 1);

    let trimmed = read_sheet(path.to_str().unwrap(), None, true, false, true).unwrap();
    assert_eq!(trimmed.columns, vec!["A"]);
    assert_eq!(trimmed.rows.len(), 1);

    let _ = std::fs::remove_file(path);
}

#[test]
fn excel_date_roundtrips_through_iso() {
    let path = temp_path("date");
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let datetime = ExcelDateTime::from_ymd(2023, 12, 31)
        .unwrap()
        .and_hms(23, 59, 59)
        .unwrap();
    let format = Format::new().set_num_format("yyyy-mm-dd hh:mm:ss");
    worksheet
        .write_datetime_with_format(0, 0, &datetime, &format)
        .unwrap();
    workbook.save(path.to_str().unwrap()).unwrap();

    let read = read_sheet(path.to_str().unwrap(), None, false, false, true).unwrap();
    let cell = &read.rows[0][0];
    assert_eq!(cell.kind, "d");
    assert_eq!(cell.value, serde_json::json!("2023-12-31T23:59:59"));

    // 写回：ISO → Excel 日期，再读仍为同一时刻
    let rewritten = SheetData {
        name: "Sheet1".to_string(),
        columns: vec!["时间".to_string()],
        rows: vec![vec![cell.clone()]],
    };
    let second = temp_path("date-rewrite");
    write_workbook(second.to_str().unwrap(), &[rewritten], true).unwrap();
    let read_back = read_sheet(second.to_str().unwrap(), None, true, false, true).unwrap();
    assert_eq!(read_back.rows[0][0].value, serde_json::json!("2023-12-31T23:59:59"));

    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(second);
}

#[test]
fn rejects_invalid_inputs() {
    assert!(write_workbook("", &[sample_sheet()], true).is_err());
    assert!(write_workbook("/tmp/arkdesk-not-xlsx.txt", &[sample_sheet()], true).is_err());
    assert!(write_workbook("/tmp/arkdesk-empty.xlsx", &[], true).is_err());

    let too_long = SheetData {
        name: "x".repeat(32),
        columns: vec!["A".to_string()],
        rows: vec![],
    };
    assert!(write_workbook("/tmp/arkdesk-long-name.xlsx", &[too_long], true).is_err());

    let duplicated = vec![sample_sheet(), sample_sheet()];
    assert!(write_workbook("/tmp/arkdesk-duplicate.xlsx", &duplicated, true).is_err());

    let illegal = SheetData {
        name: "含[非法]字符".to_string(),
        columns: vec!["A".to_string()],
        rows: vec![],
    };
    assert!(write_workbook("/tmp/arkdesk-illegal.xlsx", &[illegal], true).is_err());

    assert!(read_sheet("/tmp/arkdesk-missing.xlsx", None, true, false, true).is_err());
    assert!(read_sheet("/tmp/arkdesk-report.txt", None, true, false, true).is_err());
}

#[test]
fn string_cells_are_never_written_as_formulas() {
    let path = temp_path("injection");
    let sheet = SheetData {
        name: "Sheet1".to_string(),
        columns: vec!["值".to_string()],
        rows: vec![vec![text("=1+1")], vec![text("@SUM(A1)")]],
    };
    write_workbook(path.to_str().unwrap(), &[sheet], true).unwrap();

    let read = read_sheet(path.to_str().unwrap(), None, true, false, true).unwrap();
    assert_eq!(read.rows[0][0].kind, "s");
    assert_eq!(read.rows[0][0].value, serde_json::json!("=1+1"));
    assert_eq!(read.rows[1][0].value, serde_json::json!("@SUM(A1)"));

    let _ = std::fs::remove_file(path);
}
