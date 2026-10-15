# 设计规范 · 共享核心

> 本文件是全项目 UI 的**唯一事实源**（Single Source of Truth）。macOS 与 Windows 的平台差异见
> [`design-macos.md`](design-macos.md) 与 [`design-windows.md`](design-windows.md)；逐控件正反例见
> [`design-appendix.md`](design-appendix.md)。改造进度与迁移规则见仓库根 `AGENTS.md`。
>
> **参考源**（评审时可逐条核对）：
>
> - Apple《Human Interface Guidelines》（macOS 部分）：Layout · Color · Typography · Materials · Motion ·
>   Dark Mode · Toggles · Lists and tables · Boxes · Toolbars · Sidebars · Segmented controls · Text fields ·
>   Pop-up buttons · Search fields · Alerts · Disclosure controls · Settings · Modality · Loading ·
>   [developer.apple.com/design/human-interface-guidelines](https://developer.apple.com/design/human-interface-guidelines)
> - Microsoft《Windows app design（Fluent / WinUI 3）》：Design principles · Geometry · Layering ·
>   Materials（Mica / Acrylic）· Motion: timing and easing · Typography · Color · Commanding ·
>   Title bar · Content layout and spacing · Guidelines for app settings · Writing ·
>   [learn.microsoft.com/windows/apps/design](https://learn.microsoft.com/en-us/windows/apps/design/)
> - JetBrains《UI Guidelines》（桌面工具密度、工具窗口、对话框布局、行内校验）：
>   [jetbrains.design/intellij](https://jetbrains.design/intellij/)
> - 备查：GNOME HIG、KDE HIG

## 0. 适用范围与执行方式

- 适用于 `src/`（框架壳、设置页、示例插件）与 `plugins/`（所有工具插件）的**全部前端 UI**。
- 执行手段（缺一不可）：
  1. **本文 + 平台文档 + 附录**：设计与评审依据；
  2. **Token 层**（`src/assets/index.css`）：所有视觉取值只能取自 token，禁止任意值；
  3. **`pnpm lint`**：`scripts/lint-design.mjs` 机器拦截旧配方（见 §6 反例清单）；
  4. **PR 评审清单**（§7）：每次改动逐条自检。
- 本文中的「必须 / 禁止」是硬约束；「建议」可按场景取舍，但需在 PR 里写明理由。

## 1. 设计原则

取自 HIG《Design principles》、Fluent《Design principles》与 JetBrains 的交集，并给出本项目的落地判据。

| 原则                                         | 落地判据                                                                                                                                                             |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **清楚优先**（Simplicity / Clarity）         | 一屏一个主任务；信息靠层级（顺序、间距、对比）表达，不靠装饰；设置项能少不多（HIG：Minimize the number of settings），只在与任务相关的页面就近提供                   |
| **即时反馈**（Response / Feedback）          | 交互在 pointer-down 就有反馈；<1s 的操作不显示 spinner；长任务可取消并显示进度；错误就地说明（不弹窗、不道歉）                                                       |
| **可撤销优先于确认**（Forgiveness / Agency） | 能撤销就不拦截；只有**不可撤销且后果严重**的操作才用 Alert 确认（Windows Commanding 原文；HIG Alerts 亦要求「避免为常见可撤销操作弹窗」）                            |
| **键盘可达**（Keyboard / Accessibility）     | 所有功能都能用键盘完成；焦点可见且顺序合理；快捷键在界面中可见；Esc 关闭浮层、Enter 触发默认动作                                                                     |
| **克制装饰**（Craft / Restraint）            | 颜色只用于语义与交互；材质只用于分层；动效只用于解释因果（HIG Motion：Add motion purposefully）；盒子不叠盒子（HIG Boxes：不要用嵌套盒子表达子分组，用内边距与对齐） |

## 2. Token 体系

### 2.1 间距标尺

基础栅格 **4px**，仅允许以下档位（Fluent《Content layout and spacing》的映射：控件间距 8、控件与标签 12、
内容区块 12、表面与内容 16、按钮之间 8）：

| Token         | 值  | 典型用途                                     |
| ------------- | --- | -------------------------------------------- |
| `--space-2xs` | 4   | 图标与文字、徽标内边距                       |
| `--space-xs`  | 8   | 控件之间、行内元素之间、按钮之间             |
| `--space-sm`  | 12  | 控件与标签、内容区块之间、卡片内边距（紧凑） |
| `--space-md`  | 16  | 卡片/面板内边距、表面与内容                  |
| `--space-lg`  | 24  | 模块之间、分组之间                           |
| `--space-xl`  | 32  | 页面级分区                                   |

窗口内容边距：紧凑 16 / 常规 20（macOS 常规窗口内容边距 20pt）。禁止出现 `space-y-3/4/6` 这类自由取值混用——
统一走上述档位对应的工具类（`gap-2/3/4/6` 与 `space-y-*` 需映射到档位语义并在 lint 中白名单校验）。

### 2.2 圆角（两点式 + 浮层）

| 层级 | 值       | Tailwind       | 适用                                          |
| ---- | -------- | -------------- | --------------------------------------------- |
| 控件 | **6px**  | `rounded-md`   | 按钮、输入框、下拉、分段控件、开关轨、标签    |
| 容器 | **8px**  | `rounded-lg`   | 面板（Panel）、卡片、分组列表、对话框主体     |
| 浮层 | **12px** | `rounded-xl`   | 菜单、下拉面板、Popover、Toast、Tooltip（大） |
| 胶囊 | `9999px` | `rounded-full` | 徽标、头像、滑块柄                            |

实现方式（关键）：把 `--radius` 设为 **8px**，shadcn 的派生规则即自动给出 `md=6 / lg=8 / xl=12`，
**无需改动任何组件类名**。除浮层外禁止使用 `rounded-xl`（lint 拦截）。

参照：Fluent 几何为容器 8px / 页内控件 4px（直线相交处 0）；macOS 观感更连续。本项目取 **6 / 8 / 12**
（已评估：6px 与现有 `--radius: 6px` 视觉延续，8px 命中 Fluent 容器值）。

### 2.3 字号与字重

保留现有**三档用户字号**（根字号 13 / 14 / 15，`--app-font-size`），基准档 = 常规（14px）。
刻度与两套平台规范的对应关系：

| 语义                   | Token/Tailwind        | 正常档 | 小档 | 大档 | 对应                                     |
| ---------------------- | --------------------- | ------ | ---- | ---- | ---------------------------------------- |
| 正文 / 列表主文本      | `text-sm`（0.923rem） | 12.9   | 12   | 13.8 | macOS Body 13 · Fluent Body 14           |
| 辅助文本 / 标签 / 说明 | `text-xs`（0.846rem） | 11.8   | 11   | 12.7 | macOS Subheadline 11 · Fluent Caption 12 |
| 页面/面板标题          | `text-base`           | 14     | 13   | 15   | macOS Title 3 15 · Fluent Body Strong 14 |
| 数值/等宽数据          | 继承 + `tabular-nums` | —      | —    | —    | 表格、坐标、金额、时间                   |

- 行高：正文 1.5；标题 1.3；表格行 1.4。禁止 `leading-none` 之外的任意行高值。
- 字重：常规 400；次要强调/选中态 500；标题 600。禁止 700+（macOS Headline 例外不用于 web 界面）。
- **低于 12px 的正文字号一律禁止**（Fluent 明确：14 Semibold / 12 Regular 为可读下限）。

### 2.4 控件高度（两档 + 工具栏）

| 档位 | Tailwind        | 小档(13) | 正常(14) | 大档(15) | 用途                                         |
| ---- | --------------- | -------- | -------- | -------- | -------------------------------------------- |
| 紧凑 | `h-7` (1.75rem) | 22.75    | 24.5     | 26.25    | 工具栏/行内按钮、表格行内操作、小尺寸 Select |
| 常规 | `h-8` (2rem)    | 26       | 28       | 30       | 表单控件、面板内动作按钮（默认）             |
| 提升 | `h-9` (2.25rem) | 29.25    | 31.5     | 33.75    | 主操作按钮、页面级输入（慎用，仅主路径）     |

高度一律用 **rem 表达（随三档字号缩放）**，禁止写死 px。图标按钮尺寸与同档高度一致（`size-7/8/9`）；
列表行高：紧凑 32 / 常规 36（对应 `h-8/h-9` 的视觉节奏）。

### 2.5 层次（surface）与语义色

**层次枚举**（禁止再用 `bg-muted/30`、`bg-muted/50`、`bg-muted/80` 这类任意透明度）：

| 层      | Token                                         | 用途                     | 可否嵌套                   |
| ------- | --------------------------------------------- | ------------------------ | -------------------------- |
| base    | `bg-background`                               | 窗口底                   | —                          |
| card    | `bg-card`                                     | 面板/卡片/分组列表       | 可（一组相邻，不可叠同层） |
| raised  | `bg-popover`                                  | 浮层（菜单/弹窗/Toast）  | 浮于 base/card 之上        |
| inset   | `bg-muted`                                    | 输入槽、代码块底、只读区 | 在 card 内使用             |
| sunken  | `bg-muted/60` → 固定 token `--surface-sunken` | 空态/占位底              | 在 card 内使用             |
| console | `bg-console` / `text-console-foreground`      | 日志/终端输出            | 独立区域，仅此一处深色     |

- **一屏最多两级表面**：base → card（+ 内部 inset）。禁止 card 套 card（HIG Boxes：不要嵌套盒子表达分组，
  用内边距与对齐）；需要子分组时用**分隔线 + 标题 + 间距**。
- 颜色语义：`primary`（accent）**仅用于交互控件与选中态**；`destructive / success / warning / info`
  仅用于对应语义（错误/成功/警告/提示），不得当装饰色。同一颜色不得表达两种含义（HIG Color）。
- 半透明层上不放 accent 色（Apple Materials：颜色放在实色层上，不放在毛玻璃前景）。
- **唯一例外**：首页 Hero 为品牌签名区，允许使用 `primary` 的低饱和装饰（见 `src/layouts/Home.vue`）；
  除此之外任何位置不得用 accent 作装饰。

### 2.6 边框与分隔

- 所有边框/分隔线用 **1px hairline**（`border` = `--border`），不得混用 `border-border`（lint 拦截，
  二者当前并存是历史遗留）。
- **盒边框不叠盒边框**：容器已有边框时，内部用 `divide-y` 或 `border-t` 分隔，不再画第二个盒子。
- 表格：数据行用 `divide-y`（不用竖线）；表头用 `border-b` + 粘性；需要强调行时用 `bg-muted/60` 斑马纹（macOS 表格可选斑马纹）。
- 聚焦态用 `ring`（§2.9），**不通过改变边框颜色**表达聚焦。

### 2.7 动效

| Token        | 值    | 对照         | 用途                              |
| ------------ | ----- | ------------ | --------------------------------- |
| `--dur-fast` | 120ms | Fluent 83ms  | hover、按下、颜色/透明度变化      |
| `--dur-base` | 200ms | Fluent 167ms | 展开/收起、Tab 切换、Popover 进出 |
| `--dur-slow` | 300ms | Fluent 250ms | 对话框、页面切换、大面积材料进出  |

- 缓动：进入 `cubic-bezier(0.22, 1, 0.36, 1)`（≈ Fluent Fast Out, Slow In `cubic-bezier(0,0,0,1)`）；
  退出 `cubic-bezier(0.4, 0, 1, 1)`。
- 只动 `transform` 与 `opacity`（外加 `backdrop-filter` 材质进出）；**不允许**对 `width/height/top/left` 做动画。
- `prefers-reduced-motion: reduce` → 位移/缩放改为 200ms 交叉淡入，去掉一切 overshoot。
- **删除现有全局 `transition-duration: 75ms !important`**；保留 `[data-reka-popper-content-wrapper]`
  的 0s 例外（定位测量用的技术性覆盖，与动效规范无关，勿一并删除）。

### 2.8 材质（vibrancy）

可用区域**仅限**：侧栏、工具栏/标题栏、浮层（菜单/弹窗/Toast/Tooltip）。
**禁止**：滚动内容区、表格、表单、卡片主体使用毛玻璃（性能与可读性）。

- macOS → `NSVisualEffectView` 语义材质：侧栏 `sidebar`、工具栏 `headerView`、浮层 `popover`/`menu`、
  对话框 `sheet`（详见 `design-macos.md`）。
- Windows → `Mica`（窗口基底）/ `Mica Alt`（含导航与命令区的应用，官方推荐）/ `Acrylic`（浮层）
  （详见 `design-windows.md`）。
- **必须有纯色回退**：`prefers-reduced-transparency: reduce`、用户关闭透明、Windows 电池节能/低端硬件/
  窗口失焦 → 回退为 `bg-card`/`bg-background` 实色。回退不是降级体验，必须在两种形态下都可读。
- 同一屏最多 2 处材质；禁止材质上叠材质；材质层的前景色必须提高对比（不用 `text-muted-foreground` 作主文本）。

### 2.9 焦点与键盘

- 焦点环（**规范表达式，唯一写法**）：`focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60`，
  形状跟随控件圆角；**`focus-visible` 恒可见**，不得 `outline-none` 后无替代（`ui/` 内 11 个组件已统一，lint R6 拦截偏差）。
  Windows 由 `--focus-ring-width: 2px` 改为双描边观感（见 `design-windows.md §7`）。
- 所有自定义可交互元素必须：可聚焦（`tabindex="0"`）、有 `role` 与可读名称（`aria-label`/文本）、响应 Enter/Space。
- 复合控件键盘语义（新增要求，现状为 0 处 tabindex/3 处 role）：
  - 分段控件：`role="tablist"` + ←/→ 移动、Home/End 首尾；
  - 列表/表格行：↑/↓ 移动、Enter 激活、可 type-ahead（若为选项列表）；
  - 菜单/下拉：↑/↓ 循环、Esc 关闭并归还焦点、输入字符可定位项。
- 全局快捷键（工具栏/菜单必须一致，见平台文档）：`⌘,` 设置 · `⌘K` 命令面板 · `⌘F` 页内搜索 ·
  `⌘W` 关闭窗口 · `⌘Z / ⇧⌘Z` 撤销/重做 · `⌘1..9` 切换侧栏分组 · `Esc` 关闭浮层。
- 快捷键在界面中可见（`<kbd>` 或 tooltip），不允许“隐藏功能”。

### 2.10 图标

- 统一 `@lucide/vue`；尺寸仅 3 档：**14（行内）/ 16（按钮、菜单）/ 20（工具栏、空态）**；
  描边宽度统一 2（lucide 默认），不单独改。
- 与文字并排时用 `gap-2`，基线对齐（图标随行高居中，不手动 `mt-*`）。
- 图标按钮**必须**有 `aria-label` + tooltip（Tooltip 现状仅 1 处，属缺失项）。
- 图标不表达状态色（状态用文本/徽标），不叠加多个图标表达一个动作。

## 3. 组件语义（什么时候用哪个）

| 场景                                        | 用什么                                        | 不用什么                   |
| ------------------------------------------- | --------------------------------------------- | -------------------------- |
| 2–5 个互斥选项、需常显选择态                | **分段控件**（`Segmented`）                   | 一排开关、下拉             |
| 6+ 互斥选项 / 空间受限                      | 下拉（pop-up button）                         | 分段（HIG：分段 ≤ 5–7 项） |
| 细粒度布尔设置                              | **复选框**（标题在右）                        | 开关                       |
| 强调整组开关 / 重要开关（如「启用该功能」） | **开关**（分组表单中用 mini 尺寸）            | 复选框                     |
| 数值微调                                    | 输入框 + 步进（stepper）                      | 滑块（滑块仅用于粗略取值） |
| 即时搜索/过滤                               | `SearchField`（带放大镜、清除按钮、输入即搜） | 普通输入框 + 按钮          |
| 不可撤销的危险操作                          | `Alert`（默认按钮为安全项，破坏性项标红）     | 普通确认弹窗、toast        |
| 可撤销的操作                                | 直接执行 + 状态提示（+ 撤销入口）             | 弹窗确认                   |
| 长任务（>1s）                               | 进度条/进度环 + 可取消                        | 转圈遮罩                   |
| 页面级加载（>300ms 才有内容）               | 骨架屏（结构与真实内容一致）                  | spinner 覆盖层             |
| 结果/日志类输出                             | 只读文本块（`bg-console` 仅日志）             | 可编辑输入框               |

按钮层级：**每个视图最多一个主按钮**（`default`）；其次 `secondary`；`ghost` 用于面板头部动作；
`destructive` 仅用于破坏性操作且需 Alert 二次确认。

## 4. 三态与反馈规则

**空态**：图标（20）+ 一句主文案 + 一句说明 + 一个动作按钮；不出现空白页面。
**加载态**：<1s 不显示任何加载指示（HIG Loading：Show something as soon as possible）；
1s–10s 用骨架屏或进度条；>10s 必须可取消并显示进度。
**错误态**（统一三分法，替换现状 `toast.error` 81 处 vs 内联 35 处的混用）：

| 场景                              | 呈现                                                                                 |
| --------------------------------- | ------------------------------------------------------------------------------------ |
| 表单/工具页内、可定位的错误       | **就地进行**：控件下方 `text-xs text-destructive`；多字段错误在页面顶部加一条错误条  |
| 当前操作失败但有明确重试入口      | 工具页面板内的错误条（`border-destructive/30 bg-destructive/10`）+ 重试按钮          |
| 后台任务/系统级失败（无就地位置） | `toast.error`（唯一允许用 toast 报错的场景）                                         |
| 成功                              | **不弹 toast**：用状态变化（行/徽标/按钮态/结果区）表达；仅长任务完成可用 info toast |

可恢复的提示用 `info`/`warning` 告警条（`bg-info/10`、`bg-warning/10`），不用 toast。
服务端/后端错误文案：说明发生了什么 + 下一步怎么办，不道歉、不使用「出错了」。

## 5. 文案规范

- **按钮用动词**（保存、导出、重新选择），一个动作在流程中保持同名；按钮不带句号。
- 打开浮层/对话框的按钮带省略号（`更多选项…`），立即执行的按钮不带。
- 句子大小写（中文无此问题，但英文术语需统一）；不使用全大写。
- 术语表（本仓库统一用词）：

| 用              | 不用               |
| --------------- | ------------------ |
| 工具 / 插件     | 应用、功能模块     |
| 模块面板 / 面板 | 卡片、区块         |
| 设置            | 配置、偏好、选项页 |
| 导出 / 导入     | 保存为、下载       |
| 工作表（Excel） | sheet、页签        |
| 表头            | 列名、header       |

- 数字与单位：数字右对齐 + `tabular-nums`；单位与数字之间空格（`5 MB`）；时间用 `yyyy-MM-dd HH:mm`。
- 错误文案不道歉、不卖萌、不出现 `！` 连用；占位文案说明「可以填什么」，不写「请输入…」。

## 6. 反例清单（改造前实测，lint 会拦截其中可机器判定的项）

| 反例             | 现状证据                                                                                   | 规范要求                                      |
| ---------------- | ------------------------------------------------------------------------------------------ | --------------------------------------------- |
| 两套卡片配方并存 | `rounded-lg border bg-card` 6 个文件 / `rounded-lg border border-border bg-card` 14 个文件 | 统一 `rounded-lg border bg-card`（§2.2/§2.6） |
| 任意透明度       | `bg-muted/30` 6 处 · `bg-muted/50` 5 处 · `bg-muted/80` 若干                               | 走层次枚举（§2.5）                            |
| 圆角第三种值     | `rounded-xl` 3 处（非浮层）                                                                | 仅浮层可用 `xl`（§2.2）                       |
| 错误反馈混用     | `toast.error` **81** 处 vs 内联 `text-destructive` **35** 处                               | 三分法（§4）                                  |
| 键盘不可达       | `tabindex` **0** 处 · `role=` **3** 处                                                     | 自定义可交互元素必须可聚焦 + 有 role（§2.9）  |
| 手搓分段控件     | 4 处（crypto 四个面板）                                                                    | 用 `Segmented`（§3）                          |
| 图标按钮无提示   | Tooltip 仅 1 处；`kbd` 提示 3 处                                                           | 图标按钮必须 tooltip + `aria-label`（§2.10）  |
| 自由间距         | `space-y-3` 22 · `space-y-4` 24 · `space-y-6` 9                                            | 走间距档位（§2.1）                            |
| 全局动效压制     | `transition-duration: 75ms !important`                                                     | 动效 token（§2.7）                            |
| 卡片套卡片       | 多处（Panel 内再放 `rounded-lg border` 容器）                                              | 同一容器内用分隔线 + 标题 + 间距（§2.5）      |

## 7. PR 评审清单

1. 新增/修改的视觉取值是否**全部**来自 token（无任意值、无写死 px 高度/字号）？
2. 每屏是否**最多两级表面**（无卡片套卡片）？边框是否避免叠加？
3. 每个模块是否都用 `Panel`（或平台等价容器）包裹，无裸露模块？
4. 布尔/互斥选项的控件选型是否符合 §3（复选框 vs 开关 vs 分段 vs 下拉）？
5. 加载/空/错误三态是否齐全，且错误呈现符合 §4 三分法？
6. 键盘：Tab 顺序合理、焦点可见、Esc/Enter 语义正确、快捷键在界面可见？
7. 图标按钮是否有 `aria-label` + tooltip？字形与尺寸是否取自三档？
8. 文案是否符合 §5（动词、术语表、标点、单位）？
9. 亮色/暗色、三档字号、窗口宽度 760/1080/1440 下是否都不破版？
10. 平台差异是否只落在 §平台文档允许的差异点内（材质/窗壳/菜单/对话框语义/焦点视觉/半径）？
