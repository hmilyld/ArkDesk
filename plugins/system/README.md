# system 内置系统工具

内置的系统维护工具，固定分组（始终排在导航最后）。

## 工具

| 工具     | 入口                                 | 说明                                                                    |
| -------- | ------------------------------------ | ----------------------------------------------------------------------- |
| 数据维护 | `frontend/views/DataMaintenance.vue` | 浏览数据库表结构与数据，支持单元格编辑、新增与删除；含结构（DDL）与分页 |

## 结构

```
frontend/views/DataMaintenance.vue    页面：左表清单 + 右表详情（数据/结构/DDL）
frontend/components/TableList.vue     表清单（搜索 + 选中）
frontend/components/DataGrid.vue      数据网格（分页、单元格编辑）
frontend/components/TableSchema.vue   结构视图
frontend/components/Row*Dialog.vue    行查看/编辑对话框
frontend/shared.ts                    SQL 构建与类型
```

**纯前端插件**：无 `backend/`（不定义命令与迁移），仅消费框架的 `db_*` 命令。目录与命名规范见
[`plugins/README.md`](../README.md)。
