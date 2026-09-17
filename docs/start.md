# START —— 用这个仓库搭你自己的桌面软件

> 这份文档写给：clone 了 ArkDesk、想把它变成**自己的桌面工具软件**的人。
> 跟着走完，你会得到一个以你的软件命名、带你的图标与主题、装着你自己的工具的桌面应用。
>
> 文档分工：**start.md**（本文件）= 从零起步手册；**extending.md** = 架构与扩展开发参考；
> **AGENTS.md** = 日常开发约定与陷阱清单；**release.md** = 打包 / 发布 / 在线更新；
> **local.md** = fork 本地层；`plugins/_template/README.md` = 单插件脚手架说明。

---

## 0. 这是什么项目

一个**桌面工具箱底座**：Tauri 2 + Vue 3 + TypeScript + Tailwind CSS 4 + shadcn-vue。
它已经替你做好了所有「与应用本身无关」的脏活——窗口、托盘、侧栏导航、路由、主题、
SQLite、设置持久化、日志、错误处理、HTTP 通道——你要做的只有：写你的工具页面。

框架代码（`src/core/`、`src-tauri/src` 的 db/http/tray/updater/menu/tasks/open/…）**只引用、不修改**；
所有扩展通过插件机制完成（见第 6 节）。

## 1. 环境准备

| 依赖    | 版本            | 说明                        |
| ------- | --------------- | --------------------------- |
| Node.js | 20+             | 含 npm                      |
| pnpm    | 9+              | 包管理器（`npm i -g pnpm`） |
| Rust    | stable（1.77+） | `rustup` 安装               |

平台额外依赖：

