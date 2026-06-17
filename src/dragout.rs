use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

pub static DRAG_OUT_ACTIVE: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub async fn start_file_drag(
    app: tauri::AppHandle,
    paths: Vec<String>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    let file_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    DRAG_OUT_ACTIVE.store(true, Ordering::Relaxed);

    let result = tauri::async_runtime::spawn_blocking(move || {
        // start_drag drives the DoDragDrop message loop; blocks until user releases.
        let _ = drag::start_drag(
            &window,
            drag::DragItem::Files(file_paths),
            drag::Image::Raw(vec![]),
            |_result, _cursor| {},
            drag::Options::default(),
        );
    })
    .await
    .map_err(|e| e.to_string());

    DRAG_OUT_ACTIVE.store(false, Ordering::Relaxed);

    result
}
