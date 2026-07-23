use super::Parser;
use crate::ast::Expr;
use crate::lexer::Token;

impl Parser {
    pub fn parse_statement(&mut self) -> Result<Expr, String> {
        let expr = match self.peek() {
            Some(Token::Let) => self.parse_let()?,
            Some(Token::With) => self.parse_with()?,
            Some(Token::Fn) => self.parse_fn()?,
            Some(Token::If) => self.parse_if()?,
            Some(Token::While) => self.parse_while()?,
            Some(Token::Return) => self.parse_return()?,
            Some(Token::AtUse) => self.parse_use()?,
            Some(Token::Const) => self.parse_const()?,
            Some(Token::AtMacro) => self.parse_macro_def()?,
            Some(Token::Enum) => self.parse_enum()?,
            Some(Token::Union) => self.parse_union()?,
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

        let ty = if self.peek() == Some(&Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&Token::Eq)?;
        let val = self.parse_expr()?;
        Ok(Expr::LetDef {
            name,
            ty,
            value: Box::new(val),
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

        Ok(Expr::LensBlock {
            target,
            lens_name,
            body,
        })
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

    fn parse_use(&mut self) -> Result<Expr, String> {
        self.advance(); // consume '@use'
        self.expect(&Token::LParen)?;
        let full_path = match self.advance() {
            Some((Token::Str(s), _)) => s,
            other => {
                return Err(format!(
                    "Expected string path after @use(, got {:?}",
                    other.map(|(t, _)| t)
                ))
            }
        };
        self.expect(&Token::RParen)?;

        // Optional alias: as alias_name
        let alias = if self.peek() == Some(&Token::As) {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };

        // Parse full_path to split actual path and imports (e.g. "path::{a, b}")
        let mut path = full_path.clone();
        let mut imports = None;

        if let Some(idx) = full_path.find("::{") {
            path = full_path[..idx].to_string();
            let imports_str = &full_path[idx + 3..];
            if let Some(end_idx) = imports_str.find('}') {
                let items: Vec<String> = imports_str[..end_idx]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                imports = Some(items);
            }
        }

        Ok(Expr::Use {
            path,
            imports,
            alias,
        })
    }

    fn parse_const(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'const'
        let name = self.expect_ident()?;
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(Expr::ConstDef {
            name,
            value: Box::new(value),
        })
    }

    fn parse_macro_def(&mut self) -> Result<Expr, String> {
        self.advance(); // consume '@macro'
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        if self.peek() != Some(&Token::RParen) {
            let param_name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let param_ty = self.parse_type()?;
            params.push((param_name, param_ty));
            while self.peek() == Some(&Token::Comma) {
                self.advance();
                let param_name = self.expect_ident()?;
                self.expect(&Token::Colon)?;
                let param_ty = self.parse_type()?;
                params.push((param_name, param_ty));
            }
        }
        self.expect(&Token::RParen)?;

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

        Ok(Expr::MacroDef {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_enum(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'enum'
        let name = self.expect_ident()?;

        let underlying_type = if self.peek() == Some(&Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&Token::LBrace)?;
        let mut variants = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let variant_name = self.expect_ident()?;
            let variant_value = if self.peek() == Some(&Token::Eq) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };
            variants.push((variant_name, variant_value));
            if self.peek() == Some(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;

        Ok(Expr::EnumDef {
            name,
            underlying_type,
            variants,
        })
    }

    fn parse_union(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'union'
        let name = self.expect_ident()?;

        if self.peek() == Some(&Token::LParen) {
            self.advance();
            while self.peek() != Some(&Token::RParen) {
                self.advance();
            }
            self.expect(&Token::RParen)?;
        }

        self.expect(&Token::LBrace)?;
        let mut variants = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let variant_name = self.expect_ident()?;
            let variant_type = if self.peek() == Some(&Token::LParen) {
                self.advance();
                let ty = self.parse_type()?;
                self.expect(&Token::RParen)?;
                Some(ty)
            } else {
                None
            };
            variants.push((variant_name, variant_type));
            if self.peek() == Some(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;

        Ok(Expr::UnionDef { name, variants })
    }
}
