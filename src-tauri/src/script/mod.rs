pub mod lexer;
pub mod ast;
pub mod parser;
pub mod evaluator;

use lexer::Lexer;
use parser::Parser;
use evaluator::Evaluator;
use ast::Value;
use std::collections::HashMap;

pub fn parse(source: &str) -> Result<ast::Expr, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

pub fn evaluate(source: &str, context: &HashMap<String, String>) -> Result<String, String> {
    evaluate_with_args(source, context, &[])
}

pub fn evaluate_with_args(source: &str, context: &HashMap<String, String>, args: &[String]) -> Result<String, String> {
    let expr = parse(source)?;
    let mut evaluator = Evaluator::new();

    for (key, value) in context {
        if key == "_args_len" {
            if let Ok(n) = value.parse::<f64>() {
                evaluator.env.insert(key.clone(), Value::Num(n));
            } else {
                evaluator.env.insert(key.clone(), Value::Str(value.clone()));
            }
        } else {
            evaluator.env.insert(key.clone(), Value::Str(value.clone()));
        }
    }

    let args_values: Vec<Value> = args.iter().map(|a| Value::Str(a.clone())).collect();
    evaluator.env.insert("args".to_string(), Value::List(args_values));

    let result = evaluator.evaluate(&expr)?;
    Ok(result.to_string())
}

#[allow(dead_code)]
pub fn resolve_template(template: &str, context: &HashMap<String, String>) -> Result<String, String> {
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
                if ch == '{' {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        in_var = true;
                        brace_depth = 0;
                        var_content.clear();
                        changed = true;
                        continue;
                    }
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
                                Err(e) => new_result.push_str(&format!("{{{{{var_content} Error: {e}}}}}")),
                            }
                            in_var = false;
                            continue;
                        }
                    } else {
                        brace_depth -= 1;
                    }
                    var_content.push(ch);
                } else {
                    var_content.push(ch);
                }
            }
        }

        if in_var {
            new_result.push_str("{{");
            new_result.push_str(&var_content);
        }

        result = new_result;
    }

    Ok(result)
}
