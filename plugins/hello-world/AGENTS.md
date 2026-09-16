# hello-world 插件 — 开发约定（AGENTS）

> 示例/样板插件，演示框架全链路。项目级文档只做引用。

## 用途

一个插件内声明多个工具（`plugin.json` 的 `tools[]`），覆盖各框架能力：
IPC、SQLite CRUD、HTTP、后台任务与通知、多窗口/系统集成、页面模板等。

## 后端

- 命令写在 `backend/mod.rs`，一律 `hello_world_*` 前缀，返回 `Result<T, AppError>`。
- 表 `hello_tasks` 由 `backend/migrations.rs` 建立（作用域 = `hello-world`）；
  DDL 列默认值需与 `frontend/schema.ts` 对齐。
- 后台任务示例：`hello_world_start_task`（`crate::tasks` 进度 + 取消）。

## 前端

- `views/*.vue` 每个工具一页；`frontend/schema.ts` 为表对象（自动聚合迁移）；
  `frontend/shared.ts` 类型/常量；`settings/Settings.vue` 设置；`setup.ts` 生命周期。
- 作为样板：新插件可直接参考本目录各工具页的写法。

## 校验

```bash
pnpm lint && pnpm build
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs
```
