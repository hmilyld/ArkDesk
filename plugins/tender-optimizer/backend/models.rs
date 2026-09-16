//! 投标报价测算 — 数据模型。
//!
//! 字段与前端 `plugins/tender-optimizer/frontend/schema.ts`、`frontend/shared.ts` 保持一致
//! （serde 默认蛇形命名，TS 侧直接用相同键名）。

use serde::{Deserialize, Serialize};

/// 公司类型：T = 目标公司（有且仅有 1 家）、A = 辅助公司、U = 不可控公司
pub const COMPANY_TYPE_TARGET: &str = "T";
pub const COMPANY_TYPE_AUX: &str = "A";
pub const COMPANY_TYPE_UNCONTROLLED: &str = "U";

/// 公司行为类型（建模降价意愿）：aggressive=激进型 / normal=正常型 /
/// conservative=保守型 / auto=按报价与限价比例自动判档
pub const BEHAVIOR_AGGRESSIVE: &str = "aggressive";
pub const BEHAVIOR_NORMAL: &str = "normal";
pub const BEHAVIOR_CONSERVATIVE: &str = "conservative";
pub const BEHAVIOR_AUTO: &str = "auto";

/// 降价方式：percent=百分比 / amount=数值（万元）
pub const REDUCTION_PERCENT: &str = "percent";
pub const REDUCTION_AMOUNT: &str = "amount";

/// 投标公司
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    /// 第一轮报价（万元）
    pub round1_price: f64,
    /// 含税限价（万元）
    pub price_limit: f64,
    /// T / A / U
    pub company_type: String,
    /// aggressive / normal / conservative / auto（空 = normal）
    #[serde(default)]
    pub behavior: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl Company {
    pub fn is_target(&self) -> bool {
        self.company_type == COMPANY_TYPE_TARGET
    }

    pub fn is_controllable(&self) -> bool {
        self.company_type != COMPANY_TYPE_UNCONTROLLED
    }
}

/// 评分参数（含 P3 行为系数与辅助公司配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringParams {
    /// 有效区间下限（如 -0.15 = 低于均价 15% 以内有效）
    pub w1: f64,
    /// 有效区间上限
    pub w2: f64,
    /// 基准价浮动系数：基准价 = 有效均价 × (1 − C)
    pub c: f64,
    /// 高于基准价惩罚系数
    pub n1: f64,
    /// 低于基准价惩罚系数
    pub n2: f64,
    /// 激进型降幅系数
    pub aggressive_factor: f64,
    /// 正常型降幅系数
    pub normal_factor: f64,
    /// 保守型降幅系数
    pub conservative_factor: f64,
    /// 辅助公司间最小价差（万元）
    pub min_aux_price_diff: f64,
}

impl Default for ScoringParams {
    fn default() -> Self {
        ScoringParams {
            w1: -0.15,
            w2: 0.10,
            c: 0.0,
            n1: 1.0,
            n2: 0.5,
            aggressive_factor: 1.3,
            normal_factor: 1.0,
            conservative_factor: 0.5,
            min_aux_price_diff: 0.02,
        }
    }
}

/// 测算场景
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    /// percent / amount
    pub reduction_type: String,
    /// 降幅（负数表示降价）
    pub reduction_value: f64,
    /// 参与率 0-1
    pub participation_rate: f64,
    /// 降幅标准差（0 = 无波动）
    pub std_dev: f64,
    pub is_active: bool,
    #[serde(default)]
    pub note: Option<String>,
}

/// P3：按行为类型取降幅系数（auto 按报价/限价比判档）
pub fn behavior_factor(
    behavior: Option<&str>,
    round1_price: f64,
    price_limit: f64,
    params: &ScoringParams,
) -> f64 {
    match behavior.unwrap_or("") {
        BEHAVIOR_AGGRESSIVE => params.aggressive_factor,
        BEHAVIOR_CONSERVATIVE => params.conservative_factor,
        BEHAVIOR_AUTO => auto_behavior_factor(round1_price, price_limit, params),
        // normal / 空 / 未知
        _ => params.normal_factor,
    }
}

