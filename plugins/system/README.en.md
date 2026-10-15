# system Built-in System Tools

Built-in system maintenance tools, in a fixed group (always placed last in the navigation).

## Tools

| Tool             | Entry                                | Description                                                                                                              |
| ---------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| Data Maintenance | `frontend/views/DataMaintenance.vue` | Browse database table schemas and data, with cell editing, insertion, and deletion; includes schema (DDL) and pagination |

## Structure

```
frontend/views/DataMaintenance.vue    Page: left table list + right table detail (data/schema/DDL)
frontend/components/TableList.vue     Table list (search + selection)
frontend/components/DataGrid.vue      Data grid (pagination, cell editing)
frontend/components/TableSchema.vue   Schema view
frontend/components/Row*Dialog.vue    Row view/edit dialogs
frontend/shared.ts                    SQL construction and types
```

**Frontend-only plugin**: no `backend/` (defines no commands or migrations), and only consumes the framework's `db_*` commands. For the directory and naming conventions, see
[`plugins/README.en.md`](../README.en.md).
