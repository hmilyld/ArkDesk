# 桌面化设计改造 · 推进计划与交接

> 这份文件是**可续接的工作交接**（fork 专属，勿带上游）。新会话从这里接手即可，不必回看历史对话。
> 规范正文在 `DESIGN.md` / `DESIGN-macos.md` / `DESIGN-windows.md` / `DESIGN-appendix.md`（上游框架文档）。

## 0. 一句话现状

- **规范已定稿**（4 份文档）→ **框架层已合并进上游 main**（PR #5 `a8a792e`，走查工具修复 PR #6 `04e64fb`）
  → **插件层全部批次已迁移完成**（daily-tools / text2video / system / hello-world / network-tools / tender-optimizer）。
- 分支：`design/plugin-conformance`（`upstream/main` + `plugins/` + `tests/` + 5 份 fork 根文档），已 push origin。
- 设计 lint：**插件层 0 违规**；`src/` 尚有 **6 处上游存量告警**（见 §3 末尾），属上游告警模式、不阻塞 CI。
- 走查已用预览桥 + `scripts/design-shot.mjs` 自证（暗/亮、13/14/15 字号、win 平台、1080/1512 宽度）。

## 1. 上游（PocketArk）状态

- 本地：`../PocketArk`；`main` 已含两次合并：`a8a792e`（PR #5 设计规范/ token/组件/壳层）、`04e64fb`（PR #6 走查工具修复）。
- PR #5 的分层提交（实际 hash，文件集互不重叠）：

  | commit    | 层  | 内容                                                                                                                                         |
  | --------- | --- | -------------------------------------------------------------------------------------------------------------------------------------------- |
  | `53b2dbd` | P0  | 4 份 `DESIGN*.md` + `AGENTS.md` 两条约定                                                                                                     |
  | `f32cd6d` | P1  | `src/assets/index.css` 角色化 token + `scripts/lint-design.mjs` + `package.json` 接入                                                        |
  | `afcbfeb` | P2  | `src/components/native/*`（三态/分段/表单行/列表行/搜索框）+ `src/components/ui/*` 焦点环与浮层圆角                                          |
  | `f00770e` | P3  | 壳层（ToolShell 材质、`index.html` 平台标记、`src/dev/preview-bridge.ts`、`scripts/design-shot.mjs`、`src/layouts/*`）+ Rust `windowEffects` |

- PR #6（`fix/preview-bridge-wiring`，已合并）修了三处「合了文件但用不了」的问题：
  1. `src/main.ts` 从未 import `preview-bridge` → 浏览器预览直接崩（`transformCallback` / store `invoke` 失败）；
  2. `design-shot` 把 `?theme=&font=…` 拼在 hash 之后 → hash 路由下 `location.search` 为空，参数全部失效；
  3. 预览桥硬编码 fork 的 `arkdesk.*` 存储键 → 改为从 `src/core/theme` 导出的 `STORAGE_KEYS` 读取。

## 2. 已完成的插件批次（fork 层，均已提交）

| 批次             | 范围                                                 | 关键改动                                                                                                                                                     |
| ---------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| daily-tools      | crypto 4 面板 / ImageOcr / FileConverter / JsonTable | `Panel` 化、`Segmented` 替换手搓分段、设置行分组、选项区 HIG 化、`EmptyState`                                                                                |
| text2video       | Generator / Drafts / History                         | Generator 四段 Panel + FormRow + Segmented + ListRow；Drafts 单 Panel + 分隔线列表；History 状态语义色                                                       |
| system           | DataMaintenance / TableList                          | 左右两块 Panel 化、`SearchField`、`EmptyState`、DDL 去重复边框                                                                                               |
| hello-world      | Tool / TemplateTool                                  | 三态门面组件作为活样板                                                                                                                                       |
| network-tools    | HttpClient / Interceptor + 12 个组件                 | 分栏 pane 一律 `Panel`（保留 sticky/高度/滚动）、控制台前景 `text-console-foreground`、`SearchField`、`Segmented`、就地错误条、空态 `EmptyState`、行键盘可达 |
| tender-optimizer | Calculator / History                                 | 四段表单 Panel + `Segmented`/`Checkbox`/`FormRow`、结果 inset 块 + 粘性表头对比表、就地校验 + 可重试错误、空/载态标准件                                      |

> 提交：`1e0f79a`（network-tools）、`4823932`（tender-optimizer）。

## 3. 分支收尾（框架 vs 插件分离）

已执行（原计划是逐个 cherry-pick，实际用**按路径取**更稳，避免手挑 commit 漏文件）：

```bash
cd ../ArkDesk
git fetch upstream
git checkout -b design/plugin-conformance upstream/main
git checkout design/desktop-proto -- plugins tests          # 插件层 + 单测整体取回
git checkout design/desktop-proto -- README.md AGENTS.md START.md LOCAL.md MIGRATION.md   # 5 份 fork 根文档
git push -u origin design/plugin-conformance
```

> 判断标准：**只含 `plugins/**` 的提交**留 fork；含 `src/**`、根文档、`scripts/*`、`src-tauri/**` 的提交由上游承载。
> 框架层提交在 fork 里**不保留**（否则与上游分叉）。
> 走查脚本引用同步收敛为上游路径 `scripts/design-shot.mjs`（`scripts/local/` 那份副本已不再被引用，本分支亦无该目录）。

