# 插件开发总览

> 面向插件作者与 AI 的唯一入口文档。**目录与命名规范**在此定义；扩展机制、命令/迁移约定见根
> [`AGENTS.md`](../AGENTS.md) 的「插件扩展」，UI 规范见 [`docs/design.md`](../docs/design.md) 及平台文档。

## 1. 目录规范

```
plugins/<plugin-id>/
├── plugin.json            清单：唯一事实源（前后端扫描共用）
├── README.md              功能说明（面向使用者）
├── AGENTS.md              开发约定（面向 AI/维护者，写了才有）
├── frontend/
│   ├── views/             工具页面：每个 tools[] 项一个 PascalCase.vue
│   ├── settings/          设置面板（可选，plugin.json 声明）
│   ├── components/        插件私有组件（可选；同一功能域 ≥2 个 → components/<feature>/）
│   ├── composables/       组合式函数（可选，useXxx.ts）
│   ├── lib/               纯逻辑 TS（可选，无 Vue 依赖）：lib/<name>.ts 或 lib/<feature>/*.ts
│   ├── shared.ts          插件内共享常量/类型（可选）
│   ├── schema.ts          Drizzle 表定义（可选，被代码 import）
│   └── setup.ts           生命周期钩子（可选，构建期扫描）
└── backend/
    ├── mod.rs             #[tauri::command] 命令（**只能写在这里**）
    ├── migrations.rs      迁移聚合（导出 `all()`）
    ├── <feature>/         多文件功能域（≥2 文件）：<feature>/mod.rs + 其余 .rs
    ├── <single>.rs        单文件功能域
    └── <asset>            资源文件（建议随其功能域目录，如模板 xlsx）
```

**硬规则**

- `frontend/` 顶层**只允许**白名单：`views/ settings/ components/ composables/ lib/ shared.ts schema.ts setup.ts`。
  其他内容一律放进上述目录或 `lib/`。
- 纯逻辑（无 Vue 依赖）放 `frontend/lib/`；需要响应式/生命周期才放 `frontend/composables/`。
- 组件 `PascalCase.vue`；composable `useXxx.ts`；其余 TS 用 `kebab-case.ts`；Rust 文件 `snake_case.rs`；
  功能域目录用单数小写名词（`crypto/ table/ xlsx/ intercept/ render/`）。
- 命令只写在 `backend/mod.rs`（`build.rs` 只扫描该文件），其余 `.rs` 作为它的子模块。
- 纯前端插件（无数据库、无命令）允许只有 `frontend/`，但 `plugin.json` / `README.md` / `AGENTS.md` 仍必需。
- `components/README.md` 之类的目录占位文件不需要：说明集中在本文件与各插件 `AGENTS.md`。

**机器校验**：`scripts/lint-plugins.mjs`（R-P1~R-P6）已接入 `pnpm lint`；违规需修，确需例外时在文件内写
`lint-plugins-ignore` 并注明理由。`plugins/_template/` 同样受校验，确保脚手架产物即规范样本。

## 2. 新增插件

```bash
pnpm create-plugin        # 交互式：id / 显示名 / 图标 / 主题色 / 是否带数据库 / 生命周期钩子
```

脚手架按本规范生成骨架（含 `views/`、`settings/`、`components/`、`composables/`、`lib/`、`shared.ts`），
生成后即可运行；随后：

1. 改 `plugin.json`（id、显示名、图标、`tools[].entry`）——**唯一事实源**；
2. 实现 `frontend/views/*.vue`（UI 规范见 `docs/design.md`，模块用 `@/components/tool/Panel` 包裹）；
3. 需要后端时在 `backend/mod.rs` 加 `#[tauri::command]`（`<plugin_id>_` 前缀，返回 `Result<T, AppError>`）；
4. 需要落库时在 `backend/migrations.rs` 追加 `migration(scope, version, …)`，并在 `frontend/schema.ts` 定义表；
5. 补 `README.md`（功能）与 `AGENTS.md`（开发约定）。

## 3. 构建期注册（无需手动登记）

| 扫描点   | 位置                                | 说明                                           |
| -------- | ----------------------------------- | ---------------------------------------------- |
| 清单     | `plugins/*/plugin.json`             | 工具/设置/图标/排序                            |
| 工具页   | `plugins/*/frontend/views/*.vue`    | `manifest.tools[].entry` 指向                  |
| 设置面板 | `plugins/*/frontend/settings/*.vue` | `manifest.settings.entry` 指向                 |
| 生命周期 | `plugins/*/frontend/setup.ts`       | 插件初始化                                     |
| 后端命令 | `plugins/*/backend/mod.rs`          | `build.rs` 解析 `#[tauri::command]` 并生成注册 |
| 迁移     | `plugins/*/backend/migrations.rs`   | 聚合为 `all()`，按 scope 版本化                |

`_` 或 `.` 开头的目录不参与注册（如 `_template/`）。

## 4. 参考实现

- `plugins/_template/`：最小骨架（脚手架来源），演示目录规范与页面结构。
- `plugins/hello-world/`：完整链路演示（命令、日志、数据库、表单、表格、任务、多窗口）。
- `plugins/system/`：纯前端插件示例（无 backend，仅表格与对话框）。
- 与本规范配套的 UI 规范见 [`docs/design.md`](../docs/design.md)；视觉走查可用
  `src/dev/preview-bridge.ts` + `scripts/design-shot.mjs`（浏览器内渲染并截图）。
