use crate::script::{
    ast::*,
    lexer::{Token, TokenKind},
};
use std::borrow::Cow;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub const fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let expr = self.parse_pipe()?;
        self.consume(&TokenKind::Eof, "end of expression")?;
        Ok(expr)
    }

    fn parse_pipe<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let mut expr = self.parse_if()?;
        while self.match_token(&TokenKind::Pipe) {
            let right = self.parse_if()?;
            expr = Expr::Pipe {
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_if<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        if self.match_token(&TokenKind::If) {
            let condition = self.parse_pipe()?;
            self.consume(&TokenKind::Then, "expected 'then' after if condition")?;
            let then_branch = self.parse_pipe()?;
            let else_branch = if self.match_token(&TokenKind::Else) {
                if self.check(&TokenKind::If) {
                    Some(Box::new(self.parse_if()?))
                } else {
                    Some(Box::new(self.parse_pipe()?))
                }
            } else {
                None
            };
            return Ok(Expr::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch,
            });
        }
        self.parse_match()
    }

    fn parse_match<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        if self.match_token(&TokenKind::Match) {
            let value = self.parse_pipe()?;
            self.consume(&TokenKind::LBrace, "expected '{' after match value")?;
            let mut arms = vec![];
            let mut default = None;
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                if self.match_token(&TokenKind::Ident) && self.tokens[self.pos - 1].lexeme == "_" {
                    self.consume(&TokenKind::Arrow, "expected '=>' after default arm '_'")?;
                    default = Some(Box::new(self.parse_pipe()?));
                } else {
                    let pattern = self.parse_primary()?;
                    let pattern_val = match &pattern {
                        Expr::Literal(v) => v.clone(),
                        _ => return Err(
                            "Pattern must be a literal value (number, string, true, false, or nil)"
                                .into(),
                        ),
                    };
                    self.consume(&TokenKind::Arrow, "expected '=>' after match pattern")?;
                    let arm_expr = self.parse_pipe()?;
                    arms.push((pattern_val, Box::new(arm_expr)));
                }
                self.match_token(&TokenKind::Comma);
            }
            self.consume(&TokenKind::RBrace, "expected '}' after match arms")?;
            return Ok(Expr::Match {
                value: Box::new(value),
                arms,
                default,
            });
        }
        self.parse_let()
    }

    fn parse_let<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        if self.match_token(&TokenKind::Let) {
            let name = self.consume_ident()?;
            self.consume(&TokenKind::Eq, "expected '=' after let name")?;
            let value = self.parse_pipe()?;
            self.match_token(&TokenKind::Semicolon);
            let body = self.parse_pipe()?;
            return Ok(Expr::Let {
                name,
                value: Box::new(value),
                body: Box::new(body),
            });
        }
        self.parse_assignment()
    }

    fn parse_assignment<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let expr = self.parse_or()?;
        if self.match_token(&TokenKind::Eq) {
            match expr {
                Expr::Var(name) => {
                    let value = self.parse_assignment()?;
                    let name_clone = name.clone();
                    Ok(Expr::Let {
                        name,
                        value: Box::new(value),
                        body: Box::new(Expr::Var(name_clone)),
                    })
                }
                _ => Err("Invalid assignment target".into()),
            }
        } else {
            Ok(expr)
        }
    }

    fn parse_or<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let mut expr = self.parse_and()?;
        while self.match_token(&TokenKind::Or) {
            let right = self.parse_and()?;
            let left = expr.clone();
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Add,
                right: Box::new(Expr::Call {
                    callee: Box::new(Expr::Var("__builtin_or".into())),
                    args: vec![left, right],
                }),
            };
        }
        Ok(expr)
    }

    fn parse_and<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let mut expr = self.parse_equality()?;
        while self.match_token(&TokenKind::And) {
            let right = self.parse_equality()?;
            let left = expr.clone();
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Add,
                right: Box::new(Expr::Call {
                    callee: Box::new(Expr::Var("__builtin_and".into())),
                    args: vec![left, right],
                }),
            };
        }
        Ok(expr)
    }

    fn parse_equality<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let mut expr = self.parse_comparison()?;
        while self.match_token(&TokenKind::Eq) || self.match_token(&TokenKind::Ne) {
            let op = if self.tokens[self.pos - 1].kind == TokenKind::Eq {
                BinaryOp::Eq
            } else {
                BinaryOp::Ne
            };
            let right = self.parse_comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_comparison<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let mut expr = self.parse_addition()?;
        while self.match_token(&TokenKind::Lt)
            || self.match_token(&TokenKind::Gt)
            || self.match_token(&TokenKind::Le)
            || self.match_token(&TokenKind::Ge)
        {
            let op = match self.tokens[self.pos - 1].kind {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Le => BinaryOp::Le,
                TokenKind::Ge => BinaryOp::Ge,
                _ => unreachable!(),
            };
            let right = self.parse_addition()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_addition<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let mut expr = self.parse_multiplication()?;
        while self.match_token(&TokenKind::Plus) || self.match_token(&TokenKind::Minus) {
            let op = if self.tokens[self.pos - 1].kind == TokenKind::Plus {
                BinaryOp::Add
            } else {
                BinaryOp::Sub
            };
            let right = self.parse_multiplication()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_multiplication<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let mut expr = self.parse_unary()?;
        while self.match_token(&TokenKind::Star)
            || self.match_token(&TokenKind::Slash)
            || self.match_token(&TokenKind::Percent)
        {
            let op = match self.tokens[self.pos - 1].kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_unary<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        if self.match_token(&TokenKind::Not) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }
        if self.match_token(&TokenKind::Minus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }
        self.parse_call()
    }

    fn parse_call<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(&TokenKind::LParen) {
                let mut args = vec![];
                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_pipe()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(&TokenKind::RParen, "expected ')' after arguments")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.match_token(&TokenKind::Dot) {
                let field = self.consume_ident()?;
                expr = Expr::DotAccess {
                    target: Box::new(expr),
                    field,
                };
            } else if self.match_token(&TokenKind::LBracket) {
                let index = self.parse_pipe()?;
                self.consume(&TokenKind::RBracket, "expected ']' after index")?;
                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary<'e>(&mut self) -> Result<Expr<'e>, Cow<'static, str>> {
        if self.match_token(&TokenKind::True) {
            return Ok(Expr::Literal(Value::Bool(true)));
        }
        if self.match_token(&TokenKind::False) {
            return Ok(Expr::Literal(Value::Bool(false)));
        }
        if self.match_token(&TokenKind::Nil) {
            return Ok(Expr::Literal(Value::Nil));
        }

        if self.match_token(&TokenKind::Number) {
            let lexeme = &*self.tokens[self.pos - 1].lexeme;
            let n = lexeme.parse().map_err(|e| format!("Invalid number: {e}"))?;
            return Ok(Expr::Literal(Value::Num(n)));
        }

        if self.check(&TokenKind::String) {
            return Ok(Expr::Literal(Value::Str(self.advance().lexeme)));
        }

        if self.match_token(&TokenKind::LBracket) {
            let mut items = vec![];
            if !self.check(&TokenKind::RBracket) {
                loop {
                    items.push(self.parse_pipe()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.consume(&TokenKind::RBracket, "expected ']' after list")?;
            return Ok(Expr::List(items));
        }

        if self.match_token(&TokenKind::LParen) {
            let saved = self.pos;

            let mut params = vec![];
            let mut looks_like_params = true;
            if !self.check(&TokenKind::RParen) {
                loop {
                    if !self.check(&TokenKind::Ident) {
                        looks_like_params = false;
                        break;
                    }
                    params.push(self.advance().lexeme);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            if looks_like_params && self.check(&TokenKind::RParen) {
                self.advance();
                if self.match_token(&TokenKind::Arrow) {
                    let body = self.parse_pipe()?;
                    return Ok(Expr::Fn {
                        params,
                        body: Box::new(body),
                    });
                }

                if params.len() == 1 {
                    return Ok(Expr::Var(params.into_iter().next().unwrap()));
                }
                if params.is_empty() {
                    return Err("Empty parentheses".into());
                }
            }

            self.pos = saved;
            self.advance();
            let inner = self.parse_pipe()?;
            self.consume(&TokenKind::RParen, "expected ')' after expression")?;
            return Ok(inner);
        }

        if self.check(&TokenKind::LBrace) {
            let saved = self.pos;
            self.advance();
            let is_object = if !self.check(&TokenKind::RBrace) && self.check(&TokenKind::Ident) {
                let ident_pos = self.pos;
                self.advance();
                let has_colon = self.check(&TokenKind::Colon);
                self.pos = ident_pos;
                has_colon
            } else {
                false
            };
            self.pos = saved;

            if is_object {
                self.advance();
                let mut fields = vec![];
                if !self.check(&TokenKind::RBrace) {
                    loop {
                        let key = self.consume_ident()?;
                        self.consume(&TokenKind::Colon, "expected ':' after object key")?;
                        let value = self.parse_pipe()?;
                        fields.push((key, value));
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(&TokenKind::RBrace, "expected '}' after object")?;
                return Ok(Expr::Object(fields));
            }
        }

        if self.check(&TokenKind::Ident) && self.pos + 1 < self.tokens.len() && self.tokens[self.pos + 1].kind == TokenKind::Arrow {
            let name = self.advance().lexeme;
            self.advance();
            let body = self.parse_pipe()?;
            return Ok(Expr::Fn {
                params: vec![name],
                body: Box::new(body),
            });
        }

        if self.match_token(&TokenKind::Fn) {
            let mut params = vec![];
            if !self.check(&TokenKind::Arrow) {
                loop {
                    params.push(self.consume_ident()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.consume(&TokenKind::Arrow, "expected '->' after fn params")?;
            let body = self.parse_pipe()?;
            return Ok(Expr::Fn {
                params,
                body: Box::new(body),
            });
        }

        if self.check(&TokenKind::Ident) {
            return Ok(Expr::Var(self.advance().lexeme));
        }

        let token = self.peek();
        Err(format!(
            "Unexpected token '{:?}' at line {}",
            token.kind, token.line
        ).into())
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.tokens[self.pos - 1].clone()
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek().kind == *kind
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<Token, Cow<'static, str>> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let token = self.peek();
            Err(format!(
                "Line {}: expected {message}, got '{:?}'",
                token.line, token.kind
            ).into())
        }
    }

    fn consume_ident<'t>(&mut self) -> Result<Cow<'t, str>, Cow<'static, str>> {
        let token = self.consume(&TokenKind::Ident, "identifier")?;
        Ok(token.lexeme)
    }
}
