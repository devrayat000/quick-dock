mod assets;
mod dragout;
mod settings;

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, Ordering};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

static SHELF_VISIBLE: AtomicBool = AtomicBool::new(false);
// Per-open-session close policy
static CLOSE_POLICY: AtomicU8 = AtomicU8::new(0);
// Currently configured retrieval mode (mirrors persisted setting)
static OPEN_MODE: AtomicU8 = AtomicU8::new(0);

static MON_X: AtomicI32 = AtomicI32::new(0);
static MON_Y: AtomicI32 = AtomicI32::new(0);
static MON_W: AtomicI32 = AtomicI32::new(1920);
static MON_H: AtomicI32 = AtomicI32::new(1080);

const SHELF_W: i32 = 340;
const SHELF_MAX_H: i32 = 700;
const GAP: i32 = 10;
const PARK_X: i32 = -32000;
const TAB_PEEK: i32 = 6; // px of sliver visible on-screen in Tab mode

// CLOSE_POLICY values
const CP_CURSOR_PARK: u8 = 0;   // cursor-leave → park off-screen
const CP_CURSOR_SLIVER: u8 = 1; // cursor-leave → collapse to sliver
const CP_BLUR: u8 = 2;           // window focus-loss → hide
const CP_MANUAL: u8 = 3;         // only explicit close (tray / Esc / ✕)

// OPEN_MODE values
const OM_HOVER: u8 = 0;
const OM_TAB: u8 = 1;
const OM_TRAY: u8 = 2;

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
fn sliver_x() -> i32 {
    screen_right() - TAB_PEEK
}

/// Called once in setup: cache monitor, position window, apply Acrylic.
/// visible:true in config keeps WebView2/IDropTarget initialized; we park off-screen immediately.
fn init_window(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };

    if let Ok(Some(m)) = win.primary_monitor() {
        MON_X.store(m.position().x, Ordering::Relaxed);
        MON_Y.store(m.position().y, Ordering::Relaxed);
        MON_W.store(m.size().width as i32, Ordering::Relaxed);
        MON_H.store(m.size().height as i32, Ordering::Relaxed);
    }

    let _ = win.set_size(tauri::PhysicalSize::new(SHELF_W as u32, shelf_h() as u32));

    // Load persisted mode; atomics must be set before park position is computed
    let initial_mode = app
        .path()
        .app_config_dir()
        .map(|dir| settings::mode_to_u8(&settings::load(&dir).open_mode))
        .unwrap_or(0);
    OPEN_MODE.store(initial_mode, Ordering::Relaxed);

    // Tab mode: park at sliver so the strip is already on-screen at startup
    let (park_x, park_y) = if initial_mode == OM_TAB {
        (sliver_x(), shelf_y())
    } else {
        (PARK_X, 0)
    };
    let _ = win.set_position(tauri::PhysicalPosition::new(park_x, park_y));

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

fn do_show_shelf(app: &tauri::AppHandle, policy: u8) {
    if SHELF_VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    // Do NOT call set_focus() during OLE drag — disrupts Explorer's DoDragDrop.
    // set_focus only for CP_BLUR (tray open in Tray mode, no active drag).
    let _ = win.set_position(tauri::PhysicalPosition::new(shelf_x(), shelf_y()));
    CLOSE_POLICY.store(policy, Ordering::Relaxed);
    SHELF_VISIBLE.store(true, Ordering::Relaxed);
    let _ = win.emit("quickdock://shelf-show", ());
    if policy == CP_BLUR {
        let _ = win.set_focus();
    }
}

fn do_hide_shelf(app: &tauri::AppHandle) {
    if !SHELF_VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    // Capture policy before reset so spawn knows final park position
    let policy = CLOSE_POLICY.load(Ordering::Relaxed);
    SHELF_VISIBLE.store(false, Ordering::Relaxed);
    CLOSE_POLICY.store(CP_CURSOR_PARK, Ordering::Relaxed);

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.emit("quickdock://shelf-hide", ());
        let win_clone = win.clone();
        // Tab collapse → sliver; everything else → off-screen park
        let (park_x, park_y) = if policy == CP_CURSOR_SLIVER {
            (sliver_x(), shelf_y())
        } else {
            (PARK_X, 0)
        };
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(210));
            if !SHELF_VISIBLE.load(Ordering::Relaxed) {
                let _ = win_clone.set_position(tauri::PhysicalPosition::new(park_x, park_y));
            }
        });
    }
}

