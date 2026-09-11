/**
 * 数据库 schema 聚合（自动注册，无需手动维护）。
 *
 * 约定：插件有数据库表时，在 `plugins/<id>/frontend/schema.ts` 导出 sqliteTable 定义
 * （参考 plugins/hello-world/frontend/schema.ts），本模块经 Vite 构建期扫描自动聚合。
 *
 * 说明：
 * - 表对象的类型推断发生在各插件 schema.ts（import 表对象即得完整类型），
 *   此处运行时聚合仅服务 kdb 的 schema 注册（关系 API / 内省）
 * - Rust 侧迁移在插件 `backend/migrations.rs` 声明（构建期自动聚合，见 README「需要数据库表」）
 */

const schemaModules = import.meta.glob<Record<string, unknown>>('/plugins/*/frontend/schema.ts', {
  eager: true,
});

const collected: Record<string, unknown> = {};
for (const [file, mod] of Object.entries(schemaModules)) {
  for (const [name, exported] of Object.entries(mod)) {
    if (name in collected) {
      // 重复表名会让后者静默覆盖前者，开发期显式暴露
      console.warn(`[db/schema] 表「${name}」重复定义（${file}），已忽略后者`);
      continue;
    }
    collected[name] = exported;
  }
}

export const schema = collected;
