# 桌面化设计改造 · 推进计划与交接

> 这份文件是**可续接的工作交接**（fork 专属，勿带上游）。新会话从这里接手即可，不必回看历史对话。
> 规范正文在 `DESIGN.md` / `DESIGN-macos.md` / `DESIGN-windows.md` / `DESIGN-appendix.md`（上游框架文档）。

## 0. 一句话现状

**改造已完成并合入 `main`。** 上游框架层分 5 个 PR 全部落地（规范 → token/组件 → 壳层材质 → lint 清零 → 窗口材质），
fork 的 `main` = **上游框架 + 本地层 + 插件层（全部批次）**，状态：

- `pnpm lint`：**设计 lint 0 违规**（`src/` 与 `plugins/` 全清零）。
- `pnpm build` / `pnpm test`：全绿（vitest 71 通过）。
- `cargo check`（含本地层 ocr-rs / hudsucker 等重依赖）：通过。
- 走查已用预览桥 + `scripts/design-shot.mjs` 自证（暗/亮、13/14/15 字号、win 平台、1080/1512 宽度）。
- **待真机确认**：macOS vibrancy / Windows Mica Alt 的材质观感（浏览器预览只覆盖 Web 层）。

## 1. 上游（PocketArk）状态（全部已合并）

| PR  | squash    | 内容                                                                                                                                                                                                                     |
| --- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| #5  | `a8a792e` | 4 份 `DESIGN*.md` + `AGENTS.md` 约定；token 层（`src/assets/index.css`）+ `scripts/lint-design.mjs`；`src/components/native/*` 门面组件 + `ui/*` 焦点环/浮层圆角；`preview-bridge.ts` + `design-shot.mjs`                |
| #6  | `04e64fb` | 走查工具修复：`main.ts` 接入预览桥（此前是死代码）、`design-shot` 参数移到 hash 之前、主题键改从 `core/theme` 的 `STORAGE_KEYS` 读取                                                                                     |
| #7  | `8421592` | 框架层设计 lint 存量清零（R1/R2/R3/R4/R6；SideNav rail 固定 px 登记 `design-lint-ignore`）                                                                                                                               |
| #8  | `de687e2` | 壳层材质与设置页 pane：ToolShell/TitleBar 材质底、设置页横向 pane 条、主题三态改 `Segmented`、Toaster 位置、ToolErrorBoundary 走 `normalizeError`                                                                        |
| #9  | `8021585` | 应用窗口材质（Rust）：macOS `underWindowBackground` / Windows Mica Alt + `VIBRANCY_ACTIVE` 防实色打底                                                                                                                    |
| #10 | `eb1b9ba` | 清理 base 里的 fork 残留：UA 品牌硬编码改 `CARGO_PKG_NAME` 派生；预览桥移除 fork 插件夹具、改为**插件自带夹具扩展点**；`DESIGN-appendix` 文档纠偏；补齐 P4-1/P4-2（hello-world / system），**base lint 严格模式 0 违规** |

> #5 的 P3 层实际只上游了 `SideNav` 的一部分；其余壳层与 Rust 材质由 #8 / #9 补齐（这是本轮新增的三个 PR 的原因）。

## 2. 已完成的插件批次（fork 层）

| 批次             | 范围                                                 | 关键改动                                                                                                                                                     |
| ---------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| daily-tools      | crypto 4 面板 / ImageOcr / FileConverter / JsonTable | `Panel` 化、`Segmented` 替换手搓分段、设置行分组、选项区 HIG 化、`EmptyState`                                                                                |
| text2video       | Generator / Drafts / History                         | Generator 四段 Panel + FormRow + Segmented + ListRow；Drafts 单 Panel + 分隔线列表；History 状态语义色                                                       |
| system           | DataMaintenance / TableList                          | 左右两块 Panel 化、`SearchField`、`EmptyState`、DDL 去重复边框                                                                                               |
| hello-world      | Tool / TemplateTool                                  | 三态门面组件作为活样板                                                                                                                                       |
| network-tools    | HttpClient / Interceptor + 12 个组件                 | 分栏 pane 一律 `Panel`（保留 sticky/高度/滚动）、控制台前景 `text-console-foreground`、`SearchField`、`Segmented`、就地错误条、空态 `EmptyState`、行键盘可达 |
| tender-optimizer | Calculator / History                                 | 四段表单 Panel + `Segmented`/`Checkbox`/`FormRow`、结果 inset 块 + 粘性表头对比表、就地校验 + 可重试错误、空/载态标准件                                      |

## 3. 分支与整合方式

`main` 的整合口径（可重复执行，用于以后再同步上游）：

```bash
cd ../ArkDesk
git fetch upstream && git merge upstream/main          # 框架层随上游流入（本轮仅在 theme 键处有 1 处冲突）
git checkout design/plugin-conformance -- plugins tests README.md AGENTS.md START.md LOCAL.md MIGRATION.md
pnpm lint && pnpm build && pnpm test && cargo check --manifest-path src-tauri/Cargo.toml
```

> 判断标准：**只含 `plugins/**` 的提交**留 fork；含 `src/**`、根文档、`scripts/*`、`src-tauri/**` 的提交由上游承载。
> 框架层改动**一律不在 fork 就地改**（否则与上游分叉、merge 必冲突）。
> 冲突预案：`src/core/theme/index.ts` 保留 fork 的 `arkdesk.*` 键值，但采用上游的 `STORAGE_KEYS` 导出结构（预览桥依赖它）。

