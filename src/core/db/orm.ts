/**
 * Drizzle ORM 实例（sqlite-proxy 驱动适配）。
 *
 * - 查询构建在前端完成（类型安全、零 SQL 字符串），SQL 文本经 ipc 发往
 *   Rust 侧 sqlx 执行，按列序返回结果（无对象键序风险）
 * - schema 聚合于 core/db/schema.ts（各插件目录定义，此处统一挂载）
 * - 回调返回约定（由 drizzle sqlite-proxy 源码决定，勿改动）：
 *   - run   → { rows: [] }（执行语句；rowsAffected/lastInsertId 不回传，
 *             拿自增 id 请用 .returning()）
 *   - all   → { rows: 值数组[][] }（类型安全 select 主路径，
 *             drizzle 内部 mapResultRow 按字段序号映射）
 *   - values→ { rows: 值数组[][] }（.values() 原始模式）
 *   - get   → { rows: 单行值数组 | undefined }（mapGetResult 按序号映射）
 */

import { drizzle } from 'drizzle-orm/sqlite-proxy';

import { executeSql, queryValues } from './client';
import * as schema from './schema';

export const kdb = drizzle(
  async (sql, params, method) => {
    if (method === 'run') {
      await executeSql(sql, params);
      return { rows: [] };
    }

    const { rows } = await queryValues(sql, params);
    if (method === 'get') {
      return { rows: rows[0] };
    }
    return { rows };
  },
  { schema }
);

export { schema };
