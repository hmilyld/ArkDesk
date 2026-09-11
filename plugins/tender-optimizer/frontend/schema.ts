/**
 * tender-optimizer 数据库 schema。
 *
 * - sqliteTable 键 = TS 字段名，值 = 列名（snake_case 与迁移定义对齐）
 * - 行类型用 $inferSelect 导出（勿再手写 interface）
 */

import { integer, real, sqliteTable, text } from 'drizzle-orm/sqlite-core';
import { sql } from 'drizzle-orm';

/** 公司工作集（单方案，导入模板时整体替换） */
export const tenderCompanies = sqliteTable('tender_companies', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  name: text('name').notNull(),
  round1Price: real('round1_price').notNull(),
  priceLimit: real('price_limit').notNull(),
  companyType: text('company_type').notNull().default('U'),
  behavior: text('behavior'),
  note: text('note'),
  sortOrder: integer('sort_order').notNull().default(0),
});

/** 测算场景工作集 */
export const tenderScenarios = sqliteTable('tender_scenarios', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  name: text('name').notNull(),
  reductionType: text('reduction_type').notNull().default('percent'),
  reductionValue: real('reduction_value').notNull(),
  participationRate: real('participation_rate').notNull().default(1),
  stdDev: real('std_dev').notNull().default(0),
  isActive: integer('is_active', { mode: 'boolean' }).notNull().default(true),
  note: text('note'),
  sortOrder: integer('sort_order').notNull().default(0),
});

/** 评分参数（单行配置，id 恒为 1；plan_name 为当前工作集的方案名称） */
export const tenderParams = sqliteTable('tender_params', {
  id: integer('id').primaryKey(),
  w1: real('w1').notNull().default(-0.15),
  w2: real('w2').notNull().default(0.1),
  c: real('c').notNull().default(0),
  n1: real('n1').notNull().default(1),
  n2: real('n2').notNull().default(0.5),
  aggressiveFactor: real('aggressive_factor').notNull().default(1.3),
  normalFactor: real('normal_factor').notNull().default(1),
  conservativeFactor: real('conservative_factor').notNull().default(0.5),
  minAuxPriceDiff: real('min_aux_price_diff').notNull().default(0.02),
  numSimulations: integer('num_simulations').notNull().default(100),
  planName: text('plan_name').notNull().default(''),
});

/** 测算历史（结果 JSON 快照 + 列表页摘要列） */
export const tenderHistory = sqliteTable('tender_history', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  createdAt: text('created_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
  planName: text('plan_name').notNull().default(''),
  numCompanies: integer('num_companies').notNull(),
  numScenarios: integer('num_scenarios').notNull(),
  numSimulations: integer('num_simulations').notNull(),
  bestIndex: integer('best_index').notNull().default(0),
  bestScore: real('best_score'),
  bestScenario: text('best_scenario'),
  targetPrice: real('target_price'),
  basePrice: real('base_price'),
  companiesJson: text('companies_json').notNull(),
  scenariosJson: text('scenarios_json').notNull(),
  paramsJson: text('params_json'),
  resultsJson: text('results_json').notNull(),
});

export type TenderCompany = typeof tenderCompanies.$inferSelect;
export type TenderScenario = typeof tenderScenarios.$inferSelect;
export type TenderParams = typeof tenderParams.$inferSelect;
export type TenderHistory = typeof tenderHistory.$inferSelect;
