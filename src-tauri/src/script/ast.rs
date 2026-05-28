use std::{borrow::Cow, collections::HashMap};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Expr<'e> {
    Literal(Value<'e>),
    Var(Cow<'e, str>),
    Binary {
        left: Box<Expr<'e>>,
        op: BinaryOp,
        right: Box<Expr<'e>>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr<'e>>,
    },
    Call {
        callee: Box<Expr<'e>>,
        args: Vec<Expr<'e>>,
    },
    Index {
        target: Box<Expr<'e>>,
        index: Box<Expr<'e>>,
    },
    DotAccess {
        target: Box<Expr<'e>>,
        field: Cow<'e, str>,
    },
    If {
        condition: Box<Expr<'e>>,
        then_branch: Box<Expr<'e>>,
        else_branch: Option<Box<Expr<'e>>>,
    },
    ForLoop {
        var_name: Cow<'e, str>,
        iterable: Box<Expr<'e>>,
        body: Box<Expr<'e>>,
    },
    Let {
        name: Cow<'e, str>,
        value: Box<Expr<'e>>,
        body: Box<Expr<'e>>,
    },
    Fn {
        params: Vec<Cow<'e, str>>,
        body: Box<Expr<'e>>,
    },
    Pipe {
        left: Box<Expr<'e>>,
        right: Box<Expr<'e>>,
    },
    Match {
        value: Box<Expr<'e>>,
        arms: Vec<(Value<'e>, Box<Expr<'e>>)>,
        default: Option<Box<Expr<'e>>>,
    },
    Object(Vec<(Cow<'e, str>, Expr<'e>)>),
    Block(Vec<Expr<'e>>),
    List(Vec<Expr<'e>>),
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone)]
pub enum Value<'v> {
    Num(f64),
    Str(Cow<'v, str>),
    Bool(bool),
    Nil,
    List(Vec<Value<'v>>),
    Map(HashMap<Cow<'v, str>, Value<'v>>),
    Fn {
        params: Vec<Cow<'v, str>>,
        body: Box<Expr<'v>>,
    },
}

impl Value<'_> {
    pub fn to_string(&self) -> String {
        match self {
            Value::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            Value::Str(s) => s.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Nil => String::new(),
            Value::List(items) => {
                let strs = items.iter().map(|v| v.to_string()).collect::<Vec<_>>();
                strs.join(", ")
            }
            Value::Map(_) => "[map]".to_string(),
            Value::Fn { .. } => "[function]".to_string(),
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            Value::Str(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Num(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Nil => false,
            Value::Fn { .. } => true,
            _ => true,
        }
    }
}
