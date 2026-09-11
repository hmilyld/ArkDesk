# ArkDesk — AGENTS.md

Tauri 2 + Vue 3 + TypeScript + Tailwind 4 的桌面工具集合（Mac / Windows）。
定位为**基础项目**：新增一个工具的成本尽量低，扩展走插件约定，框架代码（`src/core`、`src-tauri/src` 的 db/http/tray/updater/menu/tasks/open/…）只引用、不修改。

## 常用命令

```bash
pnpm dev            # vite 开发（前端）
pnpm tauri dev      # Tauri 开发（先编译 Rust，需几分钟）
pnpm build          # vue-tsc --noEmit + vite build（改前端后必须跑）
pnpm preview        # 预览 vite 构建产物
pnpm test           # vitest 单测（core 纯逻辑）
pnpm test:watch     # vitest 监听模式
pnpm lint           # eslint（改前端后必须跑）
pnpm lint:fix
pnpm format         # prettier --write .（全量）
pnpm format:check
pnpm env:cpp        # 导出 macOS CXXFLAGS（本地层 ocr-rs 编译用，见 LOCAL.md）
pnpm lint:rs        # cargo clippy -D warnings（改 Rust 后必须跑）
pnpm fmt:rs         # cargo fmt
pnpm test:rs        # cargo test（http 冒烟测试默认 ignore，需网络）
pnpm tauri build    # 打包
pnpm scaffold       # clone 后一键改名（交互式：显示名/标识/主题色等）
pnpm create-plugin  # 交互式生成新插件骨架（plugins/<id>/，前后端自动注册）
pnpm icons          # 双源生成全平台图标（macOS HIG 网格 icns + Windows 全出血 ico/png）
pnpm version:bump   # 升版本号（同步 tauri.conf.json / Cargo.toml / package.json）
pnpm release        # 生成更新清单 latest.json + 校验和（发布用）
```

约定：改前端跑 `pnpm lint && pnpm build`；改 Rust 跑 `pnpm lint:rs && pnpm fmt:rs && pnpm test:rs`。

## 本地层（fork 专属）

框架自身**不下载任何资源、不含个人工具**。若你的 fork 叠加了个人工具（字体/OCR 等
资源、额外 Rust 依赖、额外权限），约定集中在「本地层」，详见仓库根 `LOCAL.md`：
`src-tauri/local-resources/`（资源）、`scripts/local/`（下载脚本）、
`Cargo.toml` 的 `local plugin deps` 段、`capabilities/local.json`（如有）。

## 插件扩展（最常见任务）

新增工具 = `pnpm create-plugin`（交互式生成骨架，推荐），或复制 `plugins/_template/` → `plugins/<plugin-id>/`，改写 `plugin.json`（唯一事实源），实现 `frontend/views/*.vue`。**前端与后端均构建期自动注册**，无需任何手动登记。

- 前端：Vite 扫描 `plugins/*/frontend/**`（views / settings / setup.ts / schema.ts）。
- 后端：`src-tauri/build.rs` 扫描 `plugins/*/backend/mod.rs`，解析 `#[tauri::command]` 自动登记命令，读取 `migrations.rs` 聚合迁移。命令须定义在 `backend/mod.rs`，函数名 = 前端调用名（`<plugin_id>_` 前缀），一律返回 `Result<T, AppError>`。
- 迁移按**作用域隔离**：`migration(scope, version, ...)`，scope = 插件 id，version 在作用域内从 1 递增。旧库历史版本用 `plugin.json` 的 `legacyMigrations`（旧全局版本 → 新本地版本）一次性桥接登记。
- `_` 或 `.` 开头目录不参与注册（如 `_template`）。

### 自动注册的约定与注意事项

