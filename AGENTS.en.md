# ArkDesk — AGENTS.md

> [中文](AGENTS.md) · English

A desktop tool collection (Mac / Windows) built with Tauri 2 + Vue 3 + TypeScript + Tailwind 4.
Positioned as a **base project**: the cost of adding a tool should be as low as possible, extensions follow the plugin convention, and framework code (`src/core`, and db/http/tray/updater/menu/tasks/open/… in `src-tauri/src`) is only referenced, never modified.

## Common commands

```bash
pnpm dev            # vite dev (frontend)
pnpm tauri dev      # Tauri dev (compiles Rust first, takes a few minutes)
pnpm build          # vue-tsc --noEmit + vite build (must run after frontend changes)
pnpm preview        # preview vite build output
pnpm test           # vitest unit tests (core pure logic)
pnpm test:watch     # vitest watch mode
pnpm lint           # eslint (must run after frontend changes)
pnpm lint:fix
pnpm format         # prettier --write . (full)
pnpm format:check
pnpm env:cpp        # export macOS CXXFLAGS (for compiling the local-layer ocr-rs, see docs/en/local.md)
pnpm lint:rs        # cargo clippy -D warnings (must run after Rust changes)
pnpm fmt:rs         # cargo fmt
pnpm test:rs        # cargo test (http smoke tests ignored by default, needs network)
pnpm tauri build    # package
pnpm scaffold       # one-click rename after clone (interactive: display name/identifier/accent color, etc.)
pnpm create-plugin  # interactively generate a new plugin skeleton (plugins/<id>/, front and back end auto-registered)
pnpm icons          # dual-source generation of all-platform icons (macOS HIG grid icns + Windows full-bleed ico/png)
pnpm version:bump   # bump version (syncs tauri.conf.json / Cargo.toml / package.json)
pnpm release        # generate the update manifest latest.json + checksums (for release)
```

Convention: after frontend changes run `pnpm lint && pnpm build`; after Rust changes run `pnpm lint:rs && pnpm fmt:rs && pnpm test:rs`.

## Dual-repository collaboration (single-maintainer mode)

This repository (ArkDesk) is a fork of the upstream framework repository **PocketArk**; both repositories are owned by the same person:

- `upstream` = `git@github.com:hmilyld/PocketArk.git` (framework base)
- `origin` = `git@github.com:hmilyld/ArkDesk.git` (this project)

**Change-ownership determination**: [`docs/ownership.json`](docs/ownership.json) is authoritative — `framework` must be back-ported upstream;
`fork` stays only in this project; `shared` follows upstream in structure but differs in content per fork (manual adaptation is required when back-porting).

**Current workflow (single-maintainer mode: change only in ArkDesk, avoiding switching repositories back and forth)**:

1. Make and commit changes directly in the ArkDesk working tree (framework + fork may share the same commit), then run `pnpm lint && pnpm build && pnpm test` locally.
2. Extract the **framework** part per `docs/ownership.json` → create a branch in `../PocketArk` and apply it →
   `gh pr create` (base `main`) → merge. **The patch must be generated from the working-tree state** (`git diff A..B` contains only
   committed content and will miss uncommitted neutralization/fixes).
3. Back in ArkDesk: `git fetch upstream && git merge upstream/main` (absorb the already-merged framework changes) → commit any fork-specific leftovers.
4. **Switch back to "upstream-first" as soon as a collaborator joins**: framework changes must always be made and merged in PocketArk first, then merged into ArkDesk
   (committing framework changes in ArkDesk first is forbidden).

Risks and cautions: if the same content is committed inconsistently on the two sides (e.g. wording changed in upstream review), the next merge will conflict;
when back-porting, keep the upstream content consistent with what is already committed in ArkDesk. The content of `shared` files (`README.md` / `AGENTS.md` / `docs/en/local.md` /
`src-tauri/Cargo.toml` / `package.json`) differs between the two sides by design; when back-porting, carry only structural changes and keep the content on each side.

