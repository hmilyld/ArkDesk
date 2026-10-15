# tender-optimizer Plugin — Development Conventions (AGENTS)

> For feature documentation see `README.en.md` in the same directory; project-level docs are only referenced.

## Backend

- Commands are written in `backend/mod.rs`, all with the `tender_optimizer_*` prefix, returning `Result<T, AppError>`.
- Submodules: `models.rs` (DTO/table models), `engine.rs` (Monte Carlo calculation), `excel.rs` (template import/export),
  `migrations.rs` (scope = `tender-optimizer`).
- The template `backend/template.xlsx` ships with the plugin and is read via `include_bytes!`.
- The calculation must be **reproducible**: the random seed is determined by the input; do not introduce a global random source.

## Frontend

- `views/Calculator.vue` (calculation), `views/History.vue` (history).
- Shared types/defaults are in `shared.ts`; database tables in `schema.ts`; the startup hook in `setup.ts`.
- Command parameters and return fields: Tauri v2 command parameters are camelCase; returned struct fields keep the naming at declaration
  (mostly snake_case in this project, and the frontend reads them under the corresponding naming).

## Verification

```bash
pnpm lint && pnpm build
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs
```
