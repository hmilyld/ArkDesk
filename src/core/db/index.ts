/**
 * 数据库访问统一封装（SQLite，自建 Rust sqlx 通道）。
 *
 * - 底层 select/execute：手写 SQL，一律 $1 占位符参数化，杜绝注入
 * - 通用方法（insert/findAll 等）：快速 CRUD，表名/列名做标识符白名单校验
 *   （标识符无法参数化，校验 + 双引号包裹是防注入的关键）
 * - Drizzle ORM：`kdb` 提供类型安全的对象化查询（见 orm.ts / README）
 * - 复杂查询（JOIN / 聚合 / 非等值条件）用 kdb 表达式或手写 SQL
 * - 多语句原子写入用 runInTransaction()（Rust 侧单事务执行，任一失败整体回滚）
 */
import { ErrorCode, normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';

import {
  executeSql,
  executeTransaction,
  queryValues,
  rowsToObjects,
  type SqlStatement,
} from './client';
import { kdb } from './orm';

export { kdb };
export { schema } from './orm';
export type { SqlStatement } from './client';

// ─────────────────────────── 底层方法 ───────────────────────────

/** 统一错误转换：记日志并抛出带 code 的标准 Error */
function toDbError(operation: string, err: unknown): Error {
  const error = normalizeError(err);
  logger.error(`db ${operation} failed: [${error.code}] ${error.message}`);
  return Object.assign(new Error(error.message), {
    code: ErrorCode.DbError,
    details: error.details,
  });
}

function assertParams(params: unknown[]): void {
  if (!Array.isArray(params)) {
    throw normalizeError({
      code: ErrorCode.InvalidInput,
      message: 'db 查询参数必须是数组（$1 占位符参数化）',
    });
  }
}

/** 参数化 SELECT 查询，返回对象数组 */
export async function select<T>(sql: string, params: unknown[] = []): Promise<T[]> {
  assertParams(params);
  try {
    const { columns, rows } = await queryValues(sql, params);
    return rowsToObjects<T>(columns, rows);
  } catch (err) {
    throw toDbError('select', err);
  }
}

/** 参数化 INSERT / UPDATE / DELETE，返回受影响行信息 */
export async function execute(
  sql: string,
  params: unknown[] = []
): Promise<{ rowsAffected: number; lastInsertId: number | null }> {
  assertParams(params);
  try {
    const result = await executeSql(sql, params);
    return { rowsAffected: result.rowsAffected, lastInsertId: result.lastInsertId };
  } catch (err) {
    throw toDbError('execute', err);
  }
}

/** 多语句原子写入：在单个事务中执行，任一失败整体回滚 */
export async function runInTransaction(
  statements: SqlStatement[]
): Promise<{ rowsAffected: number; lastInsertId: number | null }> {
  if (!Array.isArray(statements) || statements.length === 0) {
    throw normalizeError({ code: ErrorCode.InvalidInput, message: '事务语句不能为空' });
  }
  for (const statement of statements) {
    assertParams(statement.params ?? []);
  }
  try {
    const result = await executeTransaction(statements);
    return { rowsAffected: result.rowsAffected, lastInsertId: result.lastInsertId };
  } catch (err) {
    throw toDbError('transaction', err);
  }
}

// ─────────────────────────── 通用 CRUD ───────────────────────────

/** 记录对象：列名 → 值 */
export type DbRecord = Record<string, unknown>;

/** 查询选项（findAll / count 共用） */
export interface FindOptions {
  /** 等值条件（AND 连接）；复杂条件请用 kdb 表达式或 select 手写 SQL */
  where?: DbRecord;
  /** 排序："列名" 或 "列名 DESC/ASC"（标识符校验） */
  orderBy?: string;
  limit?: number;
  offset?: number;
}

/** SQLite 标识符白名单：字母/下划线开头，仅含字母数字下划线 */
const IDENTIFIER_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

/** 标识符校验（表名/列名无法参数化，校验 + 双引号包裹防注入） */
function quoteIdentifier(name: string, label: string): string {
  if (!IDENTIFIER_RE.test(name)) {
    throw normalizeError({
      code: ErrorCode.InvalidInput,
      message: `非法的${label}: ${name}`,
    });
  }
  return `"${name}"`;
}

/** 校验并构建 ORDER BY 子句（白名单：单列 + 可选 ASC/DESC） */
function buildOrderBy(orderBy: string): string {
  const parts = orderBy.trim().split(/\s+/);
  const column = quoteIdentifier(parts[0] ?? '', '排序列');
  if (parts.length === 1) return column;
  if (parts.length === 2 && /^(ASC|DESC)$/i.test(parts[1] ?? '')) {
    return `${column} ${parts[1]!.toUpperCase()}`;
  }
  throw normalizeError({
    code: ErrorCode.InvalidInput,
    message: `非法的排序子句: ${orderBy}`,
  });
}

/** 校验并构建 WHERE 子句（等值条件 AND 连接），空条件视为开发错误 */
function buildWhere(where: DbRecord): { clause: string; params: unknown[] } {
  const keys = Object.keys(where);
  if (keys.length === 0) {
    throw normalizeError({
      code: ErrorCode.InvalidInput,
      message: 'where 条件不能为空（全表操作请显式使用 select/execute）',
    });
  }
  const params = keys.map((key) => where[key]);
  const clause = keys
    .map((key, index) => `${quoteIdentifier(key, '条件列')} = $${index + 1}`)
    .join(' AND ');
  return { clause, params };
}

function assertNonNegativeInt(value: number, label: string): void {
  if (!Number.isInteger(value) || value < 0) {
    throw normalizeError({
      code: ErrorCode.InvalidInput,
      message: `${label} 必须是非负整数: ${value}`,
    });
  }
}

/** 插入一条记录，返回自增主键（表无自增主键时为 null） */
export async function insert(table: string, data: DbRecord): Promise<number | null> {
  const keys = Object.keys(data);
  if (keys.length === 0) {
    throw normalizeError({ code: ErrorCode.InvalidInput, message: '插入内容不能为空' });
  }
  const columns = keys.map((key) => quoteIdentifier(key, '列名')).join(', ');
  const placeholders = keys.map((_, index) => `$${index + 1}`).join(', ');
  const result = await execute(
    `INSERT INTO ${quoteIdentifier(table, '表名')} (${columns}) VALUES (${placeholders})`,
    keys.map((key) => data[key])
  );
  return result.lastInsertId;
}

/** 按自增主键 id 更新字段，返回受影响行数（0 = 记录不存在） */
export async function updateById(table: string, id: unknown, data: DbRecord): Promise<number> {
  const keys = Object.keys(data);
  if (keys.length === 0) {
    throw normalizeError({ code: ErrorCode.InvalidInput, message: '更新内容不能为空' });
  }
  const setClause = keys
    .map((key, index) => `${quoteIdentifier(key, '列名')} = $${index + 1}`)
    .join(', ');
  const result = await execute(
    `UPDATE ${quoteIdentifier(table, '表名')} SET ${setClause} WHERE "id" = $${keys.length + 1}`,
    [...keys.map((key) => data[key]), id]
  );
  return result.rowsAffected;
}

/** 按自增主键 id 删除，返回受影响行数（0 = 记录不存在） */
export async function deleteById(table: string, id: unknown): Promise<number> {
  return execute(`DELETE FROM ${quoteIdentifier(table, '表名')} WHERE "id" = $1`, [id]).then(
    (result) => result.rowsAffected
  );
}

/** 按等值条件删除（条件不可为空，防全表误删），返回受影响行数 */
export async function deleteWhere(table: string, where: DbRecord): Promise<number> {
  const { clause, params } = buildWhere(where);
  return execute(`DELETE FROM ${quoteIdentifier(table, '表名')} WHERE ${clause}`, params).then(
    (result) => result.rowsAffected
  );
}

/** 按自增主键 id 查询单条记录，不存在返回 null */
export async function findById<T>(table: string, id: unknown): Promise<T | null> {
  const rows = await select<T>(`SELECT * FROM ${quoteIdentifier(table, '表名')} WHERE "id" = $1`, [
    id,
  ]);
  return rows[0] ?? null;
}

/** 条件查询（等值 AND + 排序 + 分页） */
export async function findAll<T>(table: string, options: FindOptions = {}): Promise<T[]> {
  quoteIdentifier(table, '表名');
  let sql = `SELECT * FROM ${quoteIdentifier(table, '表名')}`;
  const params: unknown[] = [];

  if (options.where) {
    const { clause, params: whereParams } = buildWhere(options.where);
    sql += ` WHERE ${clause}`;
    params.push(...whereParams);
  }
  if (options.orderBy) {
    sql += ` ORDER BY ${buildOrderBy(options.orderBy)}`;
  }
  if (options.limit !== undefined) {
    assertNonNegativeInt(options.limit, 'limit');
    params.push(options.limit);
    sql += ` LIMIT $${params.length}`;
  }
  if (options.offset !== undefined) {
    assertNonNegativeInt(options.offset, 'offset');
    // SQLite 的 OFFSET 依赖 LIMIT；无 limit 时用 -1 表示不限制
    if (options.limit === undefined) {
      params.push(-1);
      sql += ` LIMIT $${params.length}`;
    }
    params.push(options.offset);
    sql += ` OFFSET $${params.length}`;
  }

  return select<T>(sql, params);
}

/** 等值条件计数（可省略条件统计全表） */
export async function count(table: string, where?: DbRecord): Promise<number> {
  quoteIdentifier(table, '表名');
  let sql = `SELECT COUNT(*) AS n FROM ${quoteIdentifier(table, '表名')}`;
  const params: unknown[] = [];
  if (where) {
    const { clause, params: whereParams } = buildWhere(where);
    sql += ` WHERE ${clause}`;
    params.push(...whereParams);
  }
  const rows = await select<{ n: number }>(sql, params);
  return rows[0]?.n ?? 0;
}

export const db = {
  // 底层：手写 SQL（复杂场景兜底）
  select,
  execute,
  runInTransaction,
  // 通用 CRUD：快速增删改查
  insert,
  updateById,
  deleteById,
  deleteWhere,
  findById,
  findAll,
  count,
} as const;
