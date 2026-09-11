/**
 * 可选：数据库表定义（Drizzle schema）。
 *
 * 使用步骤：
 * 1. 在此定义 sqliteTable（参考 plugins/hello-world/frontend/schema.ts）
 * 2. 行类型用 $inferSelect 导出，勿手写 interface
 * 3. 在 backend/migrations.rs 追加迁移（scope = 插件 id，version 作用域内递增），启动时自动执行
 * 无数据库需求的插件可删除本文件。
 */
