use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate_thumbnail(path: &str) -> Result<String, String> {
    let src = Path::new(path);
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif") {
        return Err(format!("not a supported image type: {ext}"));
    }

    let img = image::open(src).map_err(|e| e.to_string())?;
    let thumb = img.thumbnail(200, 200);

    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("img");
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let thumb_name = format!("qd_{stem}_{ts}.png");
    let thumb_path = std::env::temp_dir().join(thumb_name);

    thumb.save(&thumb_path).map_err(|e| e.to_string())?;
    Ok(thumb_path.to_string_lossy().to_string())
}

pub fn classify_path(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" | "ico" | "avif" => "image",
        _ => "file",
    }
}
