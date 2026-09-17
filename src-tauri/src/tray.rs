//! 系统托盘。
//!
//! - 左键点击：唤起主窗口（macOS 右键弹菜单）
//! - 菜单：显示主窗口 / 退出

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Runtime,
};

const MENU_SHOW: &str = "show";
const MENU_QUIT: &str = "quit";

/// macOS：显式激活应用，确保窗口真正来到最前。
///
/// 点击菜单栏（托盘）图标本身**不会**激活所属应用（AppKit 现状），而
/// `show()` / `set_focus()` 只做 `makeKeyAndOrderFront`，窗口会被排到次层：
/// 看起来「点了没反应」，切换应用后才看到窗口其实已显示。这是上游已知问题
/// （tauri-apps/tauri#14795，tray-icon 0.25.0 仍未修复），故此处自行激活应用。
#[cfg(target_os = "macos")]
fn activate_app() {
    if let Some(mtm) = objc2::MainThreadMarker::new() {
        #[allow(deprecated)] // activate() 需 macOS 14+，用旧 API 兼容更早系统
        {
            use objc2_app_kit::NSApplication;
            NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
        }
    }
}

/// 立即唤起主窗口（取消最小化 → 显示 → 聚焦 → 激活应用）
///
/// 启动门禁（`before_start` 返回 Hold）挂起期间拒绝显示：托盘 / 单实例 /
/// Dock reopen 三条路径都经过此处，统一尊重门禁，否则 fork 的启动拦截会被旁路。
fn show_main_window_now<R: Runtime>(app: &tauri::AppHandle<R>) {
    if crate::lifecycle::is_startup_held() {
        log::debug!("启动门禁挂起中，忽略唤起主窗口请求");
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    #[cfg(target_os = "macos")]
    activate_app();
}

/// 唤起主窗口。
///
/// macOS 上托盘鼠标事件的跟踪会覆盖同一次事件内的激活/排序，故立即尝试一次后，
/// 再延迟一拍（下个 runloop）重试，保证窗口稳定回到最前。
pub fn show_main_window<R: Runtime>(app: &tauri::AppHandle<R>) {
    show_main_window_now(app);

    #[cfg(target_os = "macos")]
    {
        let handle = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(80));
            let inner = handle.clone();
            let _ = handle.run_on_main_thread(move || show_main_window_now(&inner));
        });
    }
}

/// 加载内置托盘模板图（PNG → RGBA），失败返回 None（回退应用图标）
fn load_tray_icon() -> Option<tauri::image::Image<'static>> {
    let bytes = include_bytes!("../icons/tray-icon.png");
    let rgba = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(tauri::image::Image::new_owned(
        rgba.into_raw(),
        width,
        height,
    ))
}

/// 在 setup 中调用：构建托盘图标与菜单
pub fn create_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id(MENU_SHOW, "显示主窗口").build(app)?;
    let quit = MenuItemBuilder::with_id(MENU_QUIT, "退出").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

    // 托盘图标：优先专用模板图（单色、透明底）；缺失时回退应用图标。
    // macOS 图标颜色不是重点（模板图按 alpha 取形并随菜单栏明暗自适应）
    let icon = load_tray_icon().unwrap_or_else(|| {
        app.default_window_icon()
            .cloned()
            .expect("应用图标缺失，无法创建托盘")
    });

    // 应用显示名取自 tauri.conf 的 productName（改名后托盘提示自动跟随）
    let tooltip = app
        .config()
        .product_name
        .clone()
        .unwrap_or_else(|| "PocketArk".into());

    #[allow(unused_mut)]
    let mut builder = TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip(&tooltip)
        .menu(&menu)
        // 左键直接唤起窗口，菜单走右键
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => show_main_window(app),
            MENU_QUIT => {
                log::info!("用户从托盘退出");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    // macOS：模板图（单色，随菜单栏明暗自动反色，与系统图标观感一致）
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    builder.build(app)?;

    log::debug!("托盘已创建");
    Ok(())
}