- **macOS**：Xcode Command Line Tools（`xcode-select --install`）
- **Windows**：MSVC Build Tools（VS Installer 勾选「使用 C++ 的桌面开发」）+ WebView2 Runtime（Win11 自带）
- **Linux**：webkit2gtk 等系统库，见 [Tauri prerequisites](https://tauri.app/start/prerequisites/)

## 2. 首次运行

```bash
pnpm install
pnpm tauri dev      # 首次会编译 Rust，需几分钟；之后增量秒级
```

窗口弹出即成功。此时它是「暗色指挥台」样式的 ArkDesk，带着示例工具——
接下来两步把它变成你的软件：**改名**（第 3 节）→ **删示例、写工具**（第 5、6 节）。

## 3. 改名：变成你自己的软件（一键脚本）

```bash
pnpm scaffold
```

按提示回答 5 个问题（回车保持默认）：

| 提问       | 说明                                                                       |
| ---------- | -------------------------------------------------------------------------- |
| 软件显示名 | 出现在窗口、Dock/任务栏、托盘提示、设置页（中文/英文均可）                 |
| 英文标识   | kebab-case（如 `my-kit`），用于包名与二进制名                              |
| 应用标识   | `com.arkdesk.<英文标识>`，**发布后不可再改**（改了会被系统视为另一个应用） |
| 默认主题色 | 靛蓝 / 青碧 / 琥珀 / 玫红 / 湛蓝 / 国家电网绿                              |
| 默认主题   | dark / light                                                               |

脚本会自动改写 `tauri.conf.json`、`Cargo.toml`、`main.rs`、`package.json`、
文档与说明中的所有名称。完成后按提示执行：

```bash
pnpm install        # 刷新 lockfile 中的包名
pnpm tauri dev      # 以新名字运行
```

<details>
<summary>偏好手动改？以下是完整改名清单（与脚本等价）</summary>

| 文件                                                          | 改什么                                                         |
| ------------------------------------------------------------- | -------------------------------------------------------------- |
| `src-tauri/tauri.conf.json`                                   | `productName`、`identifier`（发布后不可改）、`title`           |
| `src-tauri/Cargo.toml`                                        | `[package] name`（kebab）、`[lib] name`（snake + `_lib` 后缀） |
| `src-tauri/src/main.rs`                                       | `arkdesk_lib::run()` → 新 lib 名                               |
| `package.json`                                                | `name`                                                         |
| `src/core/theme/index.ts`                                     | `DEFAULT_THEME` / `DEFAULT_ACCENT` 常量（可选）                |
| `src/content/about.md`、`README.md`、`docs/*.md`、`AGENTS.md` | 自我介绍与标题                                                 |
| `index.html`                                                  | `<title>`                                                      |

前端界面（侧栏/顶栏）与托盘提示**动态读取** productName，无需改代码。
</details>

## 4. 换图标（双源，按平台最优）

图标有两个源文件（图形路径相同，仅外框缩放/圆角按平台惯例不同）：

| 源文件                             | 风格                                        | 生成产物                 | 使用平台                         |
| ---------------------------------- | ------------------------------------------- | ------------------------ | -------------------------------- |
| `src-tauri/icons/icon-macos.svg`   | HIG 网格留白（主图形 80.5%，等效圆角 ≈185） | `icon.icns` + Store 磁贴 | macOS Dock / 访达                |
| `src-tauri/icons/icon-windows.svg` | 全出血（图形顶满画布）                      | `icon.ico` + 各级 PNG    | Windows 任务栏/资源管理器、Linux |

修改**两份 SVG 中的图形**（保持各自的外框参数），然后一键重新生成：

```bash
pnpm icons
```

脚本跑两轮 `tauri icon` 并按平台组装：icns/磁贴 = macOS 版，ico/png = Windows 版，
`public/icon.svg` 同步。各平台构建（`pnpm tauri build`）自动取用自己格式的图标。

若 macOS Dock 仍显示旧图标：`killall Dock`。

## 5. 删除示例插件

base 仓库只自带两个内置插件，按需保留/删除：

| 插件          | 说明                                          | 建议         |
| ------------- | --------------------------------------------- | ------------ |
| `hello-world` | 演示全链路（Rust/SQLite/HTTP/布局），5 个工具 | 正式使用可删 |
| `system`      | 数据维护（前端-only，框架内置）               | 保留         |

```bash
# 例如删除示例（保留 system / _template）
rm -rf plugins/hello-world
```

- 前端（侧栏/路由/设置页）与后端（Rust 命令/迁移）均构建期自动扫描，**删除目录即彻底移除**
  （迁移记录为作用域隔离，删除插件不会影响其他插件）
- `_template/` 是你的新工具脚手架，**务必保留**
- 若你的 fork 叠加了需要资源/重依赖的工具，见 [local.md](local.md)（本地层说明）

## 6. 开发你的第一个工具

**两步**：

推荐用脚手架交互生成（自动填好 id/名称/图标与可选的后端、设置、数据库等）：

```bash
pnpm create-plugin
```

或手动复制模板：

```bash
cp -r plugins/_template plugins/my-tools
```

然后编辑 `plugins/my-tools/plugin.json`：

```json
{
  "id": "my-tools",
  "name": "我的工具",
  "tools": [
    {
      "id": "my-tools-hello",
      "name": "你好",
      "description": "我的第一个工具",
      "icon": "Sparkles",
      "entry": "frontend/views/Tool.vue"
    }
  ]
}
```

再改写 `frontend/views/Tool.vue`——完成。侧栏导航、路由、标题、启停开关、设置面板、
错误边界、KeepAlive 全部自动生效，**无需任何手动注册**。

参考实现（都在 hello-world 插件里，删之前可对照）：

| 想做什么                            | 看哪个文件                                 |
| ----------------------------------- | ------------------------------------------ |
| 调用 Rust 命令、读写 SQLite         | `frontend/views/Tool.vue`（Rust 集成大全） |
| 表格 + 分页 + 弹窗增删改 + 导入导出 | `frontend/views/TableTool.vue`             |
| 各种表单控件与校验                  | `frontend/views/FormTool.vue`              |
| 发 HTTP 请求展示结果                | `frontend/views/HttpTool.vue`              |
| 页面布局与三态（空/加载/错误）写法  | `frontend/views/TemplateTool.vue`          |

进阶能力（Rust 命令、数据库表、设置面板、HTTP 采集）的完整步骤
见 [extending.md](extending.md)。

## 7. 能力扩展速查

| 需求          | 入口                                                                                               | 涉及文件                                            |
| ------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Rust 命令     | 插件 `backend/mod.rs` 写 `#[tauri::command]`（构建期自动登记）                                     | `backend/mod.rs`                                    |
| 数据库表      | `frontend/schema.ts` 定义 + `backend/migrations.rs` 追加迁移（scope=插件 id，version 作用域内 +1） | `schema.ts` + `migrations.rs`                       |
| 设置面板      | `useToolSettings` + `plugin.json` 声明 `settings.entry`                                            | `frontend/settings/Settings.vue`                    |
| HTTP 请求     | `core/http`（`http.get/post/getJson`，无 CORS）                                                    | 前端直接调用                                        |
| 导入/导出文件 | `tauri-plugin-dialog`（已接入）+ 自定义 Rust 命令                                                  | 参考 `TableTool.vue` + `hello-world/backend/mod.rs` |

## 8. 主题与品牌

- **主题色**：设置页可切 6 档；若要新增自定义色，需同步三处：
  `assets/index.css` 的 accent class → `core/theme` 的 ACCENTS → `index.html` 防闪白脚本
- **默认主题色 / 默认主题**：改 `core/theme/index.ts` 的 `DEFAULT_ACCENT` / `DEFAULT_THEME`
  （`pnpm scaffold` 也可交互式设置）
- **根字号**：设置页三档（小 13 / 正常 14 / 大 15），全部 UI 等比缩放；
  档位刻度定义见 `assets/index.css` 的注释表
- **暗色基调**：默认暗色（深空蓝灰），亮色同步维护；修改色板请对照
  `index.css` 中 `:root` / `.dark` 两段及 `core/theme` 的 `NATIVE_BG`（原生窗口底色）

## 9. 页面布局

`ToolShell` 提供页头（标题/说明/动作按钮）+ 全幅内容区；居中列/分栏用原生
Tailwind 栅格在页面内自组织（不引入任何自定义栅格概念）：

```vue
<ToolShell title="我的工具" description="说明文字">
  <div class="mx-auto grid w-full grid-cols-12">
    <div class="col-span-12 lg:col-start-3 lg:col-span-8">
      <!-- 居中 8 列，lg 以下自动满幅 -->
    </div>
  </div>
</ToolShell>
```

常用档位：表格满幅（不加类）/ 宽内容 `col-span-10` / 混合 `col-span-8` /
表单 `col-span-6`——居中列必须**偶数跨距**。空态/加载态/错误态的标准写法
见「页面模板」工具（`TemplateTool.vue`）。

## 10. 打包与分发

```bash
pnpm tauri build
```

产物位置（显式 `--target <triple>` 时会多一层 triple 目录）：

- macOS：`src-tauri/target/**/release/bundle/macos/*.app`（开启更新后另有
  `<Product>.app.tar.gz` + `.sig`）
- Windows：`src-tauri/target/**/release/bundle/nsis/*-setup.exe`（msi 视 `--bundles` 而定）

> 发布带**在线更新**的版本（签名、清单、上传、自动发布）见 [release.md](release.md)。

安全说明（可向审查方出示）：安装版**不监听任何本地端口**（前端资源经
进程内自定义协议加载，无 web 服务器）；唯一网络活动是应用主动发起的
HTTP 请求。更新日志与关于页编辑 `src/content/*.md` 即可。

## 11. 常见问题

<details>
<summary><b>macOS 下重启 dev 报「Port 1420 is already in use」</b></summary>

tauri CLI 的清理脚本在 macOS 上可能被创建为无执行位的空文件，导致退出时
vite 进程残留。一次性修复：

```bash
cat > "${TMPDIR}tauri-stop-dev-processes.sh" << 'EOF'
#!/usr/bin/env sh
getcpid() {
    cpids=$(pgrep -P $1|xargs)
    for cpid in $cpids; do
        echo "$cpid"
        getcpid $cpid
    done
}
kill $(getcpid $1)
EOF
chmod 755 "${TMPDIR}tauri-stop-dev-processes.sh"
lsof -ti:1420 | xargs -r kill -9   # 清掉当前残留
```

（上游 tauri#15098，修复 PR #15108 合并并升级 CLI 后可忽略本条）
</details>

<details>
<summary><b>关掉窗口后应用还在运行？</b></summary>

设计行为：关闭窗口默认**隐藏到托盘**（后台常驻，随时唤起）。真正退出：
托盘图标右键 → 退出（macOS 也可 Cmd+Q）。可在设置页关闭「隐藏到托盘」。
</details>

<details>
<summary><b>dev 时控制台报 ELIFECYCLE / 大数字退出码（Windows）</b></summary>

退出应用时 tauri CLI 会强制终止 vite dev server，Windows 上被终止的进程
报告 NTSTATUS 码（如 4294967295）。这是 dev 模式噪音，打包版不存在。
</details>

<details>
<summary><b>调整字号 / 主题后某些界面没跟着变？</b></summary>

所有 UI 均随设置缩放；若发现未跟随的界面，多半是硬编码了 px 数值——
改用 `rem`/语义 token（`text-sm`、`h-9` 等）。
</details>

## 12. 发布前检查清单

- [ ] `pnpm scaffold` 已改名（identifier 确认无误——发布后不可改）
- [ ] 图标已替换并重新生成
- [ ] `src/content/about.md` / `changelog.md` 已写好自己的介绍与首个版本记录
- [ ] 示例插件已删除或禁用
- [ ] `pnpm lint && pnpm build` 通过
- [ ] `pnpm tauri build` 产物在目标平台安装冒烟通过（启动/退出/托盘/最大化无闪烁）

## 13. 日常开发去哪查

- **AGENTS.md**：代码约定、易错点（改代码前必读）
- **extending.md**：架构细节与扩展开发完整步骤（Rust 命令/数据库/设置面板）
- **`plugins/_template/README.md`**：单插件文件级说明

祝开发顺利。
