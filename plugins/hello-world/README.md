# hello-world 示例插件

演示框架全链路的样板插件：一个插件内声明多个工具，覆盖 IPC、SQLite、HTTP、后台任务、
系统集成与页面模板。开发约定见同目录 [`AGENTS.md`](AGENTS.md)。

## 工具

| 工具           | 入口                              | 演示内容                          |
| -------------- | --------------------------------- | --------------------------------- |
| 示例工具       | `frontend/views/Tool.vue`         | Rust 命令、日志、SQLite notes     |
| 数据表格       | `frontend/views/TableTool.vue`    | `hello_tasks` 完整 CRUD、导入导出 |
| 数据表单       | `frontend/views/FormTool.vue`     | 表单控件与校验                    |
| HTTP 请求      | `frontend/views/HttpTool.vue`     | `core/http`、流式下载             |
| 后台任务与通知 | `frontend/views/TaskTool.vue`     | `core/tasks` 进度/取消、通知      |
| 窗口与系统集成 | `frontend/views/SystemTool.vue`   | 多窗口、打开内容、快捷键          |
| 数据访问进阶   | `frontend/views/DataTool.vue`     | 事务、二进制文件读写              |
| 页面模板       | `frontend/views/TemplateTool.vue` | 12 栅格布局与空/加载/错误态       |
