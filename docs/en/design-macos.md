# Design Specification · macOS Platform Layer

> This document covers **platform differences** only. For shared rules see [`design.md`](design.md); for Windows see [`design-windows.md`](design-windows.md).
> Based on the macOS portion of Apple's《Human Interface Guidelines》(Designing for macOS / Layout / Materials / Typography /
> Toggles / Toolbars / Sidebars / Lists and tables / Boxes / Segmented controls / Alerts / Settings / Motion /
> Dark Mode / Writing).

## 1. Window Shell

| Item                      | Rule                                                                                                                                 | Current State                          |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------- |
| Title bar style           | `titleBarStyle: Overlay` + `hiddenTitle`, content extends into the title bar (unified toolbar look)                                  | Compliant                              |
| Traffic-light inset       | Fixed inset on the left: `x: 10, y: 14` (existing values); content must not cover it                                                 | Compliant                              |
| Title bar height          | Toolbar row height **40px** (compact unified toolbar), separated from the content below by a hairline; no CSS-shadow divider         | Currently 40px                         |
| Drag region               | The entire title bar/toolbar is `data-tauri-drag-region`; buttons excepted                                                           | Compliant                              |
| Window controls           | Use the system traffic lights (do not draw your own); do not place minimize/maximize/close on the right side of the toolbar          | Compliant (only Windows draws its own) |
| Key information placement | Do not place key controls or important information at the bottom (HIG Layout: the bottom edge of a window is often moved off-screen) | Needs self-check                       |
| Full screen               | Full screen is supported; the toolbar and sidebar materials stay unchanged when entering full screen                                 | To be verified                         |

## 2. Materials (vibrancy)

Mapping between macOS's `NSVisualEffectView` semantic materials and Tauri `windowEffects`
(`tauri.conf.json > app.windows[].windowEffects`):

| Region                    | Apple semantic material | Tauri `effects`                                  | Blending      | Fallback              |
| ------------------------- | ----------------------- | ------------------------------------------------ | ------------- | --------------------- |
| Sidebar (navigation list) | Sidebar                 | `sidebar`                                        | behind window | `bg-sidebar` solid    |
| Toolbar / title bar       | Header view             | `headerView`                                     | behind window | `bg-background` solid |
| Dropdown/menu             | Menu                    | `menu` (or CSS frosted glass inside the overlay) | within window | `bg-popover` solid    |
| Overlay/Popover           | Popover                 | `popover`                                        | within window | `bg-popover` solid    |
| Dialog (sheet)            | Sheet                   | `sheet`                                          | within window | `bg-card` solid       |

- `state` should be `followsWindowActiveState`: the material automatically becomes lighter when the window loses focus, matching the macOS look.
- **Depends on `Reduce Transparency`**: on the CSS side, switch to a solid layer when `prefers-reduced-transparency: reduce` is detected,
  and stop requesting window transparency at the same time (for the corresponding Rust-side logic see `design.md` §2.8 and the migration record in `AGENTS.md`).
- **Window background contract (implemented)**: materials are enabled through the Rust `apply_window_effects()` (on macOS `underWindowBackground`
  - `followsWindowActiveState`; on Windows `tabbed` = Mica Alt). Once enabled, `set_window_background` automatically skips the solid base layer —
    white-flash prevention is instead guaranteed by an **opaque base on the content layer** (`body` is always a solid `bg-background`, only the chrome is translucent).
    If enabling the material fails (or the platform does not support it), it silently falls back to solid without affecting usability.
- **Layout premise (implemented; do not revert to an overlay-style toolbar)**: the toolbar **stays in the flow** (`TitleBar` is not absolutely positioned, `main` gets no top padding);
  blur is provided by the **sticky page header** — both the page header of `ToolShell` and the tab strip of the settings page are `sticky top-0` + material, and the blur is visible once content passes underneath them.
  An earlier "toolbar absolutely positioned + `main` top padding = toolbar height" unified-toolbar approach was tried, but it stacked with the page's own padding into a
  **visible blank band** (extra spacing above every page title) and was reverted; the toolbar's "see-through to the desktop" is handled by option B (native materials).
