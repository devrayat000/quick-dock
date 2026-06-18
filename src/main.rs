// Prevents an extra console window on Windows in release. DO NOT REMOVE.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod dragout;
mod settings;
mod state;
mod sys;

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU8, Ordering};
use std::time::Duration;

use raw_window_handle::HasWindowHandle;
use slint::winit_030::WinitWindowAccessor;
use slint::ComponentHandle;

use settings::Settings;
use state::Shelf;

slint::include_modules!();

// --- shared state -------------------------------------------------------------
static SHELF_VISIBLE: AtomicBool = AtomicBool::new(false);
static CLOSE_POLICY: AtomicU8 = AtomicU8::new(0);
static OPEN_MODE: AtomicU8 = AtomicU8::new(0);
static HWND: AtomicIsize = AtomicIsize::new(0);

static MON_X: AtomicI32 = AtomicI32::new(0);
static MON_Y: AtomicI32 = AtomicI32::new(0);
static MON_W: AtomicI32 = AtomicI32::new(1920);
static MON_H: AtomicI32 = AtomicI32::new(1080);
// Display scale factor x100 (e.g. 150 = 1.5). Monitor + cursor coords are physical px;
// the Slint UI is laid out in logical px, so window size/placement must account for scale.
static SCALE_PCT: AtomicI32 = AtomicI32::new(100);

const SHELF_W: i32 = 340;
const SHELF_MAX_H: i32 = 700;
const GAP: i32 = 10;
const PARK_X: i32 = -32000;
const TAB_PEEK: i32 = 6;

// CLOSE_POLICY
const CP_CURSOR_PARK: u8 = 0;
const CP_CURSOR_SLIVER: u8 = 1;
const CP_BLUR: u8 = 2;
const CP_MANUAL: u8 = 3;

// OPEN_MODE
const OM_HOVER: u8 = 0;
const OM_TAB: u8 = 1;
const OM_TRAY: u8 = 2;

const EVICT_TTL_MS: u64 = 15 * 60 * 1000;

// Shelf model lives on the UI thread; reached from UI-thread closures (callbacks, timers,
// drop handler, and event-loop-marshalled work) without Send gymnastics.
thread_local! {
    static SHELF: Rc<Shelf> = Shelf::new();
    static INIT_TIMER: std::cell::RefCell<Option<slint::Timer>> = std::cell::RefCell::new(None);
}

// --- geometry (all physical px; SHELF_W/MAX_H/GAP/TAB_PEEK are logical) ---------
fn scale() -> f64 {
    SCALE_PCT.load(Ordering::Relaxed) as f64 / 100.0
}
fn px(logical: i32) -> i32 {
    (logical as f64 * scale()).round() as i32
}
fn phys_w() -> i32 {
    px(SHELF_W)
}
fn phys_h() -> i32 {
    MON_H.load(Ordering::Relaxed).min(px(SHELF_MAX_H))
}
fn shelf_x() -> i32 {
    MON_X.load(Ordering::Relaxed) + MON_W.load(Ordering::Relaxed) - phys_w() - px(GAP)
}
fn shelf_y() -> i32 {
    MON_Y.load(Ordering::Relaxed) + (MON_H.load(Ordering::Relaxed) - phys_h()) / 2
}
fn screen_right() -> i32 {
    MON_X.load(Ordering::Relaxed) + MON_W.load(Ordering::Relaxed)
}
fn sliver_x() -> i32 {
    screen_right() - px(TAB_PEEK)
}

fn set_window_pos(ui: &AppWindow, x: i32, y: i32) {
    ui.window().set_position(slint::PhysicalPosition::new(x, y));
}

/// Force the software renderer to repaint the entire window. Moving the window on-screen from the
/// off-screen park can leave stale buffer content (green artifacts) in fully-transparent regions;
/// a ±1px size toggle marks the whole surface dirty so it is fully redrawn.
fn force_full_repaint(ui: &AppWindow) {
    let s = ui.window().size();
    if s.width > 0 {
        let _ = ui
            .window()
            .set_size(slint::PhysicalSize::new(s.width + 1, s.height));
        let _ = ui.window().set_size(s);
    }
}

