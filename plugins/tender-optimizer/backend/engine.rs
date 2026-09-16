//! 投标报价测算 — 核心算法（移植自 TenderOptimizer Python 版）。
//!
//! 三个特性：
//! - P1 降幅标准差：每家不可控公司降幅独立高斯波动
//! - P2 自适应搜索范围：以预测市场均价为中心，按波动幅度定搜索区间
//! - P3 公司行为模型：激进/正常/保守系数区分降价意愿
//!
//! 可复现性：每次模拟固定种子（seed + sim），同一输入结果确定；
//! 随机流与 CPython random 不同（统计分布一致，数值非逐位复现）。

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rand_distr::Normal;

use super::models::{behavior_factor, Company, Scenario, ScoringParams};

/// 根据合格投标人数 M 确定「去掉最高 / 去掉最低」的数量
fn get_remove_count(m: usize) -> (usize, usize) {
    match m {
        0..=5 => (0, 0),
        6..=10 => (1, 1),
        11..=20 => (2, 1),
        21..=30 => (2, 2),
        _ => (4, 3),
    }
}

/// 去掉最高最低后的平均价 A1
fn calc_a1(prices: &[f64]) -> f64 {
    let m = prices.len();
    if m == 0 {
        return 0.0;
    }
    let (remove_high, remove_low) = get_remove_count(m);
    let mut sorted: Vec<f64> = prices.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let remaining = &sorted[remove_low..m - remove_high];
    if remaining.is_empty() {
        mean(prices)
    } else {
        mean(remaining)
    }
}

