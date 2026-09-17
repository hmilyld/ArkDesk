# ArkDesk

自用桌面工具箱（Mac / Windows），基于 **PocketArk 基础框架**（Tauri 2 + Vue 3 + TypeScript + Tailwind 4）。

## 内置工具

| 分组     | 插件               | 工具                                                          |
| -------- | ------------------ | ------------------------------------------------------------- |
| 常用工具 | `daily-tools`      | 文件转换（→ Markdown）、图片 OCR（PaddleOCR MNN）、**加解密** |
| 投标     | `tender-optimizer` | 招标二轮报价蒙特卡洛测算、测算历史                            |
| 内容创作 | `text2video`       | 文章 → 竖屏滚动短视频（含草稿箱、处理记录）                   |
| 网络工具 | `network-tools`    | 接口测试（HTTP 调试）、请求拦截（本地 MITM 代理）             |
| 系统工具 | `system`           | 数据维护（浏览表结构、编辑数据）                              |

> 各插件的功能说明见 `plugins/<id>/README.md`，开发约定见 `plugins/<id>/AGENTS.md`。

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

> 首次运行会按需下载资源（中文字体 ~32MB、OCR 模型 ~15MB）到 `src-tauri/local-resources/`；
> 大文件不入库，仅许可与 charset 文本入库。`pnpm assets` 可单独预取；国内网络、镜像与 OCR 编译依赖见
> [`docs/local.md`](docs/local.md)。

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
| 待办                               | [`TODO.md`](TODO.md)                        |

## 本仓库与上游

本仓库派生自上游框架 [PocketArk](https://github.com/hmilyld/PocketArk)：框架代码经 `git remote` 的
`upstream` 同步，个人内容（插件、依赖、资源、密钥）集中在「本地层」（见 [`docs/local.md`](docs/local.md)）。
改动归属判定见 [`docs/ownership.json`](docs/ownership.json)，协作流程见 [`AGENTS.md`](AGENTS.md)。
