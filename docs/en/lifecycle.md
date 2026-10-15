# Lifecycle Hooks (framework-level SPI)

The base **only defines the contract and the call sites, and contains no implementation**: if a fork needs to intercept application startup / shutdown
(registration validation, authorization, data migration, cleanup reporting, …), implementing
`AppLifecycle` in `src-tauri/local/lifecycle.rs` is enough for it to be discovered automatically at build time and called, **with no registration of any kind**; when no implementation is provided everything is a no-op,
and the behavior is exactly the same as without this mechanism. Online validation, showing a registration window, reading/writing the database, etc. are all part of the fork's own implementation content;
the base does not participate and does not care.

Contract code: [`src-tauri/src/lifecycle.rs`](../../src-tauri/src/lifecycle.rs) (framework).
Implementation-location convention: [`src-tauri/local/README.md`](../../src-tauri/local/README.md) (the base provides the explanation; the implementation in that directory belongs to the fork).

## 1. Four Phases

| Method         | Timing                                                         | Return value                            | Notes                                                                                         |
| -------------- | -------------------------------------------------------------- | --------------------------------------- | --------------------------------------------------------------------------------------------- |
| `before_start` | **Before** the main window is shown (async)                    | `Continue` / `Hold` / `Quit`            | The only timing that can block the main window from being shown                               |
| `after_start`  | After the main window is shown                                 | none                                    | Non-blocking initialization; a panic does not affect running                                  |
| `before_exit`  | When the shutdown flow begins, before the process ends (async) | `Continue` / `Cancel`                   | `Cancel` aborts this shutdown                                                                 |
| `after_exit`   | The process is about to end                                    | none (**must be synchronous and fast**) | Executed on a separate thread + waited on with a timeout; past the timeout it lets it through |

Call sites (framework; changes are back-ported with upstream):

- `before_start` / `after_start`: at the end of `setup` in `src-tauri/src/lib.rs` —— after initialization
  (database migration, tray, menu, deep links) is fully ready, executed by an independent async task; only `Continue` shows the main window.
- `before_exit` / `after_exit`: `RunEvent::ExitRequested` / `RunEvent::Exit` of `app.run(...)`.
- Gating convergence point: `show_main_window_now` in `src-tauri/src/tray.rs` —— all three summon paths, the tray, single instance, and Dock reopen,
  go through here, and while `Hold` is in effect display is always refused, avoiding a bypass.

## 2. Failure and Timeout

Hooks run inside an independent task, so returning `Err` / a panic / a timeout will not crash the application; each falls back according to the policy the implementation declares itself:

| Hook           | Policy method          | Default | Meaning                                                                         |
| -------------- | ---------------------- | ------- | ------------------------------------------------------------------------------- |
| `before_start` | `on_before_start_fail` | `Halt`  | If validation fails, do not show the main window (better not to let it through) |
| `before_exit`  | `on_before_exit_fail`  | `Skip`  | Let the shutdown proceed (never trap the user in a state they cannot exit)      |

Timeout limits: `BEFORE_START_TIMEOUT` (10s), `BEFORE_EXIT_TIMEOUT` (5s), `AFTER_EXIT_TIMEOUT` (200ms).

## 3. Fork Integration

`src-tauri/local/lifecycle.rs`:

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
                open_activation_window(&app)?;
                Ok(StartupFlow::Hold)
            }
        })
    }
}
```

Only write the methods you care about (the rest use the default implementations); the name `pub struct Hooks;` is a convention and must not be changed.

## 4. The Semantics and Pitfalls of `Hold`

- `Hold` = **do not show the main window**, but the process keeps running; the fork is responsible for its own UI (such as a registration window),
  and after it is done it calls `crate::lifecycle::resume_startup(app)` to continue startup (show the main window + `after_start`).
- While `Hold` is in effect the user can still quit from the tray (the tray/menu are already created before the hook) —— this is a deliberate fallback,
  to avoid a state where "you can neither get in nor get out".
- The gate only affects "showing the main window": if a fork creates its own windows, it decides their visibility and close behavior itself;
  self-created windows will have their size/position remembered by the `window-state` plugin, so handle the exclusion on the fork side if needed.
- A hook **must not block the main thread** (`block_on` inside a hook will make startup appear frozen: the window cannot even draw the registration page).
  If you need to wait on the network, just `await` directly; the base already provides a timeout.

## 5. Ownership

| Content                                                | Ownership |
| ------------------------------------------------------ | --------- |
| `src-tauri/src/lifecycle.rs` (contract + call sites)   | framework |
| The mount generation in `src-tauri/build.rs`           | framework |
| `src-tauri/local/README.md` (convention documentation) | framework |
| `src-tauri/local/lifecycle.rs` (fork implementation)   | fork      |

For details see [`ownership.json`](../ownership.json) and [`local.md`](local.md).
