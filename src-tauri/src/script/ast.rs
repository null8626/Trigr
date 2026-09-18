use std::{borrow::Cow, collections::HashMap, fmt::{self, Display, Formatter}};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Expr<'e> {
    Literal(Value<'e>),
    Var(Cow<'e, str>),
    Binary {
        left: Box<Self>,
        op: BinaryOp,
        right: Box<Self>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Self>,
    },
    Call {
        callee: Box<Self>,
        args: Vec<Self>,
    },
    Index {
        target: Box<Self>,
        index: Box<Self>,
    },
    DotAccess {
        target: Box<Self>,
        field: Cow<'e, str>,
    },
    If {
        condition: Box<Self>,
        then_branch: Box<Self>,
        else_branch: Option<Box<Self>>,
    },
    ForLoop {
        var_name: Cow<'e, str>,
        iterable: Box<Self>,
        body: Box<Self>,
    },
    Let {
        name: Cow<'e, str>,
        value: Box<Self>,
        body: Box<Self>,
    },
    Fn {
        params: Vec<Cow<'e, str>>,
        body: Box<Self>,
    },
    Pipe {
        left: Box<Self>,
        right: Box<Self>,
    },
    Match {
        value: Box<Self>,
        arms: Vec<(Value<'e>, Box<Self>)>,
        default: Option<Box<Self>>,
    },
    Object(Vec<(Cow<'e, str>, Self)>),
    Block(Vec<Self>),
    List(Vec<Self>),
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
    pub fn as_num(&self) -> Option<f64> {
        match self {
            Self::Num(n) => Some(*n),
            Self::Str(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    pub const fn as_bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Num(n) => *n != 0.0,
            Self::Str(s) => match s {
                Cow::Borrowed(s) => !s.is_empty(),
                Cow::Owned(s) => !s.is_empty(),
            },
            Self::Nil => false,
            Self::Fn { .. } => true,
            _ => true,
        }
    }
}

impl Display for Value<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    Display::fmt(n, f)
                }
            }
            Self::Str(s) => Display::fmt(s, f),
            Self::Bool(b) => Display::fmt(b, f),
            Self::Nil => Ok(()),
            Self::List(items) => {
                if !items.is_empty() {
                    let last_index = items.len() - 1;

                    for item in items.iter().take(last_index) {
                        Display::fmt(item, f)?;
                    }

                    Display::fmt(&items[last_index], f)
                } else {
                    Ok(())
                }
            }
            Self::Map(_) => f.write_str("[map]"),
            Self::Fn { .. } => f.write_str("[function]"),
        }
    }
}