- **命令必须写在 `plugins/<id>/backend/mod.rs`**：`build.rs` 只扫描该文件解析 `#[tauri::command]`；其余 `.rs` 作为它的子模块（`pub mod xxx;`）。命令可独占一行或与 `#[tauri::command]` 同行，属性带参数（如 `#[tauri::command(rename_all = "camelCase")]`）亦可。
- **命令名全局唯一，必须带 `<plugin_id>_` 前缀**：重名会让生成的 `generate_handler!` 出现重复 match 分支；`build.rs` 会在构建期检测并直接报错。
- **迁移必须放 `plugins/<id>/backend/migrations.rs`**，导出 `pub fn all() -> Vec<Migration>`；`migration(scope, version, ...)` 中 scope = 插件 id，version 在作用域内从 1 递增。**已发布迁移不可修改，只能追加**。
- **旧库桥接**：插件若曾用过旧的全局版本号，在 `plugin.json` 声明 `legacyMigrations: { "旧全局版本": 新本地版本 }`；启动时按映射登记进 `plugin_migrations`（不重复执行 DDL）。
- **框架自有命令**列表在 `src-tauri/build.rs` 的 `FRAMEWORK_COMMANDS`：新增框架命令（db/http/updater 等）需在此追加，否则不会被注册。
- **前端入口路径**：`plugin.json` 的 `tools[].entry` / `settings.entry` 相对插件目录且**含 `frontend/` 前缀**（如 `frontend/views/Tool.vue`）。
- **构建期机制**：`build.rs` 以 `CARGO_MANIFEST_DIR` 定位仓库根 `plugins/`，用绝对 `#[path]` 引入各插件后端，并 `cargo:rerun-if-changed=plugins`；从 `src-tauri` 直接 `cargo build` 亦可（无需经过 pnpm）。
- **新增插件后**：前端 glob 变化后若未自动出现，重启 `pnpm dev`/`pnpm tauri dev`。
- **模板不被编译**：`_template` 因 `_` 前缀被前后端扫描跳过；复制后须把命令前缀改为新插件 id（`template_xxx` → `<id>_xxx`）。
- **Tailwind**：插件前端在仓库根 `plugins/`（`src/` 之外），`assets/index.css` 已加 `@source '../../plugins'`；勿移除。
- **平台说明**：`#[path]` 以正斜杠生成，Windows 亦应可用，但**尚未在 Windows 实测**。

## 应用更新（框架能力）

框架内置在线更新：前端 `src/core/updater`，Rust `src-tauri/src/updater.rs`（`updater_check` / `updater_install` / `updater_restart`）。采用官方 `tauri-plugin-updater` + 自建静态清单。

- **公钥固化**在 `tauri.conf.json > plugins.updater.pubkey`（信任根，界面不可改）；界面只配服务器地址，**仅 HTTPS**。
- **仅启动后检查一次**（`updateEnabled` + `updateAutoCheck` 控制，无轮询）；`initUpdater()` 由 `main.ts` 调用。
- Rust 侧运行时用 `updater_builder().endpoints([...])` 覆盖端点；下载进度经事件 `updater://progress` 回传。
- 设置 key：`updateEnabled` / `updateServerUrl` / `updateAutoCheck` / `updateLastCheckAt`。
- 版本唯一事实源 = `tauri.conf.json > version`；发版前 `pnpm version:bump x.y.z` 同步三处。
- 更新选择为严格 semver（远端 > 本地），**版本号必须单调递增**；清单 `version` 须与构建版本一致。
- 私钥（`~/.tauri/arkdesk.key`）不入库；构建需 `createUpdaterArtifacts` + `TAURI_SIGNING_PRIVATE_KEY_PATH`。
- 服务端清单 `latest.json` 格式与完整发布流程见 README「应用在线更新」。

## 框架能力索引

`src/core/` 按能力分模块，插件只引用、不修改；Rust 侧同名能力见 `src-tauri/src/`。