The local PocketArk copy is located at `../PocketArk` by default.

## Local layer (fork-only)

The framework itself **downloads no resources and contains no personal tools**. If your fork layers on personal tools (resources such as fonts/OCR,
extra Rust dependencies, extra permissions), the conventions are centralized in the "local layer"; see `docs/en/local.md` at the repository root:
`src-tauri/local-resources/` (resources), `scripts/local/` (download scripts),
the `local plugin deps` section of `Cargo.toml`, and `capabilities/local.json` (if present).

## Plugin extension (the most common task)

> Each plugin's **local conventions** live in `plugins/<id>/AGENTS.md` and its **feature documentation** in `plugins/<id>/README.md`;
> this document keeps only framework-level conventions and the index. Consult the corresponding plugin directory for plugin details, to avoid bloating the project-level documentation.

Adding a tool = `pnpm create-plugin` (interactively generates a skeleton, recommended), or copy `plugins/_template/` → `plugins/<plugin-id>/`, rewrite `plugin.json` (the single source of truth), and implement `frontend/views/*.vue`. **Both frontend and backend are auto-registered at build time**, with no manual registration of any kind required.

- Frontend: Vite scans `plugins/*/frontend/**` (views / settings / setup.ts / schema.ts).
- Backend: `src-tauri/build.rs` scans `plugins/*/backend/mod.rs`, parses `#[tauri::command]` to auto-register commands, and reads `migrations.rs` to aggregate migrations. Commands must be defined in `backend/mod.rs`, the function name = the frontend call name (`<plugin_id>_` prefix), and everything returns `Result<T, AppError>`.
- Migrations are **scoped**: `migration(scope, version, ...)`, where scope = plugin id and version increments from 1 within the scope. Legacy historical versions of old databases are bridged and registered once via `legacyMigrations` in `plugin.json` (old global version → new local version).
- Directories starting with `_` or `.` are excluded from registration (e.g. `_template`).

### Plugin directory conventions

Each plugin's internal structure, naming, and required files follow a uniform convention (`frontend/` top-level whitelist, pure logic in `frontend/lib/`,
composables named `useXxx.ts`, backend commands only in `backend/mod.rs`, etc.). The full specification is in **[`plugins/README.en.md`](plugins/README.en.md)**.

- Machine validation: `scripts/lint-plugins.mjs` (R-P1~R-P6) is wired into `pnpm lint`; `plugins/_template/` is validated too,
  ensuring the scaffold output is a spec sample. When an exception is genuinely needed, write `lint-plugins-ignore` in the file and state the reason.
- Always generate new plugins with `pnpm create-plugin` (the skeleton is compliant), or copy `plugins/_template/`.

### Auto-registration conventions and caveats

- **Commands must be written in `plugins/<id>/backend/mod.rs`**: `build.rs` scans only that file to parse `#[tauri::command]`; other `.rs` files serve as its submodules (`pub mod xxx;`). A command may be on its own line or on the same line as `#[tauri::command]`, and the attribute may take parameters (e.g. `#[tauri::command(rename_all = "camelCase")]`).
- **Command names must be globally unique and carry the `<plugin_id>_` prefix**: duplicate names produce duplicate match arms in the generated `generate_handler!`; `build.rs` detects this at build time and errors out directly.
- **Migrations must go in `plugins/<id>/backend/migrations.rs`**, exporting `pub fn all() -> Vec<Migration>`; in `migration(scope, version, ...)` scope = plugin id and version increments from 1 within the scope. **Published migrations must not be modified, only appended**.
- **Legacy database bridging**: if a plugin once used the old global version numbers, declare `legacyMigrations: { "旧全局版本": 新本地版本 }` in `plugin.json`; at startup it is registered into `plugin_migrations` per the mapping (DDL is not executed again).
- The **framework-owned command** list is `FRAMEWORK_COMMANDS` in `src-tauri/build.rs`: new framework commands (db/http/updater, etc.) must be appended there, otherwise they will not be registered.
- **Frontend entry paths**: `tools[].entry` / `settings.entry` in `plugin.json` are relative to the plugin directory and **include the `frontend/` prefix** (e.g. `frontend/views/Tool.vue`).
- **Build-time mechanics**: `build.rs` locates the repository-root `plugins/` via `CARGO_MANIFEST_DIR`, includes each plugin backend with an absolute `#[path]`, and emits `cargo:rerun-if-changed=plugins`; `cargo build` directly from `src-tauri` also works (no need to go through pnpm).
- **After adding a plugin**: if it does not appear automatically after the frontend glob changes, restart `pnpm dev`/`pnpm tauri dev`.
- **The template is not compiled**: `_template` is skipped by both the frontend and backend scans because of its `_` prefix; after copying, change the command prefix to the new plugin id (`template_xxx` → `<id>_xxx`).
- **Tailwind**: plugin frontends live in the repository-root `plugins/` (outside `src/`), and `assets/index.css` already has `@source '../../plugins'`; do not remove it.
- **Platform note**: `#[path]` is generated with forward slashes and should work on Windows too, but it has **not been tested on Windows yet**.

## App updates (framework capability)

The framework has built-in online updates: frontend `src/core/updater`, Rust `src-tauri/src/updater.rs` (`updater_check` / `updater_install` / `updater_restart`). It uses the official `tauri-plugin-updater` plus a self-hosted static manifest.

- **The public key is baked in** at `tauri.conf.json > plugins.updater.pubkey` (root of trust, not editable in the UI); the UI only configures the server address, **HTTPS only**.
- **Checks once after startup only** (controlled by `updateEnabled` + `updateAutoCheck`, no polling); `initUpdater()` is called by `main.ts`.
- On the Rust side, `updater_builder().endpoints([...])` overrides the endpoints at runtime; download progress is reported back via the `updater://progress` event.
- Settings keys: `updateEnabled` / `updateServerUrl` / `updateAutoCheck` / `updateLastCheckAt`.
- The single source of truth for the version = `tauri.conf.json > version`; before releasing, run `pnpm version:bump x.y.z` to sync the three places.
- Update selection uses strict semver (remote > local), and **version numbers must increase monotonically**; the manifest `version` must match the build version.
- The private key (`~/.tauri/arkdesk.key`) is not committed; local builds use `TAURI_SIGNING_PRIVATE_KEY_PATH`, and CI uses the secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Automated release: the base provides `.github/workflows/release-reusable.yml` (reusable); a fork adds a caller. The complete workflow is in [release.md](docs/en/release.md).

## Framework capability index

`src/core/` is split into modules by capability; plugins only reference it, never modify it. Capabilities of the same name on the Rust side are in `src-tauri/src/`.

