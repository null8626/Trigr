pub mod lexer;
pub mod ast;
pub mod parser;
pub mod evaluator;

use lexer::Lexer;
use parser::Parser;
use evaluator::Evaluator;
use ast::Value;
use std::{borrow::Cow, collections::HashMap, fmt::Write};

pub fn parse(source: &str) -> Result<ast::Expr<'_>, Cow<'static, str>> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

pub fn evaluate(source: &str, context: &HashMap<String, String>) -> Result<String, Cow<'static, str>> {
    evaluate_with_args(source, context, &[])
}

pub fn evaluate_with_args(source: &str, context: &HashMap<String, String>, args: &[String]) -> Result<String, Cow<'static, str>> {
    let expr = parse(source)?;
    let mut evaluator = Evaluator::default();

    for (key, value) in context {
        evaluator.env.insert(key.as_str().into(), if key == "_args_len" {
            Value::Str(value.as_str().into())
        } else if let Ok(n) = value.parse::<f64>() {
            Value::Num(n)
        } else {
            Value::Str(value.as_str().into())
        });
    }

    let args_values = args.iter().map(|a| Value::Str(a.as_str().into())).collect();
    evaluator.env.insert("args".into(), Value::List(args_values));

    let result = evaluator.evaluate(&expr)?;
    Ok(result.to_string())
}

#[allow(dead_code)]
pub fn resolve_template(template: &str, context: &HashMap<String, String>) -> Result<String, Cow<'static, str>> {
    let mut result = template.to_string();
    let mut changed = true;
    let mut iterations = 0;
    let max_iterations = 50;

    while changed && iterations < max_iterations {
        changed = false;
        iterations += 1;
        let mut new_result = String::new();
        let mut chars = result.chars().peekable();
        let mut in_var = false;
        let mut var_content = String::new();
        let mut brace_depth = 0;

        while let Some(ch) = chars.next() {
            if !in_var {
                if ch == '{' && chars.peek() == Some(&'{') {
                    chars.next();
                    in_var = true;
                    brace_depth = 0;
                    var_content.clear();
                    changed = true;
                    continue;
                }
                new_result.push(ch);
            } else {
                if ch == '{' {
                    brace_depth += 1;
                    var_content.push(ch);
                } else if ch == '}' {
                    if brace_depth == 0 {
                        if chars.peek() == Some(&'}') {
                            chars.next();
                            match evaluate(&var_content, context) {
                                Ok(value) => new_result.push_str(&value),
                                Err(e) => write!(&mut new_result, "{{{{{var_content} Error: {e}}}}}").unwrap(),
                            }
                            in_var = false;
                            continue;
                        }
                    } else {
                        brace_depth -= 1;
                    }
                }

                var_content.push(ch);
            }
        }

        if in_var {
            write!(&mut new_result, "{{{{{var_content}").unwrap();
        }

        result = new_result;
    }

    Ok(result)
}
