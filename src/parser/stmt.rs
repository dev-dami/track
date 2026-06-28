use crate::ast::Expr;
use crate::lexer::Token;
use super::Parser;

impl Parser {
    pub fn parse_statement(&mut self) -> Result<Expr, String> {
        let expr = match self.peek() {
            Some(Token::Let) => self.parse_let()?,
            Some(Token::With) => self.parse_with()?,
            Some(Token::Fn) => self.parse_fn()?,
            Some(Token::If) => self.parse_if()?,
            Some(Token::While) => self.parse_while()?,
            Some(Token::Return) => self.parse_return()?,
            _ => {
                let expr = self.parse_expr()?;
                // Handle assignment: expr = expr
                if self.peek() == Some(&Token::Eq) {
                    self.advance();
                    let value = self.parse_expr()?;
                    Expr::Assign {
                        target: Box::new(expr),
                        value: Box::new(value),
                    }
                } else {
                    expr
                }
            }
        };
        if self.peek() == Some(&Token::Semicolon) {
            self.advance();
        }
        Ok(expr)
    }

    fn parse_let(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'let'
        let _mutable = if self.peek() == Some(&Token::Mut) {
            self.advance();
            true
        } else {
            false
        };
        let name = self.expect_ident()?;

        // Optional type annotation: let x: Type = expr
        let _ty = if self.peek() == Some(&Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&Token::Eq)?;
        let val = self.parse_expr()?;
        Ok(Expr::FunctionCall {
            name: "__assign".to_string(),
            args: vec![Expr::Variable(name), val],
        })
    }

    fn parse_with(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'with'
        let target = self.expect_ident()?;
        self.expect(&Token::Arrow)?;
        let lens_name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;

        let mut body = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            body.push(self.parse_statement()?);
        }
        self.expect(&Token::RBrace)?;

        Ok(Expr::LensBlock { target, lens_name, body })
    }

    fn parse_fn(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'fn'
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;

        let mut params = Vec::new();
        if self.peek() != Some(&Token::RParen) {
            loop {
                let param_name = self.expect_ident()?;
                self.expect(&Token::Colon)?;
                let param_type = self.parse_type()?;
                params.push((param_name, param_type));
                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&Token::RParen)?;

        // Optional return type
        let return_type = if self.peek() == Some(&Token::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&Token::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            body.push(self.parse_statement()?);
        }
        self.expect(&Token::RBrace)?;

        Ok(Expr::FnDef {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_if(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'if'
        self.allow_struct = false;
        let condition = self.parse_expr();
        self.allow_struct = true;
        let condition = condition?;
        self.expect(&Token::LBrace)?;
        let mut then_body = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            then_body.push(self.parse_statement()?);
        }
        self.expect(&Token::RBrace)?;

        let else_body = if self.peek() == Some(&Token::Else) {
            self.advance();
            if self.peek() == Some(&Token::If) {
                // else if => single-element else body with another IfElse
                vec![self.parse_if()?]
            } else {
                self.expect(&Token::LBrace)?;
                let mut body = Vec::new();
                while self.peek() != Some(&Token::RBrace) {
                    body.push(self.parse_statement()?);
                }
                self.expect(&Token::RBrace)?;
                body
            }
        } else {
            Vec::new()
        };

        Ok(Expr::IfElse {
            condition: Box::new(condition),
            then_body,
            else_body,
        })
    }

    fn parse_while(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'while'
        self.allow_struct = false;
        let condition = self.parse_expr();
        self.allow_struct = true;
        let condition = condition?;
        self.expect(&Token::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            body.push(self.parse_statement()?);
        }
        self.expect(&Token::RBrace)?;

        Ok(Expr::WhileLoop {
            condition: Box::new(condition),
            body,
        })
    }

    fn parse_return(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'return'
        let value = if self.peek() == Some(&Token::Semicolon) || self.peek().is_none() {
            None
        } else {
            Some(Box::new(self.parse_expr()?))
        };
        Ok(Expr::Return { value })
    }
}
