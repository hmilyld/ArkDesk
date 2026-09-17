# 工具插件模板

推荐用脚手架交互生成：`pnpm create-plugin`（自动替换名称/图标，并按需生成后端、设置、数据库等）。

也可手动复制本目录为 `plugins/<plugin-id>`（kebab-case，= plugin.json 的 id），然后：

## 1. 改写 plugin.json

- `id` / `tools[].id`：全局唯一（工具 id 用 `<plugin-id>` 或 `<plugin-id>-<功能>`）
- `tools[].name` / `icon`（lucide 图标名）/ `entry`（**相对插件目录**，如 `frontend/views/Tool.vue`）
- 可选字段：`description`、`group`、`order`、`keywords`、`keepAlive`、`legacyMigrations`
- 可选 `settings.entry`（如 `frontend/settings/Settings.vue`）

## 2. 实现页面

编辑 `frontend/views/Tool.vue`；多功能插件在 `tools` 数组加多项并各自提供 `frontend/views/<Xxx>.vue`。
共享常量放 `frontend/shared.ts`，私有组件放 `frontend/components/`，composable 放 `frontend/composables/`。

**页面布局约定（Tailwind 原生栅格）**：用 `@/components/tool/ToolShell.vue` 包裹内容
（props：`title` / `description`，插槽：`actions` / 默认插槽），内容区全幅铺满；
需要居中列 / 分栏时在页面内直接用 Tailwind 栅格，禁止 `mx-auto max-w-*` 居中容器：

```vue
<ToolShell title="标题">
  <div class="mx-auto grid w-full grid-cols-12">
    <div class="col-span-12 space-y-4 lg:col-start-3 lg:col-span-8">
      <!-- 居中 8 列，lg 以下自动满幅；每个模块用一个 Panel -->
      <Panel title="输入">
        <template #actions><Button variant="secondary" size="sm">选择文件</Button></template>
        …
      </Panel>
      <Panel title="输出">…</Panel>
    </div>
  </div>
</ToolShell>
```

常用档位：表格满幅（不加类）、宽内容 `lg:col-start-3 lg:col-span-8`、
表单/设置 `lg:col-start-3 lg:col-span-8` 或 `md:col-start-4 md:col-span-6`。
居中列必须用偶数跨距（12 − 跨距需为偶数）才能精确居中。

**模块面板**：页面里每个功能模块（设置 / 输入 / 输出 / 结果）都用
`@/components/tool/Panel.vue` 包裹（props：`title` / `hint` / `bodyClass`，插槽：
`actions` / `title` / 默认插槽）——外框与头部条视觉由 Panel 统一，不要再手写
`rounded-lg border bg-card` 卡片，也不要在 Panel 内嵌套卡片（预览、表格等组件自身不带外框）。
空态 / 加载态 / 错误态的标准写法参考 hello-world 的「页面模板」工具。

## 3. 可选能力

- **设置面板**：实现 `frontend/settings/Settings.vue`，plugin.json 声明 `settings.entry`，
  配置读写用 `useToolSettings(pluginId, defaults)`（参考 hello-world/settings）。
  UI 统一用 `@/components/settings` 的 `SettingsSection` / `SettingsRow` / `SettingsField`
  （骨架见模板 `frontend/settings/Settings.vue`），勿自造小节标题 / 卡片样式。
- **数据库表**：`frontend/schema.ts` 定义 sqliteTable（自动聚合进 kdb）；
  Rust 侧在 `backend/migrations.rs` 追加迁移：`migration("插件id", version, ...)`，
  **version 在插件作用域内从 1 递增**（各插件互不干扰）。若插件此前用过旧的全局版本号，
  在 plugin.json 的 `legacyMigrations` 声明「旧全局版本 → 新本地版本」映射以桥接旧库。
- **Rust 命令**：在 `backend/mod.rs` 写 `#[tauri::command] pub fn <plugin_id>_xxx(...) -> Result<T, AppError>`。
  **构建期自动扫描登记，无需手动注册**（命令须定义在 `backend/mod.rs`）。
  前端调用：`import { ipc } from "@/core/ipc"; await ipc("插件id_xxx", { 参数 })`
- **HTTP 采集**：`import { http } from "@/core/http"`（`getJson/postJson`）
- **日志**：`frontend/setup.ts` 里用 `ctx.logger`，页面里用 `@/core/logger` 的 `logger`

## 目录约定

```
plugins/<plugin-id>/
├── plugin.json        清单（唯一事实源；前后端扫描共用）
├── README.md          功能说明（面向使用者）
├── AGENTS.md          开发约定（面向 AI/维护者）
├── frontend/          前端
│   ├── views/         工具页面（每个 tools[] 项一个文件）
│   ├── settings/      设置面板（可选）
│   ├── components/    插件私有组件（可选）
│   ├── composables/   组合式函数（可选，useXxx.ts）
│   ├── lib/           纯逻辑 TS（可选，无 Vue 依赖）
│   ├── schema.ts      数据库表定义（可选，自动聚合）
│   ├── setup.ts       生命周期钩子（可选，自动扫描）
│   └── shared.ts      插件内共享常量/工具（可选）
└── backend/           后端（可选）
    ├── mod.rs         Rust 命令（#[tauri::command]）+ `pub mod migrations;`
    ├── migrations.rs  迁移定义：导出 `pub fn all() -> Vec<Migration>`（可选）
    └── *.rs           其余后端代码 / 资源
```

`_` 或 `.` 开头的目录不参与注册（本模板即 `_template`）。
SQL 一律 `$1` 参数化或走 kdb 对象化查询，禁止字符串拼接。
