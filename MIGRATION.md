# 桌面化设计改造 · 推进计划与交接

> 可续接的工作交接（fork 专属，勿带上游）。新会话从这里接手即可，不必回看历史对话。
> 规范正文：`DESIGN.md` / `DESIGN-macos.md` / `DESIGN-windows.md` / `DESIGN-appendix.md`（上游框架文档）；
> 插件目录规范：`plugins/README.md`（上游）。

## 0. 现状（一句话）

**规范、tokens、组件、壳层、材质、插件批次已全部完成并进入上游**；fork 侧只剩一件事：
**在 `chore/plugin-lib-layout` 上一次性合入 `upstream/main`（解决上游插件冲突）→ 合入 main**（见 §2）。

```
upstream/main  4b378dc  (#7~#12：设计 lint 清零 / 壳层材质 / 窗口材质 / base 示例规范化 / 插件目录规范)
main           e89dbe5
 ├─ design/plugin-conformance  2d146bd   （5 个提交：插件层设计批次，已 push，未合 main）
 │   └─ chore/plugin-lib-layout 9a4e466   （3 个提交：lib/ 目录迁移 + 框架文件对齐 + 本交接修订，已 push）
 └─ design/desktop-proto                 （早期原型分支，本地；其框架层内容已由上游承载，可弃）
```

## 1. 上游（PocketArk）已完成

| PR        | 内容                                                                                                                        |
| --------- | --------------------------------------------------------------------------------------------------------------------------- |
| #5        | 桌面化设计规范（4 份文档）+ token/组件/壳层                                                                                 |
| #7        | 框架层设计 lint 存量清零（R1/R2/R3/R4/R6）                                                                                  |
| #8 / #9   | 设置页 pane 与壳层材质、窗口材质（macOS `underWindowBackground` / Windows Mica Alt）                                        |
| #10 / #11 | 清理 base 里的 fork 残留；base 示例插件（hello-world / system / _template）按规范整体改造                                   |
| **#12**   | **插件目录规范 `plugins/README.md` + 结构 lint `scripts/lint-plugins.mjs`（R-P1~R-P6，接入 `pnpm lint`）+ 模板/脚手架补齐** |

上游无需再提 PR（除非发现新问题）；`pnpm lint` 在上游同时跑 eslint + lint-design + lint-plugins。

## 2. 下一步（一次合并解决，勿分两步）

> ⚠️ 经 `git merge-tree` 实测：`design/plugin-conformance` **单独合 main 也会冲突**
> （`hello-world` / `system` 的页面被上游 #11 重写过，另有 `README.md` / `MIGRATION.md`）。
> 因此**不要**先合 conformance 再合本分支——那样要解两次同样的冲突。
> 正确做法：在已叠好的 `chore/plugin-lib-layout`（含插件设计批次 + 目录迁移）上**一次性合入 upstream/main**。

```bash
cd ../ArkDesk
git checkout chore/plugin-lib-layout           # 叠在 conformance 上：含两个分支的全部内容
git fetch upstream && git merge upstream/main  # 冲突范围见下表
```

### 冲突处理口径（预期文件已实测列出）

| 路径                                                                                 | 口径                                                                                                 |
| ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| `plugins/_template/**`、`plugins/hello-world/**`、`plugins/system/**`                | **取上游**（`git checkout --theirs`）——上游插件，上游 #11 已按规范重写且更合规                       |
| `AGENTS.md`                                                                          | **手工合并**：保留 fork 段落（双仓库协作 / 本地层 / 设计推进计划），并入上游新增段（插件目录规范等） |
| `README.md`                                                                          | 取 fork 版（fork 门面）；上游若有新增说明手工并入                                                    |
| fork 自有插件（`network-tools` / `daily-tools` / `text2video` / `tender-optimizer`） | 无冲突（预期）                                                                                       |

```bash
# 上游插件：整目录取上游版本
git checkout --theirs plugins/_template plugins/hello-world plugins/system
git add plugins/_template plugins/hello-world plugins/system
# AGENTS.md / README.md：手工编辑解决后 git add
pnpm lint && pnpm build && pnpm test     # 此时 lint 含 lint-plugins，可验证结构合规
git commit && git push
```

### 最后合入 main（并清理分支）

