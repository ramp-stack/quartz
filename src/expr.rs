use crate::value::{Expr, Value, MathOp, CompOp};
use crate::{Action, Condition};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Int(i32),
    Float(f32),
    Bool(bool),
    Str(String),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    Bang,
    LParen,
    RParen,
    Semi,
    Eof,
}

struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(), pos: 0
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.input.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn skip_whitespace(&mut self) {
        while self.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            self.advance();
        }
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => { tokens.push(Token::Eof); break; }
                Some(c) => tokens.push(self.next_token(c)?),
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self, c: char) -> Result<Token, String> {
        match c {
            '+' => { self.advance(); if self.peek() == Some('=') { self.advance(); Ok(Token::PlusAssign) } else { Ok(Token::Plus) }}
            '-' => { self.advance(); if self.peek() == Some('=') { self.advance(); Ok(Token::MinusAssign) } else { Ok(Token::Minus) }}
            '*' => { self.advance(); if self.peek() == Some('=') { self.advance(); Ok(Token::StarAssign) } else { Ok(Token::Star) }}
            '/' => { self.advance(); if self.peek() == Some('=') { self.advance(); Ok(Token::SlashAssign) } else { Ok(Token::Slash) }}
            '=' => { self.advance(); if self.peek() == Some('=') { self.advance(); Ok(Token::EqEq) } else { Ok(Token::Assign) }}
            '!' => { self.advance(); if self.peek() == Some('=') { self.advance(); Ok(Token::NotEq) } else { Ok(Token::Bang) }}
            '<' => { self.advance(); if self.peek() == Some('=') { self.advance(); Ok(Token::LtEq) } else { Ok(Token::Lt) }}
            '>' => { self.advance(); if self.peek() == Some('=') { self.advance(); Ok(Token::GtEq) } else { Ok(Token::Gt) }}
            '&' => {
                self.advance(); if self.peek() == Some('&') { self.advance(); Ok(Token::AndAnd) }
                else { Err("expected '&&', got single '&'".into())}
            }
            '|' => {
                self.advance(); if self.peek() == Some('|') { self.advance(); Ok(Token::OrOr) }
                else { Err("expected '||', got single '|'".into())}
            }
            '(' => { self.advance(); Ok(Token::LParen) }
            ')' => { self.advance(); Ok(Token::RParen) }
            ';' => { self.advance(); Ok(Token::Semi) }
            '"' => self.lex_string(),
            c if c.is_ascii_digit() => self.lex_number(),
            c if c.is_alphabetic() || c == '_' => self.lex_ident(),
            other => Err(format!("unexpected character: '{other}'")),
        }
    }

    fn lex_string(&mut self) -> Result<Token, String> {
        self.advance();
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => break,
                Some(c) => s.push(c),
                None => return Err("unterminated string literal".into()),
            }
        }
        Ok(Token::Str(s))
    }

    fn lex_number(&mut self) -> Result<Token, String> {
        let mut s = String::new();
        let mut float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else if c == '.' && !float {
                float = true;
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if float {
            s.parse::<f32>().map(Token::Float).map_err(|e| e.to_string())
        } else {
            s.parse::<i32>().map(Token::Int).map_err(|e| e.to_string())
        }
    }

    fn lex_ident(&mut self) -> Result<Token, String> {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        Ok(match s.as_str() {
            "true" => Token::Bool(true),
            "false" => Token::Bool(false),
            _ => Token::Ident(s),
        })
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len().saturating_sub(1) {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected token {:?}, got {:?}", expected, self.peek()))
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_add()
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_mul()?;
        loop {
            match self.peek() {
                Token::Plus => {
                    self.advance();
                    let r = self.parse_mul()?;
                    left = Expr::Add(Box::new(left), Box::new(r));
                }
                Token::Minus => {
                    self.advance();
                    let r = self.parse_mul()?;
                    left = Expr::Sub(Box::new(left), Box::new(r));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            match self.peek() {
                Token::Star => {
                    self.advance();
                    let r = self.parse_unary()?;
                    left = Expr::Mul(Box::new(left), Box::new(r));
                }
                Token::Slash => {
                    self.advance();
                    let r = self.parse_unary()?;
                    left = Expr::Div(Box::new(left), Box::new(r));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Token::Minus) {
            self.advance();
            let inner = self.parse_primary()?;
            Ok(Expr::Sub(Box::new(Expr::Lit(Value::I32(0))), Box::new(inner)))
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Token::Int(n) => Ok(Expr::Lit(Value::I32(n))),
            Token::Float(f) => Ok(Expr::Lit(Value::F32(f))),
            Token::Bool(b) => Ok(Expr::Lit(Value::Bool(b))),
            Token::Str(s) => Ok(Expr::Lit(Value::Str(s))),
            Token::Ident(name) => Ok(Expr::Var(name)),
            Token::LParen => {
                let inner = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(inner)
            }
            other => Err(format!("unexpected token: {:?}", other)),
        }
    }

    fn parse_condition(&mut self) -> Result<Condition, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Condition, String> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Token::OrOr) {
            self.advance();
            let right = self.parse_and()?;
            left = Condition::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Condition, String> {
        let mut left = self.parse_not()?;
        while matches!(self.peek(), Token::AndAnd) {
            self.advance();
            let right = self.parse_not()?;
            left = Condition::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Condition, String> {
        if matches!(self.peek(), Token::Bang) {
            self.advance();
            Ok(Condition::Not(Box::new(self.parse_not()?)))
        } else {
            self.parse_compare()
        }
    }

    fn parse_compare(&mut self) -> Result<Condition, String> {
        let left = self.parse_expr()?;
        let op = match self.peek() {
            Token::EqEq => Some(CompOp::Eq),
            Token::NotEq => Some(CompOp::Ne),
            Token::Lt => Some(CompOp::Lt),
            Token::LtEq => Some(CompOp::Lte),
            Token::Gt => Some(CompOp::Gt),
            Token::GtEq => Some(CompOp::Gte),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            Ok(Condition::Compare(left, op, self.parse_expr()?))
        } else {
            Ok(Condition::Compare(left, CompOp::Eq, Expr::Lit(Value::Bool(true))))
        }
    }

    fn parse_stmts(&mut self) -> Result<Vec<Action>, String> {
        let mut actions = Vec::new();
        loop {
            actions.push(self.parse_stmt()?);
            if matches!(self.peek(), Token::Semi) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(actions)
    }

    fn parse_stmt(&mut self) -> Result<Action, String> {
        let name = match self.advance() {
            Token::Ident(n) => n,
            other => return Err(format!("expected variable name, got {:?}", other)),
        };
        match self.advance() {
            Token::Assign => Ok(Action::SetVar { name, value: self.parse_expr()?}),
            Token::PlusAssign => Ok(Action::ModVar { name, op: MathOp::Add, operand: self.parse_expr()?}),
            Token::MinusAssign => Ok(Action::ModVar { name, op: MathOp::Sub, operand: self.parse_expr()?}),
            Token::StarAssign => Ok(Action::ModVar { name, op: MathOp::Mul, operand: self.parse_expr()?}),
            Token::SlashAssign => Ok(Action::ModVar { name, op: MathOp::Div, operand: self.parse_expr()?}),
            other => Err(format!("expected assignment operator, got {:?}", other)),
        }
    }
}

pub fn parse_condition(input: &str) -> Result<Condition, String> {
    let tokens = Lexer::new(input).tokenize()?;
    Parser::new(tokens).parse_condition()
}

pub fn parse_action(input: &str) -> Result<Vec<Action>, String> {
    let tokens = Lexer::new(input).tokenize()?;
    Parser::new(tokens).parse_stmts()
}