#[tauri::command]
fn show_shelf(app: tauri::AppHandle) {
    do_show_shelf(&app, CP_MANUAL);
}

#[tauri::command]
fn hide_shelf(app: tauri::AppHandle) {
    do_hide_shelf(&app);
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> settings::Settings {
    app.path()
        .app_config_dir()
        .map(|dir| settings::load(&dir))
        .unwrap_or_default()
}

#[tauri::command]
fn set_open_mode(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    let old_mode = OPEN_MODE.load(Ordering::Relaxed);
    let new_mode = settings::mode_to_u8(&mode);
    OPEN_MODE.store(new_mode, Ordering::Relaxed);

    if let Ok(config_dir) = app.path().app_config_dir() {
        settings::save(&config_dir, &settings::Settings { open_mode: mode });
    }

    // Reposition window when shelf is hidden (so sliver appears/disappears)
    if !SHELF_VISIBLE.load(Ordering::Relaxed) {
        if let Some(win) = app.get_webview_window("main") {
            if new_mode == OM_TAB {
                let _ = win.set_position(tauri::PhysicalPosition::new(sliver_x(), shelf_y()));
            } else if old_mode == OM_TAB {
                // Was Tab (sliver visible) — park fully
                let _ = win.set_position(tauri::PhysicalPosition::new(PARK_X, 0));
            }
        }
    }
    Ok(())
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

            // Focus-loss handler: closes shelf when Tray+auto-hide mode loses focus
            if let Some(win) = app.get_webview_window("main") {
                let handle_blur = handle.clone();
                win.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(false) = event {
                        if CLOSE_POLICY.load(Ordering::Relaxed) == CP_BLUR {
                            do_hide_shelf(&handle_blur);
                        }
                    }
                });
            }

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
            get_settings,
            set_open_mode,
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
                    let open_mode = OPEN_MODE.load(Ordering::Relaxed);
                    if open_mode == OM_TRAY {
                        // Tray mode: blur-close policy + focus (safe here, no active OLE drag)
                        do_show_shelf(app, CP_BLUR);
                    } else {
                        // Hover/Tab: tray opens pinned (no cursor auto-close)
                        do_show_shelf(app, CP_MANUAL);
                    }
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

        let mut hover_dwell: u8 = 0;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));

            let mut pt = POINT { x: 0, y: 0 };
            if unsafe { GetCursorPos(&mut pt) } == 0 {
                continue;
            }

            let visible = SHELF_VISIBLE.load(Ordering::Relaxed);
            let sy = shelf_y();
            let sh = shelf_h();
            let in_y_band = pt.y >= sy && pt.y < sy + sh;

            if !visible {
                let lbtn_down = unsafe { GetAsyncKeyState(0x01) } as u16 & 0x8000 != 0;

                // Drag-in: always active regardless of open mode
                if lbtn_down && pt.x >= screen_right() - 30 && in_y_band {
                    hover_dwell = 0;
                    do_show_shelf(&app, CP_CURSOR_PARK);
                } else {
                    match OPEN_MODE.load(Ordering::Relaxed) {
                        OM_HOVER => {
                            // No button held; cursor at rightmost 2px; dwell 3 ticks (~150ms)
                            if !lbtn_down && pt.x >= screen_right() - 2 && in_y_band {
                                hover_dwell = hover_dwell.saturating_add(1);
                                if hover_dwell >= 3 {
                                    hover_dwell = 0;
                                    do_show_shelf(&app, CP_CURSOR_PARK);
                                }
                            } else {
                                hover_dwell = 0;
                            }
                        }
                        OM_TAB => {
                            // Cursor enters the visible sliver strip → open
                            if !lbtn_down && pt.x >= sliver_x() && in_y_band {
                                do_show_shelf(&app, CP_CURSOR_SLIVER);
                            }
                        }
                        _ => {
                            // OM_TRAY: no edge trigger
                            hover_dwell = 0;
                        }
                    }
                }
            } else {
                // Auto-close only for cursor-based policies; guard against drag-out
                let policy = CLOSE_POLICY.load(Ordering::Relaxed);
                if policy == CP_CURSOR_PARK || policy == CP_CURSOR_SLIVER {
                    if !crate::dragout::DRAG_OUT_ACTIVE.load(Ordering::Relaxed) {
                        let sx = shelf_x();
                        let sr = screen_right();
                        let inside = pt.x >= sx && pt.x < sr && pt.y >= sy && pt.y < sy + sh;
                        if !inside {
                            do_hide_shelf(&app);
                        }
                    }
                }
            }
        }
    });
}
