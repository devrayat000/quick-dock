use tauri::Manager;

pub fn setup_edge_window(app: &mut tauri::App) -> tauri::Result<()> {
    let Some(edge_win) = app.get_webview_window("edge") else {
        return Ok(());
    };

    let monitor = match edge_win.primary_monitor()? {
        Some(m) => m,
        None => {
            // No primary monitor info; show edge win at default position as fallback
            edge_win.show()?;
            return Ok(());
        }
    };

    let size = monitor.size();
    let pos = monitor.position();

    // 2px strip at the right edge of the primary monitor
    let edge_x = pos.x + size.width as i32 - 2;
    edge_win.set_position(tauri::PhysicalPosition::new(edge_x, pos.y))?;
    edge_win.set_size(tauri::PhysicalSize::new(2_u32, size.height))?;
    edge_win.show()?;

    // Position main shelf to the left of the edge strip, vertically centred
    if let Some(main_win) = app.get_webview_window("main") {
        let shelf_w = 340_i32;
        let shelf_h = size.height.min(700);
        let shelf_x = pos.x + size.width as i32 - shelf_w - 2;
        let shelf_y = pos.y + (size.height as i32 - shelf_h as i32) / 2;

        main_win.set_position(tauri::PhysicalPosition::new(shelf_x, shelf_y))?;
        main_win.set_size(tauri::PhysicalSize::new(shelf_w as u32, shelf_h))?;
    }

    Ok(())
}
