//! 原生应用菜单（macOS menubar；Windows/Linux 为窗口菜单）。
//!
//! 菜单项统一经 `app://menu` 事件转发给前端处理（导航 / 命令面板等），
//! 使菜单与前端路由解耦。

use serde::Serialize;
use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Runtime};

use crate::events;

#[derive(Clone, Serialize)]
struct MenuPayload {
    id: String,
}

/// 构建并安装应用菜单（在 setup 中调用）
pub fn create_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let app_name = app
        .config()
        .product_name
        .clone()
        .unwrap_or_else(|| "PocketArk".into());

    let home = MenuItemBuilder::with_id("home", "首页")
        .accelerator("CmdOrCtrl+1")
        .build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "设置…")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let palette = MenuItemBuilder::with_id("command-palette", "命令面板")
        .accelerator("CmdOrCtrl+K")
        .build(app)?;

    let app_menu = SubmenuBuilder::new(app, app_name.as_str())
        .about(Some(AboutMetadata::default()))
        .separator()
        .item(&settings)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "编辑")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "视图")
        .item(&home)
        .item(&palette)
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "窗口")
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;

    let menu = MenuBuilder::new(app)
        .items(&[&app_menu, &edit_menu, &view_menu, &window_menu])
        .build()?;

    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        let id = event.id().as_ref().to_string();
        // 仅主窗口处理菜单动作（避免广播到次级窗口）
        let _ = app.emit_to("main", events::APP_MENU, MenuPayload { id });
    });

    log::debug!("应用菜单已创建");
    Ok(())
}
