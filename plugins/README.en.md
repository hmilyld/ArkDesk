# Plugin Development Overview

> The single entry-point document for plugin authors and AI. The **directory and naming conventions** are defined here; the extension mechanism and the command/migration conventions are in the "Plugin Extensions" section of the root
> [`AGENTS.en.md`](../AGENTS.en.md), and the UI conventions are in [`docs/en/design.md`](../docs/en/design.md) plus the platform docs.

## 1. Directory Conventions

```
plugins/<plugin-id>/
├── plugin.json            Manifest: single source of truth (shared by frontend and backend scanning)
├── README.md              Feature description (for users)
├── AGENTS.md              Development conventions (for AI/maintainers, only present if written)
├── frontend/
│   ├── views/             Tool pages: one PascalCase.vue per tools[] entry
│   ├── settings/          Settings panel (optional, declared in plugin.json)
│   ├── components/        Plugin-private components (optional; same feature domain ≥2 → components/<feature>/)
│   ├── composables/       Composables (optional, useXxx.ts)
│   ├── lib/               Pure-logic TS (optional, no Vue dependency): lib/<name>.ts or lib/<feature>/*.ts
│   ├── shared.ts          Shared constants/types within the plugin (optional)
│   ├── schema.ts          Drizzle table definitions (optional, imported by code)
│   └── setup.ts           Lifecycle hooks (optional, scanned at build time)
└── backend/
    ├── mod.rs             #[tauri::command] commands (**only allowed here**)
    ├── migrations.rs      Migration aggregation (exports `all()`)
    ├── <feature>/         Multi-file feature domain (≥2 files): <feature>/mod.rs + the remaining .rs
    ├── <single>.rs        Single-file feature domain
    └── <asset>            Asset files (recommended alongside their feature domain directory, e.g. template xlsx)
```

**Hard rules**

- The `frontend/` top level **only permits** the whitelist: `views/ settings/ components/ composables/ lib/ shared.ts schema.ts setup.ts`.
  Anything else must go into one of those directories or `lib/`.
- Pure logic (no Vue dependency) goes in `frontend/lib/`; use `frontend/composables/` only when reactivity/lifecycle is needed.
- Components use `PascalCase.vue`; composables use `useXxx.ts`; all other TS uses `kebab-case.ts`; Rust files use `snake_case.rs`;
  feature-domain directories use singular lowercase nouns (`crypto/ table/ xlsx/ intercept/ render/`).
- Commands are written only in `backend/mod.rs` (`build.rs` scans only that file), and the remaining `.rs` files act as its submodules.
- Frontend-only plugins (no database, no commands) may consist of `frontend/` alone, but `plugin.json` / `README.md` / `AGENTS.md` are still required.
- Directory placeholder files such as `components/README.md` are not needed: the documentation is consolidated in this file and each plugin's `AGENTS.md`.

**Machine validation**: `scripts/lint-plugins.mjs` (R-P1~R-P6) is wired into `pnpm lint`; violations must be fixed, and if an exception is genuinely needed, write
`lint-plugins-ignore` in the file and state the reason. `plugins/_template/` is validated as well, ensuring the scaffolding output is itself a compliant example.

## 2. Adding a Plugin

```bash
pnpm create-plugin        # Interactive: id / display name / icon / theme color / whether to include a database / lifecycle hooks
```

The scaffold generates a skeleton per this specification (including `views/`, `settings/`, `components/`, `composables/`, `lib/`, `shared.ts`),
which is runnable right after generation; then:

1. Edit `plugin.json` (id, display name, icon, `tools[].entry`) — the **single source of truth**;
2. Implement `frontend/views/*.vue` (for UI conventions see `docs/en/design.md`; wrap modules with `@/components/tool/Panel`);
3. When a backend is needed, add a `#[tauri::command]` in `backend/mod.rs` (with a `<plugin_id>_` prefix, returning `Result<T, AppError>`);
4. When persistence is needed, append `migration(scope, version, …)` in `backend/migrations.rs` and define the tables in `frontend/schema.ts`;
5. Add `README.md` (features) and `AGENTS.md` (development conventions).

## 3. Build-Time Registration (No Manual Registration)

| Scan point       | Location                            | Description                                                          |
| ---------------- | ----------------------------------- | -------------------------------------------------------------------- |
| Manifest         | `plugins/*/plugin.json`             | Tools/settings/icons/ordering                                        |
| Tool pages       | `plugins/*/frontend/views/*.vue`    | Pointed to by `manifest.tools[].entry`                               |
| Settings panels  | `plugins/*/frontend/settings/*.vue` | Pointed to by `manifest.settings.entry`                              |
| Lifecycle        | `plugins/*/frontend/setup.ts`       | Plugin initialization                                                |
| Backend commands | `plugins/*/backend/mod.rs`          | `build.rs` parses `#[tauri::command]` and generates the registration |
| Migrations       | `plugins/*/backend/migrations.rs`   | Aggregated into `all()`, versioned per scope                         |

Directories starting with `_` or `.` do not participate in registration (e.g. `_template/`).

## 4. Reference Implementations

- `plugins/_template/`: minimal skeleton (source of the scaffolding), demonstrating the directory conventions and page structure.
- `plugins/hello-world/`: full end-to-end demonstration (commands, logging, database, forms, tables, tasks, multi-window).
- `plugins/system/`: frontend-only plugin example (no backend, just tables and dialogs).
- The UI conventions accompanying this specification are in [`docs/en/design.md`](../docs/en/design.md); for visual inspection you can use
  `src/dev/preview-bridge.ts` + `scripts/design-shot.mjs` (renders in the browser and takes screenshots).