```bash
git checkout main && git pull --ff-only
git merge --no-ff chore/plugin-lib-layout -m "chore: 合入插件层设计批次 + 插件目录规范迁移"
pnpm lint && pnpm build && pnpm test
git push origin main
git branch -D design/plugin-conformance design/desktop-proto   # 内容已并入，删除
```

## 3. 目录规范迁移（`chore/plugin-lib-layout` 已完成的部分）

规范：`plugins/README.md`（`frontend/` 顶层白名单：`views/ settings/ components/ composables/ lib/ shared.ts schema.ts setup.ts`）。

| 插件                      | 迁移                                                                                                                                                  |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| network-tools             | 根目录 6 个纯逻辑 TS（`curl` `error` `variables` `transfer` `intercept-shared` `settings-store`）→ `frontend/lib/`；插件内引用与 4 个测试 import 同步 |
| daily-tools               | `frontend/table/*.ts`（10）→ `frontend/lib/table/`；`crypto-shared.ts` → `frontend/lib/crypto-shared.ts`；组件/视图/composable/测试 import 同步       |
| text2video                | 无需迁移（`preview.ts` 已不存在，`composables/` 合规）                                                                                                |
| tender-optimizer / system | 无需迁移（前者全平铺后端 + 视图；后者纯前端，文档由上游补充）                                                                                         |

各插件 `AGENTS.md` / `README.md` 的路径表格已同步；框架文件与上游对齐（`UpdateDialog`）。

## 4. 已完成的设计改造（回顾，勿重复劳动）

- **规范**：4 份文档（共享核心 / macOS / Windows / 组件附录），平台差异只允许落在 6 处（材质与回退、窗口壳、
  菜单与快捷键呈现、对话框按钮语义、焦点视觉、圆角档）
- **token**：角色化圆角（`--radius-control/container/overlay`，平台覆盖变量、类名不变）、动效 token
  （含 `prefers-reduced-motion`）、层次枚举（含 `bg-sunken`）、材质 token + 三种实色回退
- **组件**：`Panel`、`SettingsSection/Row/Field`、`src/components/native/*`（EmptyState/LoadingState/ErrorState/
  Segmented/FormRow/ListRow/SearchField）
- **壳层**：工具栏保持流程内、粘性页头承载模糊、侧栏 source list 与窄窗自动折叠、设置页工具栏 tab 条、
  窗口材质（macOS behind-window / Windows Mica Alt）
- **门禁**：`pnpm lint` = eslint + `lint-design.mjs`(R1~~R6) + `lint-plugins.mjs`(R-P1~~R-P6)
- **插件批次**：daily-tools、text2video、system、hello-world、network-tools、tender-optimizer 均已按规范收敛

## 5. 设计走查（自证，无需人工截图）

```bash
npx vite                     # 端口 1420
node scripts/design-shot.mjs --url "/tool/json-table" --out /tmp/a.png \
  --theme dark --font 14 [--platform win] [--eval "document.querySelector('button').click()"]
```

- 预览桥 `src/dev/preview-bridge.ts`（仅 dev + 非 Tauri 生效）：模拟 `__TAURI_INTERNALS__`、localStorage 版
  `plugin-store`、业务命令夹具（`db_query_values` 按 SQL 形态、`text2video_*` 列表）
- URL 参数：`?theme=light|dark&font=13|14|15&accent=<name>&platform=win`
- 原生窗口壳与材质（vibrancy / Mica）仍需真机确认；**Windows 平台层尚未视觉验证**（见 `DESIGN-windows.md §11`）

## 6. 硬约束（勿回退）

- 视觉取值只能来自 token；`pnpm lint` 全绿是提交前提（fork 侧设计 lint 存量已清零）。
- Panel 内不叠卡片；子分组只用分隔线 + 小标题。
- 开关只出现在列表行内、与标题同区；细粒度布尔用复选框（HIG 分工）。
- 焦点环唯一写法：`focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60`。
- 工具栏保持流程内（**不要**改回「绝对定位覆盖 + main 顶部内边距」）。
- reka-ui 2.10：`data-active:` / `data-inactive:` 无效，一律 `data-[state=…]:`；`TabsList` 横向高度被
  group 变体 `h-9` 钉死，需内联样式覆盖。
- 插件内部结构：`frontend/` 顶层白名单、纯逻辑放 `frontend/lib/`、命令只在 `backend/mod.rs`。
- 生成上游补丁必须用工作区状态（`git diff A..B` 只含已提交内容，会漏掉未提交的中性化/修正）。
