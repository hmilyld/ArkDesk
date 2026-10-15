# hello-world Plugin — Development Conventions (AGENTS)

> The **reference implementation** of the upstream base repository: when adding new capabilities (commands, migrations, forms, tasks, notifications, multi-window), demonstrate them here first,
> so that forks and documentation have a sample to compare against. For the directory conventions, see [`plugins/README.en.md`](../README.en.md); for the UI conventions, see [`docs/en/design.md`](../../docs/en/design.md).

## Conventions

- Commands are written in `backend/mod.rs` (`hello_world_` prefix, returning `Result<T, AppError>`); logic and tests go in submodules.
- Migrations are written in `backend/migrations.rs` with the scope fixed to `hello-world`; published migrations cannot be changed, only appended to.
- Frontend: pages use `@/components/tool/ToolShell` + `Panel`; settings items use
  `SettingsSection` / `SettingsRow` / `SettingsField` from `@/components/settings`; the three states use `@/components/native/*`.
- Pure logic goes in `frontend/lib/` (none in this plugin for now); shared constants/types go in `frontend/shared.ts`.
- When adding a tool, keep in sync: the `tools[]` in `plugin.json`, the corresponding `frontend/views/*.vue`, this file, and the table in `README.md`.

## Validation

```bash
pnpm lint && pnpm build && pnpm test    # includes lint-design and lint-plugins
```
