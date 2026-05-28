use std::{env, path::PathBuf};

const APP_DIR_NAME: &str = "com.rubik.trigr";
const SETTINGS_FILENAME: &str = "settings.json";

pub fn get_settings_path() -> Option<PathBuf> {
    env::var("APPDATA").map_or(None, |appdata| Some(PathBuf::from(appdata).join(APP_DIR_NAME).join(SETTINGS_FILENAME)))
}

pub fn init_settings_dir() {
    if let Some(path) = get_settings_path() && let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}