**该分支不含本地层**（`package.json` fork 脚本与依赖、`src-tauri/Cargo.toml` local deps、
`src-tauri/local-resources/`、`scripts/local/`、`capabilities/local.json`、`tauri.conf.json` 的 `arkdesk` 配置）——
本分支用于「插件层 vs 新框架」的一致性验证（`pnpm lint && pnpm build && pnpm test` 可跑）；
**要真机运行 `pnpm tauri dev`，合回 `main` 时须带上本地层**。

**上游剩余 6 处存量告警**（全在 `src/`，属上游告警模式）：`SettingsSection.vue:40` / `AboutSettings.vue:44`（R1）、
`Home.vue:102`（R3）与 `:173`（R6）、`SideNav.vue:49`（R4）、`UpdateDialog.vue:82`（R2）。
fork 侧若要恢复「设计 lint 0 违规」，需按上游优先流程再提一个收敛 PR。

## 4. 上游 PR 要点（已成文，供后续 PR 复用）

- 目标：把「桌面优先」的 UI 规范与实现落到框架层（macOS HIG 为主 + JetBrains 为辅，Windows 用 Fluent 平台层）。
- 机器门禁：`scripts/lint-design.mjs`（R1~R6），`pnpm lint` 即门禁，`pnpm lint:design` 为严格模式。
- 平台差异只允许落在 6 处：材质与回退、窗口壳、菜单与快捷键呈现、对话框按钮语义、焦点视觉、圆角档。
- Windows 平台层**未做视觉验证**（开发机 macOS），已在 `DESIGN-windows.md` 标注并与 §11 待办逐项对应。
- 走查工具：`src/dev/preview-bridge.ts` + `scripts/design-shot.mjs`（浏览器内渲染 + 可交互截图）。
- **注意**：PocketArk 的 `index.html` / `SideNav.vue` / `AboutSettings.vue` 与 fork 有分歧，已按**最小改动**应用；
  不要把 fork 的 `ArkDesk` 标题与 `arkdesk.*` 存储键带上游（PR #6 已把预览桥的键改为从 `theme` 导出）。

## 5. 剩余插件批次

无。两个批次已完成，遗留项（非阻塞）：

1. **network-tools**：`Interceptor` 的「启动/停止/系统代理/CA」仍用 toast 表达结果（系统级操作、无就地位置）；
   `HttpClient` 的「不能移动到自身或其子级」受侧栏组件契约限制，暂留 toast。
2. **tender-optimizer**：蒙特卡洛测算后端无 channel/task 参数，**无进度与取消**（后端改动属插件层，可单独排期）；
   「评分参数」面板在预览夹具下 `paramsRow` 为空 → 面板只有头部（真机迁移始终会插入该行，不影响）。
3. 预览桥对业务命令返回 `null`，因此 `useInterceptor` 的列表命令加了 `?? []` 容错（见 §7）。

## 6. 走查与自证（无需人工截图）

```bash
npx vite                     # 端口 1420
node scripts/design-shot.mjs --url "/tool/http-client" --out /tmp/a.png \
  --theme dark --font 14 [--platform win] [--eval "…点击/输入…"]
```

> ⚠️ URL 路由 id 是**全局工具 id**（`/tool/<tool-id>`），不是 `<插件>-<工具>`：
> `http-client` / `http-interceptor` / `tender-optimizer` / `tender-history` / `text2video-drafts`。
> 参数必须放在 hash 之前（`?theme=…` 在 `#` 前），PR #6 已修。

- 预览桥 `src/dev/preview-bridge.ts`：仅 dev + 非 Tauri 生效；localStorage 版 `plugin-store` + 业务命令夹具
  （`db_query_values` 按 SQL 形态、`text2video_*` 列表夹具）；URL 参数 `?theme=&font=&accent=&platform=`。
- 本次已核对：`http-client`（暗/亮、1080/1512、win）、`http-interceptor`、`tender-optimizer`（暗/亮 13）、`tender-history`。
- 原生窗口壳 / 材质（vibrancy、Mica）仍需真机确认，浏览器预览只覆盖 Web 层。

## 7. 硬约束（勿回退）

- 视觉取值只能来自 token；fork 侧设计 lint 0 违规是提交前提（当前仅 `src/` 6 处上游存量）；上游侧为告警模式不阻塞 CI。
- **Panel 内不叠卡片**；子分组只用分隔线 + 小标题；分栏 pane 用 `Panel`（满幅内容 `body-class="p-0"`）且**保留 sticky/高度/滚动意图**。
- 控制台输出框：`bg-console` 必须配 `text-console-foreground*`。
- 开关只出现在列表行内、与标题同区；细粒度布尔用复选框（HIG 分工）。
- 焦点环唯一写法：`focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60`。
- 工具栏保持流程内（**不要**改回「绝对定位覆盖 + main 顶部内边距」，会产生双重上边距）。
- reka-ui 2.10：`data-active:` / `data-inactive:` 变体无效，一律用 `data-[state=…]:`；
  `TabsList` 横向高度被 group 变体 `h-9` 钉死，需内联样式覆盖。
- 预览桥对未知业务命令返回 `null`（非数组）；插件 composable 消费列表命令时用 `?? []` 容错，
  否则浏览器预览会白屏（真机后端始终返回数组，不影响线上行为）。