// --- show / hide (UI thread only) --------------------------------------------
fn do_show_shelf(ui: &AppWindow, policy: u8) {
    if SHELF_VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    set_window_pos(ui, shelf_x(), shelf_y());
    force_full_repaint(ui);
    CLOSE_POLICY.store(policy, Ordering::Relaxed);
    SHELF_VISIBLE.store(true, Ordering::Relaxed);
    ui.set_shelf_visible(true);
    // Focus only when there is no active OLE drag (tray open in Tray mode).
    if policy == CP_BLUR {
        ui.window().with_winit_window(|w| w.focus_window());
    }
}

fn do_hide_shelf(ui: &AppWindow) {
    if !SHELF_VISIBLE.load(Ordering::Relaxed) {
        return;
    }
    let policy = CLOSE_POLICY.load(Ordering::Relaxed);
    SHELF_VISIBLE.store(false, Ordering::Relaxed);
    CLOSE_POLICY.store(CP_CURSOR_PARK, Ordering::Relaxed);
    ui.set_shelf_visible(false);

    // Tab collapse -> sliver; everything else -> off-screen park. Delay so the slide plays.
    let (px, py) = if policy == CP_CURSOR_SLIVER {
        (sliver_x(), shelf_y())
    } else {
        (PARK_X, 0)
    };
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(210));
        if !SHELF_VISIBLE.load(Ordering::Relaxed) {
            let _ = weak.upgrade_in_event_loop(move |ui| set_window_pos(&ui, px, py));
        }
    });
}

// --- inbound drop handling (UI thread) ---------------------------------------
fn handle_dropped_file(ui_weak: &slint::Weak<AppWindow>, path: std::path::PathBuf) {
    let path_str = path.to_string_lossy().to_string();
    let kind = assets::classify_path(&path_str);
    if kind == "image" {
        let id = SHELF.with(|s| s.add(state::make_image(&path_str)));
        // Generate the thumbnail off-thread, then load + attach it on the UI thread.
        let weak = ui_weak.clone();
        let p = path_str.clone();
        std::thread::spawn(move || {
            if let Ok(thumb_path) = assets::generate_thumbnail(&p) {
                let _ = weak.upgrade_in_event_loop(move |_ui| {
                    if let Ok(img) = slint::Image::load_from_path(std::path::Path::new(&thumb_path)) {
                        SHELF.with(|s| s.set_thumb(id, img));
                    }
                });
            }
        });
    } else {
        SHELF.with(|s| s.add(state::make_file(&path_str)));
    }
}

fn paste_clipboard() {
    if let Some(text) = sys::get_clipboard_text() {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            SHELF.with(|s| s.add(state::make_url(trimmed)));
        } else {
            SHELF.with(|s| s.add(state::make_text(&text)));
        }
    }
}

