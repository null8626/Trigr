use crate::script::{lexer::{Token, TokenKind}, ast::*};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub const fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Expr, String> {
        let expr = self.parse_pipe()?;
        self.consume(&TokenKind::Eof, "end of expression")?;
        Ok(expr)
    }

    fn parse_pipe(&mut self) -> Result<Expr, String> {
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

    fn parse_if(&mut self) -> Result<Expr, String> {
        if self.match_token(&TokenKind::If) {
            let condition = self.parse_pipe()?;
            self.consume(&TokenKind::Comma, "expected ',' after if condition")?;
            let then_branch = self.parse_pipe()?;
            let else_branch = if self.match_token(&TokenKind::Comma) {
                Some(Box::new(self.parse_pipe()?))
            } else {
                None
            };
            return Ok(Expr::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch,
            });
        }
        self.parse_let()
    }

    fn parse_let(&mut self) -> Result<Expr, String> {
        if self.match_token(&TokenKind::Let) {
            let name = self.consume_ident()?;
            self.consume(&TokenKind::Eq, "expected '=' after let name")?;
            let value = self.parse_pipe()?;
            self.consume(&TokenKind::Semicolon, "expected ';' after let value")?;
            let body = self.parse_pipe()?;
            return Ok(Expr::Let {
                name,
                value: Box::new(value),
                body: Box::new(body),
            });
        }
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let expr = self.parse_or()?;

        if self.match_token(&TokenKind::Eq) {
            match expr {
                Expr::Var(name) => {
                    let value = self.parse_assignment()?;
                    let name_clone = name.clone();
                    return Ok(Expr::Let {
                        name,
                        value: Box::new(value),
                        body: Box::new(Expr::Var(name_clone)),
                    });
                }
                _ => return Err("Invalid assignment target".to_string()),
            }
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_and()?;
        while self.match_token(&TokenKind::Or) {
            let right = self.parse_and()?;
            let left = expr.clone();
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Add,
                right: Box::new(Expr::Call {
                    callee: Box::new(Expr::Var("__builtin_or".to_string())),
                    args: vec![left, right],
                }),
            };
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_equality()?;
        while self.match_token(&TokenKind::And) {
            let right = self.parse_equality()?;
            let left = expr.clone();
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Add,
                right: Box::new(Expr::Call {
                    callee: Box::new(Expr::Var("__builtin_and".to_string())),
                    args: vec![left, right],
                }),
            };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
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

    fn parse_comparison(&mut self) -> Result<Expr, String> {
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

    fn parse_addition(&mut self) -> Result<Expr, String> {
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

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
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

    fn parse_unary(&mut self) -> Result<Expr, String> {
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

    fn parse_call(&mut self) -> Result<Expr, String> {
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
                let name = self.consume_ident()?;
                expr = Expr::Call {
                    callee: Box::new(Expr::Var(name)),
                    args: vec![expr],
                };
            } else if self.match_token(&TokenKind::LBrace) {
                let index = self.parse_pipe()?;
                self.consume(&TokenKind::RBrace, "expected '}' after index")?;
                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(index),
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

    fn parse_primary(&mut self) -> Result<Expr, String> {
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
            let lexeme = self.tokens[self.pos - 1].lexeme.clone();
            let n: f64 = lexeme.parse().map_err(|e| format!("Invalid number: {e}"))?;
            return Ok(Expr::Literal(Value::Num(n)));
        }

        if self.check(&TokenKind::String) {
            let s = self.advance().lexeme.clone();
            return Ok(Expr::Literal(Value::Str(s)));
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
            self.consume(&TokenKind::RBracket, "expected ']' after list items")?;
            return Ok(Expr::List(items));
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
            let name = self.advance().lexeme.clone();
            return Ok(Expr::Var(name));
        }

        let token = self.peek();
        Err(format!(
            "Unexpected token '{:?}' at line {}",
            token.kind, token.line
        ))
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
        if self.is_at_end() {
            return *kind == TokenKind::Eof;
        }
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

    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let token = self.peek();
            Err(format!("Expected {message}: found '{:?}' at line {}", token.kind, token.line))
        }
    }

    fn consume_ident(&mut self) -> Result<String, String> {
        let token = self.consume(&TokenKind::Ident, "expected identifier")?;
        Ok(token.lexeme)
    }
}
