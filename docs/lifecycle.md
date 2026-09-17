# 生命周期钩子（框架级 SPI）

base **只定义契约与调用点，不含任何实现**：fork 若需要在应用启动 / 退出环节拦截
（注册校验、授权、数据迁移、清理上报……），在 `src-tauri/local/lifecycle.rs` 实现
`AppLifecycle` 即可被构建期自动发现并调用，**无需任何登记**；不提供实现时全为 no-op，
行为与未引入该机制时完全一致。联网校验、弹注册窗口、读写数据库等都属 fork 自己的实现内容，
base 不参与、不关心。

契约代码：[`src-tauri/src/lifecycle.rs`](../src-tauri/src/lifecycle.rs)（framework）。
实现位约定：[`src-tauri/local/README.md`](../src-tauri/local/README.md)（base 提供说明，目录下实现属 fork）。

## 1. 四个环节

| 方法           | 时机                               | 返回值                       | 备注                                |
| -------------- | ---------------------------------- | ---------------------------- | ----------------------------------- |
| `before_start` | 主窗口显示**之前**（异步）         | `Continue` / `Hold` / `Quit` | 唯一能拦住主窗口显示的时机          |
| `after_start`  | 主窗口显示之后                     | 无                           | 非阻塞初始化；panic 不影响运行      |
| `before_exit`  | 退出流程开始、进程结束之前（异步） | `Continue` / `Cancel`        | `Cancel` 中止本次退出               |
| `after_exit`   | 进程即将结束                       | 无（**必须同步且快速**）     | 独立线程执行 + 限时等待，超时即放行 |

调用点（framework，改动随上游回填）：

- `before_start` / `after_start`：`src-tauri/src/lib.rs` 的 `setup` 末尾 —— 初始化
  （数据库迁移、托盘、菜单、深链接）全部就绪后，由独立异步任务执行；`Continue` 才显示主窗口。
- `before_exit` / `after_exit`：`app.run(...)` 的 `RunEvent::ExitRequested` / `RunEvent::Exit`。
- 门禁收口：`src-tauri/src/tray.rs` 的 `show_main_window_now` —— 托盘、单实例、Dock reopen
  三条唤起路径都经过此处，`Hold` 期间一律拒绝显示，避免旁路。

## 2. 失败与超时

钩子运行在独立任务里，返回 `Err` / panic / 超时都不会带崩应用，按实现自己声明的策略兜底：

| 钩子           | 策略方法               | 默认值 | 含义                                 |
| -------------- | ---------------------- | ------ | ------------------------------------ |
| `before_start` | `on_before_start_fail` | `Halt` | 校验挂了就不显示主窗口（宁可不放行） |
| `before_exit`  | `on_before_exit_fail`  | `Skip` | 放行退出（绝不把用户卡在退不出去）   |

超时上限：`BEFORE_START_TIMEOUT`（10s）、`BEFORE_EXIT_TIMEOUT`（5s）、`AFTER_EXIT_TIMEOUT`（200ms）。

## 3. fork 接入

`src-tauri/local/lifecycle.rs`：

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

只写关心的方法即可（其余用默认实现）；`pub struct Hooks;` 名称是约定，勿改。

## 4. `Hold` 的语义与陷阱

- `Hold` = **不显示主窗口**，但进程继续运行；fork 负责自己的界面（如注册窗口），
  处理完成后调用 `crate::lifecycle::resume_startup(app)` 继续启动（显示主窗口 + `after_start`）。
- `Hold` 期间用户仍可从托盘退出（托盘/菜单在钩子之前就已创建）——这是刻意的兜底，
  避免"既进不去也退不出"。
- 门禁只影响"主窗口显示"这一条：fork 若自建窗口，请自行决定其显隐与关闭行为；
  自建窗口会被 `window-state` 插件记忆尺寸/位置，需要排除时在 fork 侧处理。
- 钩子**不得阻塞主线程**（在钩子里 `block_on` 会让启动假死：窗口连注册页都画不出来）。
  需要网络等待就直接 `await`，base 已给超时。

## 5. 归属

| 内容                                          | 归属      |
| --------------------------------------------- | --------- |
| `src-tauri/src/lifecycle.rs`（契约 + 调用点） | framework |
| `src-tauri/build.rs` 中的挂载生成             | framework |
| `src-tauri/local/README.md`（约定说明）       | framework |
| `src-tauri/local/lifecycle.rs`（fork 实现）   | fork      |

详情见 [`ownership.json`](ownership.json) 与 [`local.md`](local.md)。
