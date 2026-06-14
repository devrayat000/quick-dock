mod assets;
mod dragout;
mod edge;

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

static SHELF_VISIBLE: AtomicBool = AtomicBool::new(false);
static VIBRANCY_APPLIED: AtomicBool = AtomicBool::new(false);

fn do_show_shelf(app: &tauri::AppHandle) {
    if SHELF_VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    let Some(win) = app.get_webview_window("main") else {
        return;
    };

    // Position on-screen before showing so no flash at wrong location
    if let Ok(Some(monitor)) = win.primary_monitor() {
        let s = monitor.size();
        let p = monitor.position();
        let shelf_w = 340_i32;
        let shelf_h = (s.height as i32).min(700);
        let x = p.x + s.width as i32 - shelf_w - 10;
        let y = p.y + (s.height as i32 - shelf_h) / 2;
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = win.set_size(tauri::PhysicalSize::new(shelf_w as u32, shelf_h as u32));
    }

    // First show: make visible then apply vibrancy so DWM composites the material
    if !VIBRANCY_APPLIED.load(Ordering::Relaxed) {
        let _ = win.show();
        #[cfg(target_os = "windows")]
        {
            use window_vibrancy::{apply_acrylic, apply_mica};
            if apply_acrylic(&win, Some((18, 18, 18, 90))).is_err() {
                let _ = apply_mica(&win, Some(true));
            }
            // Nudge to force DWM recomposition
            if let Ok(sz) = win.inner_size() {
                let _ = win.set_size(tauri::PhysicalSize::new(sz.width + 1, sz.height));
                let _ = win.set_size(tauri::PhysicalSize::new(sz.width, sz.height));
            }
        }
        VIBRANCY_APPLIED.store(true, Ordering::Relaxed);
    }

    let _ = win.set_focus();
    let _ = win.emit("quickdock://shelf-show", ());
    SHELF_VISIBLE.store(true, Ordering::Relaxed);
}

fn do_hide_shelf(app: &tauri::AppHandle) {
    if !SHELF_VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    // Move off-screen instead of .hide() — keeps DWM compositing the material
    // so Acrylic/Mica is still active on next show without re-applying
    let _ = win.set_position(tauri::PhysicalPosition::new(30000_i32, 0_i32));
    SHELF_VISIBLE.store(false, Ordering::Relaxed);
}

#[tauri::command]
fn show_shelf(app: tauri::AppHandle) {
    do_show_shelf(&app);
}

#[tauri::command]
fn hide_shelf(app: tauri::AppHandle) {
    do_hide_shelf(&app);
}

#[tauri::command]
fn generate_thumbnail(path: String) -> Result<String, String> {
    assets::generate_thumbnail(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn classify_path(path: String) -> String {
    assets::classify_path(&path).to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            setup_tray(app)?;
            edge::setup_edge_window(app)?;
            setup_drag_handlers(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            generate_thumbnail,
            classify_path,
            dragout::start_file_drag,
            show_shelf,
            hide_shelf,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show_hide = MenuItem::with_id(app, "show_hide", "Show / Hide Shelf", true, None::<&str>)?;
    let clear_all = MenuItem::with_id(app, "clear_all", "Clear All Items", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit QuickDock", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_hide, &clear_all, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("QuickDock – Contextual Staging Shelf")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show_hide" => {
                if SHELF_VISIBLE.load(Ordering::Relaxed) {
                    do_hide_shelf(app);
                } else {
                    do_show_shelf(app);
                }
            }
            "clear_all" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.emit("quickdock://clear-all", ());
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn setup_drag_handlers(app: &mut tauri::App) {
    // Backup path: edge window events (secondary to the polling thread)
    let app_handle = app.handle().clone();
    if let Some(edge_win) = app.get_webview_window("edge") {
        edge_win.on_window_event(move |event| match event {
            tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Enter { .. })
            | tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Over { .. }) => {
                do_show_shelf(&app_handle);
            }
            _ => {}
        });
    }

    // Primary path: poll cursor position so the trigger works regardless of
    // window hit-testing, WebView2 quirks, or drag-drop registration issues
    #[cfg(target_os = "windows")]
    start_drag_edge_poll(app.handle().clone());
}

#[cfg(target_os = "windows")]
fn start_drag_edge_poll(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetSystemMetrics};

        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));

            // VK_LBUTTON (0x01) held = mouse button down = likely a drag
            if unsafe { GetAsyncKeyState(0x01) } as u16 & 0x8000 == 0 {
                continue;
            }

            let mut pt = POINT { x: 0, y: 0 };
            if unsafe { GetCursorPos(&mut pt) } == 0 {
                continue;
            }

            // SM_CXSCREEN (0) = primary monitor width in screen coordinates
            let screen_w = unsafe { GetSystemMetrics(0) };
            if pt.x >= screen_w - 30 {
                do_show_shelf(&app);
            }
        }
    });
}
