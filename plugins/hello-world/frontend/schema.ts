/**
 * hello-world 数据库 schema（JPA Entity 的等价物）。
 *
 * - sqliteTable 键 = TS 字段名，值 = 列名（snake_case 与迁移定义对齐）
 * - 行类型用 $inferSelect 导出（勿再手写 interface）
 */

import { integer, sqliteTable, text } from 'drizzle-orm/sqlite-core';
import { sql } from 'drizzle-orm';

export const helloNotes = sqliteTable('hello_notes', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  content: text('content').notNull(),
  // default 与迁移 DDL 对齐：插入省略该列时由 SQLite 填充
  createdAt: text('created_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
});

export const helloTasks = sqliteTable('hello_tasks', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  title: text('title').notNull(),
  status: text('status').notNull().default('pending'),
  priority: text('priority').notNull().default('medium'),
  createdAt: text('created_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
});

export type HelloNote = typeof helloNotes.$inferSelect;
export type HelloTask = typeof helloTasks.$inferSelect;
