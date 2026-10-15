# Architecture and Extension Development

> Further reading for `README.md`: directory structure, the complete steps for extension development, a conventions cheat sheet, architectural conventions, and known limitations.
> Related documents: [AGENTS.md](../../AGENTS.en.md) (development conventions and pitfalls),
> [plugins/README.md](../../plugins/README.en.md) (plugin directory conventions), [design.md](design.md) (UI conventions).

## Directory Structure

```
src/
├── main.ts            #   Bootstrap: logging→exceptions→settings→theme→plugins→router→mount
├── router/            #   Router (auto-generated from the plugin registry; do not maintain by hand)
├── layouts/           #   Layout shells (title bar/sidebar/error boundary/settings page)
├── core/              #   ★ Framework core; reference only when extending, do not modify
│   ├── logger/        #   Logging (unified across both sides, takes over console)
│   ├── errors/        #   AppError normalization + global exceptions + toast
│   ├── ipc/           #   invoke wrapper (command names constrained by commands.gen.ts)
│   ├── events/        #   Typed event bus (emitEvent / onEvent, unified prefix)
│   ├── db/            #   SQLite: Drizzle(kdb) + generic CRUD + transactions + hand-written SQL fallback
│   ├── http/          #   Generic HTTP client (reqwest, no CORS; advanced send + streaming download + proxy)
│   ├── theme/         #   Light/dark/follow-system + theme colors (incl. custom primary) + font size
│   ├── notify/        #   System notifications (permission + settings toggle)
│   ├── autostart.ts   #   Launch at startup (the system is the source of truth)
│   ├── global-shortcut.ts  # Global shortcuts (bring up the window, key recording)
│   ├── shortcuts/     #   In-app shortcut registry
│   ├── search/        #   Global search / command palette (Cmd/Ctrl+K)
│   ├── tasks/         #   Background tasks (cancellation + progress events)
│   ├── taskbar.ts     #   Taskbar progress / Dock badge
│   ├── open-with/     #   Unified dispatch for opening content (CLI/deep links/drag-and-drop)
│   ├── windows/       #   Multiple windows (openAppWindow, label win-*)
│   ├── diagnostics.ts #   Diagnostics report export
│   ├── settings-transfer.ts  # Settings import / export
│   ├── updater/       #   Online updates (check/download/install; public key built in, address configurable)
│   └── plugins/       #   Plugin registry + useToolSettings
├── components/        #   ★ ToolShell (tool page shell) + ui/ (shadcn-vue generated components)
├── content/           #   About / changelog (Markdown, read by the settings page)
└── stores/            #   Pinia (global settings)

plugins/               #   ★ All your tools live here (one plugin per directory, frontend and backend together)
├── _template/         #   New-plugin template (after copying, rewrite per the README.md / AGENTS.md inside; names starting with _ are not registered)
│   ├── plugin.json    #     Manifest: the single source of truth (metadata/tool items/entry declarations)
│   ├── frontend/      #     Frontend: views/ settings/ components/ composables/ lib/ schema.ts setup.ts shared.ts
│   └── backend/       #     Backend: mod.rs (#[tauri::command] commands) + migrations.rs (optional)
├── hello-world/       #   Example plugin (multi-function plugin boilerplate; README.md + AGENTS.md)
├── system/            #   System tools (data maintenance, frontend-only)

src-tauri/
├── build.rs           # Scans plugins/ to generate command registration and migration aggregation
├── framework-commands.json  # Framework command list (single source of truth, for build.rs and frontend command-name generation)
├── local-resources/   # ★ Local-layer resources (fonts / OCR models; not checked in)
└── src/
    ├── lib.rs         # Assembly entry: plugin registration / tray / menu / single instance / window-close behavior
    ├── error.rs       # AppError (all commands return Result<T, AppError>)
    ├── events.rs      # Event-name constants (synced with the frontend core/events)
    ├── db.rs          # SQLite: sqlx pool + scoped migrations + transactions + backup/restore/reset
    ├── http.rs        # HTTP client (reqwest; advanced send + streaming download + proxy)
    ├── updater.rs     # Online updates
    ├── tasks.rs       # Background tasks (cancellation token + progress events)
    ├── open.rs        # Opening content (CLI/deep links/second launch → app://open)
    ├── menu.rs        # Native application menu (→ app://menu)
    ├── diagnostics.rs # Diagnostics report export
    ├── files.rs       # Generic file read/write commands
    ├── tray.rs        # System tray
    └── plugins/mod.rs # include of the build-time-generated plugin registration (do not edit by hand)

scripts/               # scaffold / create-plugin / gen-icons / gen-commands / prepare / bump-version / release
  local/               # ★ Local-layer scripts (resource download; skipped in CI)
docs/                  # ★ Conventions and guides (index in docs/README.md)
```

