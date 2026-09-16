/**
 * tender-optimizer 插件内共享类型与工具。
 *
 * - 「线格式」类型与 Rust 侧 serde 结构体对齐（snake_case 字段名）
 * - 「行格式」类型取自 schema.ts 的 $inferSelect（camelCase，数据库读写用）
 * - 展示格式化与标签映射供两个工具页共用
 */

import type { TenderCompany, TenderParams, TenderScenario } from './schema';

// ── 领域枚举 ────────────────────────────────────────────────────

export type CompanyType = 'T' | 'A' | 'U';
export type BehaviorType = 'aggressive' | 'normal' | 'conservative' | 'auto';
export type ReductionType = 'percent' | 'amount';

export const COMPANY_TYPE_OPTIONS = [
  { value: 'T', label: '目标公司' },
  { value: 'A', label: '辅助公司' },
  { value: 'U', label: '不可控公司' },
] as const;

export const BEHAVIOR_OPTIONS = [
  { value: 'aggressive', label: '激进型' },
  { value: 'normal', label: '正常型' },
  { value: 'conservative', label: '保守型' },
  { value: 'auto', label: '自动' },
] as const;

export const REDUCTION_TYPE_OPTIONS = [
  { value: 'percent', label: '百分比' },
  { value: 'amount', label: '数值' },
] as const;

export const COMPANY_TYPE_LABELS: Record<CompanyType, string> = {
  T: '目标',
  A: '辅助',
  U: '不可控',
};

// ── 线格式（与 Rust serde 对齐，snake_case） ────────────────────

export interface ScoringParams {
  w1: number;
  w2: number;
  c: number;
  n1: number;
  n2: number;
  aggressive_factor: number;
  normal_factor: number;
  conservative_factor: number;
  min_aux_price_diff: number;
}

export interface CompanyInput {
  name: string;
  round1_price: number;
  price_limit: number;
  company_type: CompanyType;
  behavior: BehaviorType | null;
  note?: string | null;
}

export interface ScenarioInput {
  name: string;
  reduction_type: ReductionType;
  reduction_value: number;
  participation_rate: number;
  std_dev: number;
  is_active: boolean;
  note?: string | null;
}

export interface ScenarioResult {
  scenario_name: string;
  reduction_type: ReductionType;
  reduction_value: number;
  participation_rate: number;
  std_dev: number;
  target_price: number;
  aux_prices: number[];
  target_score: number;
  base_price: number;
  num_simulations: number;
  search_center: number;
  search_range: number;
  search_step: number;
}

export interface CompanyRecommendation {
  index: number;
  recommended_price: number | null;
  change: number | null;
}

export interface CalculateResponse {
  results: ScenarioResult[];
  best_index: number;
  recommended: CompanyRecommendation[];
}

export interface ImportResult {
  plan_name: string;
  companies: CompanyInput[];
  scenarios: ScenarioInput[];
  params: ScoringParams;
  warnings: string[];
}

/** 导出结果工作簿载荷（History → Rust 原样传回） */
export interface ExportPayload {
  companies: CompanyInput[];
  results: ScenarioResult[];
  best_index: number;
}

/** 评分参数默认值（与 Rust ScoringParams::default 同步维护） */
export const DEFAULT_PARAMS: ScoringParams = {
  w1: -0.15,
  w2: 0.1,
  c: 0,
  n1: 1,
  n2: 0.5,
  aggressive_factor: 1.3,
  normal_factor: 1,
  conservative_factor: 0.5,
  min_aux_price_diff: 0.02,
};

export const DEFAULT_NUM_SIMULATIONS = 100;

/** 历史列表分页大小 */
export const PAGE_SIZE = 10;

// ── 行格式 → 线格式 ─────────────────────────────────────────────

export function toCompanyInput(row: TenderCompany): CompanyInput {
  return {
    name: row.name,
    round1_price: row.round1Price,
    price_limit: row.priceLimit,
    company_type: row.companyType as CompanyType,
    behavior: (row.behavior as BehaviorType | null) ?? null,
    note: row.note,
  };
}

export function toScenarioInput(row: TenderScenario): ScenarioInput {
  return {
    name: row.name,
    reduction_type: row.reductionType as ReductionType,
    reduction_value: row.reductionValue,
    participation_rate: row.participationRate,
    std_dev: row.stdDev,
    is_active: row.isActive,
    note: row.note,
  };
}

/** 参数行（camelCase）→ 线格式（snake_case，与 Rust serde 对齐） */
export function paramsToWire(row: TenderParams): ScoringParams {
  return {
    w1: row.w1,
    w2: row.w2,
    c: row.c,
    n1: row.n1,
    n2: row.n2,
    aggressive_factor: row.aggressiveFactor,
    normal_factor: row.normalFactor,
    conservative_factor: row.conservativeFactor,
    min_aux_price_diff: row.minAuxPriceDiff,
  };
}

// ── 展示格式化（对齐 Python 版描述文案） ────────────────────────

export function formatReduction(type: ReductionType, value: number): string {
  return type === 'percent' ? `${(value * 100).toFixed(1)}%` : `${value.toFixed(2)}万`;
}

export function formatStd(type: ReductionType, std: number): string {
  if (std === 0) return '无波动';
  return type === 'percent' ? `±${(std * 100).toFixed(1)}%` : `±${std.toFixed(2)}万`;
}

export function formatRate(rate: number): string {
  return `${Math.round(rate * 100)}%`;
}

export function formatWan(value: number | null | undefined, digits = 2): string {
  return value === null || value === undefined ? '-' : value.toFixed(digits);
}

/**
 * 逐公司推荐报价：目标公司 → 最优方案目标报价；
 * 辅助公司 → 最优方案对应位辅助报价；不可控公司 → 无。
 */
export function computeRecommended(
  companies: CompanyInput[],
  best: ScenarioResult
): CompanyRecommendation[] {
  const auxIndices = companies
    .map((c, i) => ({ c, i }))
    .filter(({ c }) => c.company_type !== 'U' && c.company_type !== 'T')
    .map(({ i }) => i);

  return companies.map((c, index) => {
    let recommended: number | null = null;
    if (c.company_type === 'T') {
      recommended = best.target_price;
    } else if (c.company_type === 'A') {
      const pos = auxIndices.indexOf(index);
      recommended = pos >= 0 ? (best.aux_prices[pos] ?? null) : null;
    }
    return {
      index,
      recommended_price: recommended,
      change: recommended === null ? null : recommended - c.round1_price,
    };
  });
}
