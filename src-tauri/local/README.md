# 本地层生命周期钩子（fork 专属目录）

base 只定义契约与调用点，**不预置任何实现**。fork 若需要在应用启动/退出环节拦截
（注册校验、授权、数据迁移、清理上报……），在此目录放 `lifecycle.rs` 即可被构建期
自动发现（`src-tauri/build.rs`）并调用，**无需任何登记**；不放则全为 no-op。

## 约定

- 文件：`src-tauri/local/lifecycle.rs`
- 必须导出 `pub struct Hooks;` 并实现 `crate::lifecycle::AppLifecycle`
- 四个环节（每个方法都有默认实现，只写需要的即可）：

| 方法           | 时机                     | 返回值 / 能力                                 |
| -------------- | ------------------------ | --------------------------------------------- |
| `before_start` | 主窗口显示**之前**，异步 | `Continue` / `Hold`（不显示，等恢复）/ `Quit` |
| `after_start`  | 主窗口显示之后           | 无（仅通知）                                  |
| `before_exit`  | 退出流程开始前，异步     | `Continue` / `Cancel`（取消退出）             |
| `after_exit`   | 进程即将结束             | 无（**必须同步且快速**）                      |

- 失败兜底（返回 Err / panic / 超时）由实现用 `on_before_start_fail` /
  `on_before_exit_fail` 声明，默认：启动 `Halt`（不显示主窗口）、退出 `Skip`（放行）。
- `before_start` 返回 `Hold` 后进程继续运行但不显示主窗口：fork 自行弹自己的界面，
  处理完成后调用 `crate::lifecycle::resume_startup(app)` 继续启动；期间用户可从托盘退出。
- 钩子运行在独立异步任务里，**不要在钩子内阻塞主线程**（内部 `block_on` 会让启动假死）。

示例：

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

该目录与 `src-tauri/local-resources/` 同属 fork 本地层（见 `docs/local.md`、
`docs/ownership.json`），不随上游 base 回填。
