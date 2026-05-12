use crate::{package::PackageManager, trigger::TriggerManager};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use rdev::{listen, EventType, Key as RdevKey};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex};

static EXPANDING: AtomicBool = AtomicBool::new(false);

pub fn start_listener(manager: TriggerManager, package_mgr: PackageManager, ender_char: char) {
    let buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let ctrl_pressed = Arc::new(AtomicBool::new(false));
    let alt_pressed = Arc::new(AtomicBool::new(false));

    std::thread::spawn(move || {
        let _ = listen(move |event| {
            if EXPANDING.load(Ordering::SeqCst) {
                return;
            }

            match event.event_type {
                EventType::KeyPress(key) => match key {
                    RdevKey::ControlLeft | RdevKey::ControlRight => {
                        ctrl_pressed.store(true, Ordering::SeqCst);
                    }
                    RdevKey::Alt | RdevKey::AltGr => {
                        alt_pressed.store(true, Ordering::SeqCst);
                    }
                    RdevKey::Backspace => {
                        if ctrl_pressed.load(Ordering::SeqCst) {
                            let mut buf = buffer.lock().unwrap();
                            strip_last_word(&mut buf);
                        } else {
                            buffer.lock().unwrap().pop();
                        }
                    }
                    RdevKey::KeyA => {
                        if ctrl_pressed.load(Ordering::SeqCst) {
                            buffer.lock().unwrap().clear();
                        } else {
                            handle_char(
                                &event,
                                &buffer,
                                &manager,
                                &package_mgr,
                                &ctrl_pressed,
                                &alt_pressed,
                                ender_char,
                            );
                        }
                    }
                    RdevKey::Escape => {
                        buffer.lock().unwrap().clear();
                    }
                    RdevKey::Space | RdevKey::Return | RdevKey::Tab => {
                        if !ctrl_pressed.load(Ordering::SeqCst)
                            && !alt_pressed.load(Ordering::SeqCst)
                        {
                            let sep = match key {
                                RdevKey::Space => " ",
                                RdevKey::Return => "\n",
                                RdevKey::Tab => "\t",
                                _ => "",
                            };
                            handle_separator(&buffer, &manager, &package_mgr, sep);
                        }
                    }
                    _ => {
                        handle_char(
                            &event,
                            &buffer,
                            &manager,
                            &package_mgr,
                            &ctrl_pressed,
                            &alt_pressed,
                            ender_char,
                        );
                    }
                },
                EventType::KeyRelease(key) => match key {
                    RdevKey::ControlLeft | RdevKey::ControlRight => {
                        ctrl_pressed.store(false, Ordering::SeqCst);
                    }
                    RdevKey::Alt | RdevKey::AltGr => {
                        alt_pressed.store(false, Ordering::SeqCst);
                    }
                    _ => {}
                },
                _ => {}
            }
        });
    });
}

fn handle_char(
    event: &rdev::Event,
    buffer: &Arc<Mutex<String>>,
    manager: &TriggerManager,
    package_mgr: &PackageManager,
    ctrl_pressed: &Arc<AtomicBool>,
    alt_pressed: &Arc<AtomicBool>,
    ender_char: char,
) {
    if ctrl_pressed.load(Ordering::SeqCst) || alt_pressed.load(Ordering::SeqCst) {
        return;
    }

    let name = match &event.name {
        Some(n) => n,
        None => return,
    };

    if name.len() != 1 {
        return;
    }

    let c = name.chars().next().unwrap();
    if c.is_control() {
        return;
    }

    if c == ender_char {
        let buf = buffer.lock().unwrap().clone();
        if try_expand_with_args(&buf, manager) {
            buffer.lock().unwrap().clear();
        }
        return;
    }

    let mut buf = buffer.lock().unwrap();
    buf.push(c);
    let buf_copy = buf.clone();
    drop(buf);

    if check_and_expand(&buf_copy, manager, package_mgr, false, &[]) {
        buffer.lock().unwrap().clear();
    }
}

