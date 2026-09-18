pub use crate::script::ast::Value;
use crate::script::ast::*;
use std::{borrow::Cow, collections::HashMap, slice};

#[derive(Default)]
pub struct Evaluator<'v> {
    pub env: HashMap<Cow<'v, str>, Value<'v>>,
}

impl<'v> Evaluator<'v> {
    pub fn evaluate<'e>(&mut self, expr: &'e Expr<'v>) -> Result<Value<'v>, Cow<'static, str>> {
        match expr {
            Expr::Literal(v) => Ok(v.clone()),
            Expr::Var(name) => self
                .env
                .get(name)
                .cloned()
                .ok_or_else(|| format!("Undefined variable: {name}").into()),
            Expr::Binary { left, op, right } => self.eval_binary(left, op, right),
            Expr::Unary { op, expr } => self.eval_unary(op, expr),
            Expr::Call { callee, args } => self.eval_call(callee, args),
            Expr::Index { target, index } => self.eval_index(target, index),
            Expr::DotAccess { target, field } => {
                let t = self.evaluate(target)?;
                match t {
                    Value::Map(map) => map
                        .get(field)
                        .cloned()
                        .ok_or_else(|| format!("Key '{field}' not found").into()),
                    _ => Err(format!("Cannot access field '{field}' on non-object").into()),
                }
            }
            Expr::Match {
                value,
                arms,
                default,
            } => {
                let val = self.evaluate(value)?;
                let mut matched = false;
                let mut result = Value::Nil;
                for (pattern, arm) in arms {
                    if Self::values_equal(&val, pattern) {
                        result = self.evaluate(arm)?;
                        matched = true;
                        break;
                    }
                }
                if !matched && let Some(def) = default {
                    result = self.evaluate(def)?;
                }
                Ok(result)
            }
            Expr::Object(fields) => {
                let mut map = std::collections::HashMap::new();
                for (key, expr) in fields {
                    map.insert(key.clone(), self.evaluate(expr)?);
                }
                Ok(Value::Map(map))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
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
            Expr::Fn { params, body } => Ok(Value::Fn {
                params: params.clone(),
                body: body.clone(),
            }),
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
                        Err("Can only pipe into functions".into())
                    }
                } else if let Expr::Var(name) = right.as_ref() {
                    let res = self.call_builtin(name, slice::from_ref(&left_val));
                    match res {
                        Ok(v) => Ok(v),
                        Err(_) => {
                            self.env.insert("__pipe_input".into(), left_val);
                            let result = self.evaluate(right)?;
                            self.env.remove(&Cow::Borrowed("__pipe_input"));
                            Ok(result)
                        }
                    }
                } else {
                    self.env.insert("__pipe_input".into(), left_val);
                    let result = self.evaluate(right)?;
                    self.env.remove(&Cow::Borrowed("__pipe_input"));
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
                let values = items.iter().map(|e| self.evaluate(e)).collect::<Result<_, _>>();
                Ok(Value::List(values?))
            }
            Expr::ForLoop { .. } => Ok(Value::Nil),
        }
    }

    fn eval_binary<'e>(&mut self, left: &'e Expr<'v>, op: &'e BinaryOp, right: &'e Expr<'v>) -> Result<Value<'v>, Cow<'static, str>> {
        let l = self.evaluate(left)?;
        let r = self.evaluate(right)?;

        match op {
            BinaryOp::Add => match (&l, &r) {
                (Value::Num(a), Value::Num(b)) => Ok(Value::Num(a + b)),

                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}").into())),

                (Value::Num(a), Value::Str(b)) => Ok(Value::Str(format!("{a}{b}").into())),

                (Value::Str(a), _) => Ok(Value::Str(format!("{a}{r}").into())),

                (_, Value::Str(b)) => Ok(Value::Str(format!("{l}{b}").into())),

                _ => Err(format!("Cannot add {l:?} and {r:?}").into()),
            },
            BinaryOp::Sub => {
                let a = l.as_num().ok_or(Cow::Borrowed("Left side must be a number"))?;
                let b = r.as_num().ok_or(Cow::Borrowed("Right side must be a number"))?;
                Ok(Value::Num(a - b))
            }
            BinaryOp::Mul => {
                let a = l.as_num().ok_or(Cow::Borrowed("Left side must be a number"))?;
                let b = r.as_num().ok_or(Cow::Borrowed("Right side must be a number"))?;
                Ok(Value::Num(a * b))
            }
            BinaryOp::Div => {
                let a = l.as_num().ok_or(Cow::Borrowed("Left side must be a number"))?;
                let b = r.as_num().ok_or(Cow::Borrowed("Right side must be a number"))?;
                if b == 0.0 {
                    Err("Division by zero".into())
                } else {
                    Ok(Value::Num(a / b))
                }
            }
            BinaryOp::Mod => {
                let a = l.as_num().ok_or(Cow::Borrowed("Left side must be a number"))?;
                let b = r.as_num().ok_or(Cow::Borrowed("Right side must be a number"))?;
                Ok(Value::Num(a % b))
            }
            BinaryOp::Eq => Ok(Value::Bool(Self::values_equal(&l, &r))),
            BinaryOp::Ne => Ok(Value::Bool(!Self::values_equal(&l, &r))),
            BinaryOp::Lt => Ok(Value::Bool(
                if let (Value::Num(a), Value::Num(b)) = (&l, &r) {
                    a < b
                } else {
                    l.to_string() < r.to_string()
                },
            )),
            BinaryOp::Gt => Ok(Value::Bool(
                if let (Value::Num(a), Value::Num(b)) = (&l, &r) {
                    a > b
                } else {
                    l.to_string() > r.to_string()
                },
            )),
            BinaryOp::Le => Ok(Value::Bool(
                if let (Value::Num(a), Value::Num(b)) = (&l, &r) {
                    a <= b
                } else {
                    l.to_string() <= r.to_string()
                },
            )),
            BinaryOp::Ge => Ok(Value::Bool(
                if let (Value::Num(a), Value::Num(b)) = (&l, &r) {
                    a >= b
                } else {
                    l.to_string() >= r.to_string()
                },
            )),
        }
    }

    fn eval_unary<'e>(&mut self, op: &'e UnaryOp, expr: &'e Expr<'v>) -> Result<Value<'v>, Cow<'static, str>> {
        let v = self.evaluate(expr)?;
        match op {
            UnaryOp::Not => Ok(Value::Bool(!v.as_bool())),
            UnaryOp::Neg => {
                let n = v.as_num().ok_or(Cow::Borrowed("Cannot negate non-number"))?;
                Ok(Value::Num(-n))
            }
        }
    }

    fn eval_call<'e>(&mut self, callee: &'e Expr<'v>, args: &[Expr<'v>]) -> Result<Value<'v>, Cow<'static, str>> {
        let mut arg_values = vec![];
        for a in args {
            arg_values.push(self.evaluate(a)?);
        }

        let Expr::Var(func_name) = callee else {
            return Err("Can only call functions and builtins".into());
        };

        self.call_builtin(func_name, &arg_values)
    }

    fn eval_call_with_values(
        &mut self,
        func_name: &str,
        arg_values: &[Value<'v>],
    ) -> Result<Value<'v>, Cow<'static, str>> {
        self.call_builtin(func_name, arg_values)
    }

    fn eval_index<'e>(&mut self, target: &'e Expr<'v>, index: &'e Expr<'v>) -> Result<Value<'v>, Cow<'static, str>> {
        let t = self.evaluate(target)?;
        let i = self.evaluate(index)?;

        match (t, i) {
            (Value::List(items), Value::Num(n)) => {
                let idx = if n < 0.0 {
                    (items.len() as f64 + n) as usize
                } else {
                    n as usize
                };
                items
                    .get(idx)
                    .cloned()
                    .ok_or_else(|| format!("Index {n} out of bounds").into())
            }
            (Value::Str(s), Value::Num(n)) => {
                let chars = s.chars().collect::<Vec<_>>();
                let idx = if n < 0.0 {
                    (chars.len() as f64 + n) as usize
                } else {
                    n as usize
                };
                chars
                    .get(idx)
                    .map(|c| Value::Str(c.to_string().into()))
                    .ok_or_else(|| format!("Index {n} out of bounds").into())
            }
            (Value::Map(map), Value::Str(key)) => map
                .get(&key)
                .cloned()
                .ok_or_else(|| format!("Key '{key}' not found").into()),
            _ => Err("Cannot index this type".into()),
        }
    }

    fn call_builtin(&mut self, name: &str, args: &[Value<'v>]) -> Result<Value<'v>, Cow<'static, str>> {
        match name {
            "upper" | "lower" | "trim" | "trim_start" | "trim_end" | "len" | "length"
            | "repeat" | "replace" | "slice" | "split" | "contains" | "starts_with"
            | "ends_with" | "substr" | "reverse" | "pad_start" | "pad_end" | "concat" | "title"
            | "join" => Self::call_str(name, args),

            "to_num" | "number" | "to_str" | "string" | "floor" | "ceil" | "ceiling" | "round"
            | "abs" | "min" | "max" | "clamp" | "rand" | "random" => Self::call_math(name, args),

            "list" | "choice" | "first" | "last" | "map" | "filter" | "sort" | "join_list" => {
                self.call_list(name, args)
            }

            "now" | "today" | "date_add" | "date_format" => Self::call_date(name, args),

            "if_then_else" | "__builtin_or" | "__builtin_and" => Self::call_logic(name, args),

            _ => Ok(self
                .env
                .get(name)
                .cloned()
                .unwrap_or_else(|| Value::Str(format!("{{{{{name}}}}}").into()))),
        }
    }

    fn arg1_str(args: &[Value<'v>]) -> String {
        args.first().map(std::string::ToString::to_string).unwrap_or_default()
    }

    fn call_str(name: &str, args: &[Value<'v>]) -> Result<Value<'v>, Cow<'static, str>> {
        match name {
            "upper" => Ok(Value::Str(Self::arg1_str(args).to_uppercase().into())),
            "lower" => Ok(Value::Str(Self::arg1_str(args).to_lowercase().into())),
            "trim" => Ok(Value::Str(Self::arg1_str(args).trim().to_string().into())),
            "trim_start" => Ok(Value::Str(Self::arg1_str(args).trim_start().to_string().into())),
            "trim_end" => Ok(Value::Str(Self::arg1_str(args).trim_end().to_string().into())),
            "len" | "length" => match args.first() {
                Some(Value::List(items)) => Ok(Value::Num(items.len() as f64)),
                Some(Value::Str(s)) => Ok(Value::Num(s.chars().count() as f64)),
                _ => Ok(Value::Num(Self::arg1_str(args).chars().count() as f64)),
            },
            "repeat" => {
                let s = args.first().ok_or(Cow::Borrowed("repeat requires a string"))?.to_string();
                let n = args
                    .get(1)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("repeat requires a count"))? as usize;
                Ok(Value::Str(s.repeat(n).into()))
            }
            "replace" => {
                let s = args
                    .first()
                    .ok_or(Cow::Borrowed("replace requires 3 arguments"))?
                    .to_string();
                let from = args
                    .get(1)
                    .ok_or(Cow::Borrowed("replace requires 3 arguments"))?
                    .to_string();
                let to = args
                    .get(2)
                    .ok_or(Cow::Borrowed("replace requires 3 arguments"))?
                    .to_string();
                Ok(Value::Str(s.replace(&from, &to).into()))
            }
            "slice" => {
                let s = args.first().ok_or(Cow::Borrowed("slice requires 3 arguments"))?.to_string();
                let start = args
                    .get(1)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("start must be a number"))? as usize;
                let end = args
                    .get(2)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("end must be a number"))? as usize;
                let chars = s.chars().collect::<Vec<_>>();
                if start > chars.len() || end > chars.len() || start > end {
                    return Err("slice indices out of bounds".into());
                }
                Ok(Value::Str(chars[start..end].iter().collect::<String>().into()))
            }
            "split" => {
                let s = args.first().ok_or(Cow::Borrowed("split requires 2 arguments"))?.to_string();
                let delim = args.get(1).ok_or(Cow::Borrowed("split requires a delimiter"))?.to_string();
                let parts = s.split(&delim).map(|p| Value::Str(p.to_string().into())).collect();
                Ok(Value::List(parts))
            }
            "contains" => {
                let s = args
                    .first()
                    .ok_or(Cow::Borrowed("contains requires 2 arguments"))?
                    .to_string();
                let sub = args
                    .get(1)
                    .ok_or(Cow::Borrowed("contains requires a substring"))?
                    .to_string();
                Ok(Value::Bool(s.contains(&sub)))
            }
            "starts_with" => {
                let s = args
                    .first()
                    .ok_or(Cow::Borrowed("starts_with requires 2 arguments"))?
                    .to_string();
                let prefix = args
                    .get(1)
                    .ok_or(Cow::Borrowed("starts_with requires a prefix"))?
                    .to_string();
                Ok(Value::Bool(s.starts_with(&prefix)))
            }
            "ends_with" => {
                let s = args
                    .first()
                    .ok_or(Cow::Borrowed("ends_with requires 2 arguments"))?
                    .to_string();
                let suffix = args
                    .get(1)
                    .ok_or(Cow::Borrowed("ends_with requires a suffix"))?
                    .to_string();
                Ok(Value::Bool(s.ends_with(&suffix)))
            }
            "substr" => {
                let s = args
                    .first()
                    .ok_or(Cow::Borrowed("substr requires 3 arguments"))?
                    .to_string();
                let start = args
                    .get(1)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("start must be a number"))? as usize;
                let len = args
                    .get(2)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("length must be a number"))? as usize;
                let chars = s.chars().collect::<Vec<_>>();
                let start = start.min(chars.len());
                let end = (start + len).min(chars.len());
                Ok(Value::Str(chars[start..end].iter().collect::<String>().into()))
            }
            "reverse" => match args.first() {
                Some(Value::List(items)) => {
                    let mut rev = items.clone();
                    rev.reverse();
                    Ok(Value::List(rev))
                }
                Some(Value::Str(s)) => Ok(Value::Str(s.chars().rev().collect())),
                _ => Err("reverse requires a list or string".into()),
            },
            "pad_start" => {
                let s = args
                    .first()
                    .ok_or(Cow::Borrowed("pad_start requires string and length"))?
                    .to_string();
                let target = args
                    .get(1)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("length must be a number"))? as usize;
                let ch = args
                    .get(2)
                    .and_then(|v| v.to_string().chars().next())
                    .unwrap_or(' ');
                let pad_len = target.saturating_sub(s.chars().count());
                let padding = std::iter::repeat_n(ch, pad_len).collect::<String>();
                Ok(Value::Str(format!("{padding}{s}").into()))
            }
            "pad_end" => {
                let s = args
                    .first()
                    .ok_or(Cow::Borrowed("pad_end requires string and length"))?
                    .to_string();
                let target = args
                    .get(1)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("length must be a number"))? as usize;
                let ch = args
                    .get(2)
                    .and_then(|v| v.to_string().chars().next())
                    .unwrap_or(' ');
                let pad_len = target.saturating_sub(s.chars().count());
                let padding = std::iter::repeat_n(ch, pad_len).collect::<String>();
                Ok(Value::Str(format!("{s}{padding}").into()))
            }
            "concat" => {
                let parts = args.iter().map(std::string::ToString::to_string).collect::<Vec<_>>();
                Ok(Value::Str(parts.join("").into()))
            }
            "title" => {
                let s = Self::arg1_str(args);
                let mut result = String::with_capacity(s.len());
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
                Ok(Value::Str(result.into()))
            }
            "join" => {
                let sep = args.get(1).map(std::string::ToString::to_string).unwrap_or_default();
                let parts = match args.first() {
                    Some(Value::List(items)) => items
                        .iter()
                        .map(std::string::ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(&sep),
                    v => v.map(std::string::ToString::to_string).unwrap_or_default(),
                };
                Ok(Value::Str(parts.into()))
            }
            _ => Err(format!("Unknown string function: {name}").into()),
        }
    }

    fn call_math(name: &str, args: &[Value<'v>]) -> Result<Value<'v>, Cow<'static, str>> {
        match name {
            "to_num" | "number" => {
                let s = Self::arg1_str(args);
                s.parse::<f64>()
                    .map(Value::Num)
                    .map_err(|_| format!("Cannot convert '{s}' to number").into())
            }
            "to_str" | "string" => Ok(Value::Str(
                args.first().unwrap_or(&Value::Nil).to_string().into(),
            )),
            "floor" => {
                let n = args
                    .first()
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("floor requires a number"))?;
                Ok(Value::Num(n.floor()))
            }
            "ceil" | "ceiling" => {
                let n = args
                    .first()
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("ceil requires a number"))?;
                Ok(Value::Num(n.ceil()))
            }
            "round" => {
                let n = args
                    .first()
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("round requires a number"))?;
                Ok(Value::Num(n.round()))
            }
            "abs" => {
                let n = args
                    .first()
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("abs requires a number"))?;
                Ok(Value::Num(n.abs()))
            }
            "min" | "max" => {
                if args.is_empty() {
                    return Err(format!("{name} requires at least one number").into());
                }
                let cmp = if name == "min" {
                    f64::min as fn(f64, f64) -> f64
                } else {
                    f64::max
                };
                let mut result = args[0].as_num().ok_or(Cow::Borrowed("min/max requires numbers"))?;
                for a in &args[1..] {
                    result = cmp(result, a.as_num().ok_or(Cow::Borrowed("min/max requires numbers"))?);
                }
                Ok(Value::Num(result))
            }
            "clamp" => {
                let v = args
                    .first()
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("clamp requires numbers"))?;
                let lo = args
                    .get(1)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("clamp requires numbers"))?;
                let hi = args
                    .get(2)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("clamp requires numbers"))?;
                Ok(Value::Num(v.clamp(lo, hi)))
            }
            "rand" | "random" => {
                let seed = nano_seed();
                if args.len() == 2 {
                    let lo = args[0].as_num().ok_or(Cow::Borrowed("random requires numbers"))? as i64;
                    let hi = args[1].as_num().ok_or(Cow::Borrowed("random requires numbers"))? as i64;
                    let range = (hi - lo + 1) as u64;
                    Ok(Value::Num((lo + ((seed % range) as i64)) as f64))
                } else if args.is_empty() {
                    Ok(Value::Num((seed % 1000) as f64 / 1000.0))
                } else {
                    Err("random takes 0 or 2 arguments".into())
                }
            }
            _ => Err(format!("Unknown math function: {name}").into()),
        }
    }

    fn call_list(&mut self, name: &str, args: &[Value<'v>]) -> Result<Value<'v>, Cow<'static, str>> {
        match name {
            "list" => Ok(Value::List(args.to_vec())),
            "choice" => {
                let items = match args.first() {
                    Some(Value::List(items)) if !items.is_empty() => items,
                    Some(Value::List(_)) => {
                        return Err("choice requires a non-empty list".into())
                    }
                    _ => {
                        if args.is_empty() {
                            return Err("choice requires arguments".into());
                        }
                        args
                    }
                };
                let seed = nano_seed();
                Ok(items[(seed % items.len() as u64) as usize].clone())
            }
            "first" => match args.first() {
                Some(Value::List(items)) => items
                    .first()
                    .cloned()
                    .ok_or(Cow::Borrowed("Empty list")),
                Some(Value::Str(s)) => s
                    .chars()
                    .next()
                    .map(|c| Value::Str(c.to_string().into()))
                    .ok_or(Cow::Borrowed("Empty string")),
                _ => Err("first requires a list or string".into()),
            },
            "last" => match args.first() {
                Some(Value::List(items)) => items
                    .last()
                    .cloned()
                    .ok_or(Cow::Borrowed("Empty list")),
                Some(Value::Str(s)) => s
                    .chars()
                    .last()
                    .map(|c| Value::Str(c.to_string().into()))
                    .ok_or_else(|| "Empty string".into()),
                _ => Err("last requires a list or string".into()),
            },
            "map" => {
                let items = match args.first() {
                    Some(Value::List(items)) => items,
                    v => v.map(slice::from_ref).unwrap_or_default(),
                };
                let fn_val = args
                    .get(1)
                    .ok_or(Cow::Borrowed("map requires a function name"))?;
                let mut results = Vec::with_capacity(items.len());
                match fn_val {
                    Value::Fn { params, body } => {
                        for item in items {
                            if params.len() == 1 {
                                self.env.insert(params[0].clone(), item.clone());
                            }
                            results.push(self.evaluate(body)?);
                            if params.len() == 1 {
                                self.env.remove(&params[0]);
                            }
                        }
                    }
                    other => {
                        let fn_name = other.to_string();
                        for item in items {
                            self.env.insert("__item".into(), item.clone());
                            results.push(self.call_builtin(&fn_name, slice::from_ref(item))?);
                            self.env.remove(&Cow::Borrowed("__item"));
                        }
                    }
                }
                Ok(Value::List(results))
            }
            "filter" => {
                let items = match args.first() {
                    Some(Value::List(items)) => items,
                    v => v.map(slice::from_ref).unwrap_or_default(),
                };
                let fn_val = args
                    .get(1)
                    .ok_or(Cow::Borrowed("filter requires a condition function"))?;
                let mut results = Vec::with_capacity(items.len());
                match fn_val {
                    Value::Fn { params, body } => {
                        for item in items {
                            if params.len() == 1 {
                                self.env.insert(params[0].clone(), item.clone());
                            }
                            let cond = self.evaluate(body)?.as_bool();
                            if cond {
                                results.push(item.clone());
                            }
                            if params.len() == 1 {
                                self.env.remove(&params[0]);
                            }
                        }
                    }
                    other => {
                        let cond_fn = other.to_string();
                        for item in items {
                            self.env.insert("__item".into(), item.clone());
                            let cond = self.call_builtin(&cond_fn, slice::from_ref(item))?;
                            if cond.as_bool() {
                                results.push(item.clone());
                            }
                            self.env.remove(&Cow::Borrowed("__item"));
                        }
                    }
                }
                Ok(Value::List(results))
            }
            "sort" => {
                let mut items = match args.first() {
                    Some(Value::List(items)) => items.clone(),
                    _ => return Err("sort requires a list".into()),
                };
                items.sort_by_key(std::string::ToString::to_string);
                Ok(Value::List(items))
            }
            "join_list" => {
                let Value::List(items) = args.first().ok_or(Cow::Borrowed("join_list requires a list"))? else {
                    return Err("join_list requires a list".into());
                };
                let sep = args.get(1).map(std::string::ToString::to_string).unwrap_or_default();
                let result = items.iter().map(std::string::ToString::to_string).collect::<Vec<_>>();
                Ok(Value::Str(result.join(&sep).into()))
            }
            _ => Err(format!("Unknown list function: {name}").into()),
        }
    }

    fn call_date(name: &str, args: &[Value<'v>]) -> Result<Value<'v>, Cow<'static, str>> {
        match name {
            "now" => {
                let fmt = args
                    .first()
                    .map_or(Cow::Borrowed("%Y-%m-%d %H:%M:%S"), |v| Cow::Owned(v.to_string()));
                Ok(Value::Str(chrono::Local::now().format(&fmt).to_string().into()))
            }
            "today" => {
                let fmt = args
                    .first()
                    .map_or(Cow::Borrowed("%Y-%m-%d"), |v| Cow::Owned(v.to_string()));
                Ok(Value::Str(chrono::Local::now().format(&fmt).to_string().into()))
            }
            "date_add" => {
                let s = args
                    .first()
                    .ok_or(Cow::Borrowed("date_add requires date string and days"))?
                    .to_string();
                let days = args
                    .get(1)
                    .and_then(Value::as_num)
                    .ok_or(Cow::Borrowed("days must be a number"))? as i64;
                match chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                    Ok(dt) => Ok(Value::Str(
                        (dt + chrono::Duration::days(days))
                            .format("%Y-%m-%d")
                            .to_string().into(),
                    )),
                    Err(_) => {
                        let dt = chrono::Local::now().date_naive();
                        Ok(Value::Str(
                            (dt + chrono::Duration::days(days)).format(&s).to_string().into(),
                        ))
                    }
                }
            }
            "date_format" => {
                let s = args
                    .first()
                    .ok_or(Cow::Borrowed("date_format requires date and format"))?
                    .to_string();
                let fmt = args
                    .get(1)
                    .ok_or(Cow::Borrowed("date_format requires a format string"))?
                    .to_string();

                chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_or_else(|_| Err(format!("Cannot parse date: {s}").into()), |dt| Ok(Value::Str(dt.format(&fmt).to_string().into())))
            }
            _ => Err(format!("Unknown date function: {name}").into()),
        }
    }

    fn call_logic(name: &str, args: &[Value<'v>]) -> Result<Value<'v>, Cow<'static, str>> {
        match name {
            "if_then_else" => {
                let cond = args.first().ok_or(Cow::Borrowed("if_then_else requires 3 arguments"))?;
                let then = args.get(1).ok_or(Cow::Borrowed("if_then_else requires 3 arguments"))?;
                let els = args.get(2).ok_or(Cow::Borrowed("if_then_else requires 3 arguments"))?;
                Ok(if cond.as_bool() {
                    then
                } else {
                    els
                }.clone())
            }
            "__builtin_or" => Ok(Value::Bool(args.iter().any(Value::as_bool))),
            "__builtin_and" => Ok(Value::Bool(args.iter().all(Value::as_bool))),
            _ => Err(format!("Unknown logic function: {name}").into()),
        }
    }

    fn values_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Num(x), Value::Num(y)) => (x - y).abs() < f64::EPSILON,
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Nil, Value::Nil) => true,
            (Value::Fn { .. }, Value::Fn { .. }) => false,
            _ => a.to_string() == b.to_string(),
        }
    }
}

fn nano_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}
