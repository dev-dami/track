use std::collections::HashMap;

use crate::ast::{BinOp, Expr, TrackType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VariableState {
    Active,
    Locked,
    Spent,
}

pub struct LinearChecker {
    pub registry: HashMap<String, VariableState>,
    pub types: HashMap<String, TrackType>,
    pub functions: HashMap<String, Option<TrackType>>,
}

fn is_copy_type(ty: &TrackType) -> bool {
    match ty {
        TrackType::I32
        | TrackType::U32
        | TrackType::I64
        | TrackType::U64
        | TrackType::Bool
        | TrackType::Ptr(_) => true,
        _ => false,
    }
}

impl LinearChecker {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
            types: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn declare(&mut self, name: String) {
        self.registry.insert(name, VariableState::Active);
    }

    pub fn is_copy_var(&self, name: &str) -> bool {
        if let Some(ty) = self.types.get(name) {
            is_copy_type(ty)
        } else {
            false
        }
    }

    pub fn read_or_move(&mut self, name: &str) -> Result<(), String> {
        match self.registry.get(name) {
            Some(VariableState::Spent) => Err(format!(
                "Compile Error: Use-after-free! Resource '{}' is already spent.",
                name
            )),
            Some(VariableState::Locked) => Err(format!(
                "Compile Error: Resource '{}' is frozen inside a lens block.",
                name
            )),
            Some(VariableState::Active) => {
                if !self.is_copy_var(name) {
                    self.registry.insert(name.to_string(), VariableState::Spent);
                }
                Ok(())
            }
            None => Err(format!(
                "Compile Error: Undeclared variable '{}'.",
                name
            )),
        }
    }

    pub fn enter_lens(&mut self, name: &str) -> Result<(), String> {
        if self.registry.get(name) == Some(&VariableState::Active) {
            self.registry.insert(name.to_string(), VariableState::Locked);
            Ok(())
        } else {
            Err(format!(
                "Compile Error: Cannot create lens. '{}' is not Active.",
                name
            ))
        }
    }

    pub fn exit_lens(&mut self, name: &str) {
        if self.registry.get(name) == Some(&VariableState::Locked) {
            self.registry.insert(name.to_string(), VariableState::Active);
        }
    }

    pub fn check_program(&mut self, program: &[Expr]) -> Result<(), String> {
        // Collect function signatures first
        for stmt in program {
            if let Expr::FnDef { name, return_type, .. } = stmt {
                self.functions.insert(name.clone(), return_type.clone());
            }
        }
        // print is built-in and returns Void
        self.functions.insert("print".to_string(), Some(TrackType::Void));

        for stmt in program {
            self.check_expr(stmt)?;
        }
        Ok(())
    }

    fn infer_type(&self, expr: &Expr) -> Option<TrackType> {
        match expr {
            Expr::IntLiteral(_) => Some(TrackType::I32),
            Expr::BoolLiteral(_) => Some(TrackType::Bool),
            Expr::StringLiteral(_) => Some(TrackType::Ptr(Box::new(TrackType::I32))),
            Expr::Variable(name) => self.types.get(name).cloned(),
            Expr::BinaryOp { op, left, .. } => {
                if is_comparison(op) {
                    Some(TrackType::Bool)
                } else {
                    self.infer_type(left)
                }
            }
            Expr::UnaryOp { op, expr } => {
                match op {
                    crate::ast::UnaryOp::Not => Some(TrackType::Bool),
                    crate::ast::UnaryOp::Neg => self.infer_type(expr),
                    crate::ast::UnaryOp::Deref => {
                        if let Some(TrackType::Ptr(inner)) = self.infer_type(expr) {
                            Some(*inner)
                        } else {
                            None
                        }
                    }
                }
            }
            Expr::ArrayLiteral { elements } => {
                let elem_type = elements.first().and_then(|e| self.infer_type(e)).unwrap_or(TrackType::I32);
                Some(TrackType::Array(Box::new(elem_type), elements.len()))
            }
            Expr::ArrayIndex { target, .. } => {
                match self.infer_type(target) {
                    Some(TrackType::Array(inner, _)) => Some(*inner),
                    Some(TrackType::Ptr(inner)) => Some(*inner),
                    _ => None,
                }
            }
            Expr::AddressOf { target } => {
                self.infer_type(target).map(|t| TrackType::Ptr(Box::new(t)))
            }
            Expr::StructInitialization { ty_name, .. } => {
                Some(TrackType::Custom(ty_name.clone()))
            }
            Expr::FunctionCall { name, .. } => {
                self.functions.get(name).cloned().flatten()
            }
            Expr::LensBlock { body, .. } => {
                body.last().and_then(|e| self.infer_type(e))
            }
            Expr::IfElse { then_body, .. } => {
                then_body.last().and_then(|e| self.infer_type(e))
            }
            Expr::WhileLoop { .. } => Some(TrackType::Void),
            Expr::Return { .. } => Some(TrackType::Void),
            Expr::Assign { .. } => Some(TrackType::Void),
            Expr::FnDef { .. } => Some(TrackType::Void),
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::IntLiteral(_) | Expr::StringLiteral(_) | Expr::BoolLiteral(_) => Ok(()),

            Expr::Variable(name) => {
                self.read_or_move(name)?;
                Ok(())
            }

            Expr::BinaryOp { left, right, .. } => {
                // Binary ops borrow both sides (copy semantics for primitives)
                self.check_borrow(left)?;
                self.check_borrow(right)?;
                Ok(())
            }

            Expr::UnaryOp { expr, .. } => self.check_expr(expr),

            Expr::ArrayLiteral { elements } => {
                for elem in elements {
                    self.check_expr(elem)?;
                }
                Ok(())
            }

            Expr::ArrayIndex { target, index } => {
                // Array index borrows both target and index
                self.check_borrow(target)?;
                self.check_borrow(index)?;
                Ok(())
            }

            Expr::AddressOf { target } => {
                // &expr borrows but doesn't consume — check without moving
                self.check_borrow(target)?;
                Ok(())
            }

            Expr::StructInitialization { fields, .. } => {
                for (_, fval) in fields {
                    self.check_expr(fval)?;
                }
                Ok(())
            }

            Expr::FunctionCall { name, args } => {
                if name == "__assign" {
                    if let Some(Expr::Variable(target)) = args.first() {
                        self.declare(target.clone());
                        if args.len() > 1 {
                            self.check_expr(&args[1])?;
                            if let Some(ty) = self.infer_type(&args[1]) {
                                self.types.insert(target.clone(), ty);
                            }
                        }
                        Ok(())
                    } else {
                        Err("Compile Error: __assign requires a variable target".to_string())
                    }
                } else {
                    for arg in args {
                        self.check_expr(arg)?;
                    }
                    Ok(())
                }
            }

            Expr::LensBlock {
                target,
                lens_name,
                body,
            } => {
                self.enter_lens(target)?;
                self.declare(lens_name.clone());
                if let Some(ty) = self.types.get(target).cloned() {
                    self.types.insert(lens_name.clone(), ty);
                }
                for expr in body {
                    self.check_expr(expr)?;
                }
                self.exit_lens(target);
                Ok(())
            }

            Expr::IfElse {
                condition,
                then_body,
                else_body,
            } => {
                self.check_expr(condition)?;

                // Clone state for each branch
                let pre_if = self.registry.clone();
                let pre_if_types = self.types.clone();

                // Check then branch
                let mut then_state = pre_if.clone();
                let mut then_types = pre_if_types.clone();
                std::mem::swap(&mut self.registry, &mut then_state);
                std::mem::swap(&mut self.types, &mut then_types);
                for stmt in then_body {
                    self.check_expr(stmt)?;
                }
                let then_end = self.registry.clone();

                // Check else branch
                let mut else_state = pre_if.clone();
                let mut else_types = pre_if_types.clone();
                std::mem::swap(&mut self.registry, &mut else_state);
                std::mem::swap(&mut self.types, &mut else_types);
                for stmt in else_body {
                    self.check_expr(stmt)?;
                }
                let else_end = self.registry.clone();

                // CFG Merge: both branches must leave variables in identical states
                let mut merged = HashMap::new();
                let mut merged_types = HashMap::new();
                for (name, _) in &pre_if {
                    let then_s = then_end.get(name).copied().unwrap_or(VariableState::Spent);
                    let else_s = else_end.get(name).copied().unwrap_or(VariableState::Spent);

                    if then_s != else_s {
                        return Err(format!(
                            "Compile Error: Resource '{}' has inconsistent state after if/else. \
                             Then branch: {:?}, Else branch: {:?}",
                            name, then_s, else_s
                        ));
                    }
                    merged.insert(name.clone(), then_s);
                    if let Some(ty) = pre_if_types.get(name) {
                        merged_types.insert(name.clone(), ty.clone());
                    }
                }

                self.registry = merged;
                self.types = merged_types;
                Ok(())
            }

            Expr::WhileLoop { condition, body } => {
                // Check condition
                self.check_expr(condition)?;

                // Run body once to check for linear violations
                let pre_loop = self.registry.clone();
                let pre_loop_types = self.types.clone();
                for stmt in body {
                    self.check_expr(stmt)?;
                }

                // Restore pre-loop state (loop continues)
                self.registry = pre_loop;
                self.types = pre_loop_types;
                Ok(())
            }

            Expr::Return { value } => {
                if let Some(val) = value {
                    self.check_expr(val)?;
                }
                Ok(())
            }

            Expr::Assign { target, value } => {
                self.check_expr(value)?;
                // For simple variable assignment, re-activate the variable
                if let Expr::Variable(name) = target.as_ref() {
                    self.registry.insert(name.clone(), VariableState::Active);
                    if let Some(ty) = self.infer_type(value) {
                        self.types.insert(name.clone(), ty);
                    }
                }
                Ok(())
            }

            Expr::FnDef {
                params,
                body,
                ..
            } => {
                // Enter function scope
                let saved_registry = self.registry.clone();
                let saved_types = self.types.clone();
                for (name, ty) in params {
                    self.declare(name.clone());
                    self.types.insert(name.clone(), ty.clone());
                }
                for stmt in body {
                    self.check_expr(stmt)?;
                }
                // Restore outer scope
                self.registry = saved_registry;
                self.types = saved_types;
                Ok(())
            }
        }
    }

    /// Check an expression without consuming it (for & borrows)
    fn check_borrow(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Variable(name) => {
                match self.registry.get(name) {
                    Some(VariableState::Spent) => Err(format!(
                        "Compile Error: Cannot borrow spent resource '{}'.",
                        name
                    )),
                    Some(VariableState::Locked) => Err(format!(
                        "Compile Error: Cannot borrow locked resource '{}'.",
                        name
                    )),
                    Some(VariableState::Active) => Ok(()), // borrow, don't consume
                    None => Err(format!(
                        "Compile Error: Undeclared variable '{}'.",
                        name
                    )),
                }
            }
            _ => self.check_expr(expr),
        }
    }
}

fn is_comparison(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Lte | BinOp::Gte
    )
}