分支现状：

- `main`：**唯一长留分支**（上游框架 + 本地层 + 插件层）。
- `design/plugin-conformance`：过渡分支（`upstream/main` + 插件层 + fork 文档），已并入 `main`，保留作对照，可删。
- `design/desktop-proto`：旧原型分支（含已被上游吸收的框架层提交），已完成使命，可删。

## 4. 上游 PR 要点（后续提 PR 复用）

- 目标：把「桌面优先」的 UI 规范与实现落到框架层（macOS HIG 为主 + JetBrains 为辅，Windows 用 Fluent 平台层）。
- 机器门禁：`scripts/lint-design.mjs`（R1~R6），`pnpm lint` 即门禁，`pnpm lint:design` 为严格模式。
- 平台差异只允许落在 6 处：材质与回退、窗口壳、菜单与快捷键呈现、对话框按钮语义、焦点视觉、圆角档。
- Windows 平台层**未做视觉验证**（开发机 macOS），已在 `DESIGN-windows.md` 标注并与 §11 待办逐项对应。
- **不要把 fork 品牌带上游**：`ArkDesk` 标题、`arkdesk.*` 存储键、`arkdesk-backup.db`、更新源地址、本地层依赖/资源/脚本。

## 5. 剩余工作与遗留

无剩余批次（base 的 P4-1/P4-2 已随上游 #10 完成，双方 lint 严格模式均 0 违规）。遗留项（非阻塞，按需排期）：

1. **network-tools**：启动/停止/系统代理/CA 仍用 toast 表达结果（系统级操作、无就地位置）；`HttpClient` 的「不能移动到自身或其子级」受侧栏组件契约限制暂留 toast。
2. **tender-optimizer**：蒙特卡洛测算后端无 channel/task 参数 → **无进度与取消**（后端改动属插件层，可单独排期）；「评分参数」面板在预览夹具下 `paramsRow` 为空 → 只有头部（真机迁移始终会插入该行）。
3. **真机验证**：macOS vibrancy（窗口失焦/聚焦的 chrome 变化）与 Windows Mica Alt 需在真机按 `DESIGN-macos.md` / `DESIGN-windows.md` §11 逐项确认并回填。

## 6. 走查与自证（无需人工截图）

```bash
npx vite                     # 端口 1420
node scripts/design-shot.mjs --url "/tool/http-client" --out /tmp/a.png \
  --theme dark --font 14 [--platform win] [--eval "…点击/输入…"]
```

> ⚠️ URL 路由 id 是**全局工具 id**（`/tool/<tool-id>`），不是 `<插件>-<工具>`：
> `http-client` / `http-interceptor` / `tender-optimizer` / `tender-history` / `text2video-drafts`。
> 参数必须放在 hash 之前（`?theme=…` 在 `#` 前），否则 `location.search` 为空、参数不生效。

- 预览桥 `src/dev/preview-bridge.ts`：仅 dev + 非 Tauri 生效；localStorage 版 `plugin-store` + 框架命令夹具
  （`db_query_values` 按 SQL 形态）；URL 参数 `?theme=&font=&accent=&platform=`。
- **插件夹具属插件**：`plugins/<id>/frontend/preview.ts` 默认导出 `Record<命令名, 返回值>`，dev 下由预览桥 glob
  自动并入（base 侧已不含任何插件业务命令）。新增走查页面时先补该文件（本仓库已有 `plugins/text2video/frontend/preview.ts`）。
- 本轮已核对：`/settings`（暗/亮 + pane 条）、`http-client`（暗/亮、1080/1512、win）、`http-interceptor`、`tender-optimizer`（暗/亮 13）、`tender-history`、`text2video-drafts`（验证插件自带夹具）。

## 7. 硬约束（勿回退）

- 视觉取值只能来自 token；设计 lint 0 违规是提交前提（fork 与上游现已均为 0，唯一例外见 §5.1）。
- **Panel 内不叠卡片**；子分组只用分隔线 + 小标题；分栏 pane 用 `Panel`（满幅内容 `body-class="p-0"`）且**保留 sticky/高度/滚动意图**。
- 控制台输出框：`bg-console` 必须配 `text-console-foreground*`。
- 开关只出现在列表行内、与标题同区；细粒度布尔用复选框（HIG 分工）。
- 焦点环唯一写法：`focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60`。
- 工具栏保持流程内（**不要**改回「绝对定位覆盖 + main 顶部内边距」，会产生双重上边距）。
- reka-ui 2.10：`data-active:` / `data-inactive:` 变体无效，一律用 `data-[state=…]:`；
  `TabsList` 横向高度被 group 变体 `h-9` 钉死，需内联样式覆盖。
- 预览桥对未知业务命令返回 `null`（非数组）；插件 composable 消费列表命令时用 `?? []` 容错，
  否则浏览器预览会白屏（真机后端始终返回数组，不影响线上行为）。
- 窗口材质：启用后 `set_window_background` 不做实色打底（否则盖住材质），防白闪靠前端不透明底；
  勿删 `VIBRANCY_ACTIVE` 的提前返回（`src-tauri/src/lib.rs`）。
- 插件预览夹具只放 `plugins/<id>/frontend/preview.ts`，**不要**再把插件命令写进 `src/dev/preview-bridge.ts`（base 不接受 fork 插件名）。
