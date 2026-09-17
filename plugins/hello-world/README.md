# hello-world 示例插件

演示插件开发全链路的最小完整实现，可作为新插件的参考实现。

## 工具

| 工具           | 入口                              | 说明                                                   |
| -------------- | --------------------------------- | ------------------------------------------------------ |
| 示例工具       | `frontend/views/Tool.vue`         | IPC 命令调用、日志管道、SQLite 笔记读写                |
| 数据表格       | `frontend/views/TableTool.vue`    | `hello_tasks` 表完整 CRUD：分页、弹窗、导入导出        |
| 数据表单       | `frontend/views/FormTool.vue`     | 表单控件与校验、参数化写入                             |
| HTTP 请求      | `frontend/views/HttpTool.vue`     | `core/http` 原始请求：状态、耗时、响应头、流式下载     |
| 后台任务与通知 | `frontend/views/TaskTool.vue`     | `core/tasks` 进度/取消、任务栏联动、事件总线、系统通知 |
| 窗口与系统集成 | `frontend/views/SystemTool.vue`   | 多窗口、打开内容分发、应用内快捷键、平台探测           |
| 数据访问进阶   | `frontend/views/DataTool.vue`     | 通用 CRUD、事务原子性、二进制文件读写                  |
| 页面模板       | `frontend/views/TemplateTool.vue` | 布局档位与空态/加载态/错误态的标准写法                 |

设置面板：`frontend/settings/Settings.vue`（问候模板等）。

## 结构

```
backend/mod.rs          hello_world_* 命令（示例/表格/表单/任务/数据）
backend/migrations.rs   hello_tasks、hello_notes 表迁移
frontend/views/         八个工具页
frontend/settings/      设置面板
frontend/schema.ts      表定义（Drizzle）
frontend/setup.ts       插件初始化示例
frontend/shared.ts      共享常量与类型
```

目录与命名规范见 [`plugins/README.md`](../README.md)。
