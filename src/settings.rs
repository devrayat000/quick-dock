use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Settings {
    #[serde(default = "default_open_mode")]
    pub open_mode: String,
}

fn default_open_mode() -> String {
    "hover".to_string()
}

pub fn mode_to_u8(mode: &str) -> u8 {
    match mode {
        "tab" => 1,
        "tray" => 2,
        _ => 0, // hover (default)
    }
}

/// %APPDATA%\SnapShelf (replaces Tauri's app_config_dir). Falls back to the temp dir.
pub fn config_dir() -> std::path::PathBuf {
    std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("SnapShelf")
}

pub fn load(config_dir: &std::path::Path) -> Settings {
    std::fs::read_to_string(config_dir.join("settings.json"))
        .ok()
        .and_then(|data| serde_json::from_str::<Settings>(&data).ok())
        .unwrap_or_default()
}

pub fn save(config_dir: &std::path::Path, settings: &Settings) {
    let _ = std::fs::create_dir_all(config_dir);
    if let Ok(data) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(config_dir.join("settings.json"), data);
    }
}
