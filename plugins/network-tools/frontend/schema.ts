/**
 * network-tools 数据库 schema（JPA Entity 的等价物）。
 *
 * - sqliteTable 键 = TS 字段名，值 = 列名（snake_case，须与 backend/migrations.rs 对齐）
 * - 列表/对象字段（query/headers/.../auth_config）以 JSON 文本存储，取用后自行解析
 * - 行类型用 $inferSelect 导出，勿手写 interface
 */

import { integer, sqliteTable, text } from 'drizzle-orm/sqlite-core';
import { sql } from 'drizzle-orm';

export const networkToolsCollections = sqliteTable('network_tools_collections', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  parentId: integer('parent_id'),
  type: text('type').notNull().default('folder'),
  name: text('name').notNull(),
  sortOrder: integer('sort_order').notNull().default(0),
  createdAt: text('created_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
  updatedAt: text('updated_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
});

export const networkToolsRequests = sqliteTable('network_tools_requests', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  collectionId: integer('collection_id'),
  name: text('name').notNull(),
  method: text('method').notNull().default('GET'),
  url: text('url').notNull().default(''),
  query: text('query').notNull().default('[]'),
  headers: text('headers').notNull().default('[]'),
  cookies: text('cookies').notNull().default('[]'),
  bodyType: text('body_type').notNull().default('none'),
  bodyText: text('body_text').notNull().default(''),
  bodyLang: text('body_lang').notNull().default('json'),
  formFields: text('form_fields').notNull().default('[]'),
  multipartFields: text('multipart_fields').notNull().default('[]'),
  binaryPath: text('binary_path').notNull().default(''),
  authType: text('auth_type').notNull().default('none'),
  authConfig: text('auth_config').notNull().default('{}'),
  sortOrder: integer('sort_order').notNull().default(0),
  createdAt: text('created_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
  updatedAt: text('updated_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
});

export const networkToolsDrafts = sqliteTable('network_tools_drafts', {
  id: integer('id').primaryKey(),
  payload: text('payload').notNull(),
  updatedAt: text('updated_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
});

export const networkToolsEnvironments = sqliteTable('network_tools_environments', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  name: text('name').notNull(),
  createdAt: text('created_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
  updatedAt: text('updated_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
});

export const networkToolsEnvVars = sqliteTable('network_tools_env_vars', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  environmentId: integer('environment_id').notNull(),
  key: text('key').notNull(),
  value: text('value').notNull().default(''),
  enabled: integer('enabled').notNull().default(1),
  isSecret: integer('is_secret').notNull().default(0),
  sortOrder: integer('sort_order').notNull().default(0),
});

export const networkToolsHistory = sqliteTable('network_tools_history', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  method: text('method').notNull(),
  url: text('url').notNull(),
  request: text('request').notNull().default('{}'),
  status: integer('status'),
  ok: integer('ok'),
  elapsedMs: integer('elapsed_ms'),
  sizeBytes: integer('size_bytes'),
  responseHeaders: text('response_headers').notNull().default('[]'),
  responseBody: text('response_body'),
  bodyTruncated: integer('body_truncated').notNull().default(0),
  error: text('error'),
  environmentId: integer('environment_id'),
  createdAt: text('created_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
});

export type NetworkToolsCollection = typeof networkToolsCollections.$inferSelect;
export type NetworkToolsRequest = typeof networkToolsRequests.$inferSelect;
export type NetworkToolsDraft = typeof networkToolsDrafts.$inferSelect;
export type NetworkToolsEnvironment = typeof networkToolsEnvironments.$inferSelect;
export type NetworkToolsEnvVar = typeof networkToolsEnvVars.$inferSelect;
export type NetworkToolsHistory = typeof networkToolsHistory.$inferSelect;
