/**
 * text2video 数据库 schema（与 Rust 迁移 v8/v9 对齐）。
 */

import { sql } from 'drizzle-orm';
import { integer, sqliteTable, text } from 'drizzle-orm/sqlite-core';

export const text2videoProcessed = sqliteTable('text2video_processed', {
  refId: text('ref_id').primaryKey(),
  kind: text('kind'),
  title: text('title'),
  status: text('status'),
  detail: text('detail'),
  video: text('video'),
  meta: text('meta'),
  createdAt: text('created_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
});

export const text2videoDrafts = sqliteTable('text2video_drafts', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  title: text('title').notNull(),
  author: text('author').notNull().default(''),
  content: text('content').notNull(),
  source: text('source').notNull().default('manual'),
  createdAt: text('created_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
  updatedAt: text('updated_at')
    .notNull()
    .default(sql`(datetime('now', 'localtime'))`),
});

export type Text2VideoProcessed = typeof text2videoProcessed.$inferSelect;
export type Text2VideoDraft = typeof text2videoDrafts.$inferSelect;
