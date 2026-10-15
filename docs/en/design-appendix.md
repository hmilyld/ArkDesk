# Design Spec · Component Appendix (Do / Don't)

> Shared rules are in [`design.md`](design.md); platform differences are in [`design-macos.md`](design-macos.md) /
> [`design-windows.md`](design-windows.md). This file holds the **per-control do/don't examples** and the
> **migration cross-reference table**; check items off against it one by one during refactoring. Register a new control
> here before implementing it.

## 1. Facade component list (`src/components/native/`)

| Facade component                                             | Based on                           | Responsibility                                                                                 | Replaces                                                   |
| ------------------------------------------------------------ | ---------------------------------- | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `Toolbar.vue`                                                | New                                | Toolbar row (40px mac / 32px win), left-side identity, right-side commands and "More" overflow | The hand-drawn part of `TitleBar.vue`                      |
| `SidebarList.vue`                                            | `SideNav.vue`                      | Source list: group headers, items, selection state, collapse                                   | `SideNav.vue`                                              |
| `Panel.vue`                                                  | Framework `tool/Panel.vue`         | Module panel (exists; add `overflow-hidden` and the material-disable constraint)               | 6 hand-written cards                                       |
| `SettingSection.vue` / `SettingRow.vue` / `SettingField.vue` | `@/components/settings`            | Group + row + field (already unified)                                                          | —                                                          |
| `ListRow.vue`                                                | New                                | List row: primary text + secondary text + trailing control/icon; keyboard reachable            | Hand-rolled `flex items-center justify-between` everywhere |
| `FormRow.vue`                                                | New                                | Label + control + **inline validation** (error text and aria)                                  | `space-y-1.5 + Label` everywhere                           |
| `SearchField.vue`                                            | `ui/input`                         | Magnifier + clear button + search-as-you-type                                                  | None (plain Input used today)                              |
| `Segmented.vue`                                              | `ui/tabs` or new (reka RadioGroup) | 2–7 item mutually exclusive segmented control, with ←/→ keyboard semantics                     | 4 hand-built (crypto's four panels)                        |
| `PopUp.vue`                                                  | `ui/select`                        | Unified dropdown (width, chevron, keyboard, empty state)                                       | `SelectTrigger` widths vary everywhere                     |
| `ToggleRow.vue`                                              | `ui/switch` / `ui/checkbox`        | Boolean option row (checkbox = fine-grained / mini switch = emphasis)                          | Switch and Label placement varies                          |
| `StatusBar.vue`                                              | New                                | Bottom status bar (process, line/column count, path)                                           | None                                                       |
| `EmptyState.vue` / `LoadingState.vue` / `ErrorState.vue`     | New                                | The three-state standard parts (icon 20 + text + action)                                       | 13 different dashed boxes                                  |
| `AlertDialog` semantic wrapper                               | `ui/alert-dialog`                  | Destructive-operation confirmation (button order by platform)                                  | `ui/dialog` usage needs review                             |
| `ProgressBar.vue`                                            | New, beyond `ui/slider`            | Determinate/indeterminate progress + cancel                                                    | `TaskProgress.vue`                                         |

> **Implementation status**: Landed with this spec are `Panel.vue` (`src/components/tool/`),
> `SettingSection/Row/Field` (`src/components/settings/`), and `EmptyState` / `LoadingState` / `ErrorState` /
> `FormRow` / `ListRow` / `SearchField` / `Segmented` (`src/components/native/`).
> The `Toolbar` / `SidebarList` / `PopUp` / `ToggleRow` / `StatusBar` / `ProgressBar` / `AlertDialog` wrappers
> are **planned items**; implement them per the conventions below when needed and fill them back into this section.

## 1.5 Known pitfalls (reka-ui 2.10 + shadcn-vue)

- **The `data-active:` / `data-inactive:` variants are dead rules**: reka's `TabsTrigger` only emits
  `data-state="active|inactive"`, so selected styles built on `data-active:` in shadcn components never match.
  Always use `data-[state=active]:` when changing styles.
- **`TabsList`'s horizontal height is pinned by a group variant**: `group-data-[orientation=horizontal]/tabs:h-9`,
  which a plain `h-*` cannot override; you need inline styles to override it.
- **Global transition suppression is disabled**: `transition-duration: 75ms !important` has been removed; animations
  go through tokens. The `transition-duration: 0s` exception on reka's popper positioning container must be kept
  (unrelated to animation; it fixes measurement-induced drift).

## 2. Per-control Do / Don't

### Buttons

| Do                                                                         | Don't                                                            |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| One primary button (`default`) per screen; the rest `secondary`/`ghost`    | Multiple `default`s on one screen competing for attention        |
| `ghost` only for panel-header/toolbar actions                              | `ghost` as the primary action                                    |
| Two height tiers (`h-8` regular / `h-7` compact), icon `size-3.5` + `mr-1` | `h-9` mixed with other sizes (except page-level primary actions) |
| Destructive operations `destructive` + Alert confirmation                  | Using red to express "Cancel" or ordinary actions                |
| Copy uses verbs consistent with the result (`Export` → "Exported")         | Generic words like `OK`/`Submit`/`Run`                           |

### Inputs / Textareas

| Do                                                                  | Don't                                                   |
| ------------------------------------------------------------------- | ------------------------------------------------------- |
| `FormRow` provides label + description + inline validation          | Placeholder only, no label                              |
| Fixed height + scroll when overflowing (`field-sizing: fixed`)      | Auto-grow causing layout shift                          |
| Numeric inputs right-aligned + `tabular-nums`                       | Numbers left-aligned, font not monospaced               |
| Validation in place (below the control, `text-xs text-destructive`) | Errors only in a toast, or validation only after submit |
| Monospace for code/paths/keys                                       | Monospace for body text                                 |

### Dropdowns / Segmented / Checkboxes / Switches / Search

| Do                                                                      | Don't                                                          |
| ----------------------------------------------------------------------- | -------------------------------------------------------------- |
| 2–5 mutually exclusive items → segmented; ≥6 items → dropdown           | A row of switches to express mutually exclusive options        |
| Fine-grained boolean → checkbox (title on the right)                    | Switch for fine-grained boolean (wrong macOS semantics)        |
| Switches inside grouped forms use mini (`size="sm"`)                    | Large switches inside grouped forms causing uneven row heights |
| A switch must share the same row and region as its title (`SettingRow`) | `justify-between` pulling title and switch to opposite ends    |
| Search field searches as you type + clear button                        | A "Search" button on the search field                          |
| Dropdown width fixed (control column aligned)                           | Each dropdown a different width, left edges ragged             |

### Panels / Groups / List rows

| Do                                                                       | Don't                                                 |
| ------------------------------------------------------------------------ | ----------------------------------------------------- |
| Every module wrapped in `Panel`; header bar + body `p-4`                 | Modules exposed bare on the page                      |
| Sub-groups within one container use `divide-y` + small heading + spacing | Cards inside cards, boxes stacked on boxes            |
| Row: primary text + secondary text (`text-xs` gray) + trailing control   | Three or more lines of explanation crammed into a row |
| List rows keyboard-focusable (`tabindex` + role)                         | Rows clickable only with the mouse                    |

### Overlays (menu / dialog / Toast / Tooltip)

| Do                                                                            | Don't                                                             |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Overlay corner radius 12 (Windows 8), unified shadow + 1px border             | Inconsistent corner radius/shadow across overlays                 |
| Esc closes and restores focus to the trigger element                          | Focus lost after closing                                          |
| Menu items ≥24px row height, icon 16, destructive items last and separated    | Menu items too dense, destructive items adjacent to ordinary ones |
| Tooltip carries supplementary description only (icon buttons, truncated text) | Tooltip carrying must-read information                            |
| Toast only for system-level failures/long-task completion                     | Using a toast to report form validation errors                    |

### Tables / data areas

| Do                                                                        | Don't                                                |
| ------------------------------------------------------------------------- | ---------------------------------------------------- |
| `divide-y` to separate rows, sticky header, numeric columns right-aligned | A box border on every row, numbers centered in cells |
| Empty data uses `EmptyState`                                              | A blank table                                        |
| Scroll areas use a solid background (no frosted glass)                    | Material overlaid on the table                       |
| Horizontal scroll when too wide + sticky first column (if needed)         | Squeezing column widths until text wraps             |

### Three states

| Do                                                           | Don't                                                              |
| ------------------------------------------------------------ | ------------------------------------------------------------------ |
| Empty state: icon (20) + primary text + description + action | Only showing "No data"                                             |
| <1s no loading state; 1–10s skeleton; >10s progress + cancel | A spinner mask over every operation                                |
| Error bar: explain the cause + next step + retry             | Uninformative copy like "Something went wrong", "Operation failed" |

## 3. Icons and shortcuts

| Do                                                                  | Don't                                     |
| ------------------------------------------------------------------- | ----------------------------------------- |
| Sizes 14/16/20 (three tiers), stroke 2                              | Custom icon size on every page            |
| Icon buttons always have `aria-label` + tooltip                     | Icon-only with no hint                    |
| Shortcuts visible in menu items/hints (`⌘K` / `Ctrl+K` by platform) | Hidden shortcuts, only documented in docs |

## 3.5 Design review workflow (self-verifying screenshots, no manual screenshots)

The Web layer can render in an ordinary browser (the preview bridge emulates the Tauri API), so screenshots can be scripted for self-verification:

```bash
npx vite                                    # start the frontend (1420)
node scripts/design-shot.mjs \
  --url "/tool/hello-table" --out /tmp/a.png \
  --theme dark --font 14 [--platform win] \
  [--eval "document.querySelector('button').click()"]   # you can click/type before taking the screenshot
```

- Preview bridge `src/dev/preview-bridge.ts`: active only under `import.meta.env.DEV` and a non-Tauri environment;
  implements the `__TAURI_INTERNALS__` protocol + a localStorage-backed `plugin-store` + framework command fixtures (including `db_query_values`).
  A plugin's own business-command fixtures go in `plugins/<id>/frontend/preview.ts` (**default-exporting** `Record<command name, return value>`),
  auto-merged during dev preview; **base contains no plugin business commands**, and a fork's plugin fixtures stay in its own repository alongside the plugin.
- URL parameters: `?theme=light|dark`, `?font=13|14|15`, `?accent=<name>`, `?platform=win`.
- Purpose: check spec clauses screen by screen and verify overlays/focus/three states; **the native window chrome and materials still need confirmation on a real machine**.

## 4. Migration cross-reference table (refactor batches)

| Batch | Scope                                                                       | Files | Risk   | Status | Notes                                     |
| ----- | --------------------------------------------------------------------------- | ----- | ------ | ------ | ----------------------------------------- |
| P3-1  | Shell: `TitleBar` `SideNav` `MainLayout` `Settings` `Home` `CommandPalette` | 6     | High   | ✅     | Includes material/window contract changes |
| P3-2  | `ui/` style overrides for 18 shadcn components (styles only)                | 18    | Medium | ✅     | Corner radius/focus/height tokenized      |
| P4-1  | `hello-world` (the live base template)                                      | 9     | Low    | ✅     | Fully rewritten as the spec exemplar      |
| P4-2  | `system`                                                                    | 6     | Low    | ✅     | Table/data maintenance page               |
| P4-3  | `_template`                                                                 | 2     | Low    | ✅     | Kept consistent with base                 |

> base contains only three plugins, `_template` / `hello-world` / `system`, so the table above is all the batches;
> **a fork's personal plugins are refactored batch by batch against the same list, with each fork maintaining its own progress**
> (the framework layer does not need to know a fork's plugin names).
> After each batch: `pnpm lint && pnpm build && pnpm test`, and self-check item by item against the review checklist in `design.md` §7;
> batches that touch Rust additionally run `pnpm fmt:rs && pnpm lint:rs && pnpm test:rs`.