fn handle_separator(
    buffer: &Arc<Mutex<String>>,
    manager: &TriggerManager,
    package_mgr: &PackageManager,
    sep: &str,
) {
    buffer.lock().unwrap().push_str(sep);
    let buf = buffer.lock().unwrap().clone();

    if check_and_expand(&buf, manager, package_mgr, true, &[]) {
        buffer.lock().unwrap().clear();
    }
}

fn try_expand_with_args(buffer: &str, manager: &TriggerManager) -> bool {
    if buffer.is_empty() {
        return false;
    }

    let enabled_triggers = manager.get_enabled_triggers();

    for trigger in &enabled_triggers {
        if !trigger.args_mode {
            continue;
        }

        if let Some(trigger_start) = buffer.rfind(&trigger.trigger_text) {
            let trigger_text_end = trigger_start + trigger.trigger_text.len();
            let args_str = &buffer[trigger_text_end..];

            let args: Vec<String> = args_str
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();

            let backspace_count = buffer[trigger_start..].chars().count() + 1;

            let resolved = manager.resolve_replacement_with_args(trigger, &args);

            EXPANDING.store(true, Ordering::SeqCst);
            let resolved_clone = resolved.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(80));
                expand_text(backspace_count, &resolved_clone);
                std::thread::sleep(std::time::Duration::from_millis(100));
                EXPANDING.store(false, Ordering::SeqCst);
            });

            return true;
        }
    }

    false
}

fn check_and_expand(
    buffer: &str,
    manager: &TriggerManager,
    package_mgr: &PackageManager,
    is_separator: bool,
    _args: &[String],
) -> bool {
    if buffer.is_empty() {
        return false;
    }

    let mut enabled_triggers = manager.get_enabled_triggers();
    enabled_triggers.extend(package_mgr.get_package_triggers());

    for trigger in &enabled_triggers {
        if trigger.args_mode {
            continue;
        }

        let buffer_to_check = if is_separator {
            buffer.trim_end_matches([' ', '\n', '\t'].as_slice())
        } else {
            buffer
        };

        if buffer_to_check.ends_with(&trigger.trigger_text) {
            let backspace_count =
                trigger.trigger_text.chars().count() + if is_separator { 1 } else { 0 };

            let resolved = manager.resolve_replacement(trigger);

            EXPANDING.store(true, Ordering::SeqCst);
            let resolved_clone = resolved.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(30));
                expand_text(backspace_count, &resolved_clone);
                std::thread::sleep(std::time::Duration::from_millis(100));
                EXPANDING.store(false, Ordering::SeqCst);
            });

            return true;
        }
    }

    false
}

fn strip_last_word(buf: &mut String) {
    while let Some(c) = buf.pop() {
        if c == ' ' || c == '\t' || c == '\n' || is_punct(c) {
            buf.push(c);
            break;
        }
    }
}

fn is_punct(c: char) -> bool {
    matches!(
        c,
        '.' | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '"'
            | '\''
            | '/'
            | '\\'
            | '|'
            | '@'
            | '#'
            | '$'
            | '%'
            | '^'
            | '&'
            | '*'
            | '+'
            | '='
            | '<'
            | '>'
            | '~'
            | '`'
    )
}

fn expand_text(backspace_count: usize, replacement: &str) {
    let mut enigo = Enigo::new(&Settings::default()).expect("Failed to init enigo");

    for _ in 0..backspace_count {
        std::thread::sleep(std::time::Duration::from_millis(5));
        let _ = enigo.key(Key::Backspace, Direction::Click);
    }

    std::thread::sleep(std::time::Duration::from_millis(30));

    for line in replacement.split('\n') {
        if !line.is_empty() {
            let _ = enigo.text(line);
        }
    }

    let newline_count = replacement.chars().filter(|&c| c == '\n').count();
    for _ in 0..newline_count {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = enigo.key(Key::Return, Direction::Click);
    }
}
