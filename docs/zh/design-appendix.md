# 设计规范 · 组件附录（Do / Don't）

> 共享规则见 [`design.md`](design.md)，平台差异见 [`design-macos.md`](design-macos.md) /
> [`design-windows.md`](design-windows.md)。本文件是**逐控件的正反例**与**迁移对照表**，
> 改造时按此逐项核对；新增控件时先在此登记再实现。

## 1. 门面组件清单（`src/components/native/`）

| 门面组件                                                     | 基于                                | 职责                                                              | 替换现状                                     |
| ------------------------------------------------------------ | ----------------------------------- | ----------------------------------------------------------------- | -------------------------------------------- |
| `Toolbar.vue`                                                | 新写                                | 工具栏行（40px mac / 32px win）、左侧标识、右侧命令与「更多」溢出 | `TitleBar.vue` 自绘部分                      |
| `SidebarList.vue`                                            | `SideNav.vue`                       | source list：分组标题、条目、选中态、折叠                         | `SideNav.vue`                                |
| `Panel.vue`                                                  | 框架 `tool/Panel.vue`               | 模块面板（已有，补 `overflow-hidden` 与材质禁用约束）             | 6 处手写卡片                                 |
| `SettingSection.vue` / `SettingRow.vue` / `SettingField.vue` | `@/components/settings`             | 分组 + 行 + 字段（已统一）                                        | —                                            |
| `ListRow.vue`                                                | 新写                                | 列表行：主文本 + 副文本 + 尾部控件/图标；键盘可达                 | 各处自写 `flex items-center justify-between` |
| `FormRow.vue`                                                | 新写                                | 标签 + 控件 + **行内校验**（错误文案与 aria）                     | 各处 `space-y-1.5 + Label`                   |
| `SearchField.vue`                                            | `ui/input`                          | 放大镜 + 清除按钮 + 输入即搜                                      | 无（现用普通 Input）                         |
| `Segmented.vue`                                              | `ui/tabs` 或新写（reka RadioGroup） | 2–7 项互斥分段控件，含 ←/→ 键盘语义                               | 4 处手搓（crypto 四面板）                    |
| `PopUp.vue`                                                  | `ui/select`                         | 统一下拉（宽度、chevron、键盘、空态）                             | 各处 `SelectTrigger` 宽度不一                |
| `ToggleRow.vue`                                              | `ui/switch` / `ui/checkbox`         | 布尔选项行（复选框 = 细粒度 / mini 开关 = 强调）                  | 开关与 Label 摆放不一                        |
| `StatusBar.vue`                                              | 新写                                | 底部状态条（进程、行列数、路径）                                  | 无                                           |
| `EmptyState.vue` / `LoadingState.vue` / `ErrorState.vue`     | 新写                                | 三态标准件（图标 20 + 文案 + 动作）                               | 13 处虚线框各不相同                          |
| `AlertDialog` 语义封装                                       | `ui/alert-dialog`                   | 危险操作确认（按钮顺序按平台）                                    | `ui/dialog` 使用面需审                       |
| `ProgressBar.vue`                                            | `ui/slider` 之外新写                | 确定/不确定进度 + 取消                                            | `TaskProgress.vue`                           |

> **实现状态**：已随本规范落地的是 `Panel.vue`（`src/components/tool/`）、
> `SettingSection/Row/Field`（`src/components/settings/`）与 `EmptyState` / `LoadingState` / `ErrorState` /
> `FormRow` / `ListRow` / `SearchField` / `Segmented`（`src/components/native/`）。
> `Toolbar` / `SidebarList` / `PopUp` / `ToggleRow` / `StatusBar` / `ProgressBar` / `AlertDialog` 封装
> 为**规划项**，用到时按下表约定实现并回填本节。

## 1.5 已知陷阱（reka-ui 2.10 + shadcn-vue）

- **`data-active:` / `data-inactive:` 变体是死规则**：reka 的 `TabsTrigger` 只输出 `data-state="active|inactive"`，
  shadcn 组件里基于 `data-active:` 的选中样式不会命中。改样式一律用 `data-[state=active]:`。
