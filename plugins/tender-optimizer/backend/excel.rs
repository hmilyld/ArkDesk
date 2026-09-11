//! 投标报价工具 — Excel 读写层。
//!
//! - 读取：calamine 解析统一模板（公司数据 / 测算场景 / 评分参数 三个 Sheet），
//!   校验与告警语义对齐 Python 版（无效行跳过并记 warning，不中断导入）
//! - 写出：rust_xlsxwriter 复刻结果工作簿（测算结果 + 公司数据 两个 Sheet，带样式）

use std::io::BufReader;

use calamine::{open_workbook, Data, DataType, Range, Reader, Xlsx};
use rust_xlsxwriter::{Color, Format, FormatAlign, Workbook, XlsxError};

use crate::error::{code, AppError};
use crate::plugins::tender_optimizer::models::{
    Company, ExportPayload, Scenario, ScoringParams, BEHAVIOR_AGGRESSIVE, BEHAVIOR_AUTO,
    BEHAVIOR_CONSERVATIVE, BEHAVIOR_NORMAL, COMPANY_TYPE_AUX, COMPANY_TYPE_TARGET,
    COMPANY_TYPE_UNCONTROLLED, REDUCTION_AMOUNT, REDUCTION_PERCENT,
};

const SHEET_COMPANIES: &str = "公司数据";
const SHEET_SCENARIOS: &str = "测算场景";
const SHEET_PARAMS: &str = "评分参数";

const VALID_BEHAVIORS: [(&str, &str); 4] = [
    ("激进型", BEHAVIOR_AGGRESSIVE),
    ("正常型", BEHAVIOR_NORMAL),
    ("保守型", BEHAVIOR_CONSERVATIVE),
    ("自动", BEHAVIOR_AUTO),
];

// ── 单元格工具 ──────────────────────────────────────────────────

