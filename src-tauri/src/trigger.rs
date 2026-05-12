use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, sync::{Arc, RwLock}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub id: String,
    pub trigger_text: String,
    pub replacement: String,
    pub enabled: bool,
    pub category: String,
    pub args_mode: bool,
    pub vars: Vec<TriggerVar>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerVar {
    pub name: String,
    pub script: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalVar {
    pub id: String,
    pub name: String,
    pub script: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TriggerFileData {
    triggers: Vec<Trigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GlobalVarFileData {
    global_vars: Vec<GlobalVar>,
}

#[derive(Clone)]
pub struct TriggerManager {
    trigger_file_path: PathBuf,
    global_var_file_path: PathBuf,
    triggers: Arc<RwLock<Vec<Trigger>>>,
    global_vars: Arc<RwLock<Vec<GlobalVar>>>,
}

impl TriggerManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let trigger_file_path = data_dir.join("triggers.json");
        let global_var_file_path = data_dir.join("global_vars.json");
        let triggers = Self::load_triggers(&trigger_file_path);
        let global_vars = Self::load_global_vars(&global_var_file_path);
        Self {
            trigger_file_path,
            global_var_file_path,
            triggers: Arc::new(RwLock::new(triggers)),
            global_vars: Arc::new(RwLock::new(global_vars)),
        }
    }

    fn load_triggers(file_path: &PathBuf) -> Vec<Trigger> {
        if file_path.exists() {
            match fs::read_to_string(file_path) {
                Ok(content) => match serde_json::from_str::<TriggerFileData>(&content) {
                    Ok(data) => data.triggers,
                    Err(_) => Vec::new(),
                },
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    fn load_global_vars(file_path: &PathBuf) -> Vec<GlobalVar> {
        if file_path.exists() {
            match fs::read_to_string(file_path) {
                Ok(content) => match serde_json::from_str::<GlobalVarFileData>(&content) {
                    Ok(data) => data.global_vars,
                    Err(_) => Vec::new(),
                },
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    fn save_triggers(&self) -> Result<(), String> {
        let triggers = self.triggers.read().map_err(|e| e.to_string())?;
        let data = TriggerFileData {
            triggers: triggers.clone(),
        };
        let content = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        if let Some(parent) = self.trigger_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&self.trigger_file_path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_global_vars(&self) -> Result<(), String> {
        let global_vars = self.global_vars.read().map_err(|e| e.to_string())?;
        let data = GlobalVarFileData {
            global_vars: global_vars.clone(),
        };
        let content = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        if let Some(parent) = self.global_var_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&self.global_var_file_path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_triggers(&self) -> Vec<Trigger> {
        self.triggers.read().unwrap().clone()
    }

    pub fn add_trigger(
        &self,
        trigger_text: String,
        replacement: String,
        category: String,
        args_mode: bool,
        vars: Vec<TriggerVar>,
    ) -> Result<Trigger, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let trigger = Trigger {
            id: uuid::Uuid::new_v4().to_string(),
            trigger_text,
            replacement,
            enabled: true,
            category,
            args_mode,
            vars,
            created_at: now.clone(),
            updated_at: now,
        };
        {
            let mut triggers = self.triggers.write().map_err(|e| e.to_string())?;
            triggers.push(trigger.clone());
        }
        self.save_triggers()?;
        Ok(trigger)
    }

    pub fn update_trigger(
        &self,
        id: String,
        trigger_text: Option<String>,
        replacement: Option<String>,
        category: Option<String>,
        args_mode: Option<bool>,
        enabled: Option<bool>,
        vars: Option<Vec<TriggerVar>>,
    ) -> Result<Trigger, String> {
        {
            let mut triggers = self.triggers.write().map_err(|e| e.to_string())?;
            let trigger = triggers
                .iter_mut()
                .find(|t| t.id == id)
                .ok_or_else(|| "Trigger not found".to_string())?;
            if let Some(text) = trigger_text {
                trigger.trigger_text = text;
            }
            if let Some(repl) = replacement {
                trigger.replacement = repl;
            }
            if let Some(cat) = category {
                trigger.category = cat;
            }
            if let Some(am) = args_mode {
                trigger.args_mode = am;
            }
            if let Some(en) = enabled {
                trigger.enabled = en;
            }
            if let Some(v) = vars {
                trigger.vars = v;
            }
            trigger.updated_at = chrono::Utc::now().to_rfc3339();
            Ok(trigger.clone())
        }
        .and_then(|trigger| {
            self.save_triggers()?;
            Ok(trigger)
        })
    }

    pub fn delete_trigger(&self, id: String) -> Result<(), String> {
        {
            let mut triggers = self.triggers.write().map_err(|e| e.to_string())?;
            let len_before = triggers.len();
            triggers.retain(|t| t.id != id);
            if triggers.len() == len_before {
                return Err("Trigger not found".to_string());
            }
        }
        self.save_triggers()
    }

    pub fn get_enabled_triggers(&self) -> Vec<Trigger> {
        self.triggers
            .read()
            .unwrap()
            .iter()
            .filter(|t| t.enabled)
            .cloned()
            .collect()
    }

    pub fn get_global_vars(&self) -> Vec<GlobalVar> {
        self.global_vars.read().unwrap().clone()
    }

    pub fn add_global_var(&self, name: String, script: String) -> Result<GlobalVar, String> {
        let global_var = GlobalVar {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            script,
            enabled: true,
        };
        {
            let mut global_vars = self.global_vars.write().map_err(|e| e.to_string())?;
            global_vars.push(global_var.clone());
        }
        self.save_global_vars()?;
        Ok(global_var)
    }

    pub fn update_global_var(
        &self,
        id: String,
        name: Option<String>,
        script: Option<String>,
        enabled: Option<bool>,
    ) -> Result<GlobalVar, String> {
        {
            let mut global_vars = self.global_vars.write().map_err(|e| e.to_string())?;
            let gv = global_vars
                .iter_mut()
                .find(|g| g.id == id)
                .ok_or_else(|| "Global variable not found".to_string())?;
            if let Some(n) = name {
                gv.name = n;
            }
            if let Some(s) = script {
                gv.script = s;
            }
            if let Some(en) = enabled {
                gv.enabled = en;
            }
            Ok(gv.clone())
        }
        .and_then(|gv| {
            self.save_global_vars()?;
            Ok(gv)
        })
    }

    pub fn delete_global_var(&self, id: String) -> Result<(), String> {
        {
            let mut global_vars = self.global_vars.write().map_err(|e| e.to_string())?;
            let len_before = global_vars.len();
            global_vars.retain(|g| g.id != id);
            if global_vars.len() == len_before {
                return Err("Global variable not found".to_string());
            }
        }
        self.save_global_vars()
    }

    pub fn get_enabled_global_vars(&self) -> Vec<GlobalVar> {
        self.global_vars
            .read()
            .unwrap()
            .iter()
            .filter(|g| g.enabled)
            .cloned()
            .collect()
    }

    pub fn resolve_replacement(&self, trigger: &Trigger) -> String {
        self.resolve_replacement_with_args(trigger, &[])
    }

    pub fn resolve_replacement_with_args(&self, trigger: &Trigger, args: &[String]) -> String {
        let mut result = trigger.replacement.clone();
        let mut var_values: HashMap<String, String> = HashMap::new();

        let global_vars = self.get_enabled_global_vars();
        for gv in &global_vars {
            let value = evaluate_script_with_args(&gv.script, &var_values, args);
            var_values.insert(gv.name.clone(), value);
        }

        for tv in &trigger.vars {
            let value = evaluate_script_with_args(&tv.script, &var_values, args);
            var_values.insert(tv.name.clone(), value);
        }

        for (name, value) in &var_values {
            let placeholder = format!("{{{{{name}}}}}");
            result = result.replace(&placeholder, value);
        }

        result = resolve_builtin_vars(&result);

        result = resolve_qlang_expressions(&result, &var_values, args);

        result
    }
}

fn resolve_qlang_expressions(
    text: &str,
    context: &HashMap<String, String>,
    args: &[String],
) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    let mut outside = String::new();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next();
            if !outside.is_empty() {
                result.push_str(&outside);
                outside.clear();
            }
            let mut expr = String::new();
            let mut found_close = false;
            loop {
                if let Some(&ch) = chars.peek() {
                    if ch == '}' {
                        chars.next();
                        if chars.peek() == Some(&'}') {
                            chars.next();
                            found_close = true;
                            break;
                        } else {
                            expr.push('}');
                        }
                    } else {
                        expr.push(ch);
                        chars.next();
                    }
                } else {
                    break;
                }
            }
            if found_close {
                let trimmed = expr.trim();
                let val = evaluate_script_with_args(trimmed, context, args);
                if val == "{{script error}}" {
                    result.push_str(&format!("{{{{{expr}}}}}"));
                } else {
                    result.push_str(&val);
                }
            } else {
                result.push_str("{{");
                result.push_str(&expr);
            }
        } else {
            outside.push(ch);
        }
    }

    if !outside.is_empty() {
        result.push_str(&outside);
    }

    result
}

fn evaluate_script_with_args(
    script: &str,
    context: &HashMap<String, String>,
    args: &[String],
) -> String {
    let mut ctx = context.clone();

    for (i, arg) in args.iter().enumerate() {
        ctx.insert(format!("_arg_{i}"), arg.clone());
    }
    ctx.insert("_args_len".to_string(), args.len().to_string());

    match crate::script::evaluate_with_args(script, &ctx, args) {
        Ok(val) => val,
        Err(_) => "{{script error}}".to_string(),
    }
}

fn resolve_builtin_vars(text: &str) -> String {
    let now = chrono::Local::now();
    let mut result = text.to_string();

    let date_replacements: Vec<(String, String)> = vec![
        ("{{date}}".into(), now.format("%Y-%m-%d").to_string()),
        ("{{time}}".into(), now.format("%H:%M:%S").to_string()),
        (
            "{{datetime}}".into(),
            now.format("%Y-%m-%d %H:%M:%S").to_string(),
        ),
        ("{{year}}".into(), now.format("%Y").to_string()),
        ("{{month}}".into(), now.format("%m").to_string()),
        ("{{day}}".into(), now.format("%d").to_string()),
        ("{{hour}}".into(), now.format("%H").to_string()),
        ("{{minute}}".into(), now.format("%M").to_string()),
        ("{{second}}".into(), now.format("%S").to_string()),
        ("{{weekday}}".into(), now.format("%A").to_string()),
        ("{{shortdate}}".into(), now.format("%m/%d/%Y").to_string()),
    ];

    for (placeholder, value) in &date_replacements {
        result = result.replace(placeholder, value);
    }

    result
}
