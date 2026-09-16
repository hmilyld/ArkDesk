//! 投标报价测算工具：基于市场降价假设，蒙特卡洛模拟竞争环境，
//! 为目标公司计算最优二轮报价方案（移植自 TenderOptimizer Python 版）。
//!
//! 命令清单（`tender_optimizer_` 前缀，构建期由 build.rs 扫描本文件自动登记）：
//! - `tender_optimizer_import_template`：解析统一模板并替换工作集
//! - `tender_optimizer_calculate`：纯函数测算（数据由前端传入）
//! - `tender_optimizer_export_result`：写出带样式的结果工作簿
//! - `tender_optimizer_download_template`：另存内置模板

pub mod engine;
pub mod excel;
pub mod migrations;
pub mod models;

use crate::db::SqlArgs;
use crate::error::{code, AppError};
use models::{
    CalculateResponse, Company, CompanyRecommendation, Scenario, ScenarioResult, ScoringParams,
};
use serde::Serialize;

/// 内置模板（源文件复制自 TenderOptimizer 的「招标报价模板.xlsx」）
const TEMPLATE_XLSX: &[u8] = include_bytes!("template.xlsx");

/// 测算前校验（硬错误与 Python 版 main.py 对齐）：
/// 报价有效性 + 有且仅有一家目标公司 + 至少一个启用场景。
/// 「首轮报价 > 限价」沿用 Python 语义：导入时告警保留、测算照常进行。
fn validate(companies: &[Company], scenarios: &[Scenario]) -> Result<(), AppError> {
    for c in companies {
        if c.name.trim().is_empty() {
            return Err(AppError::invalid_input("公司名称不能为空"));
        }
        if c.round1_price <= 0.0 || c.price_limit <= 0.0 {
            return Err(AppError::invalid_input(format!(
                "「{}」的报价与限价必须大于 0",
                c.name
            )));
        }
    }

    let target_count = companies.iter().filter(|c| c.is_target()).count();
    if target_count != 1 {
        return Err(AppError::invalid_input(format!(
            "必须有且仅有一家目标公司（T），当前 {target_count} 家"
        )));
    }

    if !scenarios.iter().any(|s| s.is_active) {
        return Err(AppError::invalid_input("至少需要一个启用的测算场景"));
    }

    Ok(())
}

/// 把引擎结果组装为 ScenarioResult（附场景快照）
fn to_scenario_result(scenario: &Scenario, outcome: engine::SimulationOutcome) -> ScenarioResult {
    ScenarioResult {
        scenario_name: scenario.name.clone(),
        reduction_type: scenario.reduction_type.clone(),
        reduction_value: scenario.reduction_value,
        participation_rate: scenario.participation_rate,
        std_dev: scenario.std_dev,
        target_price: outcome.target_price,
        aux_prices: outcome.aux_prices,
        target_score: outcome.target_score,
        base_price: outcome.base_price,
        num_simulations: outcome.num_simulations,
        search_center: outcome.search_center,
        search_range: outcome.search_range,
        search_step: outcome.search_step,
    }
}

/// 按最优方案计算逐公司推荐报价（不可控公司无推荐价）
fn compute_recommendations(
    companies: &[Company],
    best: &ScenarioResult,
) -> Vec<CompanyRecommendation> {
    let aux_indices: Vec<usize> = companies
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_controllable() && !c.is_target())
        .map(|(i, _)| i)
        .collect();

    companies
        .iter()
        .enumerate()
        .map(|(index, c)| {
            let recommended = if c.is_target() {
                Some(best.target_price)
            } else if c.is_controllable() {
                aux_indices
                    .iter()
                    .position(|&idx| idx == index)
                    .and_then(|pos| best.aux_prices.get(pos).copied())
            } else {
                None
            };
            CompanyRecommendation {
                index,
                recommended_price: recommended,
                change: recommended.map(|price| price - c.round1_price),
            }
        })
        .collect()
}

#[derive(Serialize)]
pub struct ImportResult {
    pub plan_name: String,
    pub companies: Vec<Company>,
    pub scenarios: Vec<Scenario>,
    pub params: ScoringParams,
    pub warnings: Vec<String>,
}

