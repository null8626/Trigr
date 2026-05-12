use crate::trigger::Trigger;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, sync::{Arc, RwLock}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub triggers: Vec<Trigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageFile {
    pub installed: Vec<String>,
}

pub struct PackageManager {
    file_path: PathBuf,
    packages_dir: PathBuf,
    state: Arc<RwLock<PackageFile>>,
}

impl Clone for PackageManager {
    fn clone(&self) -> Self {
        Self {
            file_path: self.file_path.clone(),
            packages_dir: self.packages_dir.clone(),
            state: self.state.clone(),
        }
    }
}

impl PackageManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let file_path = data_dir.join("packages.json");
        let packages_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/packages");
        let state = Self::load_or_create(&file_path);
        Self {
            file_path,
            packages_dir,
            state: Arc::new(RwLock::new(state)),
        }
    }

    fn load_or_create(path: &PathBuf) -> PackageFile {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(pf) = serde_json::from_str(&content) {
                    return pf;
                }
            }
        }
        PackageFile {
            installed: Vec::new(),
        }
    }

    fn save(&self) -> Result<(), String> {
        let state = self.state.read().map_err(|e| e.to_string())?;
        let content = serde_json::to_string_pretty(&*state).map_err(|e| e.to_string())?;
        fs::write(&self.file_path, content).map_err(|e| e.to_string())
    }

    pub fn get_available_packages(&self) -> Vec<Package> {
        if !self.packages_dir.exists() {
            return Vec::new();
        }
        let mut packages = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.packages_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let meta_path = path.join("meta.json");
                let triggers_path = path.join("triggers.json");
                if !meta_path.exists() || !triggers_path.exists() {
                    continue;
                }
                if let (Ok(meta_content), Ok(triggers_content)) = (
                    fs::read_to_string(&meta_path),
                    fs::read_to_string(&triggers_path),
                ) {
                    if let Ok(meta) = serde_json::from_str::<PackageMeta>(&meta_content) {
                        if let Ok(trigger_map) =
                            serde_json::from_str::<HashMap<String, String>>(&triggers_content)
                        {
                            let triggers = trigger_map
                                .into_iter()
                                .map(|(trigger, replacement)| {
                                    Self::make_trigger(&meta.id, trigger, replacement)
                                })
                                .collect();
                            packages.push(Package {
                                id: meta.id,
                                name: meta.name,
                                description: meta.description,
                                version: meta.version,
                                triggers,
                            });
                        }
                    }
                }
            }
        }
        packages
    }

    fn make_trigger(category: &str, trigger_text: String, replacement: String) -> Trigger {
        let now = chrono::Utc::now().to_rfc3339();
        let id = trigger_text.replace(":", "").to_lowercase();
        Trigger {
            id: format!("pkg:{id}"),
            trigger_text,
            replacement,
            enabled: true,
            category: category.to_string(),
            args_mode: false,
            vars: vec![],
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn get_installed_packages(&self) -> Vec<String> {
        self.state.read().unwrap().installed.clone()
    }

    pub fn install_package(&self, id: String) -> Result<(), String> {
        {
            let mut state = self.state.write().map_err(|e| e.to_string())?;
            if !self.get_available_packages().iter().any(|p| p.id == id) {
                return Err("Package not found".to_string());
            }
            if state.installed.contains(&id) {
                return Err("Package already installed".to_string());
            }
            state.installed.push(id);
        }
        self.save()
    }

    pub fn uninstall_package(&self, id: String) -> Result<(), String> {
        {
            let mut state = self.state.write().map_err(|e| e.to_string())?;
            if !state.installed.contains(&id) {
                return Err("Package not installed".to_string());
            }
            state.installed.retain(|p| p != &id);
        }
        self.save()
    }

    pub fn get_package_triggers(&self) -> Vec<Trigger> {
        let state = self.state.read().unwrap();
        let installed_ids: Vec<String> = state.installed.clone();
        drop(state);

        let mut triggers = Vec::new();
        for pkg in self.get_available_packages() {
            if installed_ids.contains(&pkg.id) {
                triggers.extend(pkg.triggers);
            }
        }
        triggers
    }
}