| Capability                    | Frontend entry                    | Rust                                         | Notes                                                                                                                                                                          |
| ----------------------------- | --------------------------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| IPC                           | `@/core/ipc`                      | `generate_handler` (generated at build time) | `ipc<T>(cmd, args)`; command names are constrained by `commands.gen.ts`; bare `invoke` is forbidden                                                                            |
| Event bus                     | `@/core/events`                   | `src/events.rs`                              | Typed `emitEvent/onEvent`, unified event prefix                                                                                                                                |
| Logging                       | `@/core/logger`                   | `log`                                        | Unified across both ends, takes over console                                                                                                                                   |
| Errors                        | `@/core/errors`                   | `error.rs`                                   | `AppError{code,message}` normalization + global toast                                                                                                                          |
| Database                      | `@/core/db`                       | `db.rs`                                      | Drizzle(kdb) + generic CRUD + scoped migrations                                                                                                                                |
| HTTP                          | `@/core/http`                     | `http.rs`                                    | reqwest, no CORS; `send()` advanced (redirect/SSL/timeout/cookie/multipart/binary/cancel); `download()` streaming + progress; driven by proxy settings                         |
| Theme                         | `@/core/theme`                    | `set_window_appearance`                      | Light/dark/follow system + accent color + font size                                                                                                                            |
| Plugin registration           | `@/core/plugins`                  | `plugins/mod.rs` (generated artifact)        | Auto-registered at build time on both frontend and backend                                                                                                                     |
| Online updates                | `@/core/updater`                  | `updater.rs`                                 | tauri-plugin-updater + static manifest                                                                                                                                         |
| System notifications          | `@/core/notify`                   | `tauri-plugin-notification`                  | Permission request + settings toggle                                                                                                                                           |
| Launch at login               | `@/core/autostart`                | `tauri-plugin-autostart`                     | Source of truth is the system                                                                                                                                                  |
| Database transactions         | `runInTransaction` of `@/core/db` | `db.rs`                                      | Batch execution in a single transaction, rollback on failure                                                                                                                   |
| Data backup/restore/reset     | Settings page "Data"              | `db.rs`                                      | Auto-restart after restore/reset                                                                                                                                               |
| Settings import/export        | `@/core/settings-transfer`        | `files.rs`                                   | Export JSON; takes effect after restart following import                                                                                                                       |
| Diagnostics report            | `@/core/diagnostics`              | `diagnostics.rs`                             | Environment info + recent logs                                                                                                                                                 |
| Single instance               | —                                 | `tauri-plugin-single-instance`               | Second launch raises the main window + forwards arguments                                                                                                                      |
| In-app shortcuts              | `@/core/shortcuts`                | —                                            | `registerShortcut('mod+k', fn)`                                                                                                                                                |
| Global shortcuts              | `@/core/global-shortcut`          | `tauri-plugin-global-shortcut`               | Driven by a setting, raises the main window                                                                                                                                    |
| Global search/command palette | `@/core/search`                   | —                                            | `Cmd/Ctrl+K` aggregates navigation and tools                                                                                                                                   |
| Taskbar progress/badge        | `@/core/taskbar`                  | Tauri Window API                             | Progress 0–100 / Dock badge                                                                                                                                                    |
| Background tasks              | `@/core/tasks`                    | `tasks.rs`                                   | Cancellation token + `task://` progress events                                                                                                                                 |
| Open content                  | `@/core/open-with`                | `open.rs`                                    | Unified dispatch for CLI / deep links / drag-and-drop (`onOpenFiles`)                                                                                                          |
| Multi-window                  | `@/core/windows`                  | `capabilities/windows.json`                  | `openAppWindow()`, label `win-*`                                                                                                                                               |
| Native app menu               | `@/core/events` (`app://menu`)    | `menu.rs`                                    | macOS menubar / Win window menu; items are forwarded to the frontend via events                                                                                                |
| Platform detection            | `@/core/platform.ts`              | —                                            | `isMac` (styling and shortcut modifier-key differences)                                                                                                                        |
| Lifecycle hooks               | —                                 | `lifecycle.rs`                               | Pre/post startup, pre/post exit SPI; a fork implements it in `src-tauri/local/lifecycle.rs` and it is mounted automatically (see [docs/en/lifecycle.md](docs/en/lifecycle.md)) |

> When adding a framework capability, register it in this table and add an explanation under "Architecture conventions" in the README.

## Key conventions

