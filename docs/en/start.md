# START — Build Your Own Desktop Software with This Repository

> This document is written for: anyone who has cloned ArkDesk and wants to turn it into **their own desktop tool software**.
> Follow it through and you will end up with a desktop application named after your software, carrying your icon and theme, and loaded with your own tools.
>
> Division of labour between the documents: **start.md** (this file) = a from-scratch getting-started guide; **extending.md** = architecture and extension development reference;
> **AGENTS.md** = day-to-day development conventions and a pitfalls checklist; **release.md** = packaging / release / online updates;
> **local.md** = the fork local layer; `plugins/_template/README.md` = single-plugin scaffold documentation.

---

## 0. What This Project Is

A **desktop toolbox foundation**: Tauri 2 + Vue 3 + TypeScript + Tailwind CSS 4 + shadcn-vue.
It has already done all the grunt work "unrelated to the app itself" for you — windows, tray, sidebar navigation, routing, theme,
SQLite, settings persistence, logging, error handling, HTTP channel — all you have to do is: write your tool pages.

Framework code (`src/core/`, and db/http/tray/updater/menu/tasks/open/… in `src-tauri/src`) is
**reference only, do not modify**; all extensions are done through the plugin mechanism (see §6).

## 1. Environment Setup

| Dependency | Version        | Description                       |
| ---------- | -------------- | --------------------------------- |
| Node.js    | 20+            | Includes npm                      |
| pnpm       | 9+             | Package manager (`npm i -g pnpm`) |
| Rust       | stable (1.77+) | Installed with `rustup`           |

Extra per-platform dependencies:

