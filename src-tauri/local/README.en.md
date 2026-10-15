# Local-layer Lifecycle Hooks (fork-specific directory)

base only defines the contract and the call points, and **ships no implementation**. If a fork needs to intercept the app startup/exit phases
(registration validation, licensing, data migration, cleanup reporting, …), placing a `lifecycle.rs` in this directory is enough for it to be
auto-discovered (`src-tauri/build.rs`) and called at build time, **with no registration needed**; if it is absent, everything is a no-op.

## Conventions

- File: `src-tauri/local/lifecycle.rs`
- It must export `pub struct Hooks;` and implement `crate::lifecycle::AppLifecycle`
- Four phases (each method has a default implementation, so just write the ones you need):

| Method         | Timing                                     | Return value / capability                                   |
| -------------- | ------------------------------------------ | ----------------------------------------------------------- |
| `before_start` | **Before** the main window is shown, async | `Continue` / `Hold` (do not show, wait for resume) / `Quit` |
| `after_start`  | After the main window is shown             | None (notification only)                                    |
| `before_exit`  | Before the exit flow begins, async         | `Continue` / `Cancel` (cancel exit)                         |
| `after_exit`   | The process is about to end                | None (**must be synchronous and fast**)                     |

- Failure fallback (returning Err / panic / timeout) is declared by the implementation via `on_before_start_fail` /
  `on_before_exit_fail`; the defaults are: startup `Halt` (do not show the main window), exit `Skip` (let it through).
- After `before_start` returns `Hold`, the process keeps running but does not show the main window: the fork shows its own UI,
  and once it finishes processing calls `crate::lifecycle::resume_startup(app)` to continue startup; during this time the user can quit from the tray.
- Hooks run in a separate async task; **do not block the main thread inside a hook** (an internal `block_on` will make startup appear to hang).

Example:

```rust
use tauri::AppHandle;

use crate::error::AppError;
use crate::lifecycle::{AppLifecycle, HookFuture, StartupFlow};

pub struct Hooks;

impl AppLifecycle for Hooks {
    fn before_start(&self, app: &AppHandle) -> HookFuture<'_, Result<StartupFlow, AppError>> {
        let app = app.clone();
        Box::pin(async move {
            if licensed(&app).await? {
                Ok(StartupFlow::Continue)
            } else {
                open_my_activation_window(&app)?;
                Ok(StartupFlow::Hold)
            }
        })
    }
}
```

This directory, together with `src-tauri/local-resources/`, belongs to the fork local layer (see `docs/en/local.md`,
`docs/ownership.json`), and is not backported with the upstream base.
