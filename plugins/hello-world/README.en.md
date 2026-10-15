# hello-world Example Plugin

A minimal but complete implementation demonstrating the full plugin development path; it can serve as a reference implementation for new plugins.

## Tools

| Tool                               | Entry                             | Description                                                                          |
| ---------------------------------- | --------------------------------- | ------------------------------------------------------------------------------------ |
| Sample Tool                        | `frontend/views/Tool.vue`         | IPC command calls, logging pipeline, SQLite note read/write                          |
| Data Table                         | `frontend/views/TableTool.vue`    | Full CRUD on the `hello_tasks` table: pagination, dialogs, import/export             |
| Data Form                          | `frontend/views/FormTool.vue`     | Form controls and validation, parameterized writes                                   |
| HTTP Request                       | `frontend/views/HttpTool.vue`     | `core/http` raw requests: status, elapsed time, response headers, streaming download |
| Background Tasks and Notifications | `frontend/views/TaskTool.vue`     | `core/tasks` progress/cancel, taskbar integration, event bus, system notifications   |
| Window and System Integration      | `frontend/views/SystemTool.vue`   | Multi-window, open-content dispatch, in-app shortcuts, platform detection            |
| Advanced Data Access               | `frontend/views/DataTool.vue`     | Generic CRUD, transaction atomicity, binary file read/write                          |
| Page Template                      | `frontend/views/TemplateTool.vue` | Standard way to write layout presets and empty/loading/error states                  |

Settings panel: `frontend/settings/Settings.vue` (greeting template, etc.).

## Structure

```
backend/mod.rs          hello_world_* commands (sample/table/form/task/data)
backend/migrations.rs   Migrations for the hello_tasks and hello_notes tables
frontend/views/         Eight tool pages
frontend/settings/      Settings panel
frontend/schema.ts      Table definitions (Drizzle)
frontend/setup.ts       Plugin initialization example
frontend/shared.ts      Shared constants and types
```

For the directory and naming conventions, see [`plugins/README.en.md`](../README.en.md).
