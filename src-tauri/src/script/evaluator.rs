use crate::script::ast::*;
pub use crate::script::ast::Value;
use std::collections::HashMap;

#[derive(Default)]
pub struct Evaluator {
    pub env: HashMap<String, Value>,
}

impl Evaluator {
    pub fn evaluate(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Literal(v) => Ok(v.clone()),
            Expr::Var(name) => {
                self.env
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: {name}"))
            }
            Expr::Binary { left, op, right } => self.eval_binary(left, op, right),
            Expr::Unary { op, expr } => self.eval_unary(op, expr),
            Expr::Call { callee, args } => self.eval_call(callee, args),
            Expr::Index { target, index } => self.eval_index(target, index),
            Expr::If { condition, then_branch, else_branch } => {
                if self.evaluate(condition)?.as_bool() {
                    self.evaluate(then_branch)
                } else if let Some(els) = else_branch {
                    self.evaluate(els)
                } else {
                    Ok(Value::Nil)
                }
            }
            Expr::Let { name, value, body } => {
                let v = self.evaluate(value)?;
                self.env.insert(name.clone(), v.clone());
                let result = self.evaluate(body)?;
                self.env.remove(name);
                Ok(result)
            }
            Expr::Fn { params, body } => Ok(Value::Map(HashMap::from([
                ("__fn_params".to_string(), Value::List(params.iter().map(|p| Value::Str(p.clone())).collect())),
                ("__fn_body".to_string(), Value::Str(format!("{body:?}"))),
            ]))),
            Expr::Pipe { left, right } => {
                let left_val = self.evaluate(left)?;
                if let Expr::Call { callee, args } = right.as_ref() {
                    let mut new_args = vec![left_val];
                    for a in args {
                        new_args.push(self.evaluate(a)?);
                    }
                    if let Expr::Var(name) = callee.as_ref() {
                        self.eval_call_with_values(name, &new_args)
                    } else {
                        Err("Can only pipe into functions".to_string())
                    }
                } else {
                    self.env.insert("__pipe_input".to_string(), left_val);
                    let result = self.evaluate(right)?;
                    self.env.remove("__pipe_input");
                    Ok(result)
                }
            }
            Expr::Block(exprs) => {
                let mut last = Value::Nil;
                for e in exprs {
                    last = self.evaluate(e)?;
                }
                Ok(last)
            }
            Expr::List(items) => {
                let values: Result<Vec<Value>, String> = items.iter().map(|e| self.evaluate(e)).collect();
                Ok(Value::List(values?))
            }
            Expr::ForLoop { .. } => Ok(Value::Nil),
        }
    }

    fn eval_binary(&mut self, left: &Expr, op: &BinaryOp, right: &Expr) -> Result<Value, String> {
        let l = self.evaluate(left)?;
        let r = self.evaluate(right)?;

        match op {
            BinaryOp::Add => {
                match (&l, &r) {
                    (Value::Num(a), Value::Num(b)) => Ok(Value::Num(a + b)),

                    (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),

                    (Value::Num(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}"))),

                    (Value::Str(a), _) => Ok(Value::Str(format!("{a}{}", r.to_string()))),

                    (_, Value::Str(b)) => Ok(Value::Str(format!("{}{b}", l.to_string()))),

                    _ => Err(format!("Cannot add {l:?} and {r:?}")),
                }
            }
            BinaryOp::Sub => {
                let a = l.as_num().ok_or("Left side must be a number")?;
                let b = r.as_num().ok_or("Right side must be a number")?;
                Ok(Value::Num(a - b))
            }
            BinaryOp::Mul => {
                let a = l.as_num().ok_or("Left side must be a number")?;
                let b = r.as_num().ok_or("Right side must be a number")?;
                Ok(Value::Num(a * b))
            }
            BinaryOp::Div => {
                let a = l.as_num().ok_or("Left side must be a number")?;
                let b = r.as_num().ok_or("Right side must be a number")?;
                if b == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(Value::Num(a / b))
                }
            }
            BinaryOp::Mod => {
                let a = l.as_num().ok_or("Left side must be a number")?;
                let b = r.as_num().ok_or("Right side must be a number")?;
                Ok(Value::Num(a % b))
            }
            BinaryOp::Eq => Ok(Value::Bool(self.values_equal(&l, &r))),
            BinaryOp::Ne => Ok(Value::Bool(!self.values_equal(&l, &r))),
            BinaryOp::Lt => {
                Ok(Value::Bool(if let (Value::Num(a), Value::Num(b)) = (&l, &r) {
                    a < b
                } else {
                    l.to_string() < r.to_string()
                }))
            }
            BinaryOp::Gt => {
                Ok(Value::Bool(if let (Value::Num(a), Value::Num(b)) = (&l, &r) {
                    a > b
                } else {
                    l.to_string() > r.to_string()
                }))
            }
            BinaryOp::Le => {
                Ok(Value::Bool(if let (Value::Num(a), Value::Num(b)) = (&l, &r) {
                    a <= b
                } else {
                    l.to_string() <= r.to_string()
                }))
            }
            BinaryOp::Ge => {
                Ok(Value::Bool(if let (Value::Num(a), Value::Num(b)) = (&l, &r) {
                    a >= b
                } else {
                    l.to_string() >= r.to_string()
                }))
            }
        }
    }

    fn eval_unary(&mut self, op: &UnaryOp, expr: &Expr) -> Result<Value, String> {
        let v = self.evaluate(expr)?;
        match op {
            UnaryOp::Not => Ok(Value::Bool(!v.as_bool())),
            UnaryOp::Neg => {
                let n = v.as_num().ok_or("Cannot negate non-number")?;
                Ok(Value::Num(-n))
            }
        }
    }

    fn eval_call(&mut self, callee: &Expr, args: &[Expr]) -> Result<Value, String> {
        let mut arg_values = vec![];
        for a in args {
            arg_values.push(self.evaluate(a)?);
        }

        if let Expr::Call { callee: inner_callee, args: inner_args } = callee && let Expr::Var(inner_name) = inner_callee.as_ref() && inner_name == "random" && inner_args.len() == 1 && let Expr::Var(q) = &inner_args[0] && q == "q" {
            return self.call_builtin("q_random", &arg_values);
        }

        let func_name = match callee {
            Expr::Var(name) => name.clone(),
            _ => return Err("Can only call functions and builtins".to_string()),
        };

        self.call_builtin(&func_name, &arg_values)
    }

    fn eval_call_with_values(&mut self, func_name: &str, arg_values: &[Value]) -> Result<Value, String> {
        self.call_builtin(func_name, arg_values)
    }

    fn eval_index(&mut self, target: &Expr, index: &Expr) -> Result<Value, String> {
        let t = self.evaluate(target)?;
        let i = self.evaluate(index)?;

        match (t, i) {
            (Value::List(items), Value::Num(n)) => {
                let idx = if n < 0.0 {
                    (items.len() as f64 + n) as usize
                } else {
                    n as usize
                };
                items.get(idx).cloned().ok_or_else(|| format!("Index {n} out of bounds"))
            }
            (Value::Str(s), Value::Num(n)) => {
                let chars: Vec<char> = s.chars().collect();
                let idx = if n < 0.0 {
                    (chars.len() as f64 + n) as usize
                } else {
                    n as usize
                };
                chars.get(idx).map(|c| Value::Str(c.to_string())).ok_or_else(|| format!("Index {n} out of bounds"))
            }
            (Value::Map(map), Value::Str(key)) => {
                map.get(&key).cloned().ok_or_else(|| format!("Key '{key}' not found"))
            }
            _ => Err("Cannot index this type".to_string()),
        }
    }

    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        match name {
            "upper" => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                Ok(Value::Str(s.to_uppercase()))
            }
            "lower" => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                Ok(Value::Str(s.to_lowercase()))
            }
            "trim" => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                Ok(Value::Str(s.trim().to_string()))
            }
            "trim_start" => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                Ok(Value::Str(s.trim_start().to_string()))
            }
            "trim_end" => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                Ok(Value::Str(s.trim_end().to_string()))
            }
            "len" | "length" => {
                let v = args.first().cloned().unwrap_or(Value::Nil);
                match v {
                    Value::List(items) => Ok(Value::Num(items.len() as f64)),
                    Value::Str(s) => Ok(Value::Num(s.chars().count() as f64)),
                    _ => Ok(Value::Num(v.to_string().chars().count() as f64)),
                }
            }
            "repeat" => {
                if args.len() < 2 {
                    return Err("repeat requires 2 arguments".to_string());
                }
                let s = args[0].to_string();
                let n = args[1].as_num().ok_or("Second arg must be a number")? as usize;
                Ok(Value::Str(s.repeat(n)))
            }
            "replace" => {
                if args.len() < 3 {
                    return Err("replace requires 3 arguments".to_string());
                }
                let s = args[0].to_string();
                let from = args[1].to_string();
                let to = args[2].to_string();
                Ok(Value::Str(s.replace(&from, &to)))
            }
            "slice" => {
                if args.len() < 3 {
                    return Err("slice requires 3 arguments: string, start, end".to_string());
                }
                let s = args[0].to_string();
                let start = args[1].as_num().ok_or("Start must be a number")? as usize;
                let end = args[2].as_num().ok_or("End must be a number")? as usize;
                let chars: Vec<char> = s.chars().collect();
                if start > chars.len() || end > chars.len() {
                    return Err("slice indices out of bounds".to_string());
                }
                Ok(Value::Str(chars[start..end].iter().collect()))
            }
            "split" => {
                if args.len() < 2 {
                    return Err("split requires 2 arguments".to_string());
                }
                let s = args[0].to_string();
                let delim = args[1].to_string();
                let parts: Vec<Value> = s.split(&delim).map(|p| Value::Str(p.to_string())).collect();
                Ok(Value::List(parts))
            }
            "join" => {
                if args.is_empty() { return Err("join requires a list".to_string()); }
                let sep = if args.len() > 1 { args[1].to_string() } else { String::new() };
                let parts = match &args[0] {
                    Value::List(items) => items.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(&sep),
                    v => v.to_string(),
                };
                Ok(Value::Str(parts))
            }
            "contains" => {
                if args.len() < 2 {
                    return Err("contains requires 2 arguments".to_string());
                }
                let s = args[0].to_string();
                let sub = args[1].to_string();
                Ok(Value::Bool(s.contains(&sub)))
            }
            "starts_with" => {
                if args.len() < 2 {
                    return Err("starts_with requires 2 arguments".to_string());
                }
                let s = args[0].to_string();
                let prefix = args[1].to_string();
                Ok(Value::Bool(s.starts_with(&prefix)))
            }
            "ends_with" => {
                if args.len() < 2 {
                    return Err("ends_with requires 2 arguments".to_string());
                }
                let s = args[0].to_string();
                let suffix = args[1].to_string();
                Ok(Value::Bool(s.ends_with(&suffix)))
            }
            "substr" => {
                if args.len() < 3 {
                    return Err("substr requires 3 arguments: string, start, length".to_string());
                }
                let s = args[0].to_string();
                let start = args[1].as_num().ok_or("Start must be a number")? as usize;
                let len = args[2].as_num().ok_or("Length must be a number")? as usize;
                let chars: Vec<char> = s.chars().collect();
                if start > chars.len() {
                    return Err("substr start out of bounds".to_string());
                }
                let end = (start + len).min(chars.len());
                Ok(Value::Str(chars[start..end].iter().collect()))
            }
            "reverse" => {
                match args.first() {
                    Some(Value::List(items)) => {
                        let mut rev = items.clone();
                        rev.reverse();
                        Ok(Value::List(rev))
                    }
                    Some(Value::Str(s)) => Ok(Value::Str(s.chars().rev().collect())),
                    _ => Err("reverse requires a list or string".to_string()),
                }
            }
            "pad_start" => {
                if args.len() < 2 {
                    return Err("pad_start requires string and length".to_string());
                }
                let s = args[0].to_string();
                let target = args[1].as_num().ok_or("Length must be a number")? as usize;
                let ch = if args.len() > 2 {
                    args[2].to_string().chars().next().unwrap_or(' ')
                } else {
                    ' '
                };
                let pad_len = target.saturating_sub(s.chars().count());
                let padding: String = std::iter::repeat_n(ch, pad_len).collect();
                Ok(Value::Str(format!("{padding}{s}")))
            }
            "pad_end" => {
                if args.len() < 2 {
                    return Err("pad_end requires string and length".to_string());
                }
                let s = args[0].to_string();
                let target = args[1].as_num().ok_or("Length must be a number")? as usize;
                let ch = if args.len() > 2 {
                    args[2].to_string().chars().next().unwrap_or(' ')
                } else {
                    ' '
                };
                let pad_len = target.saturating_sub(s.chars().count());
                let padding: String = std::iter::repeat_n(ch, pad_len).collect();
                Ok(Value::Str(format!("{s}{padding}")))
            }
            "concat" => {
                let parts: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                Ok(Value::Str(parts.join("")))
            }
            "title" => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                let mut result = String::new();
                let mut upper = true;
                for c in s.chars() {
                    if c.is_whitespace() || c == '-' || c == '_' {
                        upper = true;
                        result.push(c);
                    } else if upper {
                        result.extend(c.to_uppercase());
                        upper = false;
                    } else {
                        result.extend(c.to_lowercase());
                    }
                }
                Ok(Value::Str(result))
            }
            "to_num" | "number" => {
                let s = args.first().map(|v| v.to_string()).unwrap_or_default();
                let n: f64 = s.parse().map_err(|_| format!("Cannot convert '{s}' to number"))?;
                Ok(Value::Num(n))
            }
            "to_str" | "string" => {
                let v = args.first().cloned().unwrap_or(Value::Nil);
                Ok(Value::Str(v.to_string()))
            }
            "floor" => {
                let n = args.first().and_then(|v| v.as_num()).ok_or("floor requires a number")?;
                Ok(Value::Num(n.floor()))
            }
            "ceil" | "ceiling" => {
                let n = args.first().and_then(|v| v.as_num()).ok_or("ceil requires a number")?;
                Ok(Value::Num(n.ceil()))
            }
            "round" => {
                let n = args.first().and_then(|v| v.as_num()).ok_or("round requires a number")?;
                Ok(Value::Num(n.round()))
            }
            "abs" => {
                let n = args.first().and_then(|v| v.as_num()).ok_or("abs requires a number")?;
                Ok(Value::Num(n.abs()))
            }
            "min" => {
                if args.is_empty() {
                    return Err("min requires at least one argument".to_string());
                }
                let mut min = args[0].as_num().ok_or("min requires numbers")?;
                for a in &args[1..] {
                    let n = a.as_num().ok_or("min requires numbers")?;
                    if n < min {
                        min = n;
                    }
                }
                Ok(Value::Num(min))
            }
            "max" => {
                if args.is_empty() {
                    return Err("max requires at least one argument".to_string());
                }
                let mut max = args[0].as_num().ok_or("max requires numbers")?;
                for a in &args[1..] {
                    let n = a.as_num().ok_or("max requires numbers")?;
                    if n > max {
                        max = n;
                    }
                }
                Ok(Value::Num(max))
            }
            "clamp" => {
                if args.len() < 3 {
                    return Err("clamp requires 3 arguments: value, min, max".to_string());
                }
                let v = args[0].as_num().ok_or("clamp requires numbers")?;
                let lo = args[1].as_num().ok_or("clamp requires numbers")?;
                let hi = args[2].as_num().ok_or("clamp requires numbers")?;
                Ok(Value::Num(v.max(lo).min(hi)))
            }
            "list" => {
                Ok(Value::List(args.to_vec()))
            }
            "choice" => {
                match args.first() {
                    Some(Value::List(items)) if !items.is_empty() => {
                        use std::time::SystemTime;
                        let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
                        Ok(items[(seed % items.len() as u64) as usize].clone())
                    }
                    Some(Value::List(_)) => Err("choice requires a non-empty list".to_string()),
                    _ => {
                        let items: Vec<Value> = args.to_vec();
                        if items.is_empty() { return Err("choice requires arguments".to_string()); }
                        use std::time::SystemTime;
                        let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
                        Ok(items[(seed % items.len() as u64) as usize].clone())
                    }
                }
            }
            "rand" | "random" => {
                use std::time::SystemTime;
                let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
                if args.len() == 2 {
                    let lo = args[0].as_num().ok_or("random requires numbers")? as i64;
                    let hi = args[1].as_num().ok_or("random requires numbers")? as i64;
                    let range = (hi - lo + 1) as u64;
                    Ok(Value::Num((lo + ((seed % range) as i64)) as f64))
                } else if args.is_empty() {
                    Ok(Value::Num((seed % 1000) as f64 / 1000.0))
                } else {
                    Err("random takes 0 or 2 arguments".to_string())
                }
            }
            "q_random" => {
                use std::time::SystemTime;
                let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos() as u64;
                if args.is_empty() { return Err("q_random requires arguments".to_string()); }
                Ok(args[(seed % args.len() as u64) as usize].clone())
            }
            "first" => {
                match args.first() {
                    Some(Value::List(items)) => items.first().cloned().ok_or_else(|| "Empty list".to_string()),
                    Some(Value::Str(s)) => s.chars().next().map(|c| Value::Str(c.to_string())).ok_or_else(|| "Empty string".to_string()),
                    _ => Err("first requires a list or string".to_string()),
                }
            }
            "last" => {
                match args.first() {
                    Some(Value::List(items)) => items.last().cloned().ok_or_else(|| "Empty list".to_string()),
                    Some(Value::Str(s)) => s.chars().last().map(|c| Value::Str(c.to_string())).ok_or_else(|| "Empty string".to_string()),
                    _ => Err("last requires a list or string".to_string()),
                }
            }
            "map" => {
                if args.len() < 2 {
                    return Err("map requires a list and a function name".to_string());
                }
                let items = match &args[0] {
                    Value::List(items) => items.clone(),
                    v => vec![v.clone()],
                };
                let fn_name = args[1].to_string();
                let mut results = vec![];
                for item in &items {
                    self.env.insert("__item".to_string(), item.clone());
                    let result = self.call_builtin(&fn_name, std::slice::from_ref(item))?;
                    results.push(result);
                    self.env.remove("__item");
                }
                Ok(Value::List(results))
            }
            "filter" => {
                if args.len() < 2 {
                    return Err("filter requires a list and a condition".to_string());
                }
                let items = match &args[0] {
                    Value::List(items) => items.clone(),
                    v => vec![v.clone()],
                };
                let cond_fn = args[1].to_string();
                let mut results = vec![];
                for item in &items {
                    self.env.insert("__item".to_string(), item.clone());
                    let cond = self.call_builtin(&cond_fn, std::slice::from_ref(item))?;
                    if cond.as_bool() {
                        results.push(item.clone());
                    }
                    self.env.remove("__item");
                }
                Ok(Value::List(results))
            }
            "sort" => {
                let mut items = match args.first() {
                    Some(Value::List(items)) => items.clone(),
                    _ => return Err("sort requires a list".to_string()),
                };
                items.sort_by_key(|a| a.to_string());
                Ok(Value::List(items))
            }
            "join_list" => {
                if args.len() < 2 {
                    return Err("join_list requires a list and separator".to_string());
                }
                let items = match &args[0] {
                    Value::List(items) => items,
                    _ => return Err("join_list requires a list".to_string()),
                };
                let sep = args[1].to_string();
                let result = items.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(&sep);
                Ok(Value::Str(result))
            }
            "now" => {
                let format = args.first().map(|v| v.to_string()).unwrap_or_else(|| "%Y-%m-%d %H:%M:%S".to_string());
                Ok(Value::Str(chrono::Local::now().format(&format).to_string()))
            }
            "today" => {
                let format = args.first().map(|v| v.to_string()).unwrap_or_else(|| "%Y-%m-%d".to_string());
                Ok(Value::Str(chrono::Local::now().format(&format).to_string()))
            }
            "date_add" => {
                if args.len() < 2 {
                    return Err("date_add requires date string and days".to_string());
                }
                let s = args[0].to_string();
                let days = args[1].as_num().ok_or("Days must be a number")? as i64;

                Ok(Value::Str(if let Ok(dt) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                    let new_date = dt + chrono::Duration::days(days);
                    new_date.format("%Y-%m-%d").to_string()
                } else {
                    let dt = chrono::Local::now().date_naive();
                    let new_date = dt + chrono::Duration::days(days);
                    new_date.format(&s).to_string()
                }))
            }
            "date_format" => {
                if args.len() < 2 {
                    return Err("date_format requires date and format".to_string());
                }
                let s = args[0].to_string();
                let fmt = args[1].to_string();
                if let Ok(dt) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                    Ok(Value::Str(dt.format(&fmt).to_string()))
                } else {
                    Err(format!("Cannot parse date: {s}"))
                }
            }
            "if_then_else" => {
                if args.len() < 3 {
                    return Err("if_then_else requires 3 arguments".to_string());
                }
                if args[0].as_bool() {
                    Ok(args[1].clone())
                } else {
                    Ok(args[2].clone())
                }
            }
            "__builtin_or" => {
                Ok(Value::Bool(args.iter().any(|v| v.as_bool())))
            }
            "__builtin_and" => {
                Ok(Value::Bool(args.iter().all(|v| v.as_bool())))
            }
            _ => {
                Ok(if let Some(v) = self.env.get(name) {
                    v.clone()
                } else {
                    Value::Str(format!("{{{{{name}}}}}"))
                })
            }
        }
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Num(x), Value::Num(y)) => (x - y).abs() < f64::EPSILON,
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Nil, Value::Nil) => true,
            _ => a.to_string() == b.to_string(),
        }
    }
}