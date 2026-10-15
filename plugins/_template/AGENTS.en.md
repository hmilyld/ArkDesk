# Template Plugin — Development Conventions (AGENTS)

> This plugin's id is template. It is the **upstream template** (the directory starts with `_` and does not participate in registration); `pnpm create-plugin` copies this directory to generate new plugins.
> For the user-facing feature description, see `README.md` in the same directory; for the directory and naming conventions, see [`plugins/README.en.md`](../README.en.md);
> for the UI conventions, see [`docs/en/design.md`](../../docs/en/design.md).

## Tools

`plugin.json` declares the tool **Template Tool** (`frontend/views/Tool.vue`). Multi-feature plugins append entries to the same `tools[]`
array and each provides its own `frontend/views/<Xxx>.vue`.

## Structure

```
plugin.json               Manifest (single source of truth)
README.md / AGENTS.md     Feature description / this file
frontend/
  views/Tool.vue         Tool page (Panel + three states; page structure conventions in the file's header comment)
  settings/Settings.vue  Settings panel (optional, persisted via useToolSettings)
  components/            Private components (none for now; create a subdirectory when there are ≥2 in the same domain)
  composables/           Composables (useXxx.ts, e.g. useCounter)
  lib/                   Pure-logic TS (no Vue dependency, e.g. example.ts)
  shared.ts              Shared constants/types
  schema.ts              Drizzle table definitions (optional)
  setup.ts               Lifecycle hooks (optional)
backend/
  mod.rs                 `#[tauri::command]` commands (`template_` prefix, registered automatically at build time)
  migrations.rs          Migration aggregation (optional)
```

## Conventions

- Commands are written only in `backend/mod.rs`, always return `Result<T, AppError>`, and the function name = the frontend call name (`<id>_` prefix).
- Migrations are written in `backend/migrations.rs`, scope = plugin id, and version increments from 1 within the scope; published ones can only be appended to.
- Pure logic goes in `frontend/lib/`; use `frontend/composables/` only when reactivity/lifecycle is needed (`useXxx.ts`).
- Pages organize modules with `ToolShell` + `Panel`; the empty/loading/error three states use `@/components/native/*`;
  settings items use `SettingsSection` / `SettingsRow` / `SettingsField` from `@/components/settings`.
- Values come only from tokens (see `docs/en/design.md` for details); plugin-private copy goes in `frontend/shared.ts`.

## Validation

```bash
pnpm lint && pnpm build && pnpm test   # includes lint-design and lint-plugins (directory/naming conventions)
```
