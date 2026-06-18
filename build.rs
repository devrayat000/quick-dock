use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    // Register lucide-slint as the "@lucide" Slint library so the UI can import its icons.
    let library = HashMap::from([("lucide".to_string(), PathBuf::from(lucide_slint::lib()))]);
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library);
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}
