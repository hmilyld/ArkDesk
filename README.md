# PocketArk

可复用的**桌面应用基础框架**（Tauri 2 + Vue 3 + TypeScript + Tailwind 4）。
核心设计目标：**新增一个工具的成本尽可能低**——复制模板或 `pnpm create-plugin` 生成骨架，
即可获得完整的导航、路由、启停管理、设置面板、数据库、日志、错误处理、系统集成与在线更新能力；
框架代码（`src/core`、`src-tauri/src`）只引用、不修改。

仓库本身是「框架 + 示例」：`_template`（脚手架）、`hello-world`（教学示例）、`system`（数据维护）。
想用它做你自己的软件：clone 后执行 `pnpm scaffold`，按 [`docs/start.md`](docs/start.md) 走完即可；
叠加个人工具时把内容放进「本地层」（见 [`docs/local.md`](docs/local.md)），以便随上游 base 同步更新。

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
pnpm tauri dev      # 开发（首次编译 Rust，需几分钟）
pnpm tauri build    # 打包
pnpm lint           # eslint + 设计 / 插件结构 / 文档校验
pnpm build          # 类型检查 + 前端构建
pnpm test           # vitest 单测
```

依赖：Rust、Node 20+、pnpm。macOS 需 Xcode Command Line Tools；Linux 需 webkit2gtk 等系统库
（见 [Tauri prerequisites](https://tauri.app/start/prerequisites)）。

> base 本身不包含任何内置资源；若你的 fork 叠加了需要字体 / OCR 等资源的工具，
> 见 [`docs/local.md`](docs/local.md)（本地层说明）。

## 文档

| 想知道什么                         | 看哪里                                      |
| ---------------------------------- | ------------------------------------------- |
| 全部文档索引                       | [`docs/README.md`](docs/README.md)          |
| 从零搭自己的软件（改名/图标/打包） | [`docs/start.md`](docs/start.md)            |
| 架构与扩展开发（命令/数据库/设置） | [`docs/extending.md`](docs/extending.md)    |
| UI 设计规范                        | [`docs/design.md`](docs/design.md) 及平台篇 |
| 打包、发布与在线更新               | [`docs/release.md`](docs/release.md)        |
| 本地层（个人依赖/资源）            | [`docs/local.md`](docs/local.md)            |
| 开发约定、能力索引、易错点         | [`AGENTS.md`](AGENTS.md)                    |

## 协作与上游

本仓库即框架 base。fork 通过 `git remote` 的 `upstream` 同步框架改动；个人内容集中在「本地层」
（见 [`docs/local.md`](docs/local.md)）。改动归属判定见 [`docs/ownership.json`](docs/ownership.json)。