fn main() {
    // 1. Backend: winit + software renderer, with window attributes set at creation
    //    (transparent, undecorated, always-on-top, skip-taskbar, parked off-screen).
    {
        use slint::winit_030::winit::dpi::{PhysicalPosition, PhysicalSize};
        use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
        use slint::winit_030::winit::window::WindowLevel;

        slint::BackendSelector::new()
            .backend_name("winit".into())
            .with_winit_window_attributes_hook(|attrs| {
                attrs
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_resizable(false)
                    .with_skip_taskbar(true)
                    .with_window_level(WindowLevel::AlwaysOnTop)
                    .with_inner_size(PhysicalSize::new(SHELF_W as u32, SHELF_MAX_H as u32))
                    .with_position(PhysicalPosition::new(PARK_X, 0))
            })
            .select()
            .expect("failed to select winit backend");
    }

    let ui = AppWindow::new().expect("failed to create window");

    // Bind the shelf model.
    ui.set_items(SHELF.with(|s| s.model_rc()));

    // Load persisted open-mode before showing.
    let loaded = settings::load(&settings::config_dir());
    OPEN_MODE.store(settings::mode_to_u8(&loaded.open_mode), Ordering::Relaxed);
    ui.set_open_mode(loaded.open_mode.clone().into());

    wire_callbacks(&ui);

    // File-drop + focus-loss hook on the winit window.
    {
        use slint::winit_030::winit::event::WindowEvent;
        use slint::winit_030::EventResult;
        let weak = ui.as_weak();
        ui.window().on_winit_window_event(move |_win, event| {
            match event {
                WindowEvent::HoveredFile(_) => {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_dropping(true);
                    }
                }
                WindowEvent::HoveredFileCancelled => {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_dropping(false);
                    }
                }
                WindowEvent::DroppedFile(path) => {
                    handle_dropped_file(&weak, path.clone());
                    if let Some(ui) = weak.upgrade() {
                        ui.set_dropping(false);
                    }
                }
                WindowEvent::Focused(false) => {
                    if CLOSE_POLICY.load(Ordering::Relaxed) == CP_BLUR {
                        if let Some(ui) = weak.upgrade() {
                            do_hide_shelf(&ui);
                        }
                    }
                }
                _ => {}
            }
            EventResult::Propagate
        });
    }

    // Init once the winit window actually exists (it is created lazily after run() starts):
    // cache monitor + scale, capture HWND, apply Acrylic, size + park, start the poll thread.
    // Retry on a short timer until has_winit_window() is true, then stop.
    {
        let t = slint::Timer::default();
        let weak = ui.as_weak();
        t.start(slint::TimerMode::Repeated, Duration::from_millis(30), move || {
            let Some(ui) = weak.upgrade() else { return };
            if ui.window().has_winit_window() {
                init_winit(&ui);
                start_drag_edge_poll(ui.as_weak());
                INIT_TIMER.with(|c| {
                    if let Some(t) = c.borrow().as_ref() {
                        t.stop();
                    }
                });
            }
        });
        INIT_TIMER.with(|c| *c.borrow_mut() = Some(t));
    }

    // Tray icon + menu-event polling timer.
    let _tray = setup_tray();
    let tray_timer = start_tray_poll(ui.as_weak());

    // TTL eviction timer (every 60s).
    let evict_timer = slint::Timer::default();
    evict_timer.start(slint::TimerMode::Repeated, Duration::from_secs(60), || {
        SHELF.with(|s| s.clear_expired(EVICT_TTL_MS));
    });

    ui.run().expect("event loop error");

    drop(tray_timer);
    drop(evict_timer);
}

fn init_winit(ui: &AppWindow) {
    use raw_window_handle::RawWindowHandle;

    ui.window().with_winit_window(|w| {
        if let Some(mon) = w.current_monitor() {
            let pos = mon.position();
            let size = mon.size();
            MON_X.store(pos.x, Ordering::Relaxed);
            MON_Y.store(pos.y, Ordering::Relaxed);
            MON_W.store(size.width as i32, Ordering::Relaxed);
            MON_H.store(size.height as i32, Ordering::Relaxed);
        }
        if let Ok(handle) = w.window_handle() {
            if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                let hwnd = win32.hwnd.get();
                HWND.store(hwnd, Ordering::Relaxed);
                // Win11 rounded corners (frame removal is handled by slint no-frame + decorations).
                sys::round_corners(hwnd);
            }
        }

        // Window-level Acrylic (Mica fallback).
        if window_vibrancy::apply_acrylic(w, Some((18, 18, 18, 90))).is_err() {
            let _ = window_vibrancy::apply_mica(w, Some(true));
        }

        // No native frame / caption buttons — only our custom header controls.
        w.set_decorations(false);

        // Alt-tab / window icon (taskbar is skipped).
        if let Ok(img) = image::load_from_memory(include_bytes!("../icons/32x32.png")) {
            let rgba = img.to_rgba8();
            let (iw, ih) = rgba.dimensions();
            if let Ok(icon) =
                slint::winit_030::winit::window::Icon::from_rgba(rgba.into_raw(), iw, ih)
            {
                w.set_window_icon(Some(icon));
            }
        }
    });

    // Capture the display scale so logical UI px map to physical placement.
    SCALE_PCT.store((ui.window().scale_factor() * 100.0) as i32, Ordering::Relaxed);

    let _ = ui
        .window()
        .set_size(slint::PhysicalSize::new(phys_w() as u32, phys_h() as u32));

    // Park: Tab mode shows the sliver, otherwise fully off-screen.
    let (x, y) = if OPEN_MODE.load(Ordering::Relaxed) == OM_TAB {
        (sliver_x(), shelf_y())
    } else {
        (PARK_X, 0)
    };
    set_window_pos(ui, x, y);
}

