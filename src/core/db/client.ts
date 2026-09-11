/**
 * SQL 执行通道：Rust 侧自建 sqlx 池的 ipc 封装。
 *
 * - db_query_values：按列序返回 {columns, rows[][]}（Drizzle proxy 直接消费）
 * - db_execute：返回 {rowsAffected, lastInsertId}
 * - rowsToObjects：列名 + 值数组 → 对象数组（手写 SQL 场景）
 */

import { ipc } from '@/core/ipc';

export interface ValuesResult {
  columns: string[];
  rows: unknown[][];
}

export interface ExecuteResult {
  rowsAffected: number;
  lastInsertId: number;
}

export async function queryValues(sql: string, params: unknown[] = []): Promise<ValuesResult> {
  return ipc<ValuesResult>('db_query_values', { args: { sql, params } });
}

export async function executeSql(sql: string, params: unknown[] = []): Promise<ExecuteResult> {
  return ipc<ExecuteResult>('db_execute', { args: { sql, params } });
}

/** 事务中的单条语句 */
export interface SqlStatement {
  sql: string;
  params?: unknown[];
}

/** 在单个事务中执行多条语句（任一失败整体回滚） */
export async function executeTransaction(statements: SqlStatement[]): Promise<ExecuteResult> {
  return ipc<ExecuteResult>('db_transaction', { args: { statements } });
}

export function rowsToObjects<T = Record<string, unknown>>(
  columns: string[],
  rows: unknown[][]
): T[] {
  return rows.map((row) =>
    Object.fromEntries(columns.map((column, index) => [column, row[index] ?? null]))
  ) as T[];
}
