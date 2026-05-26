mod listener;
mod package;
mod script;
mod settings;
mod trigger;

use package::PackageManager;
use std::{ffi::CString, fs, path::PathBuf, ptr::null_mut, sync::{Arc, Mutex}};
use tauri::{menu::Menu, tray::TrayIconBuilder, Manager, State};
use trigger::{GlobalVar, TriggerManager, TriggerVar};
use windows_sys::Win32::{Foundation::ERROR_SUCCESS, System::Registry::{HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegCreateKeyExA, RegSetValueExA}};

const SETTINGS_FILENAME: &str = "settings.json";
const PACKAGES_RELATIVE_DIR: &str = "src/packages";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub ender_char: String,
    pub theme_color: String,
    pub font_size: u32,
    pub language: String,
}

impl AppSettings {
    fn load(path: &PathBuf) -> Self {
        if let Ok(Ok(s)) = fs::read_to_string(path).map(|content| serde_json::from_str(&content)) {
            s
        } else {
            Self::default()
        }
    }

    fn save(&self, path: &PathBuf) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ender_char: "!".to_string(),
            theme_color: "#8b5cf6".to_string(),
            font_size: 14,
            language: "en".to_string(),
        }
    }
}

fn enable_autostart() {
    let Ok(exe) = std::env::current_exe() else { return };
    let exe_str = exe.to_string_lossy();
    let Ok(exe_cstr) = CString::new(&*exe_str) else { return };

    let mut key = null_mut();

    unsafe {
        if RegCreateKeyExA(HKEY_CURRENT_USER, cr"Software\Microsoft\Windows\CurrentVersion\Run".as_ptr() as _, 0, 0 as _, 0, KEY_SET_VALUE, 0 as _, &mut key as _, 0 as _) == ERROR_SUCCESS {
            RegSetValueExA(key, c"trigr".as_ptr() as _, 0, REG_SZ, exe_cstr.as_ptr() as _, (exe_cstr.count_bytes() + 1) as _);
            RegCloseKey(key);
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ExportData {
    version: String,
    triggers: Vec<TriggerExport>,
    global_vars: Vec<GlobalVar>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TriggerExport {
    id: String,
    trigger_text: String,
    replacement: String,
    enabled: bool,
    category: String,
    args_mode: bool,
    vars: Vec<TriggerVar>,
    created_at: String,
    updated_at: String,
}

impl From<trigger::Trigger> for TriggerExport {
    fn from(t: trigger::Trigger) -> Self {
        Self {
            id: t.id,
            trigger_text: t.trigger_text,
            replacement: t.replacement,
            enabled: t.enabled,
            category: t.category,
            args_mode: t.args_mode,
            vars: t.vars,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

#[derive(serde::Serialize)]
struct AllData {
    triggers: Vec<trigger::Trigger>,
    global_vars: Vec<GlobalVar>,
}

#[tauri::command]
fn get_all_data(manager: State<Mutex<TriggerManager>>) -> AllData {
    let m = manager.lock().unwrap();
    AllData {
        triggers: m.get_triggers(),
        global_vars: m.get_global_vars(),
    }
}

#[tauri::command]
fn get_triggers(manager: State<Mutex<TriggerManager>>) -> Vec<trigger::Trigger> {
    manager.lock().unwrap().get_triggers()
}

#[tauri::command]
fn add_trigger(
    manager: State<Mutex<TriggerManager>>,
    trigger_text: String,
    replacement: String,
    category: String,
    args_mode: bool,
    vars: Vec<TriggerVar>,
) -> Result<trigger::Trigger, String> {
    manager.lock().unwrap().add_trigger(
        trigger_text,
        replacement,
        category,
        args_mode,
        vars,
    )
}

#[tauri::command]
fn update_trigger(
    manager: State<Mutex<TriggerManager>>,
    id: String,
    trigger_text: Option<String>,
    replacement: Option<String>,
    category: Option<String>,
    args_mode: Option<bool>,
    enabled: Option<bool>,
    vars: Option<Vec<TriggerVar>>,
) -> Result<trigger::Trigger, String> {
    manager.lock().unwrap().update_trigger(
        id,
        trigger_text,
        replacement,
        category,
        args_mode,
        enabled,
        vars,
    )
}

#[tauri::command]
fn delete_item(manager: State<Mutex<TriggerManager>>, item_type: String, id: String) -> Result<(), String> {
    let m = manager.lock().unwrap();
    match item_type.as_str() {
        "trigger" => m.delete_trigger(id),
        "global_var" => m.delete_global_var(id),
        _ => Err(format!("Unknown item type: {item_type}")),
    }
}

#[tauri::command]
fn get_global_vars(manager: State<Mutex<TriggerManager>>) -> Vec<GlobalVar> {
    manager.lock().unwrap().get_global_vars()
}

#[tauri::command]
fn add_global_var(
    manager: State<Mutex<TriggerManager>>,
    name: String,
    script: String,
) -> Result<GlobalVar, String> {
    manager.lock().unwrap().add_global_var(name, script)
}

#[tauri::command]
fn update_global_var(
    manager: State<Mutex<TriggerManager>>,
    id: String,
    name: Option<String>,
    script: Option<String>,
    enabled: Option<bool>,
) -> Result<GlobalVar, String> {
    manager
        .lock()
        .unwrap()
        .update_global_var(id, name, script, enabled)
}

#[tauri::command]
fn preview_replacement(
    manager: State<Mutex<TriggerManager>>,
    trigger_text: String,
    replacement: String,
    vars: Vec<TriggerVar>,
) -> String {
    let now = chrono::Utc::now().to_rfc3339();
    let temp_trigger = trigger::Trigger {
        id: String::new(),
        trigger_text,
        replacement,
        enabled: true,
        category: String::new(),
        args_mode: false,
        vars,
        created_at: now.clone(),
        updated_at: now,
    };
    manager.lock().unwrap().resolve_replacement(&temp_trigger)
}

#[tauri::command]
fn evaluate_script(source: String, context: std::collections::HashMap<String, String>) -> String {
    match script::evaluate(&source, &context) {
        Ok(result) => result,
        Err(e) => format!("Error: {e}"),
    }
}

#[tauri::command]
fn preview_script(source: String, args: Vec<String>) -> String {
    let context = std::collections::HashMap::new();
    match script::evaluate_with_args(&source, &context, &args) {
        Ok(result) => result,
        Err(e) => format!("Error: {e}"),
    }
}

#[tauri::command]
fn export_data(manager: State<Mutex<TriggerManager>>) -> Result<String, String> {
    let m = manager.lock().unwrap();
    let triggers: Vec<TriggerExport> = m
        .get_triggers()
        .into_iter()
        .map(TriggerExport::from)
        .collect();
    let data = ExportData {
        version: "1".to_string(),
        triggers,
        global_vars: m.get_global_vars(),
    };
    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_data_to_file(manager: State<Mutex<TriggerManager>>, path: String) -> Result<(), String> {
    let json = export_data(manager)?;
    let path = PathBuf::from(&path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn import_data(manager: State<Mutex<TriggerManager>>, json: String) -> Result<String, String> {
    let data: ExportData =
        serde_json::from_str(&json).map_err(|e| format!("Invalid format: {e}"))?;
    let m = manager.lock().unwrap();

    let trigger_count = data.triggers.len();
    for t in data.triggers {
        let _ = m.add_trigger(
            t.trigger_text,
            t.replacement,
            t.category,
            t.args_mode,
            t.vars,
        );
    }

    let gv_count = data.global_vars.len();
    for gv in data.global_vars {
        let _ = m.add_global_var(gv.name.clone(), gv.script.clone());
    }

    Ok(format!(
        "Imported {trigger_count} triggers and {gv_count} global variables"
    ))
}

#[tauri::command]
fn show_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn hide_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
fn get_settings(state: State<Arc<Mutex<AppSettings>>>) -> AppSettings {
    state.lock().unwrap().clone()
}

#[tauri::command]
fn update_settings(
    state: State<Arc<Mutex<AppSettings>>>,
    ender_char: String,
    theme_color: String,
    font_size: u32,
    language: String,
) -> Result<AppSettings, String> {
    let mut settings = state.lock().unwrap();
    settings.ender_char = ender_char;
    settings.theme_color = theme_color;
    settings.font_size = font_size;
    settings.language = language;
    if let Some(path) = settings::get_settings_path() {
        let _ = settings.save(&path);
    }
    Ok(settings.clone())
}

#[tauri::command]
fn list_packages(package_mgr: State<Arc<Mutex<PackageManager>>>) -> Vec<package::Package> {
    package_mgr.lock().unwrap().get_available_packages()
}

#[tauri::command]
fn get_installed_packages(package_mgr: State<Arc<Mutex<PackageManager>>>) -> Vec<String> {
    package_mgr.lock().unwrap().get_installed_packages()
}

#[tauri::command]
fn install_package(
    package_mgr: State<Arc<Mutex<PackageManager>>>,
    id: String,
) -> Result<(), String> {
    package_mgr.lock().unwrap().install_package(id)
}

#[tauri::command]
fn uninstall_package(
    package_mgr: State<Arc<Mutex<PackageManager>>>,
    id: String,
) -> Result<(), String> {
    package_mgr.lock().unwrap().uninstall_package(id)
}

#[tauri::command]
fn update_tray_icon(app: tauri::AppHandle, _theme_color: String) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_icon(Some(tauri::include_image!("icons/64x64.png")));
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().unwrap();
            let manager = TriggerManager::new(data_dir.clone());
            app.manage(Mutex::new(manager.clone()));

            let packages_dir = {
                let bundled = app.path().resource_dir().unwrap().join(PACKAGES_RELATIVE_DIR);
                if bundled.exists() {
                    bundled
                } else {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PACKAGES_RELATIVE_DIR)
                }
            };
            let package_mgr = PackageManager::new(data_dir.clone(), packages_dir);
            app.manage(Arc::new(Mutex::new(package_mgr.clone())));

            let settings = AppSettings::load(
                &settings::get_settings_path().unwrap_or_else(|| data_dir.join(SETTINGS_FILENAME)),
            );
            app.manage(Arc::new(Mutex::new(settings.clone())));
            settings::init_settings_dir();

            listener::start_listener(
                manager,
                package_mgr,
                settings.ender_char.chars().next().unwrap_or('!'),
            );

            enable_autostart();

            let menu = Menu::with_items(
                app,
                &[
                    &tauri::menu::MenuItem::with_id(app, "show", "Show", true, None::<&str>)
                        .unwrap(),
                    &tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)
                        .unwrap(),
                ],
            )
            .unwrap();

            let _ = TrayIconBuilder::new()
                .icon(tauri::include_image!("icons/64x64.png"))
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        std::process::exit(0);
                    }
                    _ => {}
                })
                .build(app);

            let window = app.get_webview_window("main").unwrap();
            let launched_at_startup = std::env::args().any(|a| a == "--autostart");
            if launched_at_startup {
                window.hide().unwrap();
            } else {
                window.show().unwrap();
            }

            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_data,
            get_triggers,
            add_trigger,
            update_trigger,
            delete_item,
            get_global_vars,
            add_global_var,
            update_global_var,
            preview_replacement,
            evaluate_script,
            preview_script,
            export_data,
            export_data_to_file,
            import_data,
            show_window,
            hide_window,
            get_settings,
            update_settings,
            list_packages,
            get_installed_packages,
            install_package,
            uninstall_package,
            update_tray_icon
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