| 能力               | 前端入口                          | Rust                             | 说明                                                                  |
| ------------------ | --------------------------------- | -------------------------------- | --------------------------------------------------------------------- |
| IPC                | `@/core/ipc`                      | `generate_handler`（构建期生成） | `ipc<T>(cmd, args)`，命令名受 `commands.gen.ts` 约束；禁止裸 `invoke` |
| 事件总线           | `@/core/events`                   | `src/events.rs`                  | 类型化 `emitEvent/onEvent`，统一事件前缀                              |
| 日志               | `@/core/logger`                   | `log`                            | 双端统一，接管 console                                                |
| 错误               | `@/core/errors`                   | `error.rs`                       | `AppError{code,message}` 规范化 + 全局提示                            |
| 数据库             | `@/core/db`                       | `db.rs`                          | Drizzle(kdb) + 通用 CRUD + 作用域迁移                                 |
| HTTP               | `@/core/http`                     | `http.rs`                        | reqwest，无 CORS；`download()` 流式下载 + 进度；代理设置驱动          |
| 主题               | `@/core/theme`                    | `set_window_appearance`          | 亮/暗/跟系统 + 主题色 + 字号                                          |
| 插件注册           | `@/core/plugins`                  | `plugins/mod.rs`（生成物）       | 前后端构建期自动注册                                                  |
| 在线更新           | `@/core/updater`                  | `updater.rs`                     | tauri-plugin-updater + 静态清单                                       |
| 系统通知           | `@/core/notify`                   | `tauri-plugin-notification`      | 权限申请 + 设置开关                                                   |
| 开机自启           | `@/core/autostart`                | `tauri-plugin-autostart`         | 真相源在系统                                                          |
| 数据库事务         | `@/core/db` 的 `runInTransaction` | `db.rs`                          | 单事务批量执行，失败回滚                                              |
| 数据备份/恢复/重置 | 设置页「数据」                    | `db.rs`                          | 恢复/重置后自动重启                                                   |
| 设置导入/导出      | `@/core/settings-transfer`        | `files.rs`                       | 导出 JSON；导入后重启生效                                             |
| 诊断报告           | `@/core/diagnostics`              | `diagnostics.rs`                 | 环境信息 + 最近日志                                                   |
| 单实例             | —                                 | `tauri-plugin-single-instance`   | 二次启动唤起主窗口 + 转发参数                                         |
| 应用内快捷键       | `@/core/shortcuts`                | —                                | `registerShortcut('mod+k', fn)`                                       |
| 全局快捷键         | `@/core/global-shortcut`          | `tauri-plugin-global-shortcut`   | 设置项驱动，唤起主窗口                                                |
| 全局搜索/命令面板  | `@/core/search`                   | —                                | `Cmd/Ctrl+K` 聚合导航与工具                                           |
| 任务栏进度/徽标    | `@/core/taskbar`                  | Tauri Window API                 | 进度 0–100 / Dock 徽标                                                |
| 后台任务           | `@/core/tasks`                    | `tasks.rs`                       | 取消令牌 + `task://` 进度事件                                         |
| 打开内容           | `@/core/open-with`                | `open.rs`                        | CLI / 深链接 / 拖拽统一分发（`onOpenFiles`）                          |
| 多窗口             | `@/core/windows`                  | `capabilities/windows.json`      | `openAppWindow()`，label `win-*`                                      |
| 原生应用菜单       | `@/core/events`（`app://menu`）   | `menu.rs`                        | macOS menubar / Win 窗口菜单；项经事件转发前端                        |
| 平台探测           | `@/core/platform.ts`              | —                                | `isMac`（样式与快捷键修饰键差异）                                     |

> 新增框架能力时在本表登记，并在 README「架构约定」补充说明。

## 关键约定

- **IPC**：前端一律 `await ipc<T>(cmd, args)`（`@/core/ipc`），禁止裸 `invoke`。命令名由 `scripts/gen-commands.mjs` 生成到 `src/core/ipc/commands.gen.ts`（读取 `src-tauri/framework-commands.json` 并扫描插件 `#[tauri::command]`，框架+插件命令联合类型），非注册命令会在类型检查期报错；该文件为生成物（随 `prepare`/构建更新，勿手改）。
- **错误**：Rust `AppError{code, message}`，code 与前端 `core/errors` 的 `ErrorCode` 对齐。
- **数据库**：对象化查询用 `kdb`（Drizzle sqlite-proxy）；快速 CRUD 用 `@/core/db` 的 `db.insert/findAll` 等；手写 SQL 一律 `$1` 参数化。拿自增 id 用 `.returning()`。BLOB 列经通道以 base64 返回（前端自行解码）。表名/列名做标识符白名单校验。
- **HTTP**：走 `@/core/http`（Rust reqwest，无 CORS），禁止 webview 内 `fetch` 采集。
- **日志**：用 `@/core/logger` 的 `logger` 或插件 `ctx.logger`，禁止裸 `println!`。
- **图标**：只允许 `@lucide/vue`（`lucide-vue-next` 已弃用，勿再引入）。
- **样式**：shadcn-vue 语义色（`bg-primary` 等），禁止硬编码色值；已有 `text-success/warning/info`、`bg-console` 等 token。
- **代码风格**：prettier 单引号、100 列、尾逗号 es5；提交前跑 `pnpm format`。
- **主题**：主题色/亮暗在 `core/theme` + `assets/index.css` 的 accent class，新增主题色需同步三处（CSS / ACCENTS / index.html 内联防闪白脚本）。

