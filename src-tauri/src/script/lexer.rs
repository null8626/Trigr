use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident,
    Number,
    String,
    Dot,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Not,
    If,
    Else,
    Then,
    For,
    In,
    Let,
    Return,
    Fn,
    True,
    False,
    Nil,
    Match,
    Newline,
    Semicolon,
    Colon,
    Pipe,
    Arrow,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: Cow<'static, str>,
    pub line: usize,
}

impl Token {
    pub fn new<L>(kind: TokenKind, lexeme: L, line: usize) -> Self
    where
        L: Into<Cow<'static, str>>,
    {
        Self { kind, lexeme: lexeme.into(), line }
    }
}

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, Cow<'static, str>> {
        let mut tokens = vec![];
        while !self.is_at_end() {
            let token = self.scan_token()?;
            if let Some(t) = token {
                tokens.push(t);
            }
        }
        tokens.push(Token::new(TokenKind::Eof, "", self.line));
        Ok(tokens)
    }

    fn scan_token(&mut self) -> Result<Option<Token>, Cow<'static, str>> {
        self.skip_whitespace();
        if self.is_at_end() {
            return Ok(None);
        }

        let _start = self.pos;
        let ch = self.advance();

        match ch {
            '(' => Ok(Some(Token::new(
                TokenKind::LParen,
                "(",
                self.line,
            ))),
            ')' => Ok(Some(Token::new(
                TokenKind::RParen,
                ")",
                self.line,
            ))),
            '{' => Ok(Some(Token::new(
                TokenKind::LBrace,
                "{",
                self.line,
            ))),
            '}' => Ok(Some(Token::new(
                TokenKind::RBrace,
                "}",
                self.line,
            ))),
            '[' => Ok(Some(Token::new(
                TokenKind::LBracket,
                "[",
                self.line,
            ))),
            ']' => Ok(Some(Token::new(
                TokenKind::RBracket,
                "]",
                self.line,
            ))),
            ',' => Ok(Some(Token::new(
                TokenKind::Comma,
                ",",
                self.line,
            ))),
            '.' => Ok(Some(Token::new(TokenKind::Dot, ".", self.line))),
            ';' => Ok(Some(Token::new(
                TokenKind::Semicolon,
                ";",
                self.line,
            ))),
            ':' => Ok(Some(Token::new(
                TokenKind::Colon,
                ":",
                self.line,
            ))),
            '+' => Ok(Some(Token::new(
                TokenKind::Plus,
                "+",
                self.line,
            ))),
            '-' => Ok(Some(if self.match_next('>') {
                Token::new(TokenKind::Arrow, "->", self.line)
            } else {
                Token::new(TokenKind::Minus, "-", self.line)
            })),
            '*' => Ok(Some(Token::new(
                TokenKind::Star,
                "*",
                self.line,
            ))),
            '/' => Ok(if self.match_next('/') {
                self.skip_line_comment();
                None
            } else if self.match_next('*') {
                self.skip_block_comment()?;
                None
            } else {
                Some(Token::new(TokenKind::Slash, "/", self.line))
            }),
            '%' => Ok(Some(Token::new(
                TokenKind::Percent,
                "%",
                self.line,
            ))),
            '|' => Ok(Some(Token::new(TokenKind::Pipe, "|", self.line))),
            '&' => Err(format!("Unexpected character '&' at line {}", self.line).into()),
            '!' => Ok(Some(if self.match_next('=') {
                Token::new(TokenKind::Ne, "!=", self.line)
            } else {
                return Err(format!("Unexpected character '!' at line {}. Did you mean 'not'?", self.line).into());
            })),
            '=' => Ok(Some(if self.match_next('=') {
                Token::new(TokenKind::Eq, "==", self.line)
            } else if self.match_next('>') {
                Token::new(TokenKind::Arrow, "=>", self.line)
            } else {
                Token::new(TokenKind::Eq, "=", self.line)
            })),
            '<' => Ok(Some(if self.match_next('=') {
                Token::new(TokenKind::Le, "<=", self.line)
            } else {
                Token::new(TokenKind::Lt, "<", self.line)
            })),
            '>' => Ok(Some(if self.match_next('=') {
                Token::new(TokenKind::Ge, ">=", self.line)
            } else {
                Token::new(TokenKind::Gt, ">", self.line)
            })),
            '\n' => Ok(Some(Token::new(
                TokenKind::Newline,
                "\n",
                self.line,
            ))),
            '"' => Ok(Some(self.read_string()?)),
            '\'' => Ok(Some(self.read_char_literal()?)),
            c if c.is_ascii_digit() => Ok(Some(self.read_number(c))),
            c if c.is_alphabetic() || c == '_' => Ok(Some(self.read_ident(c))),
            _ => Err(format!("Unexpected character '{ch}' at line {}", self.line).into()),
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.pos];
        self.pos += 1;
        ch
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.pos]
        }
    }

    fn peek_next(&self) -> char {
        if self.pos + 1 >= self.source.len() {
            '\0'
        } else {
            self.source[self.pos + 1]
        }
    }

    const fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn match_next(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source[self.pos] != expected {
            false
        } else {
            self.pos += 1;
            true
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), Cow<'static, str>> {
        while !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(format!("Unterminated block comment at line {}", self.line).into())
    }

    fn read_string(&mut self) -> Result<Token, Cow<'static, str>> {
        let mut s = String::new();
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance();
                match self.peek() {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '"' => s.push('"'),
                    _ => s.push('\\'),
                }
                self.advance();
            } else {
                if self.peek() == '\n' {
                    self.line += 1;
                }
                s.push(self.advance());
            }
        }
        if self.is_at_end() {
            return Err(format!("Unterminated string at line {}", self.line).into());
        }
        self.advance();
        Ok(Token::new(TokenKind::String, s, self.line))
    }

    fn read_char_literal(&mut self) -> Result<Token, Cow<'static, str>> {
        let mut s = String::new();
        if !self.is_at_end() && self.peek() != '\'' {
            if self.peek() == '\\' {
                self.advance();
                match self.peek() {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    _ => s.push(self.peek()),
                }
                self.advance();
            } else {
                s.push(self.advance());
            }
        }
        if self.is_at_end() || self.peek() != '\'' {
            return Err(format!("Unterminated char literal at line {}", self.line).into());
        }
        self.advance();
        Ok(Token::new(TokenKind::String, s, self.line))
    }

    fn read_number(&mut self, start: char) -> Token {
        let mut s = String::new();
        s.push(start);
        let mut has_dot = false;
        while !self.is_at_end() {
            let ch = self.peek();
            if ch.is_ascii_digit() {
                s.push(self.advance());
            } else if ch == '.' && !has_dot {
                has_dot = true;
                s.push(self.advance());
            } else {
                break;
            }
        }
        Token::new(TokenKind::Number, s, self.line)
    }

    fn read_ident(&mut self, start: char) -> Token {
        let mut s = String::new();
        s.push(start);
        while !self.is_at_end() {
            let ch = self.peek();
            if ch.is_alphanumeric() || ch == '_' {
                s.push(self.advance());
            } else {
                break;
            }
        }
        let kind = match s.as_str() {
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "then" => TokenKind::Then,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "let" => TokenKind::Let,
            "return" => TokenKind::Return,
            "fn" => TokenKind::Fn,
            "match" => TokenKind::Match,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            _ => TokenKind::Ident,
        };
        Token::new(kind, s, self.line)
    }
}