fn cell_str(row: &[Data], index: usize) -> String {
    row.get(index)
        .and_then(|cell| cell.get_string())
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn cell_f64(row: &[Data], index: usize) -> Option<f64> {
    // as_f64 覆盖 Int / Float / Bool / 数值字符串（get_float 会漏掉整数单元格）
    row.get(index).and_then(DataType::as_f64)
}

// ── 模板导入 ────────────────────────────────────────────────────

pub struct TemplateData {
    pub companies: Vec<Company>,
    pub scenarios: Vec<Scenario>,
    pub params: ScoringParams,
    pub warnings: Vec<String>,
}

/// 解析统一模板。公司 / 场景 Sheet 缺失报错，评分参数 Sheet 缺失用默认值。
pub fn import_template(path: &str) -> Result<TemplateData, AppError> {
    let mut workbook: Xlsx<BufReader<std::fs::File>> = open_workbook(path)
        .map_err(|err| AppError::invalid_input(format!("无法打开文件：{err}")))?;
    parse_workbook(&mut workbook)
}

/// 从已打开的工作簿解析三个 Sheet（泛型 reader，供文件与内置模板测试共用）
fn parse_workbook<R>(workbook: &mut Xlsx<R>) -> Result<TemplateData, AppError>
where
    R: std::io::Read + std::io::Seek,
{
    let mut warnings = Vec::new();

    let companies = read_companies(workbook, &mut warnings)?;
    let scenarios = read_scenarios(workbook, &mut warnings)?;
    let params = read_params(workbook);

    if companies.is_empty() {
        return Err(AppError::invalid_input("「公司数据」Sheet 中没有有效数据"));
    }

    Ok(TemplateData {
        companies,
        scenarios,
        params,
        warnings,
    })
}

fn sheet_range<R>(workbook: &mut Xlsx<R>, name: &str) -> Result<Option<Range<Data>>, AppError>
where
    R: std::io::Read + std::io::Seek,
{
    match workbook.worksheet_range(name) {
        Ok(range) => Ok(Some(range)),
        Err(_) => Ok(None),
    }
}

fn read_companies<R>(
    workbook: &mut Xlsx<R>,
    warnings: &mut Vec<String>,
) -> Result<Vec<Company>, AppError>
where
    R: std::io::Read + std::io::Seek,
{
    let Some(range) = sheet_range(workbook, SHEET_COMPANIES)? else {
        return Err(AppError::invalid_input(format!(
            "模板缺少「{SHEET_COMPANIES}」工作表"
        )));
    };

    let mut companies = Vec::new();
    for (row_idx, row) in range.rows().enumerate().skip(1) {
        let name = cell_str(row, 0);
        if name.is_empty() {
            continue;
        }

        let Some(round1_price) = cell_f64(row, 1) else {
            warnings.push(format!(
                "第 {} 行「{name}」的价格数据无效，已跳过",
                row_idx + 1
            ));
            continue;
        };
        let Some(price_limit) = cell_f64(row, 2) else {
            warnings.push(format!(
                "第 {} 行「{name}」的限价数据无效，已跳过",
                row_idx + 1
            ));
            continue;
        };
        if round1_price <= 0.0 || price_limit <= 0.0 {
            warnings.push(format!(
                "第 {} 行「{name}」的价格必须大于 0，已跳过",
                row_idx + 1
            ));
            continue;
        }
        if round1_price > price_limit {
            warnings.push(format!(
                "「{name}」的报价（{round1_price}）超过限价（{price_limit}）"
            ));
        }

        let company_type = cell_str(row, 3).to_uppercase();
        if !matches!(
            company_type.as_str(),
            COMPANY_TYPE_TARGET | COMPANY_TYPE_AUX | COMPANY_TYPE_UNCONTROLLED
        ) {
            warnings.push(format!(
                "第 {} 行「{name}」的类型无效：{company_type}，已跳过",
                row_idx + 1
            ));
            continue;
        }

        let behavior_raw = cell_str(row, 4);
        let behavior = VALID_BEHAVIORS
            .iter()
            .find(|(label, _)| *label == behavior_raw)
            .map(|(_, value)| value.to_string());

        let note_raw = cell_str(row, 5);
        let note = if note_raw.is_empty() {
            None
        } else {
            Some(note_raw)
        };

        companies.push(Company {
            id: None,
            name,
            round1_price,
            price_limit,
            company_type,
            behavior,
            note,
        });
    }

    Ok(companies)
}

fn read_scenarios<R>(
    workbook: &mut Xlsx<R>,
    warnings: &mut Vec<String>,
) -> Result<Vec<Scenario>, AppError>
where
    R: std::io::Read + std::io::Seek,
{
    let Some(range) = sheet_range(workbook, SHEET_SCENARIOS)? else {
        return Err(AppError::invalid_input(format!(
            "模板缺少「{SHEET_SCENARIOS}」工作表"
        )));
    };

    let mut scenarios = Vec::new();
    for (row_idx, row) in range.rows().enumerate().skip(1) {
        let name = cell_str(row, 0);
        if name.is_empty() {
            continue;
        }
        let row_no = row_idx + 1;

        let reduction_raw = cell_str(row, 1);
        let reduction_type = match reduction_raw.as_str() {
            "百分比" => REDUCTION_PERCENT,
            "数值" => REDUCTION_AMOUNT,
            _ => {
                warnings.push(format!(
                    "第 {row_no} 行场景「{name}」的降价方式无效：{reduction_raw}，已跳过"
                ));
                continue;
            }
        }
        .to_string();

        let Some(reduction_value) = cell_f64(row, 2) else {
            warnings.push(format!(
                "第 {row_no} 行场景「{name}」的降价数值无效，已跳过"
            ));
            continue;
        };
        let Some(participation_rate) = cell_f64(row, 3) else {
            warnings.push(format!("第 {row_no} 行场景「{name}」的参与率无效，已跳过"));
            continue;
        };

        if !(0.0..=1.0).contains(&participation_rate) {
            warnings.push(format!(
                "第 {row_no} 行场景「{name}」的参与率应在 0-1 之间，当前值：{participation_rate}"
            ));
        }
        if reduction_value > 0.0 {
            warnings.push(format!(
                "第 {row_no} 行场景「{name}」的降幅应为负数（表示降价），当前值：{reduction_value}"
            ));
        }

        let is_active_raw = cell_str(row, 4);
        let is_active = if is_active_raw.is_empty() {
            true
        } else {
            is_active_raw == "是"
        };

        let mut std_dev = cell_f64(row, 5).unwrap_or(0.0);
        if std_dev < 0.0 {
            warnings.push(format!(
                "第 {row_no} 行场景「{name}」的标准差不能为负数，已置 0"
            ));
            std_dev = 0.0;
        }

        let note_raw = cell_str(row, 6);
        let note = if note_raw.is_empty() {
            None
        } else {
            Some(note_raw)
        };

        scenarios.push(Scenario {
            id: None,
            name,
            reduction_type,
            reduction_value,
            participation_rate,
            std_dev,
            is_active,
            note,
        });
    }

    Ok(scenarios)
}

fn read_params<R>(workbook: &mut Xlsx<R>) -> ScoringParams
where
    R: std::io::Read + std::io::Seek,
{
    let mut params = ScoringParams::default();
    let Ok(Some(range)) = sheet_range(workbook, SHEET_PARAMS) else {
        return params;
    };

    for row in range.rows().skip(1) {
        let name = cell_str(row, 0);
        let Some(value) = cell_f64(row, 1) else {
            continue;
        };
        apply_param(&name, value, &mut params);
    }

    params
}

/// 参数行匹配（顺序敏感：先中文描述后拉丁代号，避免宽松匹配误中段落标题）
fn apply_param(name: &str, value: f64, params: &mut ScoringParams) {
    if name.starts_with("W1") || name.contains("区间下限") {
        params.w1 = value;
    } else if name.starts_with("W2") || name.contains("区间上限") {
        params.w2 = value;
    } else if name.contains("激进型") && name.contains("系数") {
        params.aggressive_factor = value;
    } else if name.contains("正常型") && name.contains("系数") {
        params.normal_factor = value;
    } else if name.contains("保守型") && name.contains("系数") {
        params.conservative_factor = value;
    } else if name.contains("价格差异") || name.contains("最小价差") || name.contains("最小差异")
    {
        params.min_aux_price_diff = value;
    } else if (name.starts_with('n') && name.contains('1')) || name.contains("高于基准价惩罚")
    {
        params.n1 = value;
    } else if (name.starts_with('n') && name.contains('2')) || name.contains("低于基准价惩罚")
    {
        params.n2 = value;
    } else if name.starts_with('C') || name.contains("基准价浮动") {
        params.c = value;
    }
}

// ── 结果导出 ────────────────────────────────────────────────────

fn reduction_desc(reduction_type: &str, value: f64) -> String {
    if reduction_type == REDUCTION_PERCENT {
        format!("{:+.1}%", value * 100.0)
    } else {
        format!("{:+.2}万", value)
    }
}

fn std_desc(reduction_type: &str, std: f64) -> String {
    if std == 0.0 {
        return "无波动".to_string();
    }
    if reduction_type == REDUCTION_PERCENT {
        format!("±{:.1}%", std * 100.0)
    } else {
        format!("±{:.2}万", std)
    }
}

fn behavior_label(behavior: Option<&str>) -> String {
    match behavior.unwrap_or("") {
        BEHAVIOR_AGGRESSIVE => "激进型",
        BEHAVIOR_CONSERVATIVE => "保守型",
        BEHAVIOR_AUTO => "自动",
        BEHAVIOR_NORMAL => "正常型",
        _ => "-",
    }
    .to_string()
}

fn type_label(company_type: &str) -> &'static str {
    match company_type {
        COMPANY_TYPE_TARGET => "目标",
        COMPANY_TYPE_AUX => "辅助",
        _ => "不可控",
    }
}