## Adding a Tool (the most common case, two steps)

You can interactively generate a skeleton with the scaffold (recommended):

```bash
pnpm create-plugin      # Interactive: plugin id/name/icon, plus optional parts such as backend/settings/database
```

Or copy the template manually: `plugins/_template` → `plugins/timestamp-tool`.

1. **Prepare the directory**: generate it with `pnpm create-plugin`, or copy `plugins/_template` → `plugins/timestamp-tool`
   (structure: `plugin.json` / `frontend/{views,settings,components,schema.ts,setup.ts,shared.ts}` /
   `backend/{mod.rs,migrations.rs}`; files with no content can keep a comment placeholder or be deleted)
2. **Rewrite plugin.json**: `id` (globally unique, kebab-case, = directory name),
   `tools[].name` / `tools[].icon` (lucide icon name) / `tools[].entry` (such as `frontend/views/Tool.vue`)
3. **Implement the frontend**: edit `frontend/views/Tool.vue` — wrap the content with `ToolShell`; for a centered column / split layout use the
   native Tailwind grid (`grid grid-cols-12` + `col-start-*` / `col-span-*`);
   for details see the README.md inside the `_template` directory and the tool pages in `hello-world`

Done — **neither the frontend nor the backend needs manual registration**. The frontend (navigation/routes/titles/enable-disable/settings panels/error boundaries/KeepAlive)
is scanned by Vite at build time over `plugins/*/frontend/**`; the backend (Rust commands/migrations) is registered automatically by `src-tauri/build.rs`
scanning `plugins/*/backend/mod.rs`.

## Needing Backend Commands (optional)

Just write commands directly in the plugin's `backend/mod.rs` (**scanned automatically at build time, no framework files need changing**;
commands must be defined in `backend/mod.rs`, the function name = the name called from the frontend, prefixed with `<tool_id>_` to avoid conflicts):

```rust
#[tauri::command]
pub fn timestamp_tool_now() -> Result<i64, crate::error::AppError> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| crate::error::AppError::custom("TIME_ERROR", e.to_string())))
}
```

Calling from the frontend: `await ipc<number>("timestamp_tool_now")`. All commands return `Result<T, AppError>`,
and errors automatically enter the log and the global toast.

> **Note**: commands must be defined in `plugins/<id>/backend/mod.rs`, and the command name must carry the `<plugin_id>_` prefix
> (`build.rs` detects duplicate names at build time); migrations must go in `backend/migrations.rs`.
> For the complete conventions, the build-time mechanism, and pitfalls, see [AGENTS.md](../../AGENTS.en.md) "Conventions and caveats of automatic registration".

## Needing Database Tables (optional)

The data layer is based on **Drizzle ORM** (object-oriented, type-safe queries) + a self-built sqlx channel on the Rust side for execution.

1. **Define the schema** (the equivalent of a JPA Entity): the plugin's `frontend/schema.ts`
   (see `hello-world/frontend/schema.ts`), **aggregated automatically, no manual registration needed**
   (Vite scans `plugins/*/frontend/schema.ts` at build time; import and use the table object directly to get its type):
   ```ts
   export const myItems = sqliteTable('my_items', {
     id: integer('id').primaryKey({ autoIncrement: true }),
     title: text('title').notNull(),
     createdAt: text('created_at')
       .notNull()
       .default(sql`(datetime('now','localtime'))`),
   });
   export type MyItem = typeof myItems.$inferSelect; // row type; do not write the interface by hand
   ```