/// 计算有效投标人与基准价。
/// 返回 (A1, 基准价, 有效投标人索引)。有效区间为闭区间 [A1×(1+W1), A1×(1+W2)]；
/// 无人落区间时回退为「去除最高最低后的剩余投标人」。
fn calc_effective_and_base(prices: &[f64], params: &ScoringParams) -> (f64, f64, Vec<usize>) {
    let a1 = calc_a1(prices);
    let lower = a1 * (1.0 + params.w1);
    let upper = a1 * (1.0 + params.w2);
    let mut effective_idx: Vec<usize> = (0..prices.len())
        .filter(|&i| prices[i] >= lower && prices[i] <= upper)
        .collect();

    if effective_idx.is_empty() {
        let m = prices.len();
        let (remove_high, remove_low) = get_remove_count(m);
        let mut sorted_idx: Vec<usize> = (0..m).collect();
        sorted_idx.sort_by(|&a, &b| {
            prices[a]
                .partial_cmp(&prices[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        effective_idx = sorted_idx[remove_low..m - remove_high].to_vec();
    }

    let effective_prices: Vec<f64> = effective_idx.iter().map(|&i| prices[i]).collect();
    let a2 = mean(&effective_prices);
    let base_price = a2 * (1.0 - params.c);

    (a1, base_price, effective_idx)
}

/// 单个投标人价格得分：100 − 100 × n × |报价 − 基准价| / 基准价（下限 0）
fn calc_score(price: f64, base_price: f64, params: &ScoringParams) -> f64 {
    if base_price <= 0.0 {
        return 0.0;
    }
    let n = if price >= base_price {
        params.n1
    } else {
        params.n2
    };
    (100.0 - 100.0 * n * (price - base_price).abs() / base_price).max(0.0)
}

/// 全体投标人得分
fn calc_all_scores(prices: &[f64], params: &ScoringParams) -> Vec<f64> {
    let (_, base_price, _) = calc_effective_and_base(prices, params);
    prices
        .iter()
        .map(|&p| calc_score(p, base_price, params))
        .collect()
}

/// P2：预测二轮市场均价（不可控公司按场景降幅直接折算后的均值）
fn predict_market_avg(
    companies: &[Company],
    uncontrollable_indices: &[usize],
    scenario: &Scenario,
) -> f64 {
    let predicted: Vec<f64> = uncontrollable_indices
        .iter()
        .map(|&i| {
            let p = companies[i].round1_price;
            if scenario.reduction_type == super::models::REDUCTION_PERCENT {
                p * (1.0 + scenario.reduction_value)
            } else {
                p + scenario.reduction_value
            }
        })
        .collect();
    mean(&predicted)
}

/// P2：搜索范围（标准差驱动，无标准差时取降幅的一半）
fn calc_search_range(scenario: &Scenario, target_first_price: f64) -> f64 {
    if scenario.std_dev > 0.0 {
        if scenario.reduction_type == super::models::REDUCTION_PERCENT {
            scenario.std_dev * target_first_price * 2.5
        } else {
            scenario.std_dev * 2.5
        }
    } else if scenario.reduction_type == super::models::REDUCTION_PERCENT {
        scenario.reduction_value.abs() * target_first_price * 0.5
    } else {
        scenario.reduction_value.abs() * 0.5
    }
}

/// P2：动态搜索步长 = 首轮报价 × 0.02%，钳制到 [0.001, 0.05]（万元）
fn calc_search_step(target_first_price: f64) -> f64 {
    let step = target_first_price * 0.0002;
    let step = step.clamp(0.001, 0.05);
    if step >= 0.01 {
        (step * 100.0).round() / 100.0
    } else {
        (step * 1000.0).round() / 1000.0
    }
}

/// P1 + P3：单家不可控公司的实际降幅 = 场景降幅 × 行为系数 + 高斯波动。
/// 标准差为 0 时按降幅的 15%（百分比）/ 10%（数值）隐式波动。
fn calc_company_reduction(
    rng: &mut StdRng,
    round1_price: f64,
    price_limit: f64,
    behavior: Option<&str>,
    scenario: &Scenario,
    params: &ScoringParams,
) -> f64 {
    let factor = behavior_factor(behavior, round1_price, price_limit, params);
    let base_reduction = scenario.reduction_value * factor;

    let sigma = if scenario.std_dev > 0.0 {
        scenario.std_dev
    } else if scenario.reduction_type == super::models::REDUCTION_PERCENT {
        scenario.reduction_value.abs() * 0.15
    } else {
        scenario.reduction_value.abs() * 0.1
    };

    let variation = gauss(rng, 0.0, sigma);
    base_reduction + variation
}

/// 高斯采样（sigma <= 0 时返回均值，对应 Python gauss(0, 0) = 0）
fn gauss(rng: &mut StdRng, mean: f64, sigma: f64) -> f64 {
    if sigma <= 0.0 {
        return mean;
    }
    Normal::new(mean, sigma)
        .map(|dist| rng.sample(dist))
        .unwrap_or(mean)
}

/// P2 搜索计划：搜索边界、预测中心、范围与步长
#[derive(Debug, Clone, Copy)]
struct SearchPlan {
    target_min: f64,
    target_max: f64,
    center: f64,
    range: f64,
    step: f64,
}

/// 单次模拟内的最优报价搜索。`current_prices` 为本次模拟出的不可控公司报价
/// （索引 → 报价），目标公司报价在搜索区间内按步长遍历，
/// 辅助公司取限价附近的固定档位组合，选目标得分最高的组合。
fn find_optimal_prices(
    companies: &[Company],
    target_idx: usize,
    aux_indices: &[usize],
    current_prices: &[(usize, f64)],
    params: &ScoringParams,
    plan: &SearchPlan,
) -> Option<OptimalResult> {
    let mut best_score = -1.0;
    let mut best: Option<OptimalResult> = None;

    // 有效搜索区间：中心 ± 范围与 [首轮×0.88, 首轮] 取交
    let (mut search_min, mut search_max) = (
        plan.target_min.max(plan.center - plan.range),
        plan.target_max.min(plan.center + plan.range),
    );
    if search_min >= search_max {
        search_min = plan.target_min;
        search_max = plan.target_max;
    }

    let target_prices: Vec<f64> = {
        let mut v = Vec::new();
        let mut p = search_min;
        while p < search_max {
            v.push(p);
            p += plan.step;
        }
        v
    };

    let mut all_prices: Vec<f64> = {
        let mut v = vec![0.0; companies.len()];
        for &(idx, price) in current_prices {
            v[idx] = price;
        }
        v
    };

    let num_aux = aux_indices.len();
    let aux_a_options: [f64; 4] = [0.995, 0.99, 0.985, 0.98];
    let aux_b_options: [f64; 4] = [0.975, 0.97, 0.965, 0.96];

    if num_aux == 0 {
        for &tp in &target_prices {
            all_prices[target_idx] = tp;
            let target_score = calc_all_scores(&all_prices, params)[target_idx];
            if target_score > best_score {
                best_score = target_score;
                let (_, base_price, _) = calc_effective_and_base(&all_prices, params);
                best = Some(OptimalResult {
                    target_price: round4(tp),
                    aux_prices: vec![],
                    target_score: round4(target_score),
                    base_price: round4(base_price),
                });
            }
        }
    } else if num_aux == 1 {
        let limit = companies[aux_indices[0]].price_limit;
        for &ratio in &aux_a_options {
            let aux_a = limit * ratio;
            for &tp in &target_prices {
                all_prices[target_idx] = tp;
                all_prices[aux_indices[0]] = aux_a;
                let target_score = calc_all_scores(&all_prices, params)[target_idx];
                if target_score > best_score {
                    best_score = target_score;
                    let (_, base_price, _) = calc_effective_and_base(&all_prices, params);
                    best = Some(OptimalResult {
                        target_price: round4(tp),
                        aux_prices: vec![round4(aux_a)],
                        target_score: round4(target_score),
                        base_price: round4(base_price),
                    });
                }
            }
        }
    } else {
        let limit = companies[aux_indices[0]].price_limit;
        let other_aux_fixed = limit * 0.98;
        for &a_ratio in &aux_a_options {
            let aux_a = limit * a_ratio;
            for &b_ratio in &aux_b_options {
                let aux_b = limit * b_ratio;
                // 辅助公司间最小价差约束
                if (aux_a - aux_b).abs() < params.min_aux_price_diff {
                    continue;
                }
                for &tp in &target_prices {
                    all_prices[target_idx] = tp;
                    all_prices[aux_indices[0]] = aux_a;
                    all_prices[aux_indices[1]] = aux_b;
                    for &aux_idx in aux_indices.iter().skip(2) {
                        all_prices[aux_idx] = other_aux_fixed;
                    }
                    let target_score = calc_all_scores(&all_prices, params)[target_idx];
                    if target_score > best_score {
                        best_score = target_score;
                        let (_, base_price, _) = calc_effective_and_base(&all_prices, params);
                        let mut aux_prices = vec![round4(aux_a), round4(aux_b)];
                        for _ in 2..num_aux {
                            aux_prices.push(round4(other_aux_fixed));
                        }
                        best = Some(OptimalResult {
                            target_price: round4(tp),
                            aux_prices,
                            target_score: round4(target_score),
                            base_price: round4(base_price),
                        });
                    }
                }
            }
        }
    }

    best
}

/// 蒙特卡洛模拟 + 优化：对每个启用场景跑 num_simulations 次模拟，
/// 各次结果取中位数作为推荐报价（比均值更稳健）。
/// 目标 / 辅助 / 不可控公司索引由公司类型推导。
pub fn simulate_and_optimize(
    companies: &[Company],
    scenario: &Scenario,
    params: &ScoringParams,
    seed: u64,
    num_simulations: u32,
) -> Option<SimulationOutcome> {
    let target_idx = companies.iter().position(|c| c.is_target())?;
    let aux_indices: Vec<usize> = companies
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_controllable() && !c.is_target())
        .map(|(i, _)| i)
        .collect();
    let uncontrollable_indices: Vec<usize> = companies
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_controllable())
        .map(|(i, _)| i)
        .collect();

    let target_r1 = companies[target_idx].round1_price;
    let plan = SearchPlan {
        target_min: target_r1 * 0.88,
        target_max: target_r1,
        center: predict_market_avg(companies, &uncontrollable_indices, scenario),
        range: calc_search_range(scenario, target_r1),
        step: calc_search_step(target_r1),
    };
    let search_center = plan.center;
    let search_range = plan.range;
    let search_step = plan.step;

    let mut all_results: Vec<OptimalResult> = Vec::new();

    for sim in 0..num_simulations {
        let mut rng = StdRng::seed_from_u64(seed + sim as u64);

        // 逐步模拟不可控公司的二轮报价（参与判定在前、降幅波动在后，
        // 与 Python 版随机调用顺序一致）
        let mut current_prices: Vec<(usize, f64)> =
            Vec::with_capacity(uncontrollable_indices.len());
        for &idx in &uncontrollable_indices {
            let company = &companies[idx];
            if rng.random::<f64>() < scenario.participation_rate {
                let reduction = calc_company_reduction(
                    &mut rng,
                    company.round1_price,
                    company.price_limit,
                    company.behavior.as_deref(),
                    scenario,
                    params,
                );
                let new_price = if scenario.reduction_type == super::models::REDUCTION_PERCENT {
                    company.round1_price * (1.0 + reduction)
                } else {
                    company.round1_price + reduction
                };
                current_prices.push((idx, round4(new_price.max(0.01))));
            } else {
                current_prices.push((idx, company.round1_price));
            }
        }

        if let Some(result) = find_optimal_prices(
            companies,
            target_idx,
            &aux_indices,
            &current_prices,
            params,
            &plan,
        ) {
            all_results.push(result);
        }
    }

    // 并列时取先出现的结果（与 Python max() 语义一致；max_by 会取最后一个）
    let best_sim = all_results
        .iter()
        .fold(None::<&OptimalResult>, |acc, r| match acc {
            None => Some(r),
            Some(best) if r.target_score > best.target_score => Some(r),
            _ => acc,
        })?
        .clone();

    Some(SimulationOutcome {
        target_price: round4(median(
            &all_results
                .iter()
                .map(|r| r.target_price)
                .collect::<Vec<_>>(),
        )),
        aux_prices: best_sim.aux_prices.clone(),
        target_score: round4(median(
            &all_results
                .iter()
                .map(|r| r.target_score)
                .collect::<Vec<_>>(),
        )),
        base_price: round4(median(
            &all_results.iter().map(|r| r.base_price).collect::<Vec<_>>(),
        )),
        num_simulations,
        search_center: round4(search_center),
        search_range: round4(search_range),
        search_step: round4(search_step),
    })
}

/// 单次模拟的最优解
#[derive(Debug, Clone)]
struct OptimalResult {
    target_price: f64,
    aux_prices: Vec<f64>,
    target_score: f64,
    base_price: f64,
}

/// 模拟汇总（中位数推荐 + 搜索元信息）
#[derive(Debug, Clone, serde::Serialize)]
pub struct SimulationOutcome {
    pub target_price: f64,
    pub aux_prices: Vec<f64>,
    pub target_score: f64,
    pub base_price: f64,
    pub num_simulations: u32,
    pub search_center: f64,
    pub search_range: f64,
    pub search_step: f64,
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// 中位数（偶数个取中间两数均值，与 numpy.median 一致）
fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = sorted.len();
    if m % 2 == 1 {
        sorted[m / 2]
    } else {
        (sorted[m / 2 - 1] + sorted[m / 2]) / 2.0
    }
}

fn round4(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::tender_optimizer::models::{
        COMPANY_TYPE_AUX, COMPANY_TYPE_TARGET, COMPANY_TYPE_UNCONTROLLED,
    };

    fn company(name: &str, r1: f64, limit: f64, kind: &str) -> Company {
        Company {
            id: None,
            name: name.into(),
            round1_price: r1,
            price_limit: limit,
            company_type: kind.into(),
            behavior: None,
            note: None,
        }
    }

    fn scenario_percent(value: f64, rate: f64, std: f64) -> Scenario {
        Scenario {
            id: None,
            name: "测试场景".into(),
            reduction_type: super::super::models::REDUCTION_PERCENT.into(),
            reduction_value: value,
            participation_rate: rate,
            std_dev: std,
            is_active: true,
            note: None,
        }
    }

    // ── 评分公式 ────────────────────────────────────────────────

    #[test]
    fn remove_count_table() {
        assert_eq!(get_remove_count(5), (0, 0));
        assert_eq!(get_remove_count(6), (1, 1));
        assert_eq!(get_remove_count(10), (1, 1));
        assert_eq!(get_remove_count(11), (2, 1));
        assert_eq!(get_remove_count(20), (2, 1));
        assert_eq!(get_remove_count(21), (2, 2));
        assert_eq!(get_remove_count(31), (4, 3));
    }

    #[test]
    fn a1_trims_high_and_low() {
        // 6 家 → 去掉 1 高 1 低，剩余 4 家均值
        let prices = vec![10.0, 11.0, 12.0, 13.0, 14.0, 100.0];
        assert!((calc_a1(&prices) - 12.5).abs() < 1e-9);
        // ≤5 家不去极值
        let prices = vec![1.0, 2.0, 3.0];
        assert!((calc_a1(&prices) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn effective_range_and_base_price() {
        let params = ScoringParams::default();
        // 6 家报价：均价 12.5，区间 [12.5×0.85, 12.5×1.10] = [10.625, 13.75]
        let prices = vec![10.0, 11.0, 12.0, 13.0, 14.0, 100.0];
        let (a1, base, idx) = calc_effective_and_base(&prices, &params);
        assert!((a1 - 12.5).abs() < 1e-9);
        // 区间 [10.625, 13.75]：10.0、14.0、100.0 落区间外 → 有效 3 家
        assert_eq!(idx, vec![1, 2, 3]);
        assert!((base - 12.0).abs() < 1e-9); // C = 0 → 基准价 = 有效均价 (11+12+13)/3
    }

    #[test]
    fn score_formula() {
        let params = ScoringParams {
            n1: 1.0,
            n2: 0.5,
            ..ScoringParams::default()
        };
        // 高于基准价：100 − 100×1×1.25/100 = 98.75（n1 更重）
        assert!((calc_score(101.25, 100.0, &params) - 98.75).abs() < 1e-9);
        // 低于基准价：100 − 100×0.5×1.25/100 = 99.375（n2 更轻 → 略低更优）
        assert!((calc_score(98.75, 100.0, &params) - 99.375).abs() < 1e-9);
        // 基准价非正 → 0 分
        assert_eq!(calc_score(10.0, 0.0, &params), 0.0);
        // 偏离过大钳制到 0
        assert_eq!(calc_score(200.0, 100.0, &params), 0.0);
    }

    // ── P2 搜索策略 ─────────────────────────────────────────────

    #[test]
    fn search_range_rules() {
        let with_std = scenario_percent(-0.05, 0.8, 0.015);
        let without_std = scenario_percent(-0.05, 0.8, 0.0);
        let amount = Scenario {
            reduction_type: super::super::models::REDUCTION_AMOUNT.into(),
            ..scenario_percent(-0.5, 0.8, 0.0)
        };
        // 有标准差：std × 首轮报价 × 2.5
        assert!((calc_search_range(&with_std, 20.0) - 0.015 * 20.0 * 2.5).abs() < 1e-9);
        // 无标准差：|降幅| × 首轮报价 × 0.5
        assert!((calc_search_range(&without_std, 20.0) - 0.05 * 20.0 * 0.5).abs() < 1e-9);
        // 数值方式：|降幅| × 0.5
        assert!((calc_search_range(&amount, 20.0) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn search_step_clamped() {
        // 20 万标包 → 40 元（0.004 万）
        assert!((calc_search_step(20.0) - 0.004).abs() < 1e-9);
        // 极大标包钳到 0.05（500 元）
        assert!((calc_search_step(1000.0) - 0.05).abs() < 1e-9);
        // 极小标包钳到 0.001（10 元）
        assert!((calc_search_step(1.0) - 0.001).abs() < 1e-9);
    }

    // ── 整体模拟不变量 ──────────────────────────────────────────

    #[test]
    fn simulation_invariants() {
        let companies = vec![
            company("目标公司", 19.05, 19.08, COMPANY_TYPE_TARGET),
            company("辅助公司", 18.90, 19.08, COMPANY_TYPE_AUX),
            company("对手A", 18.70, 19.08, COMPANY_TYPE_UNCONTROLLED),
            company("对手B", 18.50, 19.08, COMPANY_TYPE_UNCONTROLLED),
        ];
        let scenario = scenario_percent(-0.05, 0.8, 0.015);
        let params = ScoringParams::default();

        let outcome =
            simulate_and_optimize(&companies, &scenario, &params, 42, 100).expect("应产出模拟结果");

        // 目标报价必须落在搜索下限与首轮报价之间
        assert!(outcome.target_price >= 19.05 * 0.88 - 1e-6);
        assert!(outcome.target_price <= 19.05 + 1e-6);
        // 得分为正、基准价为正
        assert!(outcome.target_score > 0.0);
        assert!(outcome.base_price > 0.0);
        // 辅助公司取限价附近档位
        assert_eq!(outcome.aux_prices.len(), 1);
        assert!(outcome.aux_prices[0] > 18.0 && outcome.aux_prices[0] <= 19.08);
        // 固定种子可复现
        let again = simulate_and_optimize(&companies, &scenario, &params, 42, 100).unwrap();
        assert_eq!(again.target_price, outcome.target_price);
        assert_eq!(again.target_score, outcome.target_score);
    }

    #[test]
    fn simulation_deterministic_seed_change() {
        let companies = vec![
            company("目标公司", 19.05, 19.08, COMPANY_TYPE_TARGET),
            company("对手A", 18.70, 19.08, COMPANY_TYPE_UNCONTROLLED),
        ];
        let scenario = scenario_percent(-0.05, 0.8, 0.015);
        let params = ScoringParams::default();

        let a = simulate_and_optimize(&companies, &scenario, &params, 42, 30).unwrap();
        let b = simulate_and_optimize(&companies, &scenario, &params, 7, 30).unwrap();
        // 不同种子统计接近但允许细微差异
        assert!((a.target_price - b.target_price).abs() < 0.5);
    }
}
