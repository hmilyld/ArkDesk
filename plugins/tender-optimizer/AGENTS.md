# tender-optimizer 插件 — 开发约定（AGENTS）

> 功能说明见同目录 `README.md`；项目级文档只做引用。

## 后端

- 命令写在 `backend/mod.rs`，一律 `tender_optimizer_*` 前缀，返回 `Result<T, AppError>`。
- 子模块：`models.rs`（DTO/表模型）、`engine.rs`（蒙特卡洛测算）、`excel.rs`（模板导入导出）、
  `migrations.rs`（作用域 = `tender-optimizer`）。
- 模板 `backend/template.xlsx` 随插件分发，经 `include_bytes!` 读取。
- 测算需**可复现**：随机种子由输入决定，勿引入全局随机源。

## 前端

- `views/Calculator.vue`（测算）、`views/History.vue`（历史）。
- 共享类型/默认值在 `shared.ts`；数据库表在 `schema.ts`；启动钩子在 `setup.ts`。
- 命令参数与返回字段：Tauri v2 命令参数为 camelCase；返回结构体字段保持声明时的命名
  （本项目多为 snake_case，前端按对应命名读取）。

## 校验

```bash
pnpm lint && pnpm build
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs
```