2. **Append migrations** (the plugin's `backend/migrations.rs`, scope = plugin id, version increments from 1 within the scope):
   see `hello-world/backend/migrations.rs`; the column defaults in the CREATE TABLE DDL must be aligned with the schema;
   they are aggregated automatically at build time and executed idempotently at startup by `(scope, version)`. If the plugin previously used an old global version number,
   declare "old global version → new local version" in `legacyMigrations` in `plugin.json` to bridge the old database.
3. **Query** with Drizzle (type-safe, zero SQL strings):
   ```ts
   import { kdb } from '@/core/db';
   import { eq, desc } from 'drizzle-orm';
   import { myItems } from './schema';

   const rows = await kdb
     .select()
     .from(myItems)
     .where(eq(myItems.title, 'x'))
     .orderBy(desc(myItems.id));
   const [{ id }] = await kdb.insert(myItems).values({ title: 'x' }).returning({ id: myItems.id });
   await kdb.update(myItems).set({ title: 'y' }).where(eq(myItems.id, id));
   await kdb.delete(myItems).where(eq(myItems.id, id));
   ```
4. **Faster CRUD** (for equality-condition scenarios) with the generic `core/db` methods; fall back to hand-written SQL for complex aggregations/JOINs:
   ```ts
   import { db } from '@/core/db';
   await db.insert('notes', { content: 'hi' }); // identifier whitelist validation
   const rows = await db.findAll<Row>('notes', { where: { ok: 1 }, orderBy: 'id DESC', limit: 20 });
   const rows2 = await db.select<Row>('SELECT * FROM t WHERE a > $1', [value]);
   ```

Key points: table/column identifiers are whitelist-validated + values are parameterized; **to get an auto-increment id use `.returning()`**
(the proxy's run path does not return lastInsertId); timestamp column defaults must be aligned with the migration DDL;
BLOB columns are returned as base64; for multi-statement atomic writes use `runInTransaction([{ sql, params }])`
(executed in a single transaction on the Rust side; if any statement fails, the whole thing rolls back).

## Needing a Settings Panel (optional)

1. Create `frontend/settings/Settings.vue` in the plugin directory and use `useToolSettings` to read and write configuration:
   ```ts
   const config = useToolSettings<MyConfig>('timestamp-tool', { format: 'seconds' });
   config.value.format = 'milliseconds'; // changing it persists automatically
   ```
2. Declare it in `plugin.json`: `"settings": { "entry": "frontend/settings/Settings.vue" }`

Configuration is stored in the `tools.<toolId>` namespace of `settings.json` and deep-merged with the defaults
(newly added configuration fields automatically get their default value). See `hello-world/frontend/settings/Settings.vue`.

**Unified settings-page UI**: settings panels must always use the components from `@/components/settings`; do not invent your own titles / card styles,
so that they stay visually consistent with the system settings page:

```vue
<template>
  <div class="space-y-6">
    <SettingsSection title="Section title" description="Optional description">
      <SettingsRow title="Setting title" description="One-line description">
        <Switch v-model="…" />
        <!-- right-side control; use w-40 / w-64 / w-72 for the width -->
      </SettingsRow>
      <!-- dense numeric fields: use SettingsField inside the grid (see its component header comment) -->
    </SettingsSection>
  </div>
</template>
```

## Multi-Function Plugins

A single plugin directory can declare multiple tool items in the `tools` array of `plugin.json` (see `hello-world/plugin.json`): each tool item gets independent
navigation, independent enable/disable, and an independent settings tab, while sharing the code / Rust modules / migrations in the directory.

## Conventions Cheat Sheet

| Convention           | Description                                                                                                                                                                                                                             |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Plugin manifest      | `plugin.json` is the single source of truth; **both the frontend (Vite) and the backend (build.rs) register automatically at build time, no manual registration**                                                                       |
| Plugin directory     | `frontend/` (views/settings/components/composables/lib/schema.ts/setup.ts/shared.ts) + `backend/` (mod.rs/migrations.rs); directories starting with `_` are not registered                                                              |
| Tool id              | Globally unique, kebab-case, = route `/tool/:id`; within a plugin, prefixed with the plugin id                                                                                                                                          |
| Plugin ordering      | Group order = the smallest `order` in the group; tool order within a group = `order*1000 + index in the tools array`; when absent, defaults to 100 then alphabetical by directory name                                                  |
| Rust commands        | Defined in `backend/mod.rs`; function name = the name called from the frontend, `<tool_id>_` prefix to avoid conflicts                                                                                                                  |
| Command signature    | Always `Result<T, AppError>`; parameters in snake_case (the frontend passes camelCase and it is mapped automatically)                                                                                                                   |
| SQL                  | Prefer `kdb` (Drizzle object-oriented queries); for quick CRUD use `db.insert/findAll` etc.; hand-written SQL is always parameterized with `$1`                                                                                         |
| Auto-increment id    | For insert/update/delete use `.returning({ id: table.id })` to get the return value                                                                                                                                                     |
| Migration version    | `migration(scope, version, ...)` is isolated by plugin scope (scope = plugin id, version increments from 1 within each scope); released migrations **must not be modified** (append only); bridge old databases with `legacyMigrations` |
| BLOB columns         | Returned through the sqlx channel as a **base64 string** (lossless); the frontend must `atob`-decode it itself                                                                                                                          |
| Tool settings        | `useToolSettings(toolId, defaults)`; do not create your own separate storage                                                                                                                                                            |
| HTTP requests        | Always go through `core/http` (`http.getJson/postJson`…, use `http.send` for advanced cases); fetching cross-origin from within the webview is forbidden                                                                                |
| Logging              | `ctx.logger` / `logger`; bare `println!` is forbidden                                                                                                                                                                                   |
| Styling              | shadcn-vue semantic colors (`bg-primary` etc.) follow the theme color; do not hardcode color values                                                                                                                                     |
| Generated components | `src/components/ui/**` is CLI-generated; you may change styles but not the structure/logic                                                                                                                                              |

## Architectural Conventions (framework behaviors you get for free when extending)

- **Startup order**: logging → exception handling → settings load → theme → plugin registration (`setup(ctx)` hook) → routes → mount
- **Error pipeline**: Rust `AppError{code,message}` → unified conversion on the frontend → automatically logged + toast on uncaught errors;
  crashes while rendering a tool page are isolated by the error boundary (a placeholder page + retry) and do not affect the framework
- **Logging**: Rust and the frontend both write to the application log directory (on macOS `~/Library/Logs/<app identifier>/`);
  console has been taken over; the level is adjustable at runtime (settings page)
- **State retention**: tool switching uses KeepAlive by default (switching back does not lose input); `keepAlive: false` opts out
- **Window-close behavior**: hides to the tray by default (configurable in settings); the window is centered on the screen where the mouse is at every startup
- **Theme**: light/dark/follow-system (dark by default) + 6 theme colors (the accent class in `core/theme` + `assets/index.css`);
  root font size in three levels of scaling (small 13 / normal 14 / large 15), with controls and text scaling in proportion
- **HTTP**: a single global Rust reqwest instance (rustls, cookie session, 10 redirects, 30s timeout, 10MB response limit);
  on the frontend `http.getJson<T>(url)` is all you need to fetch a JSON API; `http.send()` is the advanced primitive (parameterizable redirect / SSL / timeout / proxy /
  cookie mode, supports multipart upload, binary responses, order-preserving duplicate response headers, and cancellation), using a Client isolated from fetching
- **Security trust boundary**: App-owned commands such as `db_*` / `http_request` / `hello_world_greet`
  **do not go through the capability permission system** (Tauri v2 only gates core and plugin commands); any code inside the webview can execute
  arbitrary SQL / issue arbitrary requests. Therefore you must keep the CSP strict and **not load any remote content**, treating plugins (local code)
  as within the trust boundary

## Known Limitations

- On macOS there is slight rendering lag in webview content when the window is scaled (an inherent limitation of WKWebView's cross-process compositing,
  already mitigated by layerContentsRedrawPolicy and a dark window background)
- HTTP response bodies are decoded as UTF-8 (non-UTF-8 pages such as GBK are a future extension point); for binary resources use
  `http.download()` to stream-download to a file
- Packaging build warnings: the main bundle has been split, but the full `@lucide/vue` icon set makes the `vendor-ui` chunk large
  (an inherent cost of resolving icons by name)