- **`TabsList` 横向高度被 group 变体钉死**：`group-data-[orientation=horizontal]/tabs:h-9`，普通 `h-*` 覆盖不掉，
  需用内联样式覆盖。
- **禁用全局过渡压制**：`transition-duration: 75ms !important` 已移除，动效走 token；
  reka popper 定位容器的 `transition-duration: 0s` 例外必须保留（与动效无关，是测量导致的漂移修复）。

## 2. 逐控件 Do / Don't

### 按钮

| Do                                                              | Don't                                  |
| --------------------------------------------------------------- | -------------------------------------- |
| 每屏一个主按钮（`default`），其余 `secondary`/`ghost`           | 一屏多个 `default` 争抢注意力          |
| `ghost` 只用于面板头部/工具栏动作                               | `ghost` 作为主操作                     |
| 高度取两档（`h-8` 常规 / `h-7` 紧凑），图标 `size-3.5` + `mr-1` | `h-9` 与其他尺寸混排（除页面级主操作） |
| 破坏性操作 `destructive` + Alert 确认                           | 用红色表达「取消」或普通操作           |
| 文案用动词、与结果一致（`导出` → 「已导出」）                   | `确定`/`提交`/`执行` 等泛化词          |

### 输入框 / 文本域

| Do                                              | Don't                           |
| ----------------------------------------------- | ------------------------------- |
| `FormRow` 提供标签 + 说明 + 行内校验            | 只有 placeholder 没有标签       |
| 固定高度 + 超出滚动（`field-sizing: fixed`）    | 自动增高导致布局跳动            |
| 数字输入右对齐 + `tabular-nums`                 | 数字左对齐、字体非等宽          |
| 校验就地（控件下方 `text-xs text-destructive`） | 只在 toast 报错、或提交后才校验 |
| 等宽字体用于代码/路径/密钥                      | 用等宽字体承载正文              |

### 下拉 / 分段 / 复选框 / 开关 / 搜索

| Do                                       | Don't                                  |
| ---------------------------------------- | -------------------------------------- |
| 2–5 项互斥 → 分段；≥6 项 → 下拉          | 用一排开关表达互斥选项                 |
| 细粒度布尔 → 复选框（标题在右）          | 细粒度布尔用开关（macOS 语义不对）     |
| 分组表单内的开关用 mini（`size="sm"`）   | 分组表单内用大开关导致行高不齐         |
| 开关必须与其标题同行同区（`SettingRow`） | `justify-between` 把标题与开关拉到两端 |
| 搜索框输入即搜 + 清除按钮                | 搜索框配「搜索」按钮                   |
| 下拉宽度固定（控件列对齐）               | 每个下拉宽度不同、左边缘参差           |

### 面板 / 分组 / 列表行

| Do                                            | Don't                  |
| --------------------------------------------- | ---------------------- |
| 模块一律 `Panel` 包裹；头部条 + 正文 `p-4`    | 模块裸露在页面上       |
| 同一容器内子分组用 `divide-y` + 小标题 + 间距 | 卡片套卡片、盒子叠盒子 |
| 行：主文本 + 副文本（`text-xs` 灰）+ 尾部控件 | 行内塞三行以上说明     |
| 列表行可键盘聚焦（`tabindex` + role）         | 行只能鼠标点击         |

### 浮层（菜单 / 弹窗 / Toast / Tooltip）

| Do                                            | Don't                          |
| --------------------------------------------- | ------------------------------ |
| 浮层圆角 12（Windows 8）、统一阴影 + 1px 描边 | 各浮层圆角/阴影不一致          |
| Esc 关闭并把焦点还原到触发元素                | 关闭后焦点丢失                 |
| 菜单项 ≥24px 行高、图标 16、危险项置末并分隔  | 菜单项过密、危险项与普通项相邻 |
| Tooltip 仅承载补充说明（图标按钮、截断文本）  | 用 Tooltip 承载必读信息        |
| Toast 只用于系统级失败/长任务完成             | 用 toast 报告表单校验错误      |

### 表格 / 数据区