fn wire_callbacks(ui: &AppWindow) {
    ui.on_request_hide({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                do_hide_shelf(&ui);
            }
        }
    });

    ui.on_clear_all(|| SHELF.with(|s| s.clear()));
    ui.on_remove_item(|id| SHELF.with(|s| s.remove(id)));
    ui.on_reorder_item(|from, to| {
        if from >= 0 && to >= 0 {
            SHELF.with(|s| s.reorder(from as usize, to as usize));
        }
    });

    ui.on_copy_text(|t| sys::set_clipboard_text(&t));
    ui.on_open_url(|u| sys::open_url(&u));
    ui.on_drag_out(|p| {
        let hwnd = HWND.load(Ordering::Relaxed);
        eprintln!("[drag-out] callback fired: path={:?} hwnd={:#x}", p.as_str(), hwnd);
        if hwnd != 0 {
            dragout::start_file_drag(hwnd, vec![p.to_string()]);
        } else {
            eprintln!("[drag-out] HWND is 0 — aborting");
        }
    });
    ui.on_paste_clipboard(|| paste_clipboard());

    ui.on_set_open_mode({
        let weak = ui.as_weak();
        move |mode| {
            let old = OPEN_MODE.load(Ordering::Relaxed);
            let new = settings::mode_to_u8(&mode);
            OPEN_MODE.store(new, Ordering::Relaxed);
            settings::save(
                &settings::config_dir(),
                &Settings { open_mode: mode.to_string() },
            );
            if let Some(ui) = weak.upgrade() {
                ui.set_open_mode(mode.clone());
                if !SHELF_VISIBLE.load(Ordering::Relaxed) {
                    if new == OM_TAB {
                        set_window_pos(&ui, sliver_x(), shelf_y());
                    } else if old == OM_TAB {
                        set_window_pos(&ui, PARK_X, 0);
                    }
                }
            }
        }
    });
}

// --- tray ---------------------------------------------------------------------
fn setup_tray() -> Option<tray_icon::TrayIcon> {
    use tray_icon::menu::{Menu, MenuItem};
    use tray_icon::{Icon, TrayIconBuilder};

    let img = image::load_from_memory(include_bytes!("../icons/32x32.png"))
        .ok()?
        .to_rgba8();
    let (w, h) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), w, h).ok()?;

    let menu = Menu::new();
    let show_hide = MenuItem::new("Show / Hide Shelf", true, None);
    let clear_all = MenuItem::new("Clear All Items", true, None);
    let quit = MenuItem::new("Quit SnapShelf", true, None);
    menu.append_items(&[&show_hide, &clear_all, &quit]).ok()?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("SnapShelf – Contextual Staging Shelf")
        .with_icon(icon)
        .build()
        .ok()?;

    TRAY_IDS.with(|c| {
        *c.borrow_mut() = Some(TrayIds {
            show_hide: show_hide.id().clone(),
            clear_all: clear_all.id().clone(),
            quit: quit.id().clone(),
        });
    });

    Some(tray)
}

