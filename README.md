# PocketArk

可复用的**桌面应用基础框架**（Tauri 2 + Vue 3 + TypeScript + Tailwind 4）。
核心设计目标：**新增一个工具的成本尽可能低**——复制模板或 `pnpm create-plugin` 生成骨架，
即可获得完整的导航、路由、启停管理、设置面板、数据库、日志、错误处理、系统集成与在线更新能力；
框架代码（`src/core`、`src-tauri/src`）只引用、不修改。

仓库本身是「框架 + 示例」：`_template`（脚手架）、`hello-world`（教学示例）、`system`（数据维护）。
想用它做你自己的软件：clone 后执行 `pnpm scaffold`，按 [START.md](START.md) 走完即可；
叠加个人工具时把内容放进「本地层」（见 [LOCAL.md](LOCAL.md)），以便随上游 base 同步更新。

## 技术栈

| 层        | 选型                                                                                    |
| --------- | --------------------------------------------------------------------------------------- |
| 桌面框架  | Tauri 2（tray-icon / macos-private-api feature）                                        |
| 前端      | Vue 3 + TypeScript + Vite 6                                                             |
| 视觉      | Tailwind CSS 4 + shadcn-vue（Reka UI）+ lucide 图标                                     |
| 状态/路由 | Pinia + vue-router（hash，路由由插件注册表驱动）                                        |
| 数据      | Drizzle ORM（前端对象化查询）+ Rust 自建 sqlx 通道（SQLite）+ plugin-store（设置 JSON） |
| 基建      | tauri-plugin-log / window-state、系统托盘、统一错误管道                                 |

## 快速开始

```bash
pnpm install
pnpm tauri dev      # 开发（首次会编译 Rust，需几分钟）
pnpm tauri build    # 打包
pnpm lint           # ESLint
pnpm build          # 类型检查 + 前端构建
pnpm test           # vitest 单测
```

依赖：Rust、Node 20+、pnpm。macOS 需 Xcode Command Line Tools；Linux 需
webkit2gtk 等系统库（见 [Tauri prerequisites](https://tauri.app/start/prerequisites)）。

> base 本身不包含任何内置资源；若你的 fork 叠加了需要字体/OCR 等资源的工具，
> 参见仓库根的 `LOCAL.md`（本地层说明）。

## 应用在线更新

框架内置在线更新（`core/updater` + `src-tauri/src/updater.rs`）：官方
`tauri-plugin-updater` + 自建静态清单，应用内下载并安装。设置页可配置总开关、
更新服务器地址、是否启动时自动检查（**仅在启动后检查一次**，无轮询），关于页显示当前版本。

> 签名公钥固化在 `tauri.conf.json > plugins.updater.pubkey`（信任根，不可由界面修改）；
> 界面只能填服务器地址，且仅接受 **HTTPS**。更新选择为严格 semver 比较（远端版本 >
> 本地版本才提示），因此**版本号必须单调递增**。
>
> base 仓库默认 `bundle.createUpdaterArtifacts: false`，因此**无需签名密钥即可 `tauri build`**。
> 在你自己的 app 中启用更新时：生成密钥 → 将公钥填入 `pubkey` → 开启
> `createUpdaterArtifacts`，再按 [RELEASE.md](RELEASE.md) 发布。

### 发布

版本唯一事实源是 `tauri.conf.json > version`，发版流程：

```bash
pnpm version:bump 0.2.0        # 同步 tauri.conf.json / Cargo.toml / package.json
# 在 src/content/changelog.md 增加一段 ## [0.2.0]
git tag v0.2.0 && git push --tags   # 触发发布 workflow（或 Actions 手动运行）
```

- base 提供**可复用发布流程** `.github/workflows/release-reusable.yml`
  （mac arm64 + win x64 → 签名 → 生成 `latest.json` → 归档 GitHub Release）。
- fork 只需新增一个 caller 并配置密钥/变量，即可获得同样的自动发布能力。

> **完整流程、服务器要求、`latest.json` 规范、workflow 参考与故障排查见
> [RELEASE.md](RELEASE.md)。**

## 目录结构

```
src/
├── main.ts            # 启动引导：日志→异常→设置→主题→插件→路由→挂载
├── router/            # 路由（由插件注册表自动生成，勿手动维护）
├── layouts/           # 布局壳（标题栏/侧栏/错误边界/设置页）
├── core/              # ★ 框架核心，扩展时只引用、不修改
│   ├── logger/        #   日志（双端统一，接管 console）
│   ├── errors/        #   AppError 规范化 + 全局异常 + toast
│   ├── ipc/           #   invoke 封装（命令名受 commands.gen.ts 约束）
│   ├── events/        #   类型化事件总线（emitEvent / onEvent，统一前缀）
│   ├── db/            #   SQLite：Drizzle(kdb) + 通用 CRUD + 事务 + 手写 SQL 兜底
│   ├── http/          #   通用 HTTP 客户端（reqwest，无 CORS；send 高级 + 流式下载 + 代理）
│   ├── theme/         #   亮/暗/跟系统 + 主题色（含自定义主色）+ 字号
│   ├── notify/        #   系统通知（权限 + 设置开关）
│   ├── autostart.ts   #   开机自启（系统为真相源）
│   ├── global-shortcut.ts  # 全局快捷键（唤起窗口，按键录制）
│   ├── shortcuts/     #   应用内快捷键注册表
│   ├── search/        #   全局搜索 / 命令面板（Cmd/Ctrl+K）
│   ├── tasks/         #   后台任务（取消 + 进度事件）
│   ├── taskbar.ts     #   任务栏进度 / Dock 徽标
│   ├── open-with/     #   打开内容统一分发（CLI/深链接/拖拽）
│   ├── windows/       #   多窗口（openAppWindow，label win-*）
│   ├── diagnostics.ts #   诊断报告导出
│   ├── settings-transfer.ts  # 设置导入 / 导出
│   ├── updater/       #   在线更新（检查/下载/安装，公钥内置、地址可配）
│   └── plugins/       #   插件注册表 + useToolSettings
├── components/        # ★ ToolShell（工具页壳）+ ui/（shadcn-vue 生成组件）
├── content/           # 关于 / 更新日志（Markdown，设置页读取）
└── stores/            # Pinia（全局设置）

plugins/               # ★ 你的工具都在这里（每个目录一个插件，前后端同处）
├── _template/         #   新插件模板（复制后按目录内 README.md 改写；_ 开头不注册）
│   ├── plugin.json    #     清单：唯一事实源（元数据/工具项/入口声明）
│   ├── frontend/      #     前端：views/ settings/ components/ composables/ schema.ts setup.ts shared.ts
│   └── backend/       #     后端：mod.rs（#[tauri::command] 命令）+ migrations.rs（可选）
├── hello-world/       #   示例插件（多功能插件样板）
└── system/            #   系统工具（数据维护，前端-only）

src-tauri/
├── build.rs           # 扫描 plugins/ 生成命令注册与迁移聚合
├── framework-commands.json  # 框架命令清单（单一事实源，供 build.rs 与前端命令名生成）
└── src/
    ├── lib.rs         # 组装入口：插件注册 / 托盘 / 菜单 / 单实例 / 关窗行为
    ├── error.rs       # AppError（所有命令返回 Result<T, AppError>）
    ├── events.rs      # 事件名常量（与前端 core/events 同步）
    ├── db.rs          # SQLite：sqlx 池 + 作用域迁移 + 事务 + 备份/恢复/重置
    ├── http.rs        # HTTP 客户端（reqwest；send 高级 + 流式下载 + 代理）
    ├── updater.rs     # 在线更新
    ├── tasks.rs       # 后台任务（取消令牌 + 进度事件）
    ├── open.rs        # 打开内容（CLI/深链接/二次启动 → app://open）
    ├── menu.rs        # 原生应用菜单（→ app://menu）
    ├── diagnostics.rs # 诊断报告导出
    ├── files.rs       # 通用文件读写命令
    ├── tray.rs        # 系统托盘
    └── plugins/mod.rs # include 构建期生成的插件注册（勿手改）

scripts/               # scaffold / create-plugin / gen-icons / gen-commands / prepare / bump-version / release
LOCAL.md               # 本地层说明（fork 专属；base 为约定模板）
```

## 扩展开发指南

### 新增一个工具（最常见，两步）

可用脚手架交互生成骨架（推荐）：

```bash
pnpm create-plugin      # 交互式：插件 id/名称/图标，以及后端/设置/数据库等可选部分
```

或手动复制模板：`plugins/_template` → `plugins/timestamp-tool`。

以新增一个「时间戳转换」工具为例：

1. **准备目录**：`pnpm create-plugin` 生成，或复制 `plugins/_template` → `plugins/timestamp-tool`
   （结构：`plugin.json` / `frontend/{views,settings,components,schema.ts,setup.ts,shared.ts}` /
   `backend/{mod.rs,migrations.rs}`，无内容的文件保留注释占位即可，或删除）
2. **改写 plugin.json**：`id`（全局唯一、kebab-case、= 目录名）、
   `tools[].name` / `tools[].icon`（lucide 图标名）/ `tools[].entry`（如 `frontend/views/Tool.vue`）
3. **实现前端**：编辑 `frontend/views/Tool.vue`——用 `ToolShell` 包裹内容，居中列/分栏用
   原生 Tailwind 栅格（`grid grid-cols-12` + `col-start-*` / `col-span-*`），
   细节见 `_template` 目录内 README.md 与 hello-world 各工具页

完成——**前端与后端均无需手动注册**。前端（导航/路由/标题/启停/设置面板/错误边界/KeepAlive）
由 Vite 构建期扫描 `plugins/*/frontend/**`；后端（Rust 命令/迁移）由 `src-tauri/build.rs`
扫描 `plugins/*/backend/mod.rs` 自动登记。

### 需要后端命令（可选）

在插件 `backend/mod.rs` 直接写命令即可（**构建期自动扫描，无需改任何框架文件**；
命令须定义在 `backend/mod.rs`，函数名 = 前端调用名，带 `<tool_id>_` 前缀防冲突）：

```rust
#[tauri::command]
pub fn timestamp_tool_now() -> Result<i64, crate::error::AppError> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| crate::error::AppError::custom("TIME_ERROR", e.to_string())))
}
```

前端调用：`await ipc<number>("timestamp_tool_now")`。所有命令返回 `Result<T, AppError>`，
错误会自动进入日志与全局 toast。

> **注意**：命令须定义在 `plugins/<id>/backend/mod.rs`，且命令名带 `<plugin_id>_` 前缀
> （`build.rs` 会在构建期检测重名）；迁移须放 `backend/migrations.rs`。
> 完整约定、构建期机制与陷阱见 AGENTS.md「自动注册的约定与注意事项」。

### 需要数据库表（可选）

数据层基于 **Drizzle ORM**（对象化类型安全查询）+ Rust 侧自建 sqlx 通道执行。

1. **定义 schema**（JPA Entity 的等价物）：插件 `frontend/schema.ts`
   （参考 `hello-world/frontend/schema.ts`），**自动聚合，无需手动注册**
   （Vite 构建期扫描 `plugins/*/frontend/schema.ts`；表对象直接 import 使用即得类型）：
   ```ts
   export const myItems = sqliteTable('my_items', {
     id: integer('id').primaryKey({ autoIncrement: true }),
     title: text('title').notNull(),
     createdAt: text('created_at')
       .notNull()
       .default(sql`(datetime('now','localtime'))`),
   });
   export type MyItem = typeof myItems.$inferSelect; // 行类型，勿手写 interface
   ```
2. **追加迁移**（插件 `backend/migrations.rs`，scope = 插件 id，version 在作用域内从 1 递增）：
   参考 `hello-world/backend/migrations.rs`，建表 DDL 的列默认值与 schema 对齐；
   构建期自动聚合，启动时按 `(scope, version)` 幂等执行。若插件此前用过旧的全局版本号，
   在 `plugin.json` 的 `legacyMigrations` 声明「旧全局版本 → 新本地版本」以桥接旧库。
3. **查询**用 Drizzle（类型安全、零 SQL 字符串）：
   ```ts
   import { kdb } from '@/core/db';
   import { eq, desc } from 'drizzle-orm';
   import { myItems } from './schema';

   const rows = await kdb
     .select()
     .from(myItems)
     .where(eq(myItems.title, 'x'))
     .orderBy(desc(myItems.id));
   const [{ id }] = await kdb.insert(myItems).values({ title: 'x' }).returning({ id: myItems.id });
   await kdb.update(myItems).set({ title: 'y' }).where(eq(myItems.id, id));
   await kdb.delete(myItems).where(eq(myItems.id, id));
   ```
4. **快速 CRUD**（等值条件场景）用 `core/db` 通用方法，复杂聚合/JOIN 退回手写 SQL：
   ```ts
   import { db } from '@/core/db';
   await db.insert('notes', { content: 'hi' }); // 标识符白名单校验
   const rows = await db.findAll<Row>('notes', { where: { ok: 1 }, orderBy: 'id DESC', limit: 20 });
   const rows2 = await db.select<Row>('SELECT * FROM t WHERE a > $1', [value]);
   ```

要点：表名/列名标识符白名单校验 + 值参数化；**拿自增 id 用 `.returning()`**
（proxy 的 run 路径不回传 lastInsertId）；时间戳列 default 与迁移 DDL 对齐；
BLOB 列以 base64 返回；多语句原子写入用 `runInTransaction([{ sql, params }])`
（Rust 侧单事务执行，任一失败整体回滚）。

### 需要设置面板（可选）

1. 插件目录新建 `frontend/settings/Settings.vue`，用 `useToolSettings` 读写配置：
   ```ts
   const config = useToolSettings<MyConfig>('timestamp-tool', { format: '秒' });
   config.value.format = '毫秒'; // 修改即自动持久化
   ```
2. `plugin.json` 声明：`"settings": { "entry": "frontend/settings/Settings.vue" }`

配置存储在 `settings.json` 的 `tools.<toolId>` 命名空间，与 defaults 深合并
（新增配置字段自动获得默认值）。参考 `hello-world/frontend/settings/Settings.vue`。

### 多功能插件

一个插件目录可在 `plugin.json` 的 `tools` 数组声明多个工具项（参考 `hello-world/plugin.json`）：各工具项独立导航、
独立启停、独立设置 tab，共享目录内的代码 / Rust 模块 / 迁移。

## 约定速查表

| 约定      | 说明                                                                                                                                                          |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 插件清单  | `plugin.json` 是唯一事实源；**前端（Vite）与后端（build.rs）均构建期自动注册，无需手动登记**                                                                  |
| 插件目录  | `frontend/`（views/settings/components/schema.ts/setup.ts）+ `backend/`（mod.rs/migrations.rs）；`_` 开头目录不注册                                           |
| 工具 id   | 全局唯一，kebab-case，= 路由 `/tool/:id`；插件内以插件 id 为前缀                                                                                              |
| 插件排序  | 分组序 = 组内最小 `order`；组内工具序 = `order*1000 + tools 数组下标`；缺省 100 后按目录名字母序                                                              |
| Rust 命令 | 定义在 `backend/mod.rs`；函数名 = 前端调用名，`<tool_id>_` 前缀防冲突                                                                                         |
| 命令签名  | 一律 `Result<T, AppError>`，参数 snake_case（前端传 camelCase 自动映射）                                                                                      |
| SQL       | 优先 `kdb`（Drizzle 对象化查询）；快速 CRUD 用 `db.insert/findAll` 等；手写 SQL 一律 `$1` 参数化                                                              |
| 自增 id   | insert/update/delete 用 `.returning({ id: table.id })` 拿返回值                                                                                               |
| 迁移版本  | `migration(scope, version, ...)` 按插件作用域隔离（scope=插件 id，version 各自从 1 递增），已发布迁移**不可修改**（只能追加）；旧库用 `legacyMigrations` 桥接 |
| BLOB 列   | 经 sqlx 通道以 **base64 字符串**返回（无损），前端需自行 `atob` 解码                                                                                          |
| 工具配置  | `useToolSettings(toolId, defaults)`，勿自行另建存储                                                                                                           |
| HTTP 请求 | 一律走 `core/http`（`http.getJson/postJson`…，高级用 `http.send`），禁止 webview 内 fetch 跨域采集                                                            |
| 日志      | `ctx.logger` / `logger`，禁止裸 `println!`                                                                                                                    |
| 样式      | shadcn-vue 语义色（`bg-primary` 等）跟随主题色，勿硬编码色值                                                                                                  |
| 生成组件  | `src/components/ui/**` 为 CLI 生成物，可改样式但勿改结构/逻辑                                                                                                 |

## 架构约定（框架行为，扩展时免费获得）

- **启动顺序**：日志 → 异常处理 → 设置加载 → 主题 → 插件注册（`setup(ctx)` 钩子）→ 路由 → 挂载
- **错误管道**：Rust `AppError{code,message}` → 前端统一转换 → 未捕获时自动日志 + toast；
  工具页渲染崩溃由错误边界隔离（占位页 + 重试），不影响框架
- **日志**：Rust 与前端统一写入应用日志目录（macOS 为 `~/Library/Logs/<应用 identifier>/`），
  console 已被接管；级别运行时可调（设置页）
- **状态保持**：工具切换默认 KeepAlive（切回不丢输入），`keepAlive: false` 可退出
- **关窗行为**：默认隐藏到托盘（设置可改）；窗口位置每次启动居中于鼠标所在屏幕
- **主题**：亮/暗/跟系统（默认暗色）+ 6 档主题色（`core/theme` + `assets/index.css` 的 accent class）；
  根字号三档缩放（小 13 / 正常 14 / 大 15），控件与文字等比联动
- **HTTP**：Rust reqwest 全局单例（rustls、cookie 会话、10 次重定向、30s 超时、10MB 响应上限），
  前端 `http.getJson<T>(url)` 即可采集 JSON 接口；`http.send()` 为高级原语（可参数化重定向 / SSL / 超时 / 代理 /
  cookie 模式，支持 multipart 上传、二进制响应、保序重复响应头与取消），使用与采集隔离的 Client
- **安全信任边界**：`db_*` / `http_request` / `hello_world_greet` 等 App 自有命令
  **不经过 capability 权限系统**（Tauri v2 仅门控核心与插件命令），webview 内任意代码均可执行
  任意 SQL / 发起任意请求。因此必须保持 CSP 严格、**不加载任何远程内容**，插件（本地代码）
  作为可信边界对待

## 已知限制

- macOS 窗口缩放时 webview 内容存在轻微渲染滞后（WKWebView 跨进程合成的固有限制，
  已通过 layerContentsRedrawPolicy 与暗色窗口底缓解）
- HTTP 响应体按 UTF-8 解码（非 UTF-8 页面如 GBK 为后续扩展点）；二进制资源请用
  `http.download()` 流式下载到文件
- 打包构建告警：主包已拆分，但 `@lucide/vue` 全量图标使 `vendor-ui` 分块较大
  （按名解析图标的固有开销）
