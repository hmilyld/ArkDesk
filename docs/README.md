# 文档索引

> 本目录是**规范与指南的唯一真源**，按语言分目录：[`zh/`](zh/)（中文）、[`en/`](en/)（English）。
> 根目录保留门面 [`README.md`](../README.md) / [`README.en.md`](../README.en.md)、约定入口
> [`AGENTS.md`](../AGENTS.md) / [`AGENTS.en.md`](../AGENTS.en.md)（AI 自动加载）与待办
> [`TODO.md`](../TODO.md) / [`TODO.en.md`](../TODO.en.md)。
> 就地文档不搬（保持语境）：[`plugins/README.md`](../plugins/README.md) 与各插件目录内的
> `README.md` / `AGENTS.md`（英文版为同目录的 `*.en.md`）。

| 文档（中文）                                | English                                     | 作用                                                                   | 读者             | 何时看                      |
| ------------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------- | ---------------- | --------------------------- |
| [design.md](zh/design.md)                   | [design.md](en/design.md)                   | 设计规范共享核心：token、组件语义、三态与反馈、文案、反例清单          | 所有写 UI 的人   | 动任何界面前                |
| [design-macos.md](zh/design-macos.md)       | [design-macos.md](en/design-macos.md)       | macOS 平台层：材质与回退、窗口壳、菜单与快捷键、焦点视觉、圆角档       | macOS 相关改动   | 涉及平台差异时              |
| [design-windows.md](zh/design-windows.md)   | [design-windows.md](en/design-windows.md)   | Windows 平台层（Mica Alt 等）；§11 为待真机验证清单                    | Windows 相关改动 | 涉及平台差异时              |
| [design-appendix.md](zh/design-appendix.md) | [design-appendix.md](en/design-appendix.md) | 逐控件 Do/Don't、迁移映射、走查脚本用法（§3.5）                        | UI 实现与走查    | 对照具体控件 / 脚本化截图时 |
| [extending.md](zh/extending.md)             | [extending.md](en/extending.md)             | 架构与扩展开发：目录结构、命令/数据库/设置面板步骤、约定速查、已知限制 | 插件开发者       | 写插件、加后端/数据库时     |
| [lifecycle.md](zh/lifecycle.md)             | [lifecycle.md](en/lifecycle.md)             | 生命周期钩子（SPI）：四个环节、失败策略、fork 接入、`Hold` 语义        | fork 维护者      | 启动/退出环节做拦截时       |
| [start.md](zh/start.md)                     | [start.md](en/start.md)                     | 从零搭自己的桌面软件：改名、图标、删示例、首个工具、打包、FAQ          | base 使用者      | 基于 base 起步时            |
| [release.md](zh/release.md)                 | [release.md](en/release.md)                 | 打包、发布与在线更新：签名、清单规范、workflow、排障                   | 发版的人         | 发版 / 配置更新服务器时     |
| [local.md](zh/local.md)                     | [local.md](en/local.md)                     | 本地层：个人插件、Rust 依赖、字体 / OCR 资源、编译系统依赖             | fork 维护者      | 加个人依赖或资源时          |
| [ownership.json](ownership.json)            | —                                           | 框架 / fork / shared 路径归属清单（回填 base 与判断改动归属的依据）    | 维护者、脚本     | 拆分改动、回填上游时        |

## 相关入口

- [`README.md`](../README.md) / [`README.en.md`](../README.en.md) — 项目介绍与内置工具清单
- [`AGENTS.md`](../AGENTS.md) / [`AGENTS.en.md`](../AGENTS.en.md) — 开发约定、框架能力索引、易错点（AI 工具自动加载）
- [`TODO.md`](../TODO.md) / [`TODO.en.md`](../TODO.en.md) — 待办清单（完成即删行）
- [`plugins/README.md`](../plugins/README.md) / [`plugins/README.en.md`](../plugins/README.en.md) — 插件目录结构规范（`frontend/` 顶层白名单等）

## 维护约定

- 仓库级文档一律放本目录，按语言分子目录：中文 [`zh/`](zh/)、英文 [`en/`](en/)；文件名小写短横线
  （`README.md` 例外，GitHub 只渲染该名）。
- 新增 / 移动文档时：更新本索引表 + 全仓引用；`pnpm lint` 中的 `scripts/check-docs.mjs` 会校验
  索引完整性与链接可解析。
- **中英同步**：新增或改动 `zh/` 下的文档时，须同步更新 `en/` 下的对应文件（文件名保持一致）。
- 不再新增根级 `*.md`（仅 `README.md` / `README.en.md` / `AGENTS.md` / `AGENTS.en.md` / `TODO.md` /
  `TODO.en.md` 六份）。