- Do **not** add custom backgrounds or tints to the toolbar (HIG Toolbars: a custom background interferes with the system-provided background effects).
- Do not place accent colors on translucent layers; on material layers the primary text uses `text-foreground` (not `text-muted-foreground`).

## 3. Density and Controls (macOS native comparison)

| Semantics              | This project (three size tiers) | macOS native                          |
| ---------------------- | ------------------------------- | ------------------------------------- |
| Regular control height | `h-8` (26/28/30px)              | Regular 22pt (≈ the small tier `h-7`) |
| Compact control height | `h-7` (22.75/24.5/26.25px)      | Small 19pt / Mini 16pt                |
| List row height        | 36 (compact 32)                 | Table row 24pt (list row 32pt)        |
| Panel padding          | 16 (compact 12)                 | Content margin 20pt (compact 16pt)    |

Selection rule: **forms and tool pages use the regular tier (`h-8`); toolbars, in-row table actions, and dense settings use the compact tier (`h-7`)**;
do not mix the three size tiers within a single screen.

## 4. Control Semantics (macOS-specific)

- **Switch vs. checkbox** (HIG Toggles): use a **checkbox** for fine-grained boolean settings (title on the right, aligned consistently);
  use a **switch** for "emphasis-type" settings or "the switch for a whole group of settings"; switches inside grouped forms (`SettingsSection`) use the **mini size** (`size="sm"`),
  so that the row height matches the buttons.
- **Switches appear only in list rows**: the row content provides context; they must not float on their own. Do not use `justify-between`
  to push a Label and a Switch to opposite ends of a row (on a wide panel this reads as two disconnected pieces of text).
- **Segmented control**: for 2–7 closely related options that need an always-visible selected state and that are **not** window-level view switching;
  do not mix actions and options within a segment.
- **Dropdown (pop-up button)**: ≥6 items, or long option text; the current value is shown inside the control (left-aligned + chevron on the right).
- **Text field**: use a combo box (editable dropdown) when "input + value selection" is needed; do not add embedded buttons to plain inputs.
- **Search field**: magnifier + left alignment, search-as-you-type, clear button on the right (`SearchField`).

## 4.5 Settings Page (pane switching)

- Use a **toolbar-style horizontal tab strip** for pane switching (the way Safari / Xcode preferences do it): stuck below the toolbar, same material.
  **The selected state uses a `bg-primary` solid fill + `text-primary-foreground`**, consistent with the "Theme" segmented control on the same page;
  do not use a low-saturation tint — it gets washed out when laid over frosted-glass material (same as §2.5 "do not place light accent colors on translucent layers").
  Do not use vertical navigation on the left (to avoid forming three columns with the app sidebar).
- Strip height and vertical spacing: `height: auto` + an equal `py-1.5` (**do not** use a fixed height + `items-center`;
  the horizontal group variant `h-9` of shadcn `TabsList` stacks with the container border to produce a "bottom spacing too small" look).
- Content is a single centered column (12-grid `col-start-3 col-span-8`); each group uses `SettingsSection` + `SettingsRow`.
- Settings take effect **immediately**; there is no "Apply / OK".

## 5. Dialogs and Modals

- **Prefer sheets**: confirmations/forms strongly tied to the current window context (such as rename, import parameters) use a sheet attached to the window;
  global, destructive actions that need a strong interruption use a centered Alert-style dialog.
- **Button order**: Cancel on the left, the default button on the **far right**; the default button is triggered by Return (`Return`), `Esc` triggers cancel.
  Button labels use specific verbs (`Delete`, `Save`), not the fallback semantics of `OK/Cancel` (except in purely confirmatory cases).
- **Alert restraint** (HIG Alerts): do not show Alerts for informational, common, undoable actions; do not show Alerts at startup.
- Operations that can be completed inline within a panel do not go into a modal (HIG Modality: a modal must have a clear benefit).

## 6. Menu Bar and Shortcuts

