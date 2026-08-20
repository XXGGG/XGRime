//! 系统托盘。
//!
//! 菜单没用系统原生的那套 —— 原生菜单的字号、圆角、配色全归系统管，改不了，
//! 跟应用里的样式对不上。这里改成点托盘时把一个无边框小窗口挪到鼠标旁边显示，
//! 内容是我们自己的那套组件（前端 `#tray` 那个路由），失焦就收起来。

use std::sync::Mutex;

use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

/// 上次点托盘时鼠标在哪
///
/// 菜单要贴着这个点摆。存下来是因为窗口高度是前端量完内容之后才改的 ——
/// 改完得按新高度重新贴一次，那时候鼠标可能已经挪开了。
static ANCHOR: Mutex<Option<(f64, f64)>> = Mutex::new(None);

const MENU: &str = "tray-menu";
/// 菜单窗口四周留给阴影的透明边，逻辑像素。跟前端那层 `p-3` 是同一个数，改一边就得改另一边。
const MENU_PADDING: f64 = 12.0;

pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    // 关窗口只收进托盘，不退出 —— 开机自启的应用被点一下 X 就没了，
    // 下次开机才回来，那自启就白设了。真要退出走托盘菜单里那一项。
    if let Some(main) = app.get_webview_window("main") {
        let handle = app.clone();
        main.on_window_event(move |e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                api.prevent_close();
                if let Some(w) = handle.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
        });
    }

    if let Some(menu) = app.get_webview_window(MENU) {
        let handle = app.clone();
        // 点到别处就收起来。不这么做的话它会一直浮在最上面盖住别的窗口。
        menu.on_window_event(move |e| {
            if let WindowEvent::Focused(false) = e {
                if let Some(w) = handle.get_webview_window(MENU) {
                    let _ = w.hide();
                }
            }
        });
    }

    TrayIconBuilder::with_id("xgrime")
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            tauri::Error::AssetNotFound("默认窗口图标不在".into())
        })?)
        // 模板图必须是纯单色的，拿彩色图标去标 template，macOS 会把它压成一坨黑块。
        // 等有了专门的单色菜单栏图标再开这个开关。
        .icon_as_template(false)
        .tooltip("XGRime")
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            match event {
                // 左键单击：弹自己那套菜单
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => show_menu(app),
                // 右键也弹同一个。原生菜单没接，就别让右键落空。
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Up,
                    ..
                } => show_menu(app),
                TrayIconEvent::DoubleClick { .. } => show_main(app),
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

/// 把菜单窗口挪到鼠标旁边再显示
///
/// 不用系统给的托盘矩形定位：多显示器、任务栏在左右两侧、DPI 不一样的时候，
/// 那个矩形经常不是你以为的位置。跟着鼠标走最稳，反正用户刚点完托盘。
fn show_menu(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window(MENU) else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
        return;
    }
    if let Ok(cursor) = app.cursor_position() {
        *ANCHOR.lock().unwrap() = Some((cursor.x, cursor.y));
    }
    place(app);
    let _ = win.show();
    let _ = win.set_focus();
}

/// 把菜单窗口的右下角贴到锚点左上方
fn place(app: &tauri::AppHandle) {
    let (Some(win), Some((ax, ay))) = (
        app.get_webview_window(MENU),
        *ANCHOR.lock().unwrap(),
    ) else {
        return;
    };
    let Ok(size) = win.outer_size() else { return };
    let scale = win.scale_factor().unwrap_or(1.0);
    // 窗口比看得见的卡片大一圈：那圈透明留白是给阴影的（前端的 p-3）。
    // 按窗口边缘定位的话，卡片实际离鼠标要再远这么多，看着就是「飘着」。
    // 所以先把留白加回去，再减掉真正想留的那点距离。
    let pad = (MENU_PADDING * scale) as i32;
    let gap = (4.0 * scale) as i32;
    // 往左上角摆：托盘在右下角，直接放鼠标右下会跑出屏幕
    let x = (ax as i32 - size.width as i32 + pad - gap).max(0);
    let y = (ay as i32 - size.height as i32 + pad - gap).max(0);
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
}

/// 前端按内容改完高度之后叫一次，按新高度重新贴
///
/// 不重贴的话，窗口是左上角定位的：高度一变，下边缘就跟着跑，
/// 卡片要么离鼠标老远、要么压到鼠标上。
#[tauri::command]
pub fn anchor_tray_menu(app: tauri::AppHandle) {
    place(&app);
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// 从托盘菜单里点「打开」用的
#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(MENU) {
        let _ = w.hide();
    }
    show_main(&app);
}

#[tauri::command]
pub fn hide_tray_menu(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(MENU) {
        let _ = w.hide();
    }
}

/// 真退出。托盘菜单里那一项走这里 —— 别的地方关窗口都只是收起来。
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