## 易错点（已修复过，勿回退）

- **日志回环**：`core/logger` 用 `attachLogger` + `echoing` 护栏。不要改回 `attachConsole` 或让 console 接管函数在回显期转发日志——会无限循环。
- **keepAlive**：`MainLayout` 按 `route.meta.keepAlive !== false` 分支缓存；清单里 `keepAlive: false` 才会不缓存。
- **排序**：插件分组序 = 组内最小 `order`；`manifest.order` 参与排序，勿改回目录名排序。
- **安全边界**：`db_*` / `http_request` 等 App 命令不经过 capability 门控，webview 内任意代码可执行任意 SQL/请求。必须保持 CSP 严格、不加载远程内容。
- **主题外观联动**：`applyTheme` 会同步 NSWindow.appearance / NSWindow.backgroundColor（macOS）。强制 darkAqua/aqua 会把 webview 的 `prefers-color-scheme` 钉死在对应外观上——「跟随系统」必须传 `dark: null` 清除覆盖（见 `set_window_appearance` 三态设计）。
- **工具页布局**：`ToolShell` 只提供页头 + 全幅容器，居中列 / 分栏在页面内用原生 Tailwind 栅格（`grid grid-cols-12` + `col-start-*` / `col-span-*`，居中列用偶数跨距），禁止 `mx-auto max-w-*` 居中容器；新 registry 组件若用裸 `data-checked:`/`data-open:` 等布尔变体，需转成 `data-[state=...]:`（reka-ui 2.10 只输出后者）。
- **macOS dev 端口占用**：tauri CLI（≤2.11.4）退出时靠 `$TMPDIR/tauri-stop-dev-processes.sh` 清理 dev server 进程树，但实测该脚本被创建为 **0 字节 + 0o644**（内容与权限写入双双静默失败），`!exists()` 守卫又永不重建 → vite 成为孤儿 → 下次 `tauri dev` 报端口占用。仅 chmod 不够（空脚本 = no-op），须写入原版内容 + 执行位（一次即可）：

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
  ```

  紧急兜底：`lsof -ti:1420 | xargs -r kill -9`。上游 tauri#15098，修复 PR #15108（open）合并并升级 CLI 后可移除此条。

- **本地层编译依赖（ocr-rs 等）**：这类重依赖属 fork 本地层，相关编译问题（macOS `CXXFLAGS`、Windows libclang）见 `LOCAL.md`；base 不含这些依赖，无此问题。

## 目录速览

```
src/core/            # 框架核心（logger/errors/ipc/db/http/theme/plugins），扩展只引用
src/components/tool/ # ToolShell 工具页壳（页头+全幅容器）
src/components/ui/   # shadcn-vue 生成组件（CLI 管理）
src/content/         # 关于/更新日志 Markdown（设置页读取）
scripts/             # scaffold / create-plugin / gen-icons / gen-commands / prepare / bump-version / release
  local/             # ★本地层脚本（fork-owned；base 无）：资源下载、CI 跳过
plugins/<id>/        # ★工具插件（前后端同处）：plugin.json + README.md
  frontend/          #   views/ settings/ components/ composables/ schema.ts setup.ts shared.ts
  backend/           #   mod.rs（#[tauri::command] 命令）+ migrations.rs + 其余 .rs / 资源
src/layouts/         # 布局壳（标题栏/侧栏/错误边界/设置页）
src/stores/          # Pinia（全局设置）
src-tauri/src/       # db.rs（sqlx+作用域迁移）、http.rs、tray.rs、updater.rs、tasks.rs、open.rs、menu.rs、diagnostics.rs、files.rs、plugins/mod.rs（include 生成物）
src-tauri/build.rs   # 扫描 plugins/ 生成命令注册与迁移聚合
src-tauri/local-resources/  # ★本地层资源（fork-owned；base 无）
LOCAL.md             # ★本地层说明（fork 专属）
```

生成的 shadcn 组件 `src/components/ui/**` 由 CLI 管理：可改样式，勿改结构/逻辑。
