# _template 插件 — 开发约定（AGENTS）

> 新工具骨架。复制本目录（或用 `pnpm create-plugin`）后按本文件清单改写。
> `_` 前缀使本目录被前后端扫描跳过，不会被编译/注册。

## 复制后清单

1. 目录改名 `plugins/<plugin-id>`，`plugin.json` 的 `id/name/group/tools[].entry` 同步。
2. 后端命令**必须**在 `backend/mod.rs`，函数名 = 前端调用名，带 `<plugin_id>_` 前缀，
   一律返回 `Result<T, AppError>`；多余逻辑放 `backend/` 子模块（`pub mod xxx;`）。
3. 把模板里的 `template_` 前缀改为新插件 id 前缀（避免命令重名，`build.rs` 会构建期查重）。
4. 前端入口 `frontend/views/*.vue`；可选 `schema.ts`（表对象，自动聚合迁移）、
   `setup.ts`（生命周期）、`settings/`、`shared.ts`。
5. 有数据库表时：`backend/migrations.rs` 导出 `all()`，`migration(scope=<plugin-id>, version, ...)`，
   version 在作用域内从 1 递增；**已发布迁移不可改，只能追加**。
6. 旧库桥接（曾用旧全局版本号）：`plugin.json` 声明 `legacyMigrations`。

## 校验

```bash
pnpm lint && pnpm build
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs
```
