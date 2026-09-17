# system 插件 — 开发约定（AGENTS）

> 上游内置插件：系统维护类工具的归宿。目录规范见 [`plugins/README.md`](../README.md)，UI 规范见 [`docs/design.md`](../../docs/design.md)。

## 约定

- **无后端**：不新增 `backend/`；数据访问一律通过框架 `@/core/db`（`db_query_values` / `db_execute`）。
- SQL 一律参数化（`$1`），表名/列名做标识符白名单校验（见 `frontend/shared.ts`）。
- 页面结构：模块用 `Panel` 包裹（左「表」面板 + 右「表详情」面板），动作放面板头部；
  空态/加载态用 `@/components/native/*`，搜索框用 `SearchField`。
- 危险操作（删除行/表）必须走 `AlertDialog` 且破坏性按钮用 `variant="destructive"`。
- 新增系统工具时同步 `plugin.json` 的 `tools[]` 与本文件 / `README.md` 的表格。

## 校验

```bash
pnpm lint && pnpm build && pnpm test    # 含 lint-design 与 lint-plugins
```