- **macOS**: Xcode Command Line Tools (`xcode-select --install`)
- **Windows**: MSVC Build Tools (in the VS Installer, tick "Desktop development with C++") + WebView2 Runtime (bundled with Win11)
- **Linux**: system libraries such as webkit2gtk, see [Tauri prerequisites](https://tauri.app/start/prerequisites/)

## 2. First Run

```bash
pnpm install
pnpm tauri dev      # the first run compiles Rust and takes a few minutes; after that incremental builds take seconds
```

When the window pops up you have succeeded. At this point it is ArkDesk in the "dark command console" style, with example tools —
the next two steps turn it into your software: **rename** (§3) → **delete the examples and write tools** (§5, §6).

## 3. Renaming: Make It Your Own Software (one-click script)

```bash
pnpm scaffold
```

Answer the 5 prompts (press Enter to keep the default):

| Prompt             | Description                                                                                                                        |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| Display name       | Appears in the window, Dock/taskbar, tray tooltip, and settings page (Chinese/English both fine)                                   |
| English identifier | kebab-case (e.g. `my-kit`), used for the package name and binary name                                                              |
| App identifier     | `com.arkdesk.<English identifier>`, **cannot be changed after release** (changing it makes the system treat it as a different app) |
| Default accent     | Indigo / Teal / Amber / Rose / Blue / State Grid Green                                                                             |
| Default theme      | dark / light                                                                                                                       |

The script automatically rewrites all names in `tauri.conf.json`, `Cargo.toml`, `main.rs`, `package.json`,
the documentation and the description text. When it is done, run as prompted:

```bash
pnpm install        # refresh the package name in the lockfile
pnpm tauri dev      # run with the new name
```

<details>
<summary>Prefer to change it manually? Here is the complete renaming checklist (equivalent to the script)</summary>

| File                                                             | What to change                                                         |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `src-tauri/tauri.conf.json`                                      | `productName`, `identifier` (cannot be changed after release), `title` |
| `src-tauri/Cargo.toml`                                           | `[package] name` (kebab), `[lib] name` (snake + `_lib` suffix)         |
| `src-tauri/src/main.rs`                                          | `arkdesk_lib::run()` → the new lib name                                |
| `package.json`                                                   | `name`                                                                 |
| `src/core/theme/index.ts`                                        | the `DEFAULT_THEME` / `DEFAULT_ACCENT` constants (optional)            |
| `src/content/about.md`, `README.md`, `docs/zh/*.md`, `AGENTS.md` | the self-introduction and titles                                       |
| `index.html`                                                     | `<title>`                                                              |

The frontend UI (sidebar/top bar) and the tray tooltip **read** productName **dynamically**; no code changes are needed.
</details>

## 4. Changing the Icon (two sources, optimal per platform)

The icon has two source files (the graphic paths are identical; only the frame scaling/corner radius differ per platform convention):

| Source file                        | Style                                                                     | Generated artifacts            | Platforms used                       |
| ---------------------------------- | ------------------------------------------------------------------------- | ------------------------------ | ------------------------------------ |
| `src-tauri/icons/icon-macos.svg`   | HIG grid whitespace (main graphic at 80.5%, effective corner radius ≈185) | `icon.icns` + Store tiles      | macOS Dock / Finder                  |
| `src-tauri/icons/icon-windows.svg` | Full bleed (the graphic fills the canvas)                                 | `icon.ico` + PNGs at each size | Windows taskbar/File Explorer, Linux |

Edit the **graphic in both SVG files** (keeping each one's frame parameters), then regenerate everything with one command:

```bash
pnpm icons
```

The script runs `tauri icon` twice and assembles by platform: icns/tiles = the macOS version, ico/png = the Windows version,
and `public/icon.svg` is kept in sync. Each platform's build (`pnpm tauri build`) automatically picks up the icon in its own format.

If the macOS Dock still shows the old icon: `killall Dock`.

## 5. Deleting the Example Plugins

The base repository ships with only two built-in plugins; keep or delete them as needed:

| Plugin        | Description                                                    | Recommendation                    |
| ------------- | -------------------------------------------------------------- | --------------------------------- |
| `hello-world` | Demonstrates the full stack (Rust/SQLite/HTTP/layout), 5 tools | Can be deleted for production use |
| `system`      | Data maintenance (frontend-only, built into the framework)     | Keep                              |

```bash
# for example, delete the example (keeping system / _template)
rm -rf plugins/hello-world
```

- The frontend (sidebar/routes/settings page) and the backend (Rust commands/migrations) are both scanned automatically at build time;
  **deleting the directory removes it completely** (migration records are scope-isolated, so deleting a plugin does not affect other plugins)
- `_template/` is the scaffold for your new tools, **be sure to keep it**
- If your fork layers on tools that need resources/heavy dependencies, see [local.md](local.md) (local layer documentation)

## 6. Developing Your First Tool

**Two steps**:

The recommended way is to generate interactively with the scaffold (which fills in the id/name/icon and the optional backend, settings, database, etc. automatically):

```bash
pnpm create-plugin
```

Or copy the template manually:

```bash
cp -r plugins/_template plugins/my-tools
```

Then edit `plugins/my-tools/plugin.json`:

```json
{
  "id": "my-tools",
  "name": "My Tools",
  "tools": [
    {
      "id": "my-tools-hello",
      "name": "Hello",
      "description": "My first tool",
      "icon": "Sparkles",
      "entry": "frontend/views/Tool.vue"
    }
  ]
}
```

Then rewrite `frontend/views/Tool.vue` — done. Sidebar navigation, routing, title, enable/disable toggle, settings panel,
error boundary, and KeepAlive all take effect automatically, **with no manual registration of any kind**.

Reference implementations (all in the hello-world plugin; you can consult them before deleting it):

| What you want to do                                             | Which file to look at                                      |
| --------------------------------------------------------------- | ---------------------------------------------------------- |
| Call Rust commands, read/write SQLite                           | `frontend/views/Tool.vue` (comprehensive Rust integration) |
| Table + pagination + modal create/update/delete + import/export | `frontend/views/TableTool.vue`                             |
| Various form controls and validation                            | `frontend/views/FormTool.vue`                              |
| Make an HTTP request and show the result                        | `frontend/views/HttpTool.vue`                              |
| Page layout and the three states (empty/loading/error)          | `frontend/views/TemplateTool.vue`                          |

For the complete steps for advanced capabilities (Rust commands, database tables, settings panels, HTTP fetching),
see [extending.md](extending.md).

## 7. Capability Extension Cheat Sheet

| Need               | Entry point                                                                                                                   | Files involved                                     |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| Rust command       | write `#[tauri::command]` in the plugin's `backend/mod.rs` (registered automatically at build time)                           | `backend/mod.rs`                                   |
| Database table     | define in `frontend/schema.ts` + append a migration in `backend/migrations.rs` (scope=plugin id, version +1 within the scope) | `schema.ts` + `migrations.rs`                      |
| Settings panel     | `useToolSettings` + declare `settings.entry` in `plugin.json`                                                                 | `frontend/settings/Settings.vue`                   |
| HTTP request       | `core/http` (`http.get/post/getJson`, no CORS)                                                                                | called directly from the frontend                  |
| Import/export file | `tauri-plugin-dialog` (already wired up) + custom Rust commands                                                               | see `TableTool.vue` + `hello-world/backend/mod.rs` |

## 8. Theme and Branding

- **Accent colors**: the settings page can switch between 6 presets; to add a custom color you must keep three places in sync:
  the accent class in `assets/index.css` → ACCENTS in `core/theme` → the anti-white-flash script in `index.html`
- **Default accent / default theme**: change `DEFAULT_ACCENT` / `DEFAULT_THEME` in `core/theme/index.ts`
  (`pnpm scaffold` can also set them interactively)
- **Root font size**: three presets on the settings page (Small 13 / Normal 14 / Large 15); the entire UI scales proportionally;
  for the preset scale definitions see the comment table in `assets/index.css`
- **Dark base tone**: dark by default (deep space blue-grey), with the light theme maintained in step; when changing the palette, refer to
  the `:root` / `.dark` sections in `index.css` and `NATIVE_BG` in `core/theme` (the native window background color)

## 9. Page Layout

`ToolShell` provides a page header (title/description/action buttons) + a full-width content area; centered columns / split columns are
organized within the page using the native Tailwind grid (no custom grid concepts are introduced):

```vue
<ToolShell title="My Tools" description="Description text">
  <div class="mx-auto grid w-full grid-cols-12">
    <div class="col-span-12 lg:col-start-3 lg:col-span-8">
      <!-- centered 8 columns; below lg it becomes full width automatically -->
    </div>
  </div>
</ToolShell>
```

Common presets: full-width tables (no class added) / wide content `col-span-10` / mixed `col-span-8` /
forms `col-span-6` — a centered column must use an **even span**. For the standard way to write the empty/loading/error states,
see the "Page Template" tool (`TemplateTool.vue`).

## 10. Packaging and Distribution

```bash
pnpm tauri build
```

Artifact locations (when `--target <triple>` is specified explicitly there is an extra triple directory):

- macOS: `src-tauri/target/**/release/bundle/macos/*.app` (with updates enabled there is also
  `<Product>.app.tar.gz` + `.sig`)
- Windows: `src-tauri/target/**/release/bundle/nsis/*-setup.exe` (msi depends on `--bundles`)

> For releasing a version with **online updates** (signing, manifest, upload, automatic release), see [release.md](release.md).

Security notes (which can be shown to reviewers): the installed version **does not listen on any local port** (frontend assets are loaded
through an in-process custom protocol, with no web server); the only network activity is HTTP requests initiated by the app.
The changelog and About page can be edited in `src/content/*.md`.

## 11. FAQ

<details>
<summary><b>On macOS, restarting dev reports "Port 1420 is already in use"</b></summary>

The tauri CLI's cleanup script may be created on macOS as an empty file without the execute bit, leaving
vite processes behind on exit. One-time fix:

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
lsof -ti:1420 | xargs -r kill -9   # clear the current leftovers
```

(Upstream tauri#15098; once the fix PR #15108 is merged and the CLI is upgraded, this item can be ignored)
</details>

<details>
<summary><b>Does the app keep running after the window is closed?</b></summary>

By design: closing the window **hides to the tray** by default (resident in the background, ready to be brought up at any time).
To really quit: right-click the tray icon → Quit (on macOS you can also use Cmd+Q). You can turn off "Hide to tray" on the settings page.
</details>

<details>
<summary><b>During dev the console reports ELIFECYCLE / a large numeric exit code (Windows)</b></summary>

When you quit the app the tauri CLI force-terminates the vite dev server; on Windows the terminated process
reports an NTSTATUS code (such as 4294967295). This is dev-mode noise and does not occur in the packaged version.
</details>

<details>
<summary><b>After adjusting the font size / theme, some screens did not follow along?</b></summary>

All UI scales with the settings; if you find a screen that does not follow, it is most likely hard-coded with px values —
switch to `rem`/semantic tokens (such as `text-sm`, `h-9`).
</details>

## 12. Pre-release Checklist

- [ ] `pnpm scaffold` has renamed it (identifier confirmed correct — it cannot be changed after release)
- [ ] the icon has been replaced and regenerated
- [ ] `src/content/about.md` / `changelog.md` have your own introduction and the first version's release notes written
- [ ] the example plugins have been deleted or disabled
- [ ] `pnpm lint && pnpm build` pass
- [ ] the `pnpm tauri build` artifacts pass a smoke test on the target platform (launch/quit/tray/maximize, no flicker)

## 13. Where to Look During Day-to-Day Development

- **AGENTS.md**: code conventions and common pitfalls (a must-read before changing code)
- **extending.md**: architectural details and the complete steps for extension development (Rust commands/database/settings panel)
- **`plugins/_template/README.md`**: file-level documentation for a single plugin

Happy developing.
