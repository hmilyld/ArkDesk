# 关于 PocketArk

PocketArk 是一个桌面工具集合（Mac / Windows），基于 Tauri 2 + Vue 3 + TypeScript + Tailwind 4 构建。

定位为**基础项目**：新增一个工具的成本尽量低，扩展走插件约定，框架代码（`src/core`、Rust 侧 db/http/tray）只引用、不修改。

## 特性

- **插件化**：前端构建期自动扫描注册，新工具复制模板即用
- **本地优先**：数据存 SQLite，不出本机
- **框架能力开箱即用**：日志、HTTP（无 CORS）、数据库迁移、托盘
- **原生观感**：暗色指挥台主题、可收起导航、字号三档缩放

## 技术栈

| 层     | 技术                                |
| ------ | ----------------------------------- |
| 桌面壳 | Tauri 2（Rust + sqlx + reqwest）    |
| 前端   | Vue 3 · TypeScript · Tailwind CSS 4 |
| 组件   | shadcn-vue（Reka UI）               |
| 状态   | Pinia · tauri-plugin-store          |
| 数据   | SQLite · Drizzle ORM                |

## 许可

本项目仅用于个人学习与效率工具用途。
