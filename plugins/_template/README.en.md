# Tool Plugin Template

It is recommended to generate interactively with the scaffolding: `pnpm create-plugin` (automatically substitutes names/icons and, as needed, generates the backend, settings, database, etc.).

You can also manually copy this directory to `plugins/<plugin-id>` (kebab-case, = the `id` in plugin.json), then:

## 1. Rewrite plugin.json

- `id` / `tools[].id`: globally unique (tool id uses `<plugin-id>` or `<plugin-id>-<feature>`)
- `tools[].name` / `icon` (lucide icon name) / `entry` (**relative to the plugin directory**, e.g. `frontend/views/Tool.vue`)
- Optional fields: `description`, `group`, `order`, `keywords`, `keepAlive`, `legacyMigrations`
- Optional `settings.entry` (e.g. `frontend/settings/Settings.vue`)

## 2. Implement the Pages

Edit `frontend/views/Tool.vue`; multi-feature plugins add multiple entries to the `tools` array and each provides its own `frontend/views/<Xxx>.vue`.
Put shared constants in `frontend/shared.ts`, private components in `frontend/components/`, and composables in `frontend/composables/`.

**Page layout conventions (native Tailwind grid)**: wrap the content with `@/components/tool/ToolShell.vue`
(props: `title` / `description`; slots: `actions` / default slot), and let the content area fill the full width;
when a centered column / split column is needed, use the Tailwind grid directly within the page; a `mx-auto max-w-*` centered container is forbidden:

```vue
<ToolShell title="Title">
  <div class="mx-auto grid w-full grid-cols-12">
    <div class="col-span-12 space-y-4 lg:col-start-3 lg:col-span-8">
      <!-- Centered 8 columns, automatically full width below lg; each module uses one Panel -->
      <Panel title="Input">
        <template #actions><Button variant="secondary" size="sm">Choose file</Button></template>
        …
      </Panel>
      <Panel title="Output">…</Panel>
    </div>
  </div>
</ToolShell>
```

Common presets: tables full width (no extra classes), wide content `lg:col-start-3 lg:col-span-8`,
forms/settings `lg:col-start-3 lg:col-span-8` or `md:col-start-4 md:col-span-6`.
A centered column must use an even span (12 − span must be even) to center precisely.

**Module panels**: every feature module on a page (settings / input / output / result) should be wrapped with
`@/components/tool/Panel.vue` (props: `title` / `hint` / `bodyClass`; slots:
`actions` / `title` / default slot) — the outer frame and header bar visuals are unified by Panel, so do not hand-write
`rounded-lg border bg-card` cards again, and do not nest cards inside a Panel (components such as previews and tables do not carry their own outer frame).
For the standard way to write empty / loading / error states, see the "Page Template" tool in hello-world.

## 3. Optional Capabilities

- **Settings panel**: implement `frontend/settings/Settings.vue` and declare `settings.entry` in plugin.json;
  read/write configuration with `useToolSettings(pluginId, defaults)` (see hello-world/settings).
  Use `SettingsSection` / `SettingsRow` / `SettingsField` from `@/components/settings` consistently for the UI
  (see the template `frontend/settings/Settings.vue` for the skeleton), and do not invent your own section headings / card styles.
- **Database tables**: define `sqliteTable` in `frontend/schema.ts` (automatically aggregated into kdb);
  on the Rust side, append migrations in `backend/migrations.rs`: `migration("plugin id", version, ...)`,
  and **`version` increments from 1 within the plugin scope** (plugins do not interfere with each other). If the plugin previously used an old global version number,
  declare an "old global version → new local version" mapping in `legacyMigrations` in plugin.json to bridge the old database.
- **Rust commands**: in `backend/mod.rs` write `#[tauri::command] pub fn <plugin_id>_xxx(...) -> Result<T, AppError>`.
  **Scanned and registered automatically at build time, no manual registration needed** (commands must be defined in `backend/mod.rs`).
  Frontend call: `import { ipc } from "@/core/ipc"; await ipc("plugin id_xxx", { args })`
- **HTTP fetching**: `import { http } from "@/core/http"` (`getJson/postJson`)
- **Logging**: use `ctx.logger` in `frontend/setup.ts`, and `logger` from `@/core/logger` in pages

## Directory Conventions

```
plugins/<plugin-id>/
├── plugin.json        Manifest (single source of truth; shared by frontend and backend scanning)
├── README.md          Feature description (for users)
├── AGENTS.md          Development conventions (for AI/maintainers)
├── frontend/          Frontend
│   ├── views/         Tool pages (one file per tools[] entry)
│   ├── settings/      Settings panel (optional)
│   ├── components/    Plugin-private components (optional)
│   ├── composables/   Composables (optional, useXxx.ts)
│   ├── lib/           Pure-logic TS (optional, no Vue dependency)
│   ├── schema.ts      Database table definitions (optional, auto-aggregated)
│   ├── setup.ts       Lifecycle hooks (optional, auto-scanned)
│   └── shared.ts      Shared constants/utilities within the plugin (optional)
└── backend/           Backend (optional)
    ├── mod.rs         Rust commands (#[tauri::command]) + `pub mod migrations;`
    ├── migrations.rs  Migration definitions: exports `pub fn all() -> Vec<Migration>` (optional)
    └── *.rs           Remaining backend code / assets
```

Directories starting with `_` or `.` do not participate in registration (this template is `_template`).
SQL must always be `$1`-parameterized or use kdb object-based queries; string concatenation is forbidden.
