//! ArkDesk 应用入口：框架级组装。
//!
//! 插件注册顺序：log（最早，保证后续日志可见）→ store → sql（含迁移）→ opener → dialog
//! 日志级别：插件以 Trace 全量注册，实际级别由 settings store 读取后经
//! `log::set_max_level` 运行时控制（前端 set_log_level 命令可动态调整）。

mod db;
mod diagnostics;
mod error;
// 事件名常量：当前仅 updater 使用，其余常量供后续阶段（单实例/任务/HTTP 下载）使用
#[allow(dead_code)]
mod events;
mod files;
mod http;
mod menu;
mod open;
mod plugins;
mod tasks;
mod tray;
mod updater;

use log::LevelFilter;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};
use tauri_plugin_store::StoreExt;

/// 应用退出流程标记：RunEvent::ExitRequested 置位后，窗口关闭不再走
/// 「隐藏到托盘」拦截，否则退出流程中窗口只被隐藏、进程无法结束
static EXITING: AtomicBool = AtomicBool::new(false);

/// 解析日志级别字符串（与前端 LogLevel 对齐）
fn parse_level(level: &str) -> Option<LevelFilter> {
    match level {
        "trace" => Some(LevelFilter::Trace),
        "debug" => Some(LevelFilter::Debug),
        "info" => Some(LevelFilter::Info),
        "warn" => Some(LevelFilter::Warn),
        "error" => Some(LevelFilter::Error),
        _ => None,
    }
}

/// 运行时调整日志级别（持久化由前端 settings store 负责）
#[tauri::command]
fn set_log_level(level: String) -> Result<(), error::AppError> {
    let filter = parse_level(&level)
        .ok_or_else(|| error::AppError::invalid_input(format!("非法日志级别: {level}")))?;
    log::set_max_level(filter);
    log::info!("日志级别已调整为 {filter}");
    Ok(())
}

/// 重启应用（导入设置 / 恢复数据库等场景）
#[tauri::command]
fn app_restart(app: tauri::AppHandle) -> Result<(), error::AppError> {
    app.restart()
}

/// 同步窗口外观到应用主题（暗色指挥台）。
/// resize / 最大化时新暴露的区域由 WebKit 在 webview 内部绘制「视图底色」，
/// 颜色跟随视图 effectiveAppearance（系统浅色 → 白、系统深色 → 黑），
/// 且 NSWindow 背景色与 underPageBackgroundColor 均无法覆盖该层（上游已知限制，
/// 见 tauri#13898 / #14288 / #1564）。将 NSWindow.appearance 同步为主题外观，
/// 使色带与应用亮暗底色一致到肉眼不可辨。
///
/// `dark = None` 表示「跟随系统」：清空外观覆盖（继承系统）——
/// 强制 darkAqua/aqua 会把 webview 的 prefers-color-scheme 钉死在对应外观上，
/// 导致跟随系统模式失效，故必须三态处理。返回设置后的实际暗色状态。
/// 非 macOS 为空操作（返回 false）。
#[tauri::command]
fn set_window_appearance(
    window: tauri::WebviewWindow,
    dark: Option<bool>,
) -> Result<bool, error::AppError> {
    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{
            NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua,
            NSAppearanceNameDarkAqua, NSApplication, NSWindow,
        };

        let appearance = match dark {
            Some(true) => Some(
                NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }).ok_or_else(
                    || error::AppError::custom(error::code::UNKNOWN, "系统外观不可用"),
                )?,
            ),
            Some(false) => Some(
                NSAppearance::appearanceNamed(unsafe { NSAppearanceNameAqua }).ok_or_else(
                    || error::AppError::custom(error::code::UNKNOWN, "系统外观不可用"),
                )?,
            ),
            // 跟随系统：nil = 继承系统外观
            None => None,
        };

        let ns_window = window.ns_window().map_err(|err| {
            error::AppError::custom(error::code::IPC_ERROR, format!("获取 NSWindow 失败: {err}"))
        })? as *mut NSWindow;
        if !ns_window.is_null() {
            // 借用指针设置外观，不转移所有权（窗口由 tao 持有）
            unsafe { (*ns_window).setAppearance(appearance.as_deref()) };
        }

        // 返回设置后的实际暗色状态：强制模式 = 指定值；跟随系统 = 应用外观
        let effective_dark = match dark {
            Some(v) => v,
            None => {
                let mtm = MainThreadMarker::new()
                    .ok_or_else(|| error::AppError::custom(error::code::UNKNOWN, "非主线程"))?;
                let eff_name = NSApplication::sharedApplication(mtm)
                    .effectiveAppearance()
                    .name();
                eff_name.isEqualToString(unsafe { NSAppearanceNameDarkAqua })
            }
        };
        log::debug!("窗口外观已同步: dark={effective_dark}");
        Ok(effective_dark)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        Ok(dark.unwrap_or(false))
    }
}

