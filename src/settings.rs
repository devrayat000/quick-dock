// Persistence for the single setting (open mode). One word in a plain-text file — no serde/json.

use std::path::{Path, PathBuf};

pub fn mode_to_u8(mode: &str) -> u8 {
    match mode {
        "tab" => 1,
        "tray" => 2,
        _ => 0, // hover (default)
    }
}

/// %APPDATA%\SnapShelf (replaces Tauri's app_config_dir). Falls back to the temp dir.
pub fn config_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("SnapShelf")
}

fn file(config_dir: &Path) -> PathBuf {
    config_dir.join("open_mode")
}

/// Load the persisted open mode, validated; defaults to "hover".
pub fn load_open_mode(config_dir: &Path) -> String {
    std::fs::read_to_string(file(config_dir))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s == "hover" || s == "tab" || s == "tray")
        .unwrap_or_else(|| "hover".to_string())
}

pub fn save_open_mode(config_dir: &Path, mode: &str) {
    let _ = std::fs::create_dir_all(config_dir);
    let _ = std::fs::write(file(config_dir), mode);
}