fn start_tray_poll(weak: slint::Weak<AppWindow>) -> slint::Timer {
    let timer = slint::Timer::default();
    // Capture menu ids by polling the global receiver each tick.
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(150), move || {
        while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            let Some(ui) = weak.upgrade() else { continue };
            TRAY_IDS.with(|ids| {
                if let Some(ids) = ids.borrow().as_ref() {
                    if event.id == ids.show_hide {
                        toggle_from_tray(&ui);
                    } else if event.id == ids.clear_all {
                        SHELF.with(|s| s.clear());
                    } else if event.id == ids.quit {
                        slint::quit_event_loop().ok();
                    }
                }
            });
        }
    });
    timer
}

// Tray menu ids, stored on the UI thread for the poll timer to compare against.
thread_local! {
    static TRAY_IDS: std::cell::RefCell<Option<TrayIds>> = std::cell::RefCell::new(None);
}
struct TrayIds {
    show_hide: tray_icon::menu::MenuId,
    clear_all: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}

fn toggle_from_tray(ui: &AppWindow) {
    if SHELF_VISIBLE.load(Ordering::Relaxed) {
        do_hide_shelf(ui);
    } else if OPEN_MODE.load(Ordering::Relaxed) == OM_TRAY {
        do_show_shelf(ui, CP_BLUR);
    } else {
        do_show_shelf(ui, CP_MANUAL);
    }
}

// --- cursor edge poll (50ms) --------------------------------------------------
fn start_drag_edge_poll(weak: slint::Weak<AppWindow>) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    std::thread::spawn(move || {
        let show = |policy: u8| {
            let _ = weak.upgrade_in_event_loop(move |ui| do_show_shelf(&ui, policy));
        };
        let hide = || {
            let _ = weak.upgrade_in_event_loop(|ui| do_hide_shelf(&ui));
        };

        let mut hover_dwell: u8 = 0;
        loop {
            std::thread::sleep(Duration::from_millis(50));

            let mut pt = POINT { x: 0, y: 0 };
            if unsafe { GetCursorPos(&mut pt) } == 0 {
                continue;
            }

            let visible = SHELF_VISIBLE.load(Ordering::Relaxed);
            let sy = shelf_y();
            let sh = phys_h();
            let in_y_band = pt.y >= sy && pt.y < sy + sh;

            if !visible {
                let lbtn_down = unsafe { GetAsyncKeyState(0x01) } as u16 & 0x8000 != 0;

                if lbtn_down && pt.x >= screen_right() - 30 && in_y_band {
                    hover_dwell = 0;
                    show(CP_CURSOR_PARK);
                } else {
                    match OPEN_MODE.load(Ordering::Relaxed) {
                        OM_HOVER => {
                            if !lbtn_down && pt.x >= screen_right() - 2 && in_y_band {
                                hover_dwell = hover_dwell.saturating_add(1);
                                if hover_dwell >= 3 {
                                    hover_dwell = 0;
                                    show(CP_CURSOR_PARK);
                                }
                            } else {
                                hover_dwell = 0;
                            }
                        }
                        OM_TAB => {
                            if !lbtn_down && pt.x >= sliver_x() && in_y_band {
                                show(CP_CURSOR_SLIVER);
                            }
                        }
                        _ => {
                            hover_dwell = 0;
                        }
                    }
                }
            } else {
                let policy = CLOSE_POLICY.load(Ordering::Relaxed);
                if policy == CP_CURSOR_PARK || policy == CP_CURSOR_SLIVER {
                    if !dragout::DRAG_OUT_ACTIVE.load(Ordering::Relaxed) {
                        let sx = shelf_x();
                        let sr = screen_right();
                        let inside = pt.x >= sx && pt.x < sr && pt.y >= sy && pt.y < sy + sh;
                        if !inside {
                            hide();
                        }
                    }
                }
            }
        }
    });
}
