# Design Spec · Shared Core

> This document is the **single source of truth** for UI across the entire project. For platform
> differences between macOS and Windows, see
> [`design-macos.md`](design-macos.md) and [`design-windows.md`](design-windows.md); for per-control
> do/don't examples, see [`design-appendix.md`](design-appendix.md). For rework progress and migration
> rules, see `AGENTS.md` at the repository root.
>
> **References** (to check item by item during review):
>
> - Apple, "Human Interface Guidelines" (macOS portion): Layout · Color · Typography · Materials · Motion ·
>   Dark Mode · Toggles · Lists and tables · Boxes · Toolbars · Sidebars · Segmented controls · Text fields ·
>   Pop-up buttons · Search fields · Alerts · Disclosure controls · Settings · Modality · Loading ·
>   [developer.apple.com/design/human-interface-guidelines](https://developer.apple.com/design/human-interface-guidelines)
> - Microsoft, "Windows app design (Fluent / WinUI 3)": Design principles · Geometry · Layering ·
>   Materials (Mica / Acrylic) · Motion: timing and easing · Typography · Color · Commanding ·
>   Title bar · Content layout and spacing · Guidelines for app settings · Writing ·
>   [learn.microsoft.com/windows/apps/design](https://learn.microsoft.com/en-us/windows/apps/design/)
> - JetBrains, "UI Guidelines" (desktop tool density, tool windows, dialog layout, inline validation):
>   [jetbrains.design/intellij](https://jetbrains.design/intellij/)
> - Also for reference: GNOME HIG, KDE HIG

## 0. Scope and Enforcement

- Applies to **all frontend UI** in `src/` (framework shell, settings pages, example plugins) and `plugins/` (all tool plugins).
- Enforcement mechanisms (all required):
  1. **This document + platform docs + appendix**: the basis for design and review;
  2. **Token layer** (`src/assets/index.css`): all visual values may only come from tokens; arbitrary values are forbidden;
  3. **`pnpm lint`**: `scripts/lint-design.mjs` mechanically blocks old recipes (see the §6 anti-pattern list);
  4. **PR review checklist** (§7): self-check every change item by item.
- "Must / must not" in this document are hard constraints; "recommended" may be traded off by scenario, but the rationale must be stated in the PR.

## 1. Design Principles

Drawn from the intersection of the HIG "Design principles", Fluent "Design principles", and JetBrains, with concrete criteria for landing them in this project.

| Principle                                          | Concrete criterion                                                                                                                                                                                                                                                  |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Clarity first** (Simplicity / Clarity)           | One primary task per screen; information is expressed through hierarchy (order, spacing, contrast), not decoration; settings should be as few as possible (HIG: Minimize the number of settings), offered inline only on pages relevant to the task                 |
| **Immediate feedback** (Response / Feedback)       | Interactions give feedback on pointer-down; operations <1s show no spinner; long tasks are cancellable and show progress; errors are explained in place (no dialogs, no apologies)                                                                                  |
| **Undo over confirmation** (Forgiveness / Agency)  | Prefer undo to blocking; only **irreversible operations with severe consequences** use an Alert to confirm (Windows Commanding, verbatim; HIG Alerts likewise requires "avoid dialogs for common reversible actions")                                               |
| **Keyboard accessible** (Keyboard / Accessibility) | Every function can be completed with a keyboard; focus is visible and in a sensible order; shortcuts are visible in the UI; Esc closes overlays, Enter triggers the default action                                                                                  |
| **Restrained decoration** (Craft / Restraint)      | Color is used only for semantics and interaction; materials only for layering; motion only to explain causality (HIG Motion: Add motion purposefully); no boxes inside boxes (HIG Boxes: don't use nested boxes to express sub-grouping; use padding and alignment) |

## 2. Token System

### 2.1 Spacing Scale

Base grid **4px**; only the following steps are allowed (Fluent "Content layout and spacing" mapping: control spacing 8,
control and label 12, content blocks 12, surface and content 16, between buttons 8):

| Token         | Value | Typical use                                                       |
| ------------- | ----- | ----------------------------------------------------------------- |
| `--space-2xs` | 4     | Icons and text, badge padding                                     |
| `--space-xs`  | 8     | Between controls, between inline elements, between buttons        |
| `--space-sm`  | 12    | Control and label, between content blocks, card padding (compact) |
| `--space-md`  | 16    | Card/panel padding, surface and content                           |
| `--space-lg`  | 24    | Between modules, between groups                                   |
| `--space-xl`  | 32    | Page-level sections                                               |

Window content margins: compact 16 / regular 20 (macOS regular window content margin is 20pt). Mixing free-form
values such as `space-y-3/4/6` is forbidden — consistently use the utility classes corresponding to the above steps
(`gap-2/3/4/6` and `space-y-*` must map to step semantics and be whitelist-checked in lint).

### 2.2 Corner Radius (two-point scheme + overlays)

| Level     | Value    | Tailwind       | Applies to                                                            |
| --------- | -------- | -------------- | --------------------------------------------------------------------- |
| Control   | **6px**  | `rounded-md`   | Buttons, inputs, dropdowns, segmented controls, switch tracks, badges |
| Container | **8px**  | `rounded-lg`   | Panels, cards, grouped lists, dialog body                             |
| Overlay   | **12px** | `rounded-xl`   | Menus, dropdown panels, Popover, Toast, Tooltip (large)               |
| Pill      | `9999px` | `rounded-full` | Badges, avatars, slider thumbs                                        |

Implementation (key): set `--radius` to **8px**, and shadcn's derivation rules automatically yield
`md=6 / lg=8 / xl=12`, **with no need to change any component class names**. Except for overlays,
`rounded-xl` is forbidden (lint blocks it).

Reference: Fluent geometry is 8px for containers / 4px for in-page controls (0 where straight lines meet);
macOS feels more continuous. This project uses **6 / 8 / 12**
(already evaluated: 6px continues the existing `--radius: 6px` visually, and 8px matches Fluent's container value).

### 2.3 Font Size and Weight

Keep the existing **three user font-size steps** (root font size 13 / 14 / 15, `--app-font-size`), with the base
step = regular (14px). The scale and its correspondence to the two platform guidelines:

| Semantic                           | Token/Tailwind             | Regular | Small | Large | Correspondence                           |
| ---------------------------------- | -------------------------- | ------- | ----- | ----- | ---------------------------------------- |
| Body / list primary text           | `text-sm` (0.923rem)       | 12.9    | 12    | 13.8  | macOS Body 13 · Fluent Body 14           |
| Secondary text / labels / captions | `text-xs` (0.846rem)       | 11.8    | 11    | 12.7  | macOS Subheadline 11 · Fluent Caption 12 |
| Page/panel titles                  | `text-base`                | 14      | 13    | 15    | macOS Title 3 15 · Fluent Body Strong 14 |
| Numeric / monospaced data          | inherited + `tabular-nums` | —       | —     | —     | Tables, coordinates, amounts, time       |

- Line height: body 1.5; headings 1.3; table rows 1.4. Arbitrary line-height values other than `leading-none` are forbidden.
- Font weight: regular 400; secondary emphasis/selected state 500; headings 600. 700+ is forbidden (the macOS Headline exception does not apply to web UI).
- **Body font sizes below 12px are always forbidden** (Fluent states explicitly: 14 Semibold / 12 Regular are the readable lower bounds).

### 2.4 Control Height (two steps + toolbar)

| Step     | Tailwind        | Small (13) | Regular (14) | Large (15) | Use                                                                       |
| -------- | --------------- | ---------- | ------------ | ---------- | ------------------------------------------------------------------------- |
| Compact  | `h-7` (1.75rem) | 22.75      | 24.5         | 26.25      | Toolbar/inline buttons, in-table row actions, small Select                |
| Regular  | `h-8` (2rem)    | 26         | 28           | 30         | Form controls, action buttons inside panels (default)                     |
| Elevated | `h-9` (2.25rem) | 29.25      | 31.5         | 33.75      | Primary action buttons, page-level inputs (use sparingly, main path only) |

Heights are always expressed in **rem (scaling with the three font-size steps)**; hard-coded px is forbidden.
Icon button sizes match the height of the same step (`size-7/8/9`);
list row height: compact 32 / regular 36 (matching the visual rhythm of `h-8/h-9`).

### 2.5 Surfaces and Semantic Colors

**Surface enumeration** (arbitrary opacities such as `bg-muted/30`, `bg-muted/50`, `bg-muted/80` are forbidden):

| Surface | Token                                          | Use                                                 | Nesting allowed?                                          |
| ------- | ---------------------------------------------- | --------------------------------------------------- | --------------------------------------------------------- |
| base    | `bg-background`                                | Window background                                   | —                                                         |
| card    | `bg-card`                                      | Panels/cards/grouped lists                          | Yes (one group adjacent, no stacking of the same surface) |
| raised  | `bg-popover`                                   | Overlays (menus/dialogs/Toast)                      | Floats above base/card                                    |
| inset   | `bg-muted`                                     | Input wells, code block background, read-only areas | Used inside card                                          |
| sunken  | `bg-muted/60` → fixed token `--surface-sunken` | Empty/placeholder background                        | Used inside card                                          |
| console | `bg-console` / `text-console-foreground`       | Log/terminal output                                 | Isolated region, the only dark surface                    |

- **At most two surface levels per screen**: base → card (+ inner inset). Cards inside cards are forbidden (HIG Boxes: don't nest boxes to express grouping;
  use padding and alignment); when you need sub-grouping, use a **divider + heading + spacing**.
- Color semantics: `primary` (accent) is **used only for interactive controls and selected states**; `destructive / success / warning / info`
  are used only for their corresponding semantics (error/success/warning/info) and must not be decorative colors. The same color must not express two meanings (HIG Color).
- Don't place accent colors on translucent layers (Apple Materials: put color on solid layers, not on frosted-glass foregrounds).
- **Sole exception**: the Home Hero is a brand signature area and may use a low-saturation decorative `primary` (see `src/layouts/Home.vue`);
  no other location may use accent as decoration.

### 2.6 Borders and Dividers

- All borders/dividers use a **1px hairline** (`border` = `--border`); `border-border` must not be mixed in (lint blocks it;
  their current coexistence is historical legacy).
- **Box borders don't stack**: when a container already has a border, separate content inside with `divide-y` or `border-t`, rather than drawing a second box.
- Tables: data rows use `divide-y` (no vertical lines); the header uses `border-b` + sticky; when a row needs emphasis use `bg-muted/60` zebra striping (macOS tables may have zebra striping).
- Focus is expressed with `ring` (§2.9), **not by changing border color**.

### 2.7 Motion

| Token        | Value | Reference    | Use                                                       |
| ------------ | ----- | ------------ | --------------------------------------------------------- |
| `--dur-fast` | 120ms | Fluent 83ms  | hover, press, color/opacity changes                       |
| `--dur-base` | 200ms | Fluent 167ms | expand/collapse, Tab switch, Popover enter/exit           |
| `--dur-slow` | 300ms | Fluent 250ms | dialogs, page transitions, large-area material enter/exit |

- Easing: enter `cubic-bezier(0.22, 1, 0.36, 1)` (≈ Fluent Fast Out, Slow In `cubic-bezier(0,0,0,1)`);
  exit `cubic-bezier(0.4, 0, 1, 1)`.
- Animate only `transform` and `opacity` (plus `backdrop-filter` for material enter/exit); animating
  `width/height/top/left` is **not allowed**.
- `prefers-reduced-motion: reduce` → translate/scale becomes a 200ms cross-fade, removing all overshoot.
- **Remove the existing global `transition-duration: 75ms !important`**; keep the 0s exception for
  `[data-reka-popper-content-wrapper]` (a technical override for positioning measurement, unrelated to the motion
  spec — don't delete it along with the rest).

### 2.8 Materials (vibrancy)

Allowed regions are **limited to**: sidebar, toolbar/title bar, overlays (menus/dialogs/Toast/Tooltip).
**Forbidden**: frosted glass on scrolling content areas, tables, forms, or card bodies (performance and readability).

- macOS → `NSVisualEffectView` semantic materials: sidebar `sidebar`, toolbar `headerView`, overlays `popover`/`menu`,
  dialogs `sheet` (see `design-macos.md` for details).
- Windows → `Mica` (window base) / `Mica Alt` (apps with navigation and command areas, officially recommended) / `Acrylic` (overlays)
  (see `design-windows.md` for details).
- **A solid-color fallback is mandatory**: `prefers-reduced-transparency: reduce`, user-disabled transparency, Windows battery saver/low-end hardware,
  window unfocused → fall back to solid `bg-card`/`bg-background`. The fallback is not a degraded experience; it must be readable in both forms.
- At most 2 materials per screen; no material on top of material; foreground colors on material layers must have higher contrast (don't use `text-muted-foreground` for primary text).

### 2.9 Focus and Keyboard

- Focus ring (**canonical expression, the only form**): `focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60`;
  shape follows the control's corner radius; **`focus-visible` is always visible**; `outline-none` without a replacement is forbidden (all 11 components under `ui/` have been unified; lint R6 blocks deviations).
  On Windows, `--focus-ring-width: 2px` changes it to a double-stroke look (see `design-windows.md §7`).
- All custom interactive elements must: be focusable (`tabindex="0"`), have a `role` and a readable name (`aria-label`/text), and respond to Enter/Space.
- Composite control keyboard semantics (new requirement; currently 0 tabindex / 3 role):
  - Segmented controls: `role="tablist"` + ←/→ to move, Home/End for first/last;
  - List/table rows: ↑/↓ to move, Enter to activate, type-ahead support (if it's an option list);
  - Menus/dropdowns: ↑/↓ to cycle, Esc to close and return focus, typing characters locates items.
- Global shortcuts (toolbar/menu must be consistent, see platform docs): `⌘,` settings · `⌘K` command palette · `⌘F` in-page search ·
  `⌘W` close window · `⌘Z / ⇧⌘Z` undo/redo · `⌘1..9` switch sidebar group · `Esc` close overlay.
- Shortcuts are visible in the UI (`<kbd>` or tooltip); "hidden features" are not allowed.

### 2.10 Icons

- Uniformly `@lucide/vue`; only 3 sizes: **14 (inline) / 16 (buttons, menus) / 20 (toolbar, empty state)**;
  stroke width is uniformly 2 (lucide default), not changed individually.
- When placed next to text use `gap-2`, baseline-aligned (icons center with the line height, no manual `mt-*`).
- Icon buttons **must** have `aria-label` + tooltip (Tooltip currently exists in only 1 place, a gap).
- Icons don't express status colors (status uses text/badges), and multiple icons aren't stacked to express one action.

## 3. Component Semantics (which to use when)

| Scenario                                                                                 | Use                                                                             | Don't use                              |
| ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | -------------------------------------- |
| 2–5 mutually exclusive options where the selection state must always be shown            | **Segmented control** (`Segmented`)                                             | A row of switches, dropdown            |
| 6+ mutually exclusive options / limited space                                            | Dropdown (pop-up button)                                                        | Segmented (HIG: segmented ≤ 5–7 items) |
| Fine-grained boolean settings                                                            | **Checkbox** (label on the right)                                               | Switch                                 |
| Emphasizing a whole group of switches / an important switch (e.g. "Enable this feature") | **Switch** (use mini size in grouped forms)                                     | Checkbox                               |
| Fine-tuning a numeric value                                                              | Input + stepper                                                                 | Slider (sliders only for rough values) |
| Live search/filter                                                                       | `SearchField` (with magnifier, clear button, search-as-you-type)                | Plain input + button                   |
| Irreversible dangerous operations                                                        | `Alert` (the default button is the safe one, the destructive one is marked red) | A plain confirmation dialog, toast     |
| Reversible operations                                                                    | Execute directly + status feedback (+ undo entry point)                         | Dialog confirmation                    |
| Long tasks (>1s)                                                                         | Progress bar/ring + cancellable                                                 | Spinner overlay                        |
| Page-level loading (content only appears after >300ms)                                   | Skeleton (structure matching the real content)                                  | spinner overlay                        |
| Result/log-style output                                                                  | Read-only text block (`bg-console` for logs only)                               | Editable input                         |

Button hierarchy: **at most one primary button per view** (`default`); then `secondary`; `ghost` for panel header actions;
`destructive` only for destructive operations, and it requires a second Alert confirmation.

## 4. Three States and Feedback Rules

**Empty state**: icon (20) + one line of primary copy + one line of explanation + one action button; never a blank page.
**Loading state**: <1s shows no loading indicator at all (HIG Loading: Show something as soon as possible);
1s–10s use a skeleton or progress bar; >10s must be cancellable and show progress.
**Error state** (unified three-way split, replacing the current mix of 81 `toast.error` vs 35 inline):

| Scenario                                                           | Presentation                                                                                                                         |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| Errors inside a form/tool page that can be located                 | **In place**: `text-xs text-destructive` below the control; for multi-field errors add an error bar at the top of the page           |
| The current operation failed but there's a clear retry entry point | An error bar inside the tool page panel (`border-destructive/30 bg-destructive/10`) + a retry button                                 |
| Background task/system-level failure (no in-place location)        | `toast.error` (the only scenario where reporting errors via toast is allowed)                                                        |
| Success                                                            | **No toast**: express it through state changes (row/badge/button state/result area); only long-task completion may use an info toast |

Recoverable notices use `info`/`warning` alert bars (`bg-info/10`, `bg-warning/10`), not toasts.
Server/backend error copy: explain what happened + what to do next; don't apologize, and don't use "something went wrong".

## 5. Copy Guidelines

- **Buttons use verbs** (Save, Export, Choose again), and one action keeps the same name throughout a flow; buttons have no trailing period.
- Buttons that open an overlay/dialog take an ellipsis (`More options…`); buttons that execute immediately do not.
- Sentence case (not an issue in Chinese, but English terms must be consistent); no all-caps.
- Glossary (the repository's unified wording):

| Use                  | Don't use                                |
| -------------------- | ---------------------------------------- |
| Tool / plugin        | App, feature module                      |
| Module panel / panel | Card, block                              |
| Settings             | Configuration, preferences, options page |
| Export / Import      | Save as, download                        |
| Worksheet (Excel)    | sheet, tab                               |
| Header               | column name, header                      |

- Numbers and units: numbers are right-aligned + `tabular-nums`; a space between the unit and the number (`5 MB`); time uses `yyyy-MM-dd HH:mm`.
- Error copy doesn't apologize, isn't cutesy, and doesn't use consecutive `!`; placeholder copy explains "what you can enter", not "Please enter…".

## 6. Anti-Pattern List (measured before the rework; lint blocks the machine-checkable items)

| Anti-pattern                   | Current evidence                                                                               | Requirement                                                        |
| ------------------------------ | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| Two card recipes coexisting    | `rounded-lg border bg-card` in 6 files / `rounded-lg border border-border bg-card` in 14 files | Unify on `rounded-lg border bg-card` (§2.2/§2.6)                   |
| Arbitrary opacity              | `bg-muted/30` 6 places · `bg-muted/50` 5 places · `bg-muted/80` several                        | Use the surface enumeration (§2.5)                                 |
| A third corner-radius value    | `rounded-xl` 3 places (non-overlay)                                                            | Only overlays may use `xl` (§2.2)                                  |
| Mixed error feedback           | `toast.error` **81** places vs inline `text-destructive` **35** places                         | The three-way split (§4)                                           |
| Not keyboard-reachable         | `tabindex` **0** places · `role=` **3** places                                                 | Custom interactive elements must be focusable + have a role (§2.9) |
| Hand-rolled segmented controls | 4 places (the four crypto panels)                                                              | Use `Segmented` (§3)                                               |
| Icon buttons with no hint      | Tooltip in only 1 place; `kbd` hints 3 places                                                  | Icon buttons must have tooltip + `aria-label` (§2.10)              |
| Free-form spacing              | `space-y-3` 22 · `space-y-4` 24 · `space-y-6` 9                                                | Use the spacing steps (§2.1)                                       |
| Global motion suppression      | `transition-duration: 75ms !important`                                                         | Motion tokens (§2.7)                                               |
| Cards inside cards             | Many places (a `rounded-lg border` container inside a Panel)                                   | Use divider + heading + spacing inside the same container (§2.5)   |

## 7. PR Review Checklist

1. Do **all** the new/modified visual values come from tokens (no arbitrary values, no hard-coded px heights/font sizes)?
2. Does each screen have **at most two surface levels** (no cards inside cards)? Do borders avoid stacking?
3. Is every module wrapped in a `Panel` (or the platform-equivalent container), with no bare modules?
4. Does the control choice for boolean/mutually exclusive options follow §3 (checkbox vs switch vs segmented vs dropdown)?
5. Are all three states — loading/empty/error — present, and does error presentation follow the §4 three-way split?
6. Keyboard: is the Tab order sensible, is focus visible, are Esc/Enter semantics correct, and are shortcuts visible in the UI?
7. Do icon buttons have `aria-label` + tooltip? Are glyphs and sizes taken from the three steps?
8. Does the copy follow §5 (verbs, glossary, punctuation, units)?
9. Do light/dark, the three font-size steps, and window widths 760/1080/1440 all avoid breaking the layout?
10. Do platform differences fall only within the differences allowed in the §platform docs (materials/fallback, window shell, menus/shortcut presentation, dialog button semantics, focus visual, radius step)?
