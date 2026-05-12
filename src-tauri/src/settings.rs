use std::{env, path::PathBuf};

pub fn get_settings_path() -> Option<PathBuf> {
    env::var("APPDATA").ok().map(|appdata| {
        PathBuf::from(appdata).join("com.rubik.trigr").join("settings.json")
    })
}

pub fn init_settings_dir() {
    if let Some(path) = get_settings_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
}