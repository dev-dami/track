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
            Some(Token::For) => self.parse_for()?,
            Some(Token::Return) => self.parse_return()?,
            Some(Token::Import) | Some(Token::Use) => self.parse_import()?,

            Some(Token::Const) => self.parse_const()?,
            Some(Token::TypeDef) => self.parse_type_alias()?,
            Some(Token::Struct) => self.parse_struct()?,
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
        let mutable = if self.peek() == Some(&Token::Mut) {
            self.advance();
            true
        } else {
            false
        };

        if self.peek() == Some(&Token::LParen) {
            let pattern = self.parse_pattern()?;
            self.expect(&Token::Eq)?;
            let val = self.parse_expr()?;
            Ok(Expr::LetDestructure {
                pattern,
                mutable,
                value: Box::new(val),
            })
        } else {
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
                mutable,
                ty,
                value: Box::new(val),
            })
        }
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

        // Optional generic parameters: fn name<T, U>(...)
        let generics = if self.peek() == Some(&Token::Lt) {
            self.advance();
            let mut params = Vec::new();
            loop {
                params.push(self.expect_ident()?);
                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(&Token::Gt)?;
            params
        } else {
            Vec::new()
        };

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
            generics,
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

    fn parse_for(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'for'
        let var = self.expect_ident()?;
        self.expect(&Token::In)?;
        self.allow_struct = false;
        let iter = self.parse_expr();
        self.allow_struct = true;
        let iter = iter?;
        self.expect(&Token::LBrace)?;
        let mut body = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            body.push(self.parse_statement()?);
        }
        self.expect(&Token::RBrace)?;

        Ok(Expr::ForIn {
            var,
            iter: Box::new(iter),
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

    fn parse_import(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'import'
        let path = match self.advance() {
            Some((Token::Str(s), _)) => s,
            other => {
                return Err(format!(
                    "Expected string path after import, got {:?}",
                    other.map(|(t, _)| t)
                ))
            }
        };

        let alias = if self.peek() == Some(&Token::As) {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };

        let imports = if self.peek() == Some(&Token::ColonColon) {
            self.advance();
            self.expect(&Token::LBrace)?;
            let mut items = Vec::new();
            while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
                let item = self.expect_ident()?;
                items.push(item);
                if self.peek() == Some(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(&Token::RBrace)?;
            Some(items)
        } else {
            None
        };

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

    fn parse_type_alias(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'type'
        let name = self.expect_ident()?;
        self.expect(&Token::Eq)?;
        let target = self.parse_type()?;
        Ok(Expr::TypeAlias { name, target })
    }

    fn parse_struct(&mut self) -> Result<Expr, String> {
        self.advance(); // consume 'struct'
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            let field_name = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let field_type = self.parse_type()?;
            fields.push((field_name, field_type));
            if self.peek() == Some(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Expr::TypeAlias {
            name,
            target: crate::ast::TrackType::Custom("Struct".to_string()),
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
