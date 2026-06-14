mod assets;
mod dragout;

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

static SHELF_VISIBLE: AtomicBool = AtomicBool::new(false);
static OPENED_BY_TRIGGER: AtomicBool = AtomicBool::new(false);

// Primary monitor geometry cached at startup so the poll thread never calls
// primary_monitor() (IPC to UI thread) in a tight loop.
static MON_X: AtomicI32 = AtomicI32::new(0);
static MON_Y: AtomicI32 = AtomicI32::new(0);
static MON_W: AtomicI32 = AtomicI32::new(1920);
static MON_H: AtomicI32 = AtomicI32::new(1080);

const SHELF_W: i32 = 340;
const SHELF_MAX_H: i32 = 700;
const GAP: i32 = 10;      // px between shelf right edge and screen right edge
const PARK_X: i32 = -32000; // guaranteed off all monitors

fn shelf_x() -> i32 {
    MON_X.load(Ordering::Relaxed) + MON_W.load(Ordering::Relaxed) - SHELF_W - GAP
}
fn shelf_y() -> i32 {
    let mh = MON_H.load(Ordering::Relaxed);
    MON_Y.load(Ordering::Relaxed) + (mh - mh.min(SHELF_MAX_H)) / 2
}
fn shelf_h() -> i32 {
    MON_H.load(Ordering::Relaxed).min(SHELF_MAX_H)
}
fn screen_right() -> i32 {
    MON_X.load(Ordering::Relaxed) + MON_W.load(Ordering::Relaxed)
}

/// Called once in setup: cache monitor, move window off-screen, show it (so
/// WebView2 composites and registers its IDropTarget), then apply Acrylic.
/// The window stays at PARK_X until do_show_shelf moves it on-screen.
fn init_window(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };

    // Cache primary monitor geometry
    if let Ok(Some(m)) = win.primary_monitor() {
        MON_X.store(m.position().x, Ordering::Relaxed);
        MON_Y.store(m.position().y, Ordering::Relaxed);
        MON_W.store(m.size().width as i32, Ordering::Relaxed);
        MON_H.store(m.size().height as i32, Ordering::Relaxed);
    }

    // Size the shelf once based on monitor height
    let _ = win.set_size(tauri::PhysicalSize::new(
        SHELF_W as u32,
        shelf_h() as u32,
    ));

    // Window is visible:true from config so WebView2 + IDropTarget are fully
    // initialized before this runs. Just park it off-screen.
    let _ = win.set_position(tauri::PhysicalPosition::new(PARK_X, 0));

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
}

fn do_show_shelf(app: &tauri::AppHandle, by_trigger: bool) {
    if SHELF_VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    // Only reposition — size was set once at startup.
    // Do NOT call set_focus(): SetForegroundWindow during an OLE drag disrupts
    // Explorer's DoDragDrop loop and can cause the "not allowed" cursor.
    let _ = win.set_position(tauri::PhysicalPosition::new(shelf_x(), shelf_y()));
    OPENED_BY_TRIGGER.store(by_trigger, Ordering::Relaxed);
    SHELF_VISIBLE.store(true, Ordering::Relaxed);
    let _ = win.emit("quickdock://shelf-show", ());
}

fn do_hide_shelf(app: &tauri::AppHandle) {
    if !SHELF_VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    SHELF_VISIBLE.store(false, Ordering::Relaxed);
    OPENED_BY_TRIGGER.store(false, Ordering::Relaxed);
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.emit("quickdock://shelf-hide", ());
        let win_clone = win.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(210));
            if !SHELF_VISIBLE.load(Ordering::Relaxed) {
                let _ = win_clone.set_position(tauri::PhysicalPosition::new(PARK_X, 0));
            }
        });
    }
}

#[tauri::command]
fn show_shelf(app: tauri::AppHandle) {
    do_show_shelf(&app, false);
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
            let handle = app.handle().clone();
            init_window(&handle);
            setup_tray(app)?;
            #[cfg(target_os = "windows")]
            start_drag_edge_poll(handle);
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
                    do_show_shelf(app, false);
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

#[cfg(target_os = "windows")]
fn start_drag_edge_poll(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));

            let mut pt = POINT { x: 0, y: 0 };
            if unsafe { GetCursorPos(&mut pt) } == 0 {
                continue;
            }

            let visible = SHELF_VISIBLE.load(Ordering::Relaxed);

            if !visible {
                // Open: drag (left button held) approaching the right screen edge
                let lbtn_down = unsafe { GetAsyncKeyState(0x01) } as u16 & 0x8000 != 0;
                if lbtn_down && pt.x >= screen_right() - 30 {
                    do_show_shelf(&app, true);
                }
            } else if OPENED_BY_TRIGGER.load(Ordering::Relaxed) {
                // Auto-close: cursor left the shelf zone.
                // Right boundary = screen_right() (not shelf right edge) so the
                // 10px gap between shelf and screen edge does NOT trigger a close —
                // which was causing rapid open/close flicker AND killing OLE DragEnter.
                let sx = shelf_x();
                let sy = shelf_y();
                let sh = shelf_h();
                let sr = screen_right();
                let inside = pt.x >= sx && pt.x < sr && pt.y >= sy && pt.y < sy + sh;
                if !inside {
                    do_hide_shelf(&app);
                }
            }
        }
    });
}