/// 导入统一模板：解析三个 Sheet 并整体替换工作集（公司 / 场景 / 参数），
/// 并记录方案名称（空串保持原名称）。无效行跳过并收集 warning 返回给前端提示。
#[tauri::command]
pub async fn tender_optimizer_import_template(
    path: String,
    plan_name: Option<String>,
) -> Result<ImportResult, AppError> {
    if !path.to_ascii_lowercase().ends_with(".xlsx") {
        return Err(AppError::invalid_input("仅支持 .xlsx 模板文件"));
    }

    let plan_name = plan_name
        .map(|name| name.trim().to_string())
        .unwrap_or_default();

    let data = tauri::async_runtime::spawn_blocking(move || excel::import_template(&path))
        .await
        .map_err(|err| AppError::custom(code::UNKNOWN, format!("导入任务失败：{err}")))??;

    // 替换工作集（框架通道暂无跨语句事务，逐条执行；数据量小可接受）
    crate::db::db_execute(SqlArgs {
        sql: "DELETE FROM tender_companies".to_string(),
        params: vec![],
    })
    .await?;
    crate::db::db_execute(SqlArgs {
        sql: "DELETE FROM tender_scenarios".to_string(),
        params: vec![],
    })
    .await?;
    crate::db::db_execute(SqlArgs {
        sql: "UPDATE tender_params SET w1 = $1, w2 = $2, c = $3, n1 = $4, n2 = $5,
              aggressive_factor = $6, normal_factor = $7, conservative_factor = $8,
              min_aux_price_diff = $9,
              plan_name = COALESCE(NULLIF($10, ''), plan_name)
              WHERE id = 1"
            .to_string(),
        params: vec![
            serde_json::json!(data.params.w1),
            serde_json::json!(data.params.w2),
            serde_json::json!(data.params.c),
            serde_json::json!(data.params.n1),
            serde_json::json!(data.params.n2),
            serde_json::json!(data.params.aggressive_factor),
            serde_json::json!(data.params.normal_factor),
            serde_json::json!(data.params.conservative_factor),
            serde_json::json!(data.params.min_aux_price_diff),
            serde_json::json!(plan_name),
        ],
    })
    .await?;

    for (order, c) in data.companies.iter().enumerate() {
        crate::db::db_execute(SqlArgs {
            sql: "INSERT INTO tender_companies
                  (name, round1_price, price_limit, company_type, behavior, note, sort_order)
                  VALUES ($1, $2, $3, $4, $5, $6, $7)"
                .to_string(),
            params: vec![
                serde_json::json!(c.name),
                serde_json::json!(c.round1_price),
                serde_json::json!(c.price_limit),
                serde_json::json!(c.company_type),
                serde_json::json!(c.behavior),
                serde_json::json!(c.note),
                serde_json::json!(order as i64),
            ],
        })
        .await?;
    }

    for (order, s) in data.scenarios.iter().enumerate() {
        crate::db::db_execute(SqlArgs {
            sql: "INSERT INTO tender_scenarios
                  (name, reduction_type, reduction_value, participation_rate, std_dev,
                   is_active, note, sort_order)
                  VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
                .to_string(),
            params: vec![
                serde_json::json!(s.name),
                serde_json::json!(s.reduction_type),
                serde_json::json!(s.reduction_value),
                serde_json::json!(s.participation_rate),
                serde_json::json!(s.std_dev),
                serde_json::json!(s.is_active),
                serde_json::json!(s.note),
                serde_json::json!(order as i64),
            ],
        })
        .await?;
    }

    log::info!(
        "模板导入完成：公司 {} 家，场景 {} 个，警告 {} 条",
        data.companies.len(),
        data.scenarios.len(),
        data.warnings.len()
    );

    Ok(ImportResult {
        plan_name,
        companies: data.companies,
        scenarios: data.scenarios,
        params: data.params,
        warnings: data.warnings,
    })
}

/// 执行测算：对每个启用场景跑蒙特卡洛模拟（数据由前端传入，纯函数无状态）。
/// 返回各场景结果、最优场景下标与逐公司推荐报价。
/// 计算量随模拟次数 × 辅助组合数增长，放阻塞线程池避免卡 UI。
#[tauri::command]
pub async fn tender_optimizer_calculate(
    companies: Vec<Company>,
    scenarios: Vec<Scenario>,
    params: ScoringParams,
    num_simulations: Option<u32>,
) -> Result<CalculateResponse, AppError> {
    validate(&companies, &scenarios)?;

    let simulations = num_simulations.unwrap_or(100).clamp(1, 2000);

    let response = tauri::async_runtime::spawn_blocking(move || {
        let mut results: Vec<ScenarioResult> = Vec::new();
        for scenario in scenarios.iter().filter(|s| s.is_active) {
            if let Some(outcome) =
                engine::simulate_and_optimize(&companies, scenario, &params, 42, simulations)
            {
                results.push(to_scenario_result(scenario, outcome));
            }
        }
        if results.is_empty() {
            return None;
        }

        // 并列时取先出现的场景（与 Python max() 语义一致）
        let best_index = results.iter().enumerate().fold(0usize, |acc, (i, r)| {
            if r.target_score > results[acc].target_score {
                i
            } else {
                acc
            }
        });

        let recommended = compute_recommendations(&companies, &results[best_index]);

        Some(CalculateResponse {
            results,
            best_index,
            recommended,
        })
    })
    .await
    .map_err(|err| AppError::custom(code::UNKNOWN, format!("测算任务失败：{err}")))?
    .ok_or_else(|| AppError::custom(code::PLUGIN_ERROR, "测算失败：所有场景均未产出有效结果"))?;

    log::info!(
        "测算完成：{} 个场景，最优得分 {:.4}",
        response.results.len(),
        response.results[response.best_index].target_score
    );

    Ok(response)
}

/// 导出结果工作簿（路径由前端 save 对话框提供），返回文件路径
#[tauri::command]
pub fn tender_optimizer_export_result(
    payload: models::ExportPayload,
    path: String,
) -> Result<String, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("导出路径不能为空"));
    }
    if payload.results.is_empty() {
        return Err(AppError::invalid_input("没有可导出的测算结果"));
    }
    let best_index = payload.best_index.min(payload.results.len() - 1);
    let payload = models::ExportPayload {
        companies: payload.companies,
        results: payload.results,
        best_index,
    };
    excel::export_results(&payload, &path)
}

/// 另存内置模板文件（路径由前端 save 对话框提供），返回文件路径
#[tauri::command]
pub fn tender_optimizer_download_template(path: String) -> Result<String, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::invalid_input("保存路径不能为空"));
    }
    std::fs::write(&path, TEMPLATE_XLSX)?;
    log::info!("模板已另存: {path}");
    Ok(path)
}
