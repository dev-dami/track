use crate::ast::{BinOp, Expr, UnaryOp};
use crate::lexer::Token;
use super::Parser;

impl Parser {
    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.peek() == Some(&Token::PipePipe) || self.peek() == Some(&Token::Pipe) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        while self.peek() == Some(&Token::AmpAmp) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise()?;
        loop {
            let op = match self.peek() {
                Some(Token::EqEq) => BinOp::Eq,
                Some(Token::Neq) => BinOp::Neq,
                Some(Token::Lt) => BinOp::Lt,
                Some(Token::Gt) => BinOp::Gt,
                Some(Token::Lte) => BinOp::Lte,
                Some(Token::Gte) => BinOp::Gte,
                _ => break,
            };
            self.advance();
            let right = self.parse_bitwise()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bitwise(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift()?;
        loop {
            let op = match self.peek() {
                Some(Token::Amp) if self.tokens.get(self.pos + 1).map_or(false, |(t, _)| !matches!(t, Token::Amp)) => BinOp::BitAnd,
                Some(Token::PipePipe) => break,
                _ => break,
            };
            self.advance();
            let right = self.parse_shift()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Some(Token::Shl) => BinOp::Shl,
                Some(Token::Shr) => BinOp::Shr,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                Some(Token::Percent) => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            Some(Token::Bang) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            Some(Token::Star) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Deref,
                    expr: Box::new(expr),
                })
            }
            Some(Token::Amp) => {
                self.advance();
                // &mut x and &x are both address-of; mut is consumed but not stored
                if self.peek() == Some(&Token::Mut) {
                    self.advance();
                }
                let expr = self.parse_unary()?;
                Ok(Expr::AddressOf {
                    target: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Some(Token::Dot) => {
                    // UFCS: expr.method(args)  =>  method(expr, args)
                    self.advance();
                    let method_name = self.expect_ident()?;
                    self.expect(&Token::LParen)?;
                    let mut args = vec![expr];
                    if self.peek() != Some(&Token::RParen) {
                        args.push(self.parse_expr()?);
                        while self.peek() == Some(&Token::Comma) {
                            self.advance();
                            args.push(self.parse_expr()?);
                        }
                    }
                    self.expect(&Token::RParen)?;
                    expr = Expr::FunctionCall { name: method_name, args };
                }
                Some(Token::LBracket) => {
                    // Array index: expr[index]
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::ArrayIndex {
                        target: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                Some(Token::LParen) => {
                    // Function call: expr(args)
                    if let Expr::Variable(ref name) = expr {
                        let name = name.clone();
                        self.advance();
                        let mut args = Vec::new();
                        if self.peek() != Some(&Token::RParen) {
                            args.push(self.parse_expr()?);
                            while self.peek() == Some(&Token::Comma) {
                                self.advance();
                                args.push(self.parse_expr()?);
                            }
                        }
                        self.expect(&Token::RParen)?;
                        expr = Expr::FunctionCall { name, args };
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::Int(val)) => {
                self.advance();
                Ok(Expr::IntLiteral(val))
            }
            Some(Token::Str(s)) => {
                self.advance();
                Ok(Expr::StringLiteral(s))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::BoolLiteral(true))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::BoolLiteral(false))
            }
            Some(Token::If) => self.parse_if_expr(),
            Some(Token::At) => self.parse_macro_call(),
            Some(Token::Match) => self.parse_match(),
            Some(Token::Ident(_)) => {
                let name = self.parse_namespaced_ident()?;

                // Struct init: TypeName { fields }
                if self.allow_struct && self.peek() == Some(&Token::LBrace) {
                    self.advance();
                    let mut fields = Vec::new();
                    while self.peek() != Some(&Token::RBrace) {
                        let field_name = self.expect_ident()?;
                        self.expect(&Token::Colon)?;
                        let field_val = self.parse_expr()?;
                        fields.push((field_name, field_val));
                        if self.peek() == Some(&Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RBrace)?;
                    return Ok(Expr::StructInitialization { ty_name: name, fields });
                }

                Ok(Expr::Variable(name))
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Some(Token::LBracket) => {
                // Array literal: [expr, expr, ...]
                self.advance();
                let mut elements = Vec::new();
                if self.peek() != Some(&Token::RBracket) {
                    elements.push(self.parse_expr()?);
                    while self.peek() == Some(&Token::Comma) {
                        self.advance();
                        elements.push(self.parse_expr()?);
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::ArrayLiteral { elements })
            }
            other => Err(format!("Parser error: unexpected token {:?}", other)),
        }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
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
                vec![self.parse_if_expr()?]
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

    pub fn parse_namespaced_ident(&mut self) -> Result<String, String> {
        let mut name = self.expect_ident()?;
        while self.peek() == Some(&Token::ColonColon) {
            self.advance();
            let sub_name = self.expect_ident()?;
            name = format!("{}::{}", name, sub_name);
        }
        Ok(name)
    }

    fn parse_macro_call(&mut self) -> Result<Expr, String> {
        self.advance(); // consume '@'
        let name = self.expect_ident()?;
        
        let mut args = Vec::new();
        let mut body = None;

        if self.peek() == Some(&Token::LParen) {
            self.advance();
            if self.peek() != Some(&Token::RParen) {
                args.push(self.parse_expr()?);
                while self.peek() == Some(&Token::Comma) {
                    self.advance();
                    args.push(self.parse_expr()?);
                }
            }
            self.expect(&Token::RParen)?;
        }

        if self.peek() == Some(&Token::LBrace) {
            self.advance();
            let mut block_body = Vec::new();
            while self.peek() != Some(&Token::RBrace) {
                block_body.push(self.parse_statement()?);
            }
            self.expect(&Token::RBrace)?;
            body = Some(block_body);
        }

        Ok(Expr::MacroCall {
            name,
            args,
            body,
        })
    }

    fn parse_match(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'match'
        let saved_allow = self.allow_struct;
        self.allow_struct = false;
        let target = self.parse_expr()?;
        self.allow_struct = saved_allow;
        self.expect(&Token::LBrace)?;
        
        let mut arms = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let pattern = self.parse_pattern()?;
            
            let guard = if self.peek() == Some(&Token::If) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };
            
            self.expect(&Token::FatArrow)?;
            let body = self.parse_statement()?;
            
            arms.push(crate::ast::MatchArm {
                pattern,
                guard,
                body,
            });

            if self.peek() == Some(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;

        Ok(Expr::Match {
            target: Box::new(target),
            arms,
        })
    }

    fn parse_pattern(&mut self) -> Result<crate::ast::Pattern, String> {
        match self.peek() {
            Some(Token::Underscore) => {
                self.advance();
                Ok(crate::ast::Pattern::Wildcard)
            }
            Some(Token::Ident(_)) => {
                let name = self.parse_namespaced_ident()?;
                if name.contains("::") {
                    let parts: Vec<&str> = name.split("::").collect();
                    let enum_or_union = parts[0].to_string();
                    let variant = parts[1].to_string();
                    
                    let binding = if self.peek() == Some(&Token::LParen) {
                        self.advance();
                        let b = self.expect_ident()?;
                        self.expect(&Token::RParen)?;
                        Some(b)
                    } else {
                        None
                    };
                    
                    Ok(crate::ast::Pattern::Variant {
                        enum_or_union,
                        variant,
                        binding,
                    })
                } else {
                    Ok(crate::ast::Pattern::Ident(name))
                }
            }
            other => Err(format!("Expected pattern, got {:?}", other)),
        }
    }
}