/// 写出结果工作簿（复刻 Python 版双 Sheet 布局与样式），成功返回文件路径
pub fn export_results(payload: &ExportPayload, path: &str) -> Result<String, AppError> {
    let mut workbook = Workbook::new();

    // 样式：表头蓝底白字居中；数字右对齐带格式；文本居中
    let header_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xFFFFFF))
        .set_background_color(Color::RGB(0x4472C4))
        .set_align(FormatAlign::Center);
    let text_fmt = Format::new().set_align(FormatAlign::Center);
    let num2_fmt = Format::new()
        .set_num_format("0.00")
        .set_align(FormatAlign::Right);
    let int_fmt = Format::new().set_align(FormatAlign::Right);
    let num4_fmt = Format::new()
        .set_num_format("0.0000")
        .set_align(FormatAlign::Right);
    let change_fmt = Format::new()
        .set_num_format("+0.0000;-0.0000;0")
        .set_align(FormatAlign::Right);

    let aux_indices: Vec<usize> = payload
        .companies
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_controllable() && !c.is_target())
        .map(|(i, _)| i)
        .collect();
    let best = payload.results.get(payload.best_index);

    // 写入主体统一收口 XlsxError
    let mut write_all = || -> Result<(), XlsxError> {
        // ── Sheet 1：测算结果 ──
        let sheet = workbook.add_worksheet().set_name("测算结果")?;
        let mut headers: Vec<String> = vec![
            "场景".into(),
            "降价方式".into(),
            "降价数值".into(),
            "标准差".into(),
            "参与率".into(),
            "预测均价(万)".into(),
            "搜索范围(万)".into(),
            "目标报价(万)".into(),
            "基准价(万)".into(),
            "得分".into(),
            "模拟次数".into(),
        ];
        for i in 0..aux_indices.len() {
            headers.push(format!("辅助{}(万)", i + 1));
        }
        for (col, header) in headers.iter().enumerate() {
            sheet.write_with_format(0, col as u16, header, &header_fmt)?;
        }

        for (row_idx, r) in payload.results.iter().enumerate() {
            let row = (row_idx + 1) as u32;
            let type_name = if r.reduction_type == REDUCTION_PERCENT {
                "百分比"
            } else {
                "数值"
            };
            sheet.write_with_format(row, 0, &r.scenario_name, &text_fmt)?;
            sheet.write_with_format(row, 1, type_name, &text_fmt)?;
            sheet.write_with_format(
                row,
                2,
                reduction_desc(&r.reduction_type, r.reduction_value),
                &text_fmt,
            )?;
            sheet.write_with_format(row, 3, std_desc(&r.reduction_type, r.std_dev), &text_fmt)?;
            sheet.write_with_format(
                row,
                4,
                format!("{}%", (r.participation_rate * 100.0).round()),
                &text_fmt,
            )?;
            sheet.write_with_format(row, 5, r.search_center, &num2_fmt)?;
            sheet.write_with_format(row, 6, format!("±{:.2}", r.search_range), &text_fmt)?;
            sheet.write_with_format(row, 7, r.target_price, &num4_fmt)?;
            sheet.write_with_format(row, 8, r.base_price, &num2_fmt)?;
            sheet.write_with_format(row, 9, r.target_score, &num2_fmt)?;
            sheet.write_with_format(row, 10, r.num_simulations, &int_fmt)?;
            for (i, &aux_price) in r.aux_prices.iter().enumerate() {
                sheet.write_with_format(row, (11 + i) as u16, aux_price, &num4_fmt)?;
            }
        }
        sheet.autofit();

        // ── Sheet 2：公司数据 ──
        let sheet = workbook.add_worksheet().set_name(SHEET_COMPANIES)?;
        let company_headers = [
            "公司名称",
            "第一轮报价(万)",
            "含税限价(万)",
            "类型",
            "行为类型",
            "推荐报价(万)",
            "变化(万)",
        ];
        for (col, header) in company_headers.iter().enumerate() {
            sheet.write_with_format(0, col as u16, *header, &header_fmt)?;
        }

        for (row_idx, c) in payload.companies.iter().enumerate() {
            let row = (row_idx + 1) as u32;
            sheet.write_with_format(row, 0, &c.name, &text_fmt)?;
            sheet.write_with_format(row, 1, c.round1_price, &num2_fmt)?;
            sheet.write_with_format(row, 2, c.price_limit, &num2_fmt)?;
            sheet.write_with_format(row, 3, type_label(&c.company_type), &text_fmt)?;
            sheet.write_with_format(row, 4, behavior_label(c.behavior.as_deref()), &text_fmt)?;

            // 推荐报价：目标公司 → 最优方案目标报价；辅助公司 → 对应位辅助报价
            let recommended = best.and_then(|b| {
                if c.is_target() {
                    Some(b.target_price)
                } else if c.is_controllable() {
                    aux_indices
                        .iter()
                        .position(|&idx| idx == row_idx)
                        .and_then(|aux_pos| b.aux_prices.get(aux_pos).copied())
                } else {
                    None
                }
            });

            if let Some(price) = recommended {
                sheet.write_with_format(row, 5, price, &num4_fmt)?;
                sheet.write_with_format(row, 6, price - c.round1_price, &change_fmt)?;
            }
        }
        sheet.autofit();

        Ok(())
    };
    write_all().map_err(|err| AppError::custom(code::IO_ERROR, format!("导出文件失败：{err}")))?;

    workbook
        .save(path)
        .map_err(|err| AppError::custom(code::IO_ERROR, format!("保存文件失败：{err}")))?;

    log::info!("测算结果已导出: {path}");
    Ok(path.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_matching_order() {
        let mut params = ScoringParams::default();
        // 段落标题行（值为空时不会进入 apply_param，这里验证即便误入也不破坏 W1/W2）
        apply_param("【公司行为系数(P3)】", 1.0, &mut params);
        assert_eq!(params.w1, -0.15);

        apply_param("W1 (区间下限)", -0.2, &mut params);
        assert_eq!(params.w1, -0.2);
        apply_param("n2 (低于基准价惩罚)", 0.6, &mut params);
        assert_eq!(params.n2, 0.6);
        apply_param("C (基准价浮动系数)", 0.05, &mut params);
        assert_eq!(params.c, 0.05);
        apply_param("激进型降幅系数", 1.4, &mut params);
        assert_eq!(params.aggressive_factor, 1.4);
        apply_param("辅助公司最小价差(万)", 0.03, &mut params);
        assert_eq!(params.min_aux_price_diff, 0.03);
    }

    #[test]
    fn import_bundled_template() {
        // 用内置模板做回归：解析结果须满足测算前置条件
        let cursor = std::io::Cursor::new(super::super::TEMPLATE_XLSX);
        let mut workbook =
            <Xlsx<std::io::Cursor<&[u8]>> as Reader<std::io::Cursor<&[u8]>>>::new(cursor).unwrap();
        let data = parse_workbook(&mut workbook).expect("内置模板应可解析");

        assert_eq!(data.companies.iter().filter(|c| c.is_target()).count(), 1);
        assert!(data.companies.iter().any(|c| !c.is_controllable()));
        assert!(data.scenarios.iter().any(|s| s.is_active));
        // 模板显式配置的最小价差应被读取（此前 Python 版因表头「最小价差」失配而落默认值）
        assert!((data.params.min_aux_price_diff - 0.02).abs() < 1e-9);
    }

    #[test]
    fn import_roundtrip_integer_cells() {
        // 回环回归：整数报价在 xlsx 内部存为整数字符串 → calamine 读 Int，
        // 须能正常读取（get_float 会漏掉 Int，须走 as_f64）
        let mut wb = Workbook::new();

        let ws = wb.add_worksheet().set_name(SHEET_COMPANIES).unwrap();
        let headers = [
            "公司名称",
            "第一轮报价(万)",
            "含税限价(万)",
            "类型",
            "公司行为",
            "说明",
        ];
        for (col, h) in headers.iter().enumerate() {
            ws.write(0, col as u16, *h).unwrap();
        }
        ws.write(1, 0, "目标公司").unwrap();
        ws.write(1, 1, 19_i64).unwrap(); // 整数报价
        ws.write(1, 2, 19.08_f64).unwrap();
        ws.write(1, 3, "T").unwrap();
        ws.write(2, 0, "对手公司").unwrap();
        ws.write(2, 1, 18_i64).unwrap();
        ws.write(2, 2, 19_i64).unwrap();
        ws.write(2, 3, "U").unwrap();

        let ws = wb.add_worksheet().set_name(SHEET_SCENARIOS).unwrap();
        let headers = [
            "场景名称",
            "降价方式",
            "降价数值",
            "参与率",
            "有效",
            "标准差",
            "说明",
        ];
        for (col, h) in headers.iter().enumerate() {
            ws.write(0, col as u16, *h).unwrap();
        }
        ws.write(1, 0, "中性场景").unwrap();
        ws.write(1, 1, "百分比").unwrap();
        ws.write(1, 2, -0.05_f64).unwrap();
        ws.write(1, 3, 0.8_f64).unwrap();
        ws.write(1, 4, "是").unwrap();
        ws.write(1, 5, 0.015_f64).unwrap();

        let ws = wb.add_worksheet().set_name(SHEET_PARAMS).unwrap();
        ws.write(0, 0, "参数名称").unwrap();
        ws.write(0, 1, "参数值").unwrap();
        ws.write(1, 0, "W1 (区间下限)").unwrap();
        ws.write(1, 1, -0.2_f64).unwrap();

        let path = std::env::temp_dir().join("tender_roundtrip_integer_test.xlsx");
        wb.save(&path).unwrap();

        let data = import_template(path.to_str().unwrap()).expect("回环模板应可解析");
        let _ = std::fs::remove_file(&path);

        assert_eq!(data.companies.len(), 2);
        assert_eq!(data.companies[0].round1_price, 19.0);
        assert_eq!(data.companies[1].round1_price, 18.0);
        assert_eq!(data.companies[1].price_limit, 19.0);
        assert_eq!(data.companies[0].company_type, "T");
        assert_eq!(data.scenarios.len(), 1);
        assert_eq!(data.scenarios[0].participation_rate, 0.8);
        assert!(data.scenarios[0].is_active);
        assert_eq!(data.params.w1, -0.2);
    }

    #[test]
    fn descriptions() {
        assert_eq!(reduction_desc(REDUCTION_PERCENT, -0.05), "-5.0%");
        assert_eq!(reduction_desc(REDUCTION_AMOUNT, -0.5), "-0.50万");
        assert_eq!(std_desc(REDUCTION_PERCENT, 0.0), "无波动");
        assert_eq!(std_desc(REDUCTION_PERCENT, 0.015), "±1.5%");
        assert_eq!(std_desc(REDUCTION_AMOUNT, 0.1), "±0.10万");
        assert_eq!(behavior_label(Some("aggressive")), "激进型");
        assert_eq!(type_label(COMPANY_TYPE_TARGET), "目标");
    }
}