- **IPC**: The frontend always uses `await ipc<T>(cmd, args)` (`@/core/ipc`); bare `invoke` is forbidden. Command names are generated by `scripts/gen-commands.mjs` into `src/core/ipc/commands.gen.ts` (it reads `src-tauri/framework-commands.json` and scans plugin `#[tauri::command]`s, giving a union type of framework + plugin commands); unregistered commands raise an error at type-check time. That file is a generated artifact (updated by `prepare`/build, do not edit by hand).
- **Errors**: Rust `AppError{code, message}`; code aligns with `ErrorCode` in the frontend `core/errors`.
- **Database**: Use `kdb` (Drizzle sqlite-proxy) for object-style queries; use `db.insert/findAll` and the like from `@/core/db` for quick CRUD; hand-written SQL is always parameterized with `$1`. Use `.returning()` to obtain the auto-increment id. BLOB columns are returned over the channel as base64 (the frontend decodes them itself). Table/column names are validated against an identifier whitelist.
- **HTTP**: Go through `@/core/http` (Rust reqwest, no CORS); using `fetch` inside the webview for scraping is forbidden. Use `getJson/postJson/request` for simple/scraping requests; use `http.send()` when you need custom redirects, SSL verification, timeouts, cookie mode, multipart uploads, binary responses, or cancellation (Rust `http_send`, isolated from the global Client used for scraping).
- **Logging**: Use `logger` from `@/core/logger` or the plugin's `ctx.logger`; bare `println!` is forbidden.
- **Icons**: Only `@lucide/vue` is allowed (`lucide-vue-next` is deprecated, do not introduce it again).
- **Styling**: Use shadcn-vue semantic colors (`bg-primary`, etc.); hard-coded color values are forbidden. Tokens such as `text-success/warning/info` and `bg-console` already exist.
- **Design specification (single source of truth)**: All UI visuals and interactions follow `docs/en/design.md` (shared core: tokens, component semantics, tri-state and feedback, copy, anti-pattern list) +
  `docs/en/design-macos.md` / `docs/en/design-windows.md` (platform layer) + `docs/en/design-appendix.md` (per-control Do/Don't and migration mapping). Visual values may only come from tokens;
  machine validation is handled by `scripts/lint-design.mjs` in `pnpm lint` (R1 legacy card recipe / R2 arbitrary-opacity surfaces / R3 non-overlay rounded-xl /
  R4 hard-coded control sizes / R5 motion suppression / R6 focus-ring expression). **The current backlog is at zero**, so `pnpm lint` is a hard gate from now on;
  `pnpm lint:design` is the strict mode (usable directly in CI).
  **Platform differences are only allowed in**: material and fallback, window shell, menu and shortcut presentation, dialog button semantics, focus visuals, and corner-radius tier (implemented by overriding tokens via `[data-platform]`).
- **Documentation index**: All specifications and guides are in `docs/`; see the index at [`docs/README.md`](docs/README.md); for to-dos see [`TODO.en.md`](TODO.en.md).
  Documentation governance conventions (single source of truth, naming, lifecycle) are in `docs/README.md`.
- **Design migration risk register**: The Windows platform layer (`docs/en/design-windows.md`) has **not yet been visually verified** (the dev machine is macOS) —
  until the first verification on Windows, changes touching Windows are "statically correct" and must be confirmed item by item against the §11 to-do list in that file and back-filled.
- **Design walkthrough**: `src/dev/preview-bridge.ts` (active only in dev and outside Tauri) lets the web layer render in the browser,
  and together with `scripts/design-shot.mjs` allows scripted screenshots to check against the spec (usage in `docs/en/design-appendix.md §3.5`).
- **Tool page modules**: Every functional module on a page (settings / input / output / result / list) is always wrapped in `@/components/tool/Panel` (outer frame + header-bar title + top-right actions + body). Do not leave modules bare on the page, and do not nest cards inside a Panel — components such as previews and tables carry no outer frame themselves; the outer frame is left to the Panel. Use `text-xs font-medium text-muted-foreground` for the header title, `size="sm"` for action buttons, and `size-3.5` for icons; the body defaults to `space-y-3 p-4` (override with `body-class` for full-bleed cases); use `rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive` for the error bar.
- **Settings page UI**: The system settings page and all plugin settings panels uniformly use `SettingsSection` / `SettingsRow` / `SettingsField` from `@/components/settings` (single source of truth, visuals follow the system settings page); do not invent your own section headings or card styles. For dense numeric fields, use a grid + `SettingsField` inside `SettingsSection`.
- **Code style**: prettier with single quotes, 100 columns, es5 trailing commas; run `pnpm format` before committing.
- **Theme**: Accent color/light-dark are in `core/theme` + the accent class in `assets/index.css`; adding an accent color requires syncing three places (CSS / ACCENTS / the anti-flash inline script in index.html).
- **Console/output text**: `bg-console` must be paired with `text-console-foreground*`, otherwise text is unreadable on the dark background.
- **Switch vs. checkbox (HIG)**: Switches appear only inside list rows, in the same area as the title (the row content provides context); use checkboxes for fine-grained booleans — do not replace multi-select with a row of switches.
- **The one and only focus-ring expression**: `focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60` (validated by `scripts/lint-design.mjs` R6).
- **Window material**: When enabled, `set_window_background` does not paint a solid base (otherwise it would cover the material); the anti-flash relies on an opaque background in the frontend. Do not remove the early return for `VIBRANCY_ACTIVE` in `src-tauri/src/lib.rs`.
- **Lifecycle hooks**: `src-tauri/src/lifecycle.rs` only defines the contract and call sites; the base contains no implementation. A fork writes `pub struct Hooks;` in `src-tauri/local/lifecycle.rs` and it is mounted automatically by `build.rs` (no file means a no-op). Hooks run in a separate async task — **do not block the main thread inside a hook** (`block_on` makes startup hang, so that not even self-created windows are drawn). After `before_start` returns `Hold`, the main window is shown only once `lifecycle::resume_startup()` is called; during that period tray/single-instance/Dock activation is uniformly rejected by `tray.rs` (do not bypass that choke point).
- **Preview fixtures**: Plugin fixtures live only in `plugins/<id>/frontend/preview.ts` (default-exports `Record<命令名, 返回值>`, globbed in by the preview bridge in dev); do **not** write them into `src/dev/preview-bridge.ts` (the base does not accept fork plugin names). The preview bridge returns `null` (not an array) for unknown business commands; use `?? []` on list commands for fault tolerance, or the browser preview shows a blank screen.

## Pitfalls (already fixed, do not regress)

- **Logging loop**: `core/logger` uses `attachLogger` + the `echoing` guard. Do not revert to `attachConsole` or let the console takeover function forward logs during the echo period — it will loop infinitely.
- **keepAlive**: `MainLayout` caches based on `route.meta.keepAlive !== false`; only `keepAlive: false` in the manifest makes it not cache.
- **Ordering**: The plugin group order = the minimum `order` in the group; `manifest.order` participates in sorting — do not revert to sorting by directory name.
- **Security boundary**: App commands such as `db_*` / `http_request` are not gated by capabilities, so any code inside the webview can execute arbitrary SQL/requests. CSP must stay strict and no remote content may be loaded.
- **Theme-appearance coupling**: `applyTheme` syncs NSWindow.appearance / NSWindow.backgroundColor (macOS). Forcing darkAqua/aqua pins the webview's `prefers-color-scheme` to the corresponding appearance — "follow system" must pass `dark: null` to clear the override (see the tri-state design of `set_window_appearance`).
- **Tool page layout**: `ToolShell` only provides the page header + a full-width container; centered columns / split panes use the native Tailwind grid inside the page (`grid grid-cols-12` + `col-start-*` / `col-span-*`, with even spans for centered columns). `mx-auto max-w-*` centered containers are forbidden. If a new registry component uses bare boolean variants like `data-checked:`/`data-open:`, convert them to `data-[state=...]:` (reka-ui 2.10 only outputs the latter).
- **macOS dev port in use**: When the tauri CLI (≤2.11.4) exits, it relies on `$TMPDIR/tauri-stop-dev-processes.sh` to clean up the dev server process tree, but in practice that script gets created as **0 bytes + 0o644** (both the content and permission writes fail silently), and the `!exists()` guard never recreates it → vite becomes orphaned → the next `tauri dev` reports the port in use. chmod alone is not enough (an empty script = no-op); you must write the original content plus the execute bit (once is enough):

  ```bash
  cat > "${TMPDIR}tauri-stop-dev-processes.sh" << 'EOF'
  #!/usr/bin/env sh
  getcpid() {
      cpids=$(pgrep -P $1|xargs)
      for cpid in $cpids; do
          echo "$cpid"
          getcpid $cpid
      done
  }
  kill $(getcpid $1)
  EOF
  chmod 755 "${TMPDIR}tauri-stop-dev-processes.sh"
  ```

  Emergency fallback: `lsof -ti:1420 | xargs -r kill -9`. Upstream tauri#15098; this item can be removed once fix PR #15108 (open) is merged and the CLI is upgraded.

- **macOS tray click does not activate the window**: With `show_menu_on_left_click(false)`, clicking the menu-bar icon does not make AppKit activate the owning app; `show()`/`set_focus()` only do `makeKeyAndOrderFront`, so the window is pushed to a lower layer — it looks like "nothing happened when clicked", and the window only becomes visible after switching to another app (the Dock icon works because macOS activates the app). Upstream tauri#14795 (still unfixed in tray-icon 0.25.0). `tray::show_main_window` has been changed to retry immediately plus one tick later (next runloop), and to call `NSApp.activateIgnoringOtherApps(true)` on macOS; `RunEvent::Reopen` also explicitly raises the main window. Do not remove these two spots or it will regress.

- **Local-layer build dependencies (ocr-rs, etc.)**: Heavy dependencies like these belong to the fork's local layer; the fork handles the related build issues (macOS `CXXFLAGS`, Windows libclang) itself and documents them in `docs/en/local.md`; the base does not contain these dependencies and has no such problems.

## Directory overview

```
src/core/            # framework core (logger/errors/ipc/db/http/theme/plugins); extensions only reference it
src/components/tool/ # ToolShell tool page shell (header + full-width container) + Panel module panel
src/components/ui/   # shadcn-vue generated components (managed by CLI)
src/content/         # About/changelog Markdown (read by the settings page)
scripts/             # scaffold / create-plugin / gen-icons / gen-commands / prepare / bump-version / release
  local/             # ★local-layer scripts (fork-owned; absent in base): resource downloads, CI skip
plugins/<id>/        # ★tool plugins (frontend and backend together): plugin.json + README.md + AGENTS.md
  frontend/          #   views/ settings/ components/ composables/ lib/ schema.ts setup.ts shared.ts
  backend/           #   mod.rs (#[tauri::command] commands) + migrations.rs + other .rs / resources
src/layouts/         # layout shells (title bar/sidebar/error boundary/settings page)
src/stores/          # Pinia (global settings)
src-tauri/src/       # db.rs (sqlx + scoped migrations), http.rs, tray.rs, updater.rs, tasks.rs, open.rs, menu.rs, diagnostics.rs, files.rs, lifecycle.rs, plugins/mod.rs (include-generated artifact)
src-tauri/build.rs   # scans plugins/ to generate command registration and migration aggregation; scans local/lifecycle.rs to generate hook mounting
src-tauri/local/     # ★local-layer code (fork-owned; base keeps only README conventions): lifecycle.rs = lifecycle hook implementation
src-tauri/local-resources/  # ★local-layer resources (fork-owned; absent in base)
docs/                # ★specifications and guides (index at docs/README.md; doc governance in that file)
  local.md           #   local-layer explanation (fork-only)
  lifecycle.md       #   lifecycle hook (SPI) contract and integration
```

Generated shadcn components `src/components/ui/**` are managed by the CLI: you may change styles, but not structure/logic.
