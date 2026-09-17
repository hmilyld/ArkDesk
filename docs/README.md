# 文档索引

> 本目录是**规范与指南的唯一真源**。根目录只保留三份文件：
> [`README.md`](../README.md)（门面）、[`AGENTS.md`](../AGENTS.md)（约定入口，AI 自动加载）、
> [`TODO.md`](../TODO.md)（待办）。
> 就地文档不搬（保持语境）：[`plugins/README.md`](../plugins/README.md) 与各插件目录内的
> `README.md` / `AGENTS.md`。

| 文档                                     | 作用                                                                   | 读者             | 何时看                      |
| ---------------------------------------- | ---------------------------------------------------------------------- | ---------------- | --------------------------- |
| [design.md](design.md)                   | 设计规范共享核心：token、组件语义、三态与反馈、文案、反例清单          | 所有写 UI 的人   | 动任何界面前                |
| [design-macos.md](design-macos.md)       | macOS 平台层：材质与回退、窗口壳、菜单与快捷键、焦点视觉、圆角档       | macOS 相关改动   | 涉及平台差异时              |
| [design-windows.md](design-windows.md)   | Windows 平台层（Mica Alt 等）；§11 为待真机验证清单                    | Windows 相关改动 | 涉及平台差异时              |
| [design-appendix.md](design-appendix.md) | 逐控件 Do/Don't、迁移映射、走查脚本用法（§3.5）                        | UI 实现与走查    | 对照具体控件 / 脚本化截图时 |
| [extending.md](extending.md)             | 架构与扩展开发：目录结构、命令/数据库/设置面板步骤、约定速查、已知限制 | 插件开发者       | 写插件、加后端/数据库时     |
| [start.md](start.md)                     | 从零搭自己的桌面软件：改名、图标、删示例、首个工具、打包、FAQ          | base 使用者      | 基于 base 起步时            |
| [release.md](release.md)                 | 打包、发布与在线更新：签名、清单规范、workflow、排障                   | 发版的人         | 发版 / 配置更新服务器时     |
| [local.md](local.md)                     | 本地层：个人插件、Rust 依赖、字体 / OCR 资源、编译系统依赖             | fork 维护者      | 加个人依赖或资源时          |
| [ownership.json](ownership.json)         | 框架 / fork / shared 路径归属清单（回填 base 与判断改动归属的依据）    | 维护者、脚本     | 拆分改动、回填上游时        |

## 相关入口

- [`README.md`](../README.md) — 项目介绍与内置工具清单
- [`AGENTS.md`](../AGENTS.md) — 开发约定、框架能力索引、易错点（AI 工具自动加载）
- [`TODO.md`](../TODO.md) — 待办清单（完成即删行）
- [`plugins/README.md`](../plugins/README.md) — 插件目录结构规范（`frontend/` 顶层白名单等）

## 维护约定

- 仓库级文档一律放本目录，文件名小写短横线（`README.md` 例外，GitHub 只渲染该名）。
- 新增 / 移动文档时：更新本索引表 + 全仓引用；`pnpm lint` 中的 `scripts/check-docs.mjs` 会校验
  索引完整性与链接可解析。
- 不再新增根级 `*.md`（仅 `README.md` / `AGENTS.md` / `TODO.md` 三份）。