- The global menu bar is provided by `src-tauri/src/menu.rs`; **every command in the toolbar must have a corresponding item in the menu bar**
  (HIG Toolbars), and conversely, if a menu item affects the current view, it must also appear in the toolbar or the command palette.
- Standard menus and key bindings:

| Action               | Key binding  | Menu                   |
| -------------------- | ------------ | ---------------------- |
| Settings             | `⌘,`         | App name › Settings…   |
| Command palette      | `⌘K`         | View › Command Palette |
| In-page search       | `⌘F`         | Edit › Find            |
| Close window         | `⌘W`         | File › Close           |
| Close all/quit       | `⇧⌘W` / `⌘Q` | File › Quit            |
| Undo/redo            | `⌘Z` / `⇧⌘Z` | Edit                   |
| Switch sidebar group | `⌘1..9`      | View                   |
| Reload (dev only)    | `⌘R`         | View                   |
| Full screen          | `⌃⌘F`        | View                   |
| Close overlay        | `Esc`        | —                      |

- Every shortcut must be visible in the interface (in a tooltip or on the right side of a menu item); do not hide functionality.
- Context menus use the system right-click (the existing `EditorContextMenu`), with item order: common → separator → destructive item last.

## 7. Sidebar and Lists

- The sidebar is a **source list**: group headings (`text-xs`, `text-muted-foreground`) + items (icon 16 + text 13, row height 28–32),
  the selected state uses `bg-sidebar-accent` (translucent), not a border or accent line; when the container window becomes narrow, **automatically collapse the sidebar** (HIG Sidebars).
- Do not place key information/actions at the bottom of the sidebar (HIG: the bottom edge of a window may be moved off-screen).
- Tables: row height 24–32, sticky header, `divide-y` separators, numeric columns right-aligned + `tabular-nums`, optional zebra striping;
  on macOS, column resizing and header sorting may be considered (**not in this round**, see `design.md` §0 scope).

## 8. Fonts and Colors

Mapping between macOS system font sizes and this project's three tiers (HIG macOS built-in text styles):

| HIG style             | Size/line height | This project                                                                      |
| --------------------- | ---------------- | --------------------------------------------------------------------------------- |
| Large Title / Title 1 | 26/32 · 22/26    | Page Hero title (rarely used)                                                     |
| Title 2 / Title 3     | 17/22 · 15/20    | Page title (`text-base` ↗ large tier)                                             |
| Headline              | 13/16 Bold       | Panel title (`text-xs` + 600)                                                     |
| Body                  | 13/16            | List body text (`text-sm`)                                                        |
| Callout / Subheadline | 12/15 · 11/14    | Supplementary description (`text-xs`)                                             |
| Footnote / Caption    | 10/13            | Badges, very small notes (this project's lower bound is 11, see `design.md` §2.3) |

- Font family: system fonts preferred (`-apple-system` / `SF Pro Text` / `SF Mono`), already configured; do not introduce third-party fonts.
- Dynamic font size: the three user font-size tiers (13/14/15 root font size) are equivalent to macOS's text-size adjustment, and control dimensions scale with rem.
- **Dark Mode**: respect the system appearance ("Follow system" must not override it); it must also be self-tested under `Increase Contrast` and
  `Reduce Transparency` (HIG Dark Mode / Color).
- Accent color: users can choose it in settings (the existing 6 accents); **use it only for interactive controls and selected states**; not for decoration, not for translucent layers.

## 9. Motion

- Use the shared durations/easings from `design.md` §2.7 (the macOS look leans toward 200–300ms).
- Window/material entrance and exit: animate the blur radius together with the scale (the material "arrives in place" rather than fading in).
- Respect `prefers-reduced-motion`; do not use continuous looping animations as the sole state indicator.

## 10. Scope and To-dos for This Round

- This round is **consistency only**: no new features (table sorting/column width, disclosure collapsing, and true sheet presentation are all listed as follow-ups).
- To-dos (check off one by one during migration): traffic-light inset not covered by content · automatic sidebar collapsing · alignment of toolbar and menu commands ·
  material and opaque-base contract rework · focus rings and keyboard reachability · self-test of Dark Mode/contrast/reduce transparency.
