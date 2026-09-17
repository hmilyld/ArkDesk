# hello-world 插件 — 开发约定（AGENTS）

> 上游基础仓库的**参考实现**：新增能力（命令、迁移、表单、任务、通知、多窗口）时优先在此演示，
> 让 fork 与文档有可对照的样本。目录规范见 [`plugins/README.md`](../README.md)，UI 规范见 [`docs/design.md`](../../docs/design.md)。

## 约定

- 命令写在 `backend/mod.rs`（`hello_world_` 前缀，返回 `Result<T, AppError>`）；逻辑与测试放子模块。
- 迁移写在 `backend/migrations.rs`，scope 固定 `hello-world`；已发布迁移不可改，只能追加。
- 前端：页面用 `@/components/tool/ToolShell` + `Panel`；设置项用 `@/components/settings` 的
  `SettingsSection` / `SettingsRow` / `SettingsField`；三态用 `@/components/native/*`。
- 纯逻辑放 `frontend/lib/`（本插件暂无），共享常量/类型放 `frontend/shared.ts`。
- 新增工具时同步：`plugin.json` 的 `tools[]`、对应 `frontend/views/*.vue`、本文件与 `README.md` 的表格。

## 校验

```bash
pnpm lint && pnpm build && pnpm test    # 含 lint-design 与 lint-plugins
```