/// NSWindow 背景色（与前端 core/theme 的 NATIVE_BG 对齐，改动需同步）。
/// 仅 macOS 使用，非 macOS 编译时整段剔除避免 dead_code 警告。
#[cfg(target_os = "macos")]
const WINDOW_BG_DARK: (f64, f64, f64) = (22.0 / 255.0, 22.0 / 255.0, 28.0 / 255.0);
#[cfg(target_os = "macos")]
const WINDOW_BG_LIGHT: (f64, f64, f64) = (247.0 / 255.0, 247.0 / 255.0, 249.0 / 255.0);

/// 设置 NSWindow 背景色（macOS）。
/// WKWebView 经 wry transparent 路径设为 drawsBackground=false 后自身透明，
/// resize / 最大化新暴露的区域透出的是 NSWindow 背景色——不跟随主题就会闪白。
/// wry 无窗口级背景色 API，故直接经 objc2-app-kit 设置。
#[cfg(target_os = "macos")]
fn apply_window_background(ns_window: *mut std::ffi::c_void, dark: bool) {
    use objc2_app_kit::{NSColor, NSWindow};

    let (r, g, b) = if dark {
        WINDOW_BG_DARK
    } else {
        WINDOW_BG_LIGHT
    };
    let ns_window = ns_window as *mut NSWindow;
    if ns_window.is_null() {
        return;
    }
    let color = NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0);
    unsafe { (*ns_window).setBackgroundColor(Some(&color)) };
}

/// 前端主题切换时同步 NSWindow 背景色（经 ipc 调用）
#[tauri::command]
fn set_window_background(window: tauri::WebviewWindow, dark: bool) -> Result<(), error::AppError> {
    #[cfg(target_os = "macos")]
    {
        let ns_window = window.ns_window().map_err(|err| {
            error::AppError::custom(error::code::IPC_ERROR, format!("获取 NSWindow 失败: {err}"))
        })?;
        apply_window_background(ns_window, dark);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, dark);
    Ok(())
}

/// 安装 panic 钩子：崩溃信息写入日志，避免静默退出
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("PANIC: {info}");
        default_hook(info);
    }));
}

