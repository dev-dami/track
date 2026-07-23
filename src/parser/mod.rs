use crate::ast::{Expr, TrackType};
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<(Token, std::ops::Range<usize>)>,
    pos: usize,
    pub allow_struct: bool,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, std::ops::Range<usize>)>, _source: String) -> Self {
        Self {
            tokens,
            pos: 0,
            allow_struct: true,
        }
    }

    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    pub fn advance(&mut self) -> Option<(Token, std::ops::Range<usize>)> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    pub fn expect(&mut self, expected: &Token) -> Result<(), String> {
        match self.advance() {
            Some((ref tok, _)) if tok == expected => Ok(()),
            other => Err(format!(
                "Parser error: expected {:?}, got {:?}",
                expected,
                other.map(|(t, _)| t)
            )),
        }
    }

    pub fn expect_ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Some((Token::Ident(name), _)) => Ok(name),
            other => Err(format!(
                "Parser error: expected identifier, got {:?}",
                other.map(|(t, _)| t)
            )),
        }
    }

    pub fn parse_type(&mut self) -> Result<TrackType, String> {
        match self.peek().cloned() {
            Some(Token::TyI32) => {
                self.advance();
                Ok(TrackType::I32)
            }
            Some(Token::TyU32) => {
                self.advance();
                Ok(TrackType::U32)
            }
            Some(Token::TyI64) => {
                self.advance();
                Ok(TrackType::I64)
            }
            Some(Token::TyU64) => {
                self.advance();
                Ok(TrackType::U64)
            }
            Some(Token::TyBool) => {
                self.advance();
                Ok(TrackType::Bool)
            }
            Some(Token::TyVoid) => {
                self.advance();
                Ok(TrackType::Void)
            }
            Some(Token::TyPtr) => {
                self.advance();
                self.expect(&Token::Lt)?;
                let inner = self.parse_type()?;
                self.expect(&Token::Gt)?;
                Ok(TrackType::Ptr(Box::new(inner)))
            }
            Some(Token::Amp) => {
                self.advance();
                let inner = self.parse_type()?;
                Ok(TrackType::Ref(Box::new(inner)))
            }
            Some(Token::Ident(name)) => {
                self.advance();
                Ok(TrackType::Custom(name))
            }
            Some(Token::LBracket) => {
                self.advance();
                let elem_type = self.parse_type()?;
                self.expect(&Token::Semicolon)?;
                match self.advance() {
                    Some((Token::Int(n), _)) => {
                        self.expect(&Token::RBracket)?;
                        Ok(TrackType::Array(Box::new(elem_type), n as usize))
                    }
                    other => Err(format!(
                        "Parser error: expected array size, got {:?}",
                        other.map(|(t, _)| t)
                    )),
                }
            }
            other => Err(format!("Parser error: expected type, got {:?}", other)),
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Expr>, String> {
        let mut stmts = Vec::new();
        while self.peek().is_some() {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }
}

mod expr;
mod stmt;
