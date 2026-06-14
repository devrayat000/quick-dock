use tauri::Manager;

pub fn setup_edge_window(app: &mut tauri::App) -> tauri::Result<()> {
    let Some(edge_win) = app.get_webview_window("edge") else {
        return Ok(());
    };

    let monitor = match edge_win.primary_monitor()? {
        Some(m) => m,
        None => {
            edge_win.show()?;
            return Ok(());
        }
    };

    let size = monitor.size();
    let pos = monitor.position();

    // 10px hittable strip flush to the right edge of the primary monitor
    let edge_x = pos.x + size.width as i32 - 10;
    edge_win.set_position(tauri::PhysicalPosition::new(edge_x, pos.y))?;
    edge_win.set_size(tauri::PhysicalSize::new(10_u32, size.height))?;
    edge_win.show()?;

    Ok(())
}
