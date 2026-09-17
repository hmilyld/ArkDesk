# 模板插件 — 开发约定（AGENTS）

> 本插件 id 为 template。这是**上游模板**（目录以 `_` 开头，不参与注册），`pnpm create-plugin` 会复制本目录生成新插件。
> 面向使用者的功能说明见同目录 `README.md`；目录与命名规范见 [`plugins/README.md`](../README.md)；
> UI 规范见 [`docs/design.md`](../../docs/design.md)。

## 工具

`plugin.json` 声明工具 **模板工具**（`frontend/views/Tool.vue`）。多功能插件在同一 `tools[]`
数组里追加条目，并各自提供 `frontend/views/<Xxx>.vue`。

## 结构

```
plugin.json               清单（唯一事实源）
README.md / AGENTS.md     功能说明 / 本文件
frontend/
  views/Tool.vue         工具页面（Panel + 三态；页面结构约定见文件头注释）
  settings/Settings.vue  设置面板（可选，useToolSettings 持久化）
  components/            私有组件（本目录暂无；同域 ≥2 个时建子目录）
  composables/           组合式函数（useXxx.ts，如 useCounter）
  lib/                   纯逻辑 TS（无 Vue 依赖，如 example.ts）
  shared.ts              共享常量/类型
  schema.ts              Drizzle 表定义（可选）
  setup.ts               生命周期钩子（可选）
backend/
  mod.rs                 `#[tauri::command]` 命令（`模板插件_` 前缀，构建期自动登记）
  migrations.rs          迁移聚合（可选）
```

## 约定

- 命令只写在 `backend/mod.rs`，一律返回 `Result<T, AppError>`，函数名 = 前端调用名（`<id>_` 前缀）。
- 迁移写在 `backend/migrations.rs`，scope = 插件 id，version 在作用域内从 1 递增；已发布的只能追加。
- 纯逻辑放 `frontend/lib/`；需要响应式/生命周期才放 `frontend/composables/`（`useXxx.ts`）。
- 页面用 `ToolShell` + `Panel` 组织模块；空/加载/错误三态用 `@/components/native/*`；
  设置项用 `@/components/settings` 的 `SettingsSection` / `SettingsRow` / `SettingsField`。
- 取值只来自 token（详见 `docs/design.md`）；插件私有文案放 `frontend/shared.ts`。

## 校验

```bash
pnpm lint && pnpm build && pnpm test   # 含 lint-design 与 lint-plugins（目录/命名规范）
```
