mod assets;
mod dragout;
mod edge;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

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
            setup_vibrancy(app);
            edge::setup_edge_window(app)?;
            setup_drag_handlers(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            generate_thumbnail,
            classify_path,
            dragout::start_file_drag
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
                if let Some(win) = app.get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        let _ = win.hide();
                    } else {
                        let _ = win.show();
                        let _ = win.set_focus();
                        let _ = win.emit("quickdock://shelf-show", ());
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

fn setup_vibrancy(app: &mut tauri::App) {
    #[cfg(target_os = "windows")]
    if let Some(window) = app.get_webview_window("main") {
        use window_vibrancy::{apply_acrylic, apply_mica};
        if apply_mica(&window, Some(true)).is_err() {
            let _ = apply_acrylic(&window, Some((18, 18, 18, 180)));
        }
    }
}

fn setup_drag_handlers(app: &mut tauri::App) {
    // Main window: handle inbound file drops from OS
    if let Some(window) = app.get_webview_window("main") {
        let win_clone = window.clone();
        window.on_window_event(move |event| {
            match event {
                tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, position }) => {
                    let payload = serde_json::json!({
                        "paths": paths.iter()
                            .map(|p| p.to_string_lossy().to_string())
                            .collect::<Vec<_>>(),
                        "position": { "x": position.x, "y": position.y }
                    });
                    let _ = win_clone.emit("quickdock://drop", payload);
                }
                tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Enter { .. }) => {
                    let _ = win_clone.emit("quickdock://drag-enter", ());
                }
                tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Leave) => {
                    let _ = win_clone.emit("quickdock://drag-leave", ());
                }
                _ => {}
            }
        });
    }

    // Edge window: slide shelf in when a drag hovers the right-edge strip
    if let Some(edge_win) = app.get_webview_window("edge") {
        if let Some(main_win) = app.get_webview_window("main") {
            edge_win.on_window_event(move |event| {
                match event {
                    tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Enter { .. })
                    | tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Over { .. }) => {
                        let _ = main_win.show();
                        let _ = main_win.set_focus();
                        let _ = main_win.emit("quickdock://shelf-show", ());
                    }
                    _ => {}
                }
            });
        }
    }
}
