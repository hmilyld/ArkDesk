# system Plugin — Development Conventions (AGENTS)

> An upstream built-in plugin: the home of system-maintenance tools. For the directory conventions, see [`plugins/README.en.md`](../README.en.md); for the UI conventions, see [`docs/en/design.md`](../../docs/en/design.md).

## Conventions

- **No backend**: do not add a `backend/`; all data access goes through the framework's `@/core/db` (`db_query_values` / `db_execute`).
- SQL is always parameterized (`$1`), and table/column names undergo identifier whitelist validation (see `frontend/shared.ts`).
- Page structure: modules are wrapped with `Panel` (a left "Tables" panel + a right "Table Detail" panel), with actions placed in the panel header;
  the empty/loading states use `@/components/native/*`, and the search box uses `SearchField`.
- Dangerous operations (deleting rows/tables) must go through `AlertDialog`, and the destructive button must use `variant="destructive"`.
- When adding a system tool, keep the `tools[]` in `plugin.json` and the table in this file / `README.md` in sync.

## Validation

```bash
pnpm lint && pnpm build && pnpm test    # includes lint-design and lint-plugins
```