| Do                                        | Don't                          |
| ----------------------------------------- | ------------------------------ |
| `divide-y` 分隔行、表头粘性、数字列右对齐 | 每行画盒子边框、单元格居中数字 |
| 空数据用 `EmptyState`                     | 空白表格                       |
| 滚动区用实色底（禁毛玻璃）                | 表格上叠材质                   |
| 超宽时横向滚动 + 首列粘性（如需）         | 压缩列宽到文字换行             |

### 三态

| Do                                             | Don't                              |
| ---------------------------------------------- | ---------------------------------- |
| 空态：图标（20）+ 主文案 + 说明 + 动作         | 只显示「暂无数据」                 |
| <1s 不显示加载态；1–10s 骨架；>10s 进度 + 取消 | 一切操作都盖转圈遮罩               |
| 错误条：说明原因 + 下一步 + 重试               | 「出错了」「操作失败」等无信息文案 |

## 3. 图标与快捷键

| Do                                                  | Don't                    |
| --------------------------------------------------- | ------------------------ |
| 尺寸 14/16/20 三档，描边 2                          | 每个页面自定义图标大小   |
| 图标按钮必有 `aria-label` + tooltip                 | 纯图标无提示             |
| 快捷键在菜单项/提示中可见（`⌘K` / `Ctrl+K` 按平台） | 隐藏快捷键、只在文档里写 |

## 3.5 设计走查工作流（自证截图，无需人工截图）

Web 层可在普通浏览器里渲染（预览桥模拟 Tauri API），因此可以脚本化截图自查：

```bash
npx vite                                    # 起前端（1420）
node scripts/design-shot.mjs \
  --url "/tool/hello-table" --out /tmp/a.png \
  --theme dark --font 14 [--platform win] \
  [--eval "document.querySelector('button').click()"]   # 可先点击/输入再截图
```

- 预览桥 `src/dev/preview-bridge.ts`：仅 `import.meta.env.DEV` 且非 Tauri 环境生效；
  实现 `__TAURI_INTERNALS__` 协议 + localStorage 版 `plugin-store` + 框架命令夹具（含 `db_query_values`）。
  插件自己的业务命令夹具放 `plugins/<id>/frontend/preview.ts`（**默认导出** `Record<命令名, 返回值>`），
  dev 预览时自动并入；**base 不含任何插件业务命令**，fork 的插件夹具随插件一起留 own 仓库。
- URL 参数：`?theme=light|dark`、`?font=13|14|15`、`?accent=<name>`、`?platform=win`。
- 用途：逐屏核对规范条款、验证浮层/焦点/三态；**原生窗口壳与材质仍需真机确认**。

## 4. 迁移对照表（改造批次）

| 批次 | 范围                                                                     | 文件数 | 风险 | 状态 | 说明                    |
| ---- | ------------------------------------------------------------------------ | ------ | ---- | ---- | ----------------------- |
| P3-1 | 壳：`TitleBar` `SideNav` `MainLayout` `Settings` `Home` `CommandPalette` | 6      | 高   | ✅   | 含材质/窗口契约改造     |
| P3-2 | `ui/` 样式覆盖 18 个 shadcn 组件（只改样式）                             | 18     | 中   | ✅   | 圆角/焦点/高度 token 化 |
| P4-1 | `hello-world`（base 活样板）                                             | 9      | 低   | ✅   | 作为规范示范全面重写    |
| P4-2 | `system`                                                                 | 6      | 低   | ✅   | 表格/数据维护页         |
| P4-3 | `_template`                                                              | 2      | 低   | ✅   | 与 base 保持一致        |

> base 只含 `_template` / `hello-world` / `system` 三个插件，上表即全部批次；
> **fork 的个人插件按同一清单逐批整改，进度由各 fork 自行维护**（框架层不需要知道 fork 的插件名）。
> 每批完成后：`pnpm lint && pnpm build && pnpm test`，并按 `design.md` §7 评审清单逐条自检；
> 涉及 Rust 的批次额外跑 `pnpm fmt:rs && pnpm lint:rs && pnpm test:rs`。
