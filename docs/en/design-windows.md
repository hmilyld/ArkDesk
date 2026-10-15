# Design Specification · Windows Platform Layer

> This document covers **platform differences** only. For shared rules see [`design.md`](design.md); for macOS see [`design-macos.md`](design-macos.md).
> Based on Microsoft's《Windows app design (Fluent / WinUI 3)》:
> [Design principles](https://learn.microsoft.com/en-us/windows/apps/design/design-principles) ·
> [Geometry](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/geometry) ·
> [Layering](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/layering) ·
> [Materials](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/materials) ·
> [Mica](https://learn.microsoft.com/en-us/windows/apps/design/style/mica) ·
> [Motion: timing and easing](https://learn.microsoft.com/en-us/windows/apps/design/motion/timing-and-easing) ·
> [Typography](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography) ·
> [Commanding](https://learn.microsoft.com/en-us/windows/apps/design/basics/commanding-basics) ·
> [Title bar](https://learn.microsoft.com/en-us/windows/apps/design/basics/titlebar-design) ·
> [Content layout and spacing](https://learn.microsoft.com/en-us/windows/apps/design/basics/content-basics) ·
> [Guidelines for app settings](https://learn.microsoft.com/en-us/windows/apps/design/app-settings/guidelines-for-app-settings) ·
> [Writing style](https://learn.microsoft.com/en-us/windows/apps/design/style/writing-style)

> ⚠️ **Verification status**: this platform layer **has not yet undergone visual verification** (the development machine is macOS, with no Windows environment). All values come from the official documentation and
> are implemented as "statically correct"; after the first verification on Windows, screenshots and deviation records need to be filled in. For the risk register see `AGENTS.md`.

## 1. Window Shell

| Item                 | Rule                                                                                                                                                      | Basis                                        | Current State                                                                    |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | -------------------------------------------------------------------------------- |
| Title bar            | Custom-drawn (`set_decorations(false)`), height **32px**                                                                                                  | Title bar: standard title bar height is 32px | Currently 40px → needs to change to 32px                                         |
| Window icon          | Left side, vertically centered (8px margin top and bottom at a height of 32px); single click opens the system window menu, double click closes the window | Title bar behavior conventions               | Currently no icon → needs to be added                                            |
| Title text           | Left side, `text-sm`, centered within the 32px                                                                                                            | Title bar                                    | Currently a breadcrumb (mac style) → on Windows change to "App name — Page name" |
| Title bar buttons    | Right side, width **46px**, height 32px, the close button uses `#C42B1C` on hover (system semantic red)                                                   | Fluent caption button size                   | Currently `w-11`(44px) × 40px → needs to change to 46×32                         |
| Drag region          | The entire title bar is draggable except for the buttons; **double-clicking an empty area of the title bar maximizes/restores**                           | System behavior                              | Pending                                                                          |
| Window corner radius | Handled by the system (Win11 8px); **no corner rounding when the window is snapped** — do not draw the window corners yourself                            | Geometry                                     | No change needed                                                                 |

## 2. Materials (Mica / Acrylic)

| Region                           | Material                                                                                                                                                 | Tauri `effects`                                     | Fallback (must be implemented) |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ------------------------------ |
| Window base + title bar          | **Mica Alt** (officially recommended: for apps with navigation and command areas, Mica Alt provides contrast between the title bar and the command area) | `tabbed`                                            | `bg-background` solid          |
| Simple window without navigation | Mica                                                                                                                                                     | `mica`                                              | `bg-background` solid          |
| Overlays (menu/dropdown/Flyout)  | Acrylic                                                                                                                                                  | `acrylic` (or CSS frosted glass inside the overlay) | `bg-popover` solid             |

**Official degradation conditions (must be covered item by item)**: the user turns off "Transparency effects", battery saver mode, low-end hardware, the window loses focus, older versions of Windows
→ always fall back to solid. Implementation: `prefers-reduced-transparency` on the CSS side; on the Rust side, do not request the material when detection fails;
the unfocused state cannot use Tauri `state` (Windows only supports macOS), so switch to a solid via the CSS `:window-inactive` equivalent class or a JS window-focus listener.

- On Windows the sidebar belongs to the **content layer** (Fluent layering: the base layer holds menus/commands/navigation), so the sidebar **does not** use Acrylic;
  it uses a solid `bg-sidebar` + the Mica Alt window base; do not enable a material separately for the sidebar.
- Same as macOS: materials are used only in these regions; blur is forbidden in scrolling content areas/tables/forms; at most 2 materials.

## 3. Geometry and Spacing (overriding shared values)

Fluent geometry: **8px for top-level containers / 4px for in-page elements / 0px where straight lines meet**. Therefore, on Windows the shared corner radii are overridden:

```css
[data-platform='win'] {
  --radius-md: 4px; /* in-page controls: buttons, inputs, dropdowns, segments */
  --radius-lg: 8px; /* containers: panels, cards, dialogs */
  --radius-xl: 8px; /* overlays: Fluent top-level containers are also 8px */
}
```

Spacing (a direct mapping of the Fluent《Content layout and spacing》text, consistent with the shared scale; Windows uses the same values):

| Relationship                        | Value |
| ----------------------------------- | ----- |
| Between buttons                     | 8     |
| Between a button and a dropdown     | 8     |
| Between a control and a group title | 8     |
| Between a control and a label       | 12    |
| Between content blocks              | 12    |
| Between a surface and its content   | 16    |

## 4. Fonts (Fluent type ramp mapping)

| Fluent style    | Size/line height     | This project                                                                       |
| --------------- | -------------------- | ---------------------------------------------------------------------------------- |
| Display (large) | 68/92 · 40/52        | Not used (this app has no Hero)                                                    |
| Title           | 28/36 · 20/28        | Page title upper bound (`text-base` or `text-lg` in the large tier, use sparingly) |
| Subtitle        | 20/28 (semibold)     | Settings page section title                                                        |
| Body Strong     | 14/20 (semibold 600) | Panel title, emphasized row title                                                  |
| Body            | 14/20                | Body text, list primary text (`text-sm` @ the normal tier)                         |
| Caption         | 12/16                | Supplementary description, labels (`text-xs`)                                      |

- Font family: `Segoe UI Variable` (the Windows system font); available weights are 300/350/400/600/700, but this project still uses only 400/500/600 (shared §2.3).
- **Readability lower bound**: body text is a minimum of 14 Regular / 12 Semibold (Fluent text); anything below this is forbidden → this project's small tier must be ≥12.
- Casing: sentence case; do not use all-caps titles.

## 5. Motion (Fluent standard durations and curves)

| Semantics                                     | Fluent named value                           | This project's `win` override |
| --------------------------------------------- | -------------------------------------------- | ----------------------------- |
| Control normal (expand/toggle/color)          | `ControlNormalAnimationDuration` **250ms**   | `--dur-slow: 250ms`           |
| Control fast (hover/press/small displacement) | `ControlFastAnimationDuration` **167ms**     | `--dur-base: 167ms`           |
| Very fast (instant feedback, micro states)    | `ControlFasterAnimationDuration` **83ms**    | `--dur-fast: 83ms`            |
| Enter curve                                   | Fast Out, Slow In `cubic-bezier(0, 0, 0, 1)` | Same value                    |
| Exit curve                                    | Fast In, Slow Out `cubic-bezier(1, 0, 1, 1)` | Same value                    |

All other rules are the same as shared §2.7 (animate only transform/opacity; cross-fade for reduced-motion).

## 6. Elevation (shadows and strokes)

Fluent《Layering》expresses hierarchy with an "elevation value + 1px stroke":

| Layer                   | Elevation | Stroke |
| ----------------------- | --------- | ------ |
| Window                  | 128       | 1px    |
| Dialog                  | 128       | 1px    |
| Flyout / menu / Popover | 32        | 1px    |

Implementation: on Windows, overlay shadows use `--shadow-flyout` (the soft shadow corresponding to elevation 32) + `border` (1px stroke);
dialogs use `--shadow-dialog` (elevation 128) + 1px stroke. **Shadows and strokes are used in pairs**; a shadow without a stroke is not allowed.

## 7. Focus and Keyboard

- Focus visual: a **2px double stroke** (dark outer / light inner, or the reverse, ensuring visibility on any background color), with a shape that **follows the control's corner radius**;
  unlike macOS's accent halo, Windows does not use a colored glow.
- Keyboard: `Ctrl` replaces `⌘`; `Alt` focuses the command area within the window (command bar/menu); `Tab` order is consistent; `Esc` closes overlays and dialogs.
- Shortcut mapping table (a one-to-one correspondence with macOS):

| Action               | macOS        | Windows             |
| -------------------- | ------------ | ------------------- |
| Settings             | `⌘,`         | `Ctrl+,`            |
| Command palette      | `⌘K`         | `Ctrl+K`            |
| In-page search       | `⌘F`         | `Ctrl+F`            |
| Close window         | `⌘W`         | `Ctrl+W`            |
| Quit                 | `⌘Q`         | `Alt+F4`            |
| Undo/redo            | `⌘Z` / `⇧⌘Z` | `Ctrl+Z` / `Ctrl+Y` |
| Switch sidebar group | `⌘1..9`      | `Ctrl+1..9`         |
| Full screen          | `⌃⌘F`        | `F11`               |

When shortcuts are shown in the interface, display them per platform (`Ctrl+K` / `⌘K`), provided by `core/platform.ts`.

## 8. Commands and Menus (Windows has no menu bar)

Current state: on Windows, `set_decorations(false)` makes the native menu bar invisible (`src-tauri/src/lib.rs`),
so **the HIG rule "toolbar commands must have a corresponding item in the menu bar" has no counterpart on Windows**; instead:

- **In-window command area (CommandBar)**: place the primary commands on the right side of the title bar or the left side of the toolbar + a **"More" overflow menu**
  (Fluent Commanding: low-frequency commands are tucked into overflow);
- **Settings entry**: place it as the last item of the "More" overflow menu (recommended by the official《Guidelines for app settings》);
  if discoverability matters, it can be placed directly on the toolbar;
- **Right-click context menu**: operations targeting the selected object (table rows, list items);
- Every command must have a keyboard path (`Ctrl+*` or `Alt+*`); mouse-only reachability is not allowed.

## 9. Dialogs and Feedback

- **Dialog type**: Windows uses ContentDialog (an in-window modal), and has **no concept of a sheet**; do not imitate the attached sheet of macOS.
- **Button order**: the primary action is on the **left**, `Cancel`/`Close` on the **right** (the opposite of macOS);
  the primary button of a destructive operation uses the system safe color/destructive color and states the consequence in the copy.
- Irreversible operations with serious consequences → a confirmation dialog (Fluent Commanding text); undoable operations execute directly + provide an undo entry.
- Info bars (InfoBar semantics): use a warning bar for recoverable problems (`info`/`warning`/`error` three tiers, including a close button and action buttons);
  do not use a toast to carry explanations that need to be read; use a toast for system-level failures.
- Success feedback is the same as shared §4 (no toast; use a state change).

## 10. Settings Page

- Keep it simple and centered on binary (on/off) controls (official《Guidelines for app settings》text, "make use of binary controls");
  put complex parameters on the page where the feature lives; do not pile them into the settings page.
- Settings take effect **immediately**; there is no "Apply/OK" button (consistent with macOS and with this project's current state).
- Section titles use Subtitle semantics (`text-sm`/600); groups use a card + `divide-y` (same as shared §2.5; do not stack boxes).

## 11. Scope and To-dos for This Round

- This round is **consistency only**; no new features are introduced.
- To-dos (confirm item by item during the first Windows verification and fill in):
  1. Title bar 32px + 46×32 buttons + window icon and menu behavior;
  2. Mica Alt / Acrylic taking effect and all degradation paths;
  3. Double-stroke focus visual;
  4. Command area (CommandBar) and overflow menu, settings entry placement;
  5. Dialog button order (primary action on the left);
  6. `Ctrl` key display and `Alt` command-area focus;
  7. 4px/8px corner radius overrides taking effect;
  8. Layout does not break at higher/lower font-size tiers.
- After any of the above passes verification, this document's "Verification status" must be updated and the item removed from the risk register in `AGENTS.md`.
