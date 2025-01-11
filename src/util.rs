use std::path::{Path, PathBuf};

pub fn get_asset_path(asset: &str) -> PathBuf {
    // First, try looking for assets relative to the executable
    let executable_path = std::env::current_exe().expect("Failed to get executable path");
    let executable_dir = executable_path
        .parent()
        .expect("Failed to get executable directory");

    // Check different possible asset locations
    let possible_paths = vec![
        // 1. Check next to the executable
        executable_dir.join("assets").join(asset),
        // 2. Check in Resources folder (for macOS .app bundles)
        executable_dir.join("../Resources/assets").join(asset),
        // 3. Check relative to CARGO_MANIFEST_DIR (for development)
        Path::new(&env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(asset),
    ];

    // Try each path and return the first one that exists
    for path in possible_paths {
        if path.exists() {
            return path;
        }
    }

    panic!("Could not find asset: {}", asset);
}