/// 将主窗口居中到鼠标所在显示器（多显示器场景下跟随当前屏幕）
fn center_window_on_cursor_monitor(app: &tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let Ok(cursor) = app.cursor_position() else {
        return;
    };

    let Ok(Some(monitor)) = window.monitor_from_point(cursor.x, cursor.y) else {
        return;
    };

    let Ok(size) = window.outer_size() else {
        return;
    };

    let x = monitor.position().x + ((monitor.size().width as i32 - size.width as i32) / 2).max(0);
    let y = monitor.position().y + ((monitor.size().height as i32 - size.height as i32) / 2).max(0);
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

/// macOS 缩放同步优化：WKWebView 内容渲染默认滞后于窗口 frame 变化，
/// 表现为放大时「窗口先变大、内容再跟上」（缩小因内容被裁剪而不明显）。
/// 通过 layerContentsRedrawPolicy = duringViewResize 强制 AppKit 在缩放
/// 过程中同步重绘 layer，消除内容与窗口的错帧。
/// （旧属性 layerContentsRedrawDuringViewResize 已在新 macOS 中移除，此为新 API）
#[cfg(target_os = "macos")]
fn optimize_resize_sync(window: &tauri::WebviewWindow) {
    use objc2::rc::Retained;
    use objc2_app_kit::{NSView, NSViewLayerContentsRedrawPolicy, NSWindow};

    let result = window.with_webview(move |webview| unsafe {
        let set_sync = |view: &NSView| {
            view.setLayerContentsRedrawPolicy(NSViewLayerContentsRedrawPolicy::DuringViewResize);
        };

        // WKWebView 自身与其所在窗口的 contentView 同时设置，确保覆盖生效
        let webview_view = webview.inner() as *mut NSView;
        if let Some(view) = webview_view.as_ref() {
            set_sync(view);
        }

        let ns_window = Retained::from_raw(webview.ns_window() as *mut NSWindow);
        if let Some(window) = ns_window.as_ref() {
            if let Some(content_view) = window.contentView() {
                set_sync(&content_view);
            }
        }
    });

    if let Err(err) = result {
        log::warn!("resize 同步优化失败: {err}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_panic_hook();
    // 启动参数中的文件/URL 先入队（webview 就绪后由前端 take_pending_open 取走）
    open::stash_pending(open::normalize_args(std::env::args().skip(1)));

    let builder = tauri::Builder::default()
        // 单实例必须最先注册：第二个实例启动时唤起主窗口并转发参数
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            log::info!("检测到第二个实例: {argv:?}");
            tray::show_main_window(app);
            // argv[0] 为可执行文件路径，跳过；其余文件/URL 交给统一打开入口
            open::emit_open(app, open::normalize_args(argv.into_iter().skip(1)), "cli");
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(LevelFilter::Trace)
                .rotation_strategy(RotationStrategy::KeepSome(5))
                .max_file_size(5_000_000)
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build());

    let app = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // 编辑区右键菜单的自定义复制/粘贴（见 EditorContextMenu）
        .plugin(tauri_plugin_clipboard_manager::init())
        // 系统通知与开机自启（设置项驱动）
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        // 仅记忆窗口大小/最大化状态：位置每次启动居中（见 setup）
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        & !tauri_plugin_window_state::StateFlags::POSITION,
                )
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Windows/Linux：移除系统标题栏（macOS 用 titleBarStyle Overlay，不走此分支）。
            // 窗口先隐藏创建（visible: false），装饰设置完成后在 setup 末尾统一显示，
            // 避免启动瞬间闪现原生标题栏。
            #[cfg(not(target_os = "macos"))]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(false);
            }

            // NSWindow 背景打底：默认暗色基调（resize / 最大化新暴露区域显示该色，
            // 防止 WKWebView 绘制滞后期间透出系统默认浅色；JS 侧按存储主题校正）
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(ns_window) = window.ns_window() {
                    apply_window_background(ns_window, true);
                }
            }

            // 数据库：固定路径 + 执行迁移（兼容历史 _sqlx_migrations 记录）
            db::init(app.handle());
            tauri::async_runtime::block_on(async {
                db::migrate().await.map_err(|err| format!("{err}"))?;
                log::info!("数据库迁移就绪");
                Ok::<(), String>(())
            })?;

            // 窗口居中：优先鼠标所在屏幕（多显示器场景），失败时保底 tauri.conf 的 center
            center_window_on_cursor_monitor(app);

            // macOS：webview 缩放同步重绘（消除放大时内容滞后错帧）
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                optimize_resize_sync(&window);
            }

            // 从设置读取日志级别（前端 watch 同步持久化），运行时即时生效
            if let Ok(store) = app.store("settings.json") {
                if let Some(level) = store
                    .get("logLevel")
                    .and_then(|value| value.as_str().map(String::from))
                    .and_then(|ref s| parse_level(s))
                {
                    log::set_max_level(level);
                }
            }

            tray::create_tray(app.handle())?;
            menu::create_menu(app.handle())?;

            // 深链接：打开 arkdesk://… 时转发给前端（经 app://open 统一入口）
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    let urls: Vec<String> =
                        event.urls().iter().map(|url| url.to_string()).collect();
                    open::emit_open(&handle, urls, "deep-link");
                });
            }

            // 初始化全部完成后显示窗口（创建时 visible: false，避免启动闪帧/闪标题栏）
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }

            log::info!("ArkDesk 启动完成");
            Ok(())
        })
        // 关窗行为：默认隐藏到托盘（可在设置中关闭，改为直接退出）。
        // 应用退出流程（Cmd+Q / Dock 退出 / 托盘退出）中必须放行关闭——否则窗口
        // 只被隐藏、退出流程卡死，进程无法结束（只能 kill）
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if EXITING.load(Ordering::Relaxed) {
                    return;
                }

                // 仅主窗口走「隐藏到托盘」；次级窗口（win-*）直接关闭销毁
                if window.label() != "main" {
                    return;
                }

                let close_to_tray = window
                    .app_handle()
                    .get_store("settings.json")
                    .and_then(|store| store.get("closeToTray"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true);

                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        // 命令注册：框架命令 + 各插件 backend 命令（构建期自动聚合，见 build.rs）
        .invoke_handler(crate::plugins::handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // 独立处理运行事件：退出流程开始时置位标记，放行后续的窗口关闭
    app.run(|_app, event| match event {
        tauri::RunEvent::ExitRequested { .. } => {
            EXITING.store(true, Ordering::Relaxed);
            log::info!("应用退出流程开始");
        }
        tauri::RunEvent::Exit => log::info!("应用退出完成"),
        _ => {}
    });
}
