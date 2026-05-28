use super::util::{optional_fill, tauri_reexport};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap, fmt::Write, fs::{self, File}, io::{BufReader, BufWriter}, path::PathBuf, sync::{Arc, RwLock}};
use uuid::Uuid;

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
    pub id: Uuid,
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

const TRIGGERS_FILENAME: &str = "triggers.json";
const GLOBAL_VARS_FILENAME: &str = "global_vars.json";

impl TriggerManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let trigger_file_path = data_dir.join(TRIGGERS_FILENAME);
        let global_var_file_path = data_dir.join(GLOBAL_VARS_FILENAME);
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
        File::open(file_path).map_or(None, |file| serde_json::from_reader::<_, TriggerFileData>(BufReader::new(file)).map_or(None, |data| Some(data.triggers))).unwrap_or_default()
    }

    fn load_global_vars(file_path: &PathBuf) -> Vec<GlobalVar> {
        File::open(file_path).map_or(None, |file| serde_json::from_reader::<_, GlobalVarFileData>(BufReader::new(file)).map_or(None, |data| Some(data.global_vars))).unwrap_or_default()
    }

    fn save_triggers(&self) -> Result<(), Cow<'static, str>> {
        let triggers = self.triggers.read().map_err(|e| e.to_string())?;
        let data = TriggerFileData {
            triggers: triggers.clone(),
        };
        if let Some(parent) = self.trigger_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        serde_json::to_writer_pretty(BufWriter::new(File::open(&self.trigger_file_path).map_err(|e| e.to_string())?), &data).map_err(|e| e.to_string().into())
    }

    fn save_global_vars(&self) -> Result<(), Cow<'static, str>> {
        let global_vars = self.global_vars.read().map_err(|e| e.to_string())?;
        let data = GlobalVarFileData {
            global_vars: global_vars.clone(),
        };
        if let Some(parent) = self.global_var_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        serde_json::to_writer_pretty(BufWriter::new(File::open(&self.global_var_file_path).map_err(|e| e.to_string())?), &data).map_err(|e| e.to_string().into())
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

    pub fn delete_trigger(&self, id: Uuid) -> Result<(), Cow<'static, str>> {
        {
            let mut triggers = self.triggers.write().map_err(|e| e.to_string())?;
            let len_before = triggers.len();
            triggers.retain(|t| t.id.starts_with("pkg") || t.id.parse::<Uuid>().unwrap() != id);
            if triggers.len() == len_before {
                return Err("Trigger not found".into());
            }
        }
        self.save_triggers()
    }

    pub fn delete_global_var(&self, id: Uuid) -> Result<(), Cow<'static, str>> {
        {
            let mut global_vars = self.global_vars.write().map_err(|e| e.to_string())?;
            let len_before = global_vars.len();
            global_vars.retain(|g| g.id != id);
            if global_vars.len() == len_before {
                return Err("Global variable not found".into());
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
        let mut result = trigger.replacement.to_string();
        let mut var_values = HashMap::new();

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

        result = resolve_trill_expressions(&result, &var_values, args);

        result
    }
}

tauri_reexport! {
    impl TriggerManager {
        pub fn get_triggers(self: &Self) -> Vec<Trigger> {
            self.triggers.read().expect("trigger read lock poisoned").clone()
        }

        pub fn add_trigger(
            self: &Self,
            trigger_text: String,
            replacement: String,
            category: String,
            args_mode: bool,
            vars: Vec<TriggerVar>
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
            self: &Self,
            id: String,
            trigger_text: Option<String>,
            replacement: Option<String>,
            category: Option<String>,
            args_mode: Option<bool>,
            enabled: Option<bool>,
            vars: Option<Vec<TriggerVar>>
        ) -> Result<Trigger, String> {
            {
                let mut triggers = self.triggers.write().map_err(|e| e.to_string())?;
                let trigger = triggers
                    .iter_mut()
                    .find(|t| t.id == id)
                    .ok_or_else(|| "Trigger not found".to_string())?;

                optional_fill!(trigger, trigger_text, replacement, category, args_mode, enabled, vars);

                trigger.updated_at = chrono::Utc::now().to_rfc3339();
                Ok(trigger.clone())
            }
            .and_then(|trigger| {
                self.save_triggers()?;
                Ok(trigger)
            })
        }

        pub fn get_global_vars(self: &Self) -> Vec<GlobalVar> {
            self.global_vars.read().unwrap().clone()
        }

        pub fn add_global_var(self: &Self, name: String, script: String) -> Result<GlobalVar, Cow<'static, str>> {
            let global_var = GlobalVar {
                id: uuid::Uuid::new_v4(),
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
            self: &Self,
            id: Uuid,
            name: Option<String>,
            script: Option<String>,
            enabled: Option<bool>
        ) -> Result<GlobalVar, Cow<'static, str>> {
            let mut global_vars = self.global_vars.write().map_err(|e| e.to_string())?;
            let gv = global_vars
                .iter_mut()
                .find(|g| g.id == id)
                .ok_or(Cow::Borrowed("Global variable not found"))?;

            optional_fill!(gv, name, script, enabled);

            let result = Ok(gv.clone());

            self.save_global_vars()?;

            result
        }
    }
}

fn resolve_trill_expressions(
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
            while let Some(&ch) = chars.peek() {
                chars.next();

                if ch == '}' {
                    if chars.peek() == Some(&'}') {
                        chars.next();
                        found_close = true;
                        break;
                    } else {
                        expr.push('}');
                    }
                } else {
                    expr.push(ch);
                }
            }

            if found_close {
                let trimmed = expr.trim();
                let val = evaluate_script_with_args(trimmed, context, args);
                if val == "{{script error}}" {
                    write!(&mut result, "{{{{{expr}}}}}").unwrap();
                } else {
                    result.push_str(&val);
                }
            } else {
                write!(&mut result, "{{{{{expr}").unwrap();
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

const DATE_REPLACEMENTS: [(&str, &str); 11] = [
    ("{{date}}", "%Y-%m-%d"),
    ("{{time}}", "%H:%M:%S"),
    ("{{datetime}}", "%Y-%m-%d %H:%M:%S"),
    ("{{year}}", "%Y"),
    ("{{month}}", "%m"),
    ("{{day}}", "%d"),
    ("{{hour}}", "%H"),
    ("{{minute}}", "%M"),
    ("{{second}}", "%S"),
    ("{{weekday}}", "%A"),
    ("{{shortdate}}", "%m/%d/%Y")
];

fn resolve_builtin_vars(text: &str) -> String {
    let now = chrono::Local::now();
    let mut result = text.to_string();

    for (placeholder, value) in &DATE_REPLACEMENTS {
        result = result.replace(placeholder, &now.format(value).to_string());
    }

    result
}
