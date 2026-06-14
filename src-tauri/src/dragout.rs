use std::path::PathBuf;
use tauri::Manager;

#[tauri::command]
pub async fn start_file_drag(
    app: tauri::AppHandle,
    paths: Vec<String>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    let file_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    tauri::async_runtime::spawn_blocking(move || {
        // drag::start_drag is blocking; it drives the DoDragDrop message loop
        // on Windows and returns when the user releases the drag.
        let _ = drag::start_drag(
            &window,
            drag::DragItem::Files(file_paths),
            drag::Image::Raw(vec![]),
            |_result, _cursor| {},
            drag::Options::default(),
        );
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