/// 单场景测算结果（含场景快照，供历史记录与导出复用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_name: String,
    pub reduction_type: String,
    pub reduction_value: f64,
    pub participation_rate: f64,
    pub std_dev: f64,
    pub target_price: f64,
    pub aux_prices: Vec<f64>,
    pub target_score: f64,
    pub base_price: f64,
    pub num_simulations: u32,
    pub search_center: f64,
    pub search_range: f64,
    pub search_step: f64,
}

/// 公司推荐报价（按公司数组下标对应；不可控公司无推荐价）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyRecommendation {
    pub index: usize,
    pub recommended_price: Option<f64>,
    pub change: Option<f64>,
}

/// calculate 命令响应：全部场景结果 + 最优场景 + 逐公司推荐报价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculateResponse {
    pub results: Vec<ScenarioResult>,
    pub best_index: usize,
    pub recommended: Vec<CompanyRecommendation>,
}

/// 导出结果工作簿的载荷（前端从历史记录取回后原样传回）
#[derive(Debug, Clone, Deserialize)]
pub struct ExportPayload {
    pub companies: Vec<Company>,
    pub results: Vec<ScenarioResult>,
    pub best_index: usize,
}

/// 自动判档：报价越接近限价越激进（0.98 以上激进、0.95 以上偏激进、
/// 0.90 以上正常、0.85 以上偏保守、其余保守），档间线性插值
fn auto_behavior_factor(round1_price: f64, price_limit: f64, params: &ScoringParams) -> f64 {
    if price_limit <= 0.0 {
        return params.normal_factor;
    }

    let ratio = round1_price / price_limit;

    if ratio > 0.98 {
        params.aggressive_factor
    } else if ratio > 0.95 {
        params.normal_factor + (params.aggressive_factor - params.normal_factor) * 0.5
    } else if ratio > 0.90 {
        params.normal_factor
    } else if ratio > 0.85 {
        params.conservative_factor + (params.normal_factor - params.conservative_factor) * 0.5
    } else {
        params.conservative_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavior_explicit_types() {
        let params = ScoringParams::default();
        assert_eq!(
            behavior_factor(Some("aggressive"), 19.0, 20.0, &params),
            1.3
        );
        assert_eq!(behavior_factor(Some("normal"), 19.0, 20.0, &params), 1.0);
        assert_eq!(
            behavior_factor(Some("conservative"), 19.0, 20.0, &params),
            0.5
        );
        // 空/未知回退正常型
        assert_eq!(behavior_factor(None, 19.0, 20.0, &params), 1.0);
        assert_eq!(behavior_factor(Some("whatever"), 19.0, 20.0, &params), 1.0);
    }

    #[test]
    fn behavior_auto_thresholds() {
        let params = ScoringParams::default();
        // 报价/限价比驱动判档（与 Python 版阈值一致）
        assert_eq!(behavior_factor(Some("auto"), 19.7, 20.0, &params), 1.3); // 0.985 > 0.98
        assert_eq!(behavior_factor(Some("auto"), 19.5, 20.0, &params), 1.15); // 0.975 → (0.95, 0.98] 插值
        assert_eq!(behavior_factor(Some("auto"), 19.0, 20.0, &params), 1.0); // 0.95 → (0.90, 0.95] 正常
        assert_eq!(behavior_factor(Some("auto"), 18.0, 20.0, &params), 0.75); // 0.90 → (0.85, 0.90] 插值
        assert_eq!(behavior_factor(Some("auto"), 16.0, 20.0, &params), 0.5); // 0.80 ≤ 0.85 保守
    }

    #[test]
    fn company_helpers() {
        let target = Company {
            id: None,
            name: "T".into(),
            round1_price: 19.0,
            price_limit: 20.0,
            company_type: "T".into(),
            behavior: None,
            note: None,
        };
        assert!(target.is_target());
        assert!(target.is_controllable());
    }
}
