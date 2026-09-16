# 桌面化设计改造 · 推进计划与交接

> 这份文件是**可续接的工作交接**（fork 专属，勿带上游）。新会话从这里接手即可，不必回看历史对话。
> 规范正文在 `DESIGN.md` / `DESIGN-macos.md` / `DESIGN-windows.md` / `DESIGN-appendix.md`（上游框架文档）。

## 0. 一句话现状

- **规范已定稿**（4 份文档）→ **框架层改动已推上游分支待合并** → **插件层已迁移 daily-tools / text2video / system / hello-world**，
  剩 `network-tools`、`tender-optimizer` 两个批次。
- 设计 lint 已**零违规且为硬门禁**；`pnpm lint` 含 `scripts/lint-design.mjs`（R1~R6）。

## 1. 上游（PocketArk）状态

- 本地：`../PocketArk`，分支 `design/desktop-spec`（**已 push**，尚未开 PR / 未跑完整 CI）。
- 分层提交（4 个，文件集互不重叠）：
  | commit    | 层  | 内容                                                                                                                                         |
  | --------- | --- | -------------------------------------------------------------------------------------------------------------------------------------------- |
  | `47a9cee` | P0  | 4 份 `DESIGN*.md` + `AGENTS.md` 两条约定                                                                                                     |
  | `4c13509` | P1  | `src/assets/index.css` 角色化 token + `scripts/lint-design.mjs` + `package.json` 接入                                                        |
  | `f583880` | P2  | `src/components/native/*`（三态/分段/表单行/列表行/搜索框）+ `src/components/ui/*` 焦点环与浮层圆角                                          |
  | `b08d000` | P3  | 壳层（ToolShell 材质、`index.html` 平台标记、`src/dev/preview-bridge.ts`、`scripts/design-shot.mjs`、`src/layouts/*`）+ Rust `windowEffects` |

### 下一步命令（严格按序）

```bash
cd ../PocketArk && git checkout design/desktop-spec
pnpm lint && pnpm build && pnpm test          # 尚未跑；vue-tsc 已过
gh pr create --base main --title "feat(design): 桌面化设计规范与 token/组件/壳层统一" --body "<见 §4 要点>"
gh pr checks --watch                          # 等 CI 绿
gh pr merge --squash --delete-branch
```

> 合并后回到 fork：
>
> ```bash
> cd ../ArkDesk && git fetch upstream && git merge upstream/main
> ```
>
> 然后**丢弃本地的框架层提交**（见 §3），只保留插件层提交。

## 2. 已完成的插件批次（fork 层，均已提交）

| 批次        | 范围                                                 | 关键改动                                                                                               |
| ----------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| daily-tools | crypto 4 面板 / ImageOcr / FileConverter / JsonTable | `Panel` 化、`Segmented` 替换手搓分段、设置行分组、选项区 HIG 化、`EmptyState`                          |
| text2video  | Generator / Drafts / History                         | Generator 四段 Panel + FormRow + Segmented + ListRow；Drafts 单 Panel + 分隔线列表；History 状态语义色 |
| system      | DataMaintenance / TableList                          | 左右两块 Panel 化、`SearchField`、`EmptyState`、DDL 去重复边框                                         |
| hello-world | Tool / TemplateTool                                  | 三态门面组件作为活样板                                                                                 |

## 3. 分支收尾（框架 vs 插件分离）

`design/desktop-proto` 上既有框架层提交（`2049436` 起）也有插件层提交。上游合并后应：

```bash
cd ../ArkDesk
git checkout -b design/plugin-conformance upstream/main
# 只挑插件层提交（P4 批次）：
git cherry-pick <各插件批次的 commit>
# 例如：117303a(JSON 表格) 072cc51(Panel 统一) 72a45bf e84dcaf 24ac7c1 d85db52 ab4a594 ef8477f 3964d6b …
git push -u origin design/plugin-conformance
```

> 判断标准：**只含 `plugins/**` 的提交**留 fork；含 `src/**`、根文档、`scripts/*`、`src-tauri/**` 的提交由上游承载。
> 框架层提交在 fork 里**不保留**（否则与上游分叉）。

## 4. 上游 PR 描述要点

- 目标：把「桌面优先」的 UI 规范与实现落到框架层（macOS HIG 为主 + JetBrains 为辅，Windows 用 Fluent 平台层）。
- 机器门禁：`scripts/lint-design.mjs`（R1~R6），`pnpm lint` 即门禁，`pnpm lint:design` 为严格模式。
- 平台差异只允许落在 6 处：材质与回退、窗口壳、菜单与快捷键呈现、对话框按钮语义、焦点视觉、圆角档。
- Windows 平台层**未做视觉验证**（开发机 macOS），已在 `DESIGN-windows.md` 标注并与 §11 待办逐项对应。
- 走查工具：`src/dev/preview-bridge.ts` + `scripts/design-shot.mjs`（浏览器内渲染 + 可交互截图）。
- **注意**：PocketArk 的 `index.html` / `SideNav.vue` / `AboutSettings.vue` 与 fork 有分歧，已按**最小改动**应用，
  不要把 fork 的 `ArkDesk` 标题与 `arkdesk.*` 存储键带上游。

## 5. 剩余插件批次

1. **network-tools**（14 个组件）：请求/响应为 sticky 分栏 pane、控制台输出框；需先判定「模块面板 vs 分栏 pane」再动，
   属高风险批次。检查点：`bg-console` 前景色用 `text-console-foreground*`、分栏容器用 `Panel` + `body-class="p-0"`。
2. **tender-optimizer**（2 个视图）：Calculator 大表单 + 结果表格；已有 AlertDialog（导入确认已改破坏色），
   需把工具栏式头部与表单/表格对齐规范。

## 6. 走查与自证（无需人工截图）

```bash
npx vite                     # 端口 1420
node scripts/design-shot.mjs --url "/tool/text2video-drafts" --out /tmp/a.png \
  --theme dark --font 14 [--platform win] [--eval "…点击/输入…"]
```

- 预览桥 `src/dev/preview-bridge.ts`：仅 dev + 非 Tauri 生效；localStorage 版 `plugin-store` + 业务命令夹具
  （`db_query_values` 按 SQL 形态、`text2video_*` 列表夹具）；URL 参数 `?theme=&font=&accent=&platform=`。
- 原生窗口壳 / 材质（vibrancy、Mica）仍需真机确认，浏览器预览只覆盖 Web 层。

## 7. 硬约束（勿回退）

- 视觉取值只能来自 token；fork 侧 `pnpm lint` 全绿（设计 lint 0 违规）是提交前提；上游侧为告警模式不阻塞 CI。
- **Panel 内不叠卡片**；子分组只用分隔线 + 小标题。
- 开关只出现在列表行内、与标题同区；细粒度布尔用复选框（HIG 分工）。
- 焦点环唯一写法：`focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60`。
- 工具栏保持流程内（**不要**改回「绝对定位覆盖 + main 顶部内边距」，会产生双重上边距）。
- reka-ui 2.10：`data-active:` / `data-inactive:` 变体无效，一律用 `data-[state=…]:`；
  `TabsList` 横向高度被 group 变体 `h-9` 钉死，需内联样式覆盖。
