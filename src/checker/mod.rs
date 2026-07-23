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
    pub borrows: HashMap<String, Vec<String>>,
    pub lens_locked: std::collections::HashSet<String>,
    pub current_params: std::collections::HashSet<String>,
    pub current_return_type: Option<TrackType>,
}

fn is_copy_type(ty: &TrackType) -> bool {
    matches!(
        ty,
        TrackType::I32
            | TrackType::U32
            | TrackType::I64
            | TrackType::U64
            | TrackType::Bool
            | TrackType::Ref(_)
    )
}

impl Default for LinearChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl LinearChecker {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
            types: HashMap::new(),
            functions: HashMap::new(),
            borrows: HashMap::new(),
            lens_locked: std::collections::HashSet::new(),
            current_params: std::collections::HashSet::new(),
            current_return_type: None,
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
                "Compile Error: Resource '{}' is frozen (either locked in a lens or borrowed).",
                name
            )),
            Some(VariableState::Active) => {
                if !self.is_copy_var(name) {
                    self.registry.insert(name.to_string(), VariableState::Spent);
                }
                Ok(())
            }
            None => Err(format!("Compile Error: Undeclared variable '{}'.", name)),
        }
    }

    pub fn update_borrow_states(&mut self) {
        // Collect all variables that are borrowed by currently Active reference variables
        let mut borrowed_vars = std::collections::HashSet::new();
        for (name, state) in &self.registry {
            if *state == VariableState::Active {
                if let Some(ty) = self.types.get(name) {
                    if matches!(ty, TrackType::Ref(_)) {
                        if let Some(provs) = self.borrows.get(name) {
                            for p in provs {
                                borrowed_vars.insert(p.clone());
                            }
                        }
                    }
                }
            }
        }

        // Update registry states
        for (name, state) in self.registry.iter_mut() {
            let is_lens_locked = self.lens_locked.contains(name);
            let is_borrowed = borrowed_vars.contains(name);

            if is_lens_locked || is_borrowed {
                if *state == VariableState::Active {
                    *state = VariableState::Locked;
                }
            } else if *state == VariableState::Locked {
                *state = VariableState::Active;
            }
        }
    }

    pub fn enter_lens(&mut self, name: &str) -> Result<(), String> {
        if self.registry.get(name) == Some(&VariableState::Active) {
            self.lens_locked.insert(name.to_string());
            self.registry
                .insert(name.to_string(), VariableState::Locked);
            Ok(())
        } else {
            Err(format!(
                "Compile Error: Cannot create lens. '{}' is not Active.",
                name
            ))
        }
    }

    pub fn exit_lens(&mut self, name: &str) {
        self.lens_locked.remove(name);
        if self.registry.get(name) == Some(&VariableState::Locked) {
            self.registry
                .insert(name.to_string(), VariableState::Active);
        }
        self.update_borrow_states();
    }

    pub fn check_program(&mut self, program: &[Expr]) -> Result<(), String> {
        // Collect function signatures first
        for stmt in program {
            if let Expr::FnDef {
                name, return_type, ..
            } = stmt
            {
                self.functions.insert(name.clone(), return_type.clone());
            }
        }
        // print is built-in and returns Void
        self.functions
            .insert("print".to_string(), Some(TrackType::Void));

        // Memory functions
        self.functions.insert(
            "alloc".to_string(),
            Some(TrackType::Ptr(Box::new(TrackType::Custom(
                "u8".to_string(),
            )))),
        );
        self.functions
            .insert("memset".to_string(), Some(TrackType::Void));
        self.functions
            .insert("memcpy".to_string(), Some(TrackType::Void));
        self.functions
            .insert("memcmp".to_string(), Some(TrackType::I32));

        // String functions
        self.functions
            .insert("str_len".to_string(), Some(TrackType::U32));
        self.functions
            .insert("str_eq".to_string(), Some(TrackType::Bool));
        self.functions.insert(
            "str_from_literal".to_string(),
            Some(TrackType::Custom("Str".to_string())),
        );
        self.functions.insert(
            "str_concat".to_string(),
            Some(TrackType::Custom("Str".to_string())),
        );

        // Vec functions
        self.functions.insert(
            "vec_init".to_string(),
            Some(TrackType::Custom("Vec".to_string())),
        );
        self.functions
            .insert("vec_push".to_string(), Some(TrackType::Void));
        self.functions
            .insert("vec_get".to_string(), Some(TrackType::I32));
        self.functions
            .insert("vec_set".to_string(), Some(TrackType::Void));
        self.functions
            .insert("vec_pop".to_string(), Some(TrackType::I32));

        // I/O functions
        self.functions
            .insert("print_str".to_string(), Some(TrackType::Void));
        self.functions
            .insert("print_int".to_string(), Some(TrackType::Void));
        self.functions
            .insert("print_hex".to_string(), Some(TrackType::Void));
        self.functions.insert(
            "read_line".to_string(),
            Some(TrackType::Custom("Str".to_string())),
        );
        self.functions.insert(
            "file_open".to_string(),
            Some(TrackType::Ptr(Box::new(TrackType::Custom(
                "File".to_string(),
            )))),
        );
        self.functions.insert(
            "file_read_all".to_string(),
            Some(TrackType::Custom("Str".to_string())),
        );
        self.functions
            .insert("file_write".to_string(), Some(TrackType::Void));

        // Ring buffer functions
        self.functions.insert(
            "ring_init".to_string(),
            Some(TrackType::Custom("Ring".to_string())),
        );
        self.functions
            .insert("ring_push".to_string(), Some(TrackType::Bool));
        self.functions
            .insert("ring_pop".to_string(), Some(TrackType::I32));
        self.functions
            .insert("ring_peek".to_string(), Some(TrackType::I32));
        self.functions
            .insert("ring_full".to_string(), Some(TrackType::Bool));
        self.functions
            .insert("ring_empty".to_string(), Some(TrackType::Bool));
        self.functions
            .insert("ring_count".to_string(), Some(TrackType::U32));

        // Math functions
        self.functions
            .insert("math_abs".to_string(), Some(TrackType::I32));
        self.functions
            .insert("math_max".to_string(), Some(TrackType::I32));
        self.functions
            .insert("math_min".to_string(), Some(TrackType::I32));
        self.functions
            .insert("math_pow".to_string(), Some(TrackType::I64));
        self.functions
            .insert("math_sqrt".to_string(), Some(TrackType::I64));

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
            Expr::UnaryOp { op, expr } => match op {
                crate::ast::UnaryOp::Not => Some(TrackType::Bool),
                crate::ast::UnaryOp::Neg => self.infer_type(expr),
                crate::ast::UnaryOp::Deref => match self.infer_type(expr) {
                    Some(TrackType::Ptr(inner)) | Some(TrackType::Ref(inner)) => Some(*inner),
                    _ => None,
                },
            },
            Expr::ArrayLiteral { elements } => {
                let elem_type = elements
                    .first()
                    .and_then(|e| self.infer_type(e))
                    .unwrap_or(TrackType::I32);
                Some(TrackType::Array(Box::new(elem_type), elements.len()))
            }
            Expr::ArrayIndex { target, .. } => match self.infer_type(target) {
                Some(TrackType::Array(inner, _)) => Some(*inner),
                Some(TrackType::Ptr(inner)) => Some(*inner),
                Some(TrackType::Ref(inner)) => match *inner {
                    TrackType::Array(elem, _) => Some(*elem),
                    TrackType::Ptr(elem) => Some(*elem),
                    other => Some(other),
                },
                _ => None,
            },
            Expr::AddressOf { target } => {
                self.infer_type(target).map(|t| TrackType::Ref(Box::new(t)))
            }
            Expr::StructInitialization { ty_name, .. } => Some(TrackType::Custom(ty_name.clone())),
            Expr::FunctionCall { name, .. } => self.functions.get(name).cloned().flatten(),
            Expr::LensBlock { body, .. } => body.last().and_then(|e| self.infer_type(e)),
            Expr::IfElse { then_body, .. } => then_body.last().and_then(|e| self.infer_type(e)),
            Expr::WhileLoop { .. } => Some(TrackType::Void),
            Expr::Return { .. } => Some(TrackType::Void),
            Expr::Assign { .. } => Some(TrackType::Void),
            Expr::FnDef { .. } => Some(TrackType::Void),
            Expr::Use { .. } => Some(TrackType::Void),
            Expr::ConstDef { .. } => Some(TrackType::Void),
            Expr::MacroDef { .. } => Some(TrackType::Void),
            Expr::MacroCall { name, .. } => {
                if name == "now" {
                    Some(TrackType::I64)
                } else {
                    self.functions.get(name).cloned().flatten()
                }
            }
            Expr::LetDef { .. } => Some(TrackType::Void),
            Expr::EnumDef { .. } => Some(TrackType::Void),
            Expr::UnionDef { .. } => Some(TrackType::Void),
            Expr::Match { arms, .. } => arms.first().and_then(|arm| self.infer_type(&arm.body)),
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
                // These are compiler-inserted automatically — users cannot call them directly
                const FORBIDDEN: &[&str] = &["free", "str_free", "vec_free", "file_close"];
                if FORBIDDEN.contains(&name.as_str()) {
                    return Err(format!(
                        "Compile Error: '{}' is managed automatically by the compiler. \
                         Linear types are freed at their spend points — do not call this directly.",
                        name
                    ));
                }
                for arg in args {
                    self.check_expr(arg)?;
                }
                Ok(())
            }

            Expr::LetDef { name, ty, value } => {
                self.check_expr(value)?;
                let inferred = self.infer_type(value);
                let final_ty = if let Some(annotated_ty) = ty {
                    annotated_ty.clone()
                } else {
                    inferred.unwrap_or(TrackType::Void)
                };

                self.declare(name.clone());
                self.types.insert(name.clone(), final_ty.clone());

                if matches!(final_ty, TrackType::Ref(_)) {
                    let prov = self.get_provenance(value);
                    self.borrows.insert(name.clone(), prov);
                }
                self.update_borrow_states();
                Ok(())
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
                let pre_if_borrows = self.borrows.clone();

                // Check then branch
                let mut then_state = pre_if.clone();
                let mut then_types = pre_if_types.clone();
                let mut then_borrows = pre_if_borrows.clone();
                std::mem::swap(&mut self.registry, &mut then_state);
                std::mem::swap(&mut self.types, &mut then_types);
                std::mem::swap(&mut self.borrows, &mut then_borrows);
                for stmt in then_body {
                    self.check_expr(stmt)?;
                    self.update_borrow_states();
                }
                let then_end = self.registry.clone();
                let then_end_borrows = self.borrows.clone();

                // Check else branch
                let mut else_state = pre_if.clone();
                let mut else_types = pre_if_types.clone();
                let mut else_borrows = pre_if_borrows.clone();
                std::mem::swap(&mut self.registry, &mut else_state);
                std::mem::swap(&mut self.types, &mut else_types);
                std::mem::swap(&mut self.borrows, &mut else_borrows);
                for stmt in else_body {
                    self.check_expr(stmt)?;
                    self.update_borrow_states();
                }
                let else_end = self.registry.clone();
                let else_end_borrows = self.borrows.clone();

                // CFG Merge: both branches must leave variables in identical states
                let mut merged = HashMap::new();
                let mut merged_types = HashMap::new();
                let mut merged_borrows = HashMap::new();
                for name in pre_if.keys() {
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
                    // For borrows, take union or then branch (they must match or merge)
                    if let Some(b) = then_end_borrows
                        .get(name)
                        .or_else(|| else_end_borrows.get(name))
                    {
                        merged_borrows.insert(name.clone(), b.clone());
                    }
                }

                self.registry = merged;
                self.types = merged_types;
                self.borrows = merged_borrows;
                self.update_borrow_states();
                Ok(())
            }

            Expr::WhileLoop { condition, body } => {
                // Check condition
                self.check_expr(condition)?;

                // Run body to check for linear violations inside the loop
                let mut pre_loop = self.registry.clone();
                let pre_loop_types = self.types.clone();
                let pre_loop_borrows = self.borrows.clone();

                for stmt in body {
                    self.check_expr(stmt)?;
                    self.update_borrow_states();
                }

                // Propagate loop body state: variables consumed inside loop become Spent post-loop
                for (name, pre_state) in &pre_loop.clone() {
                    if *pre_state == VariableState::Active {
                        if let Some(post_state) = self.registry.get(name) {
                            if *post_state == VariableState::Spent {
                                pre_loop.insert(name.clone(), VariableState::Spent);
                            }
                        }
                    }
                }

                self.registry = pre_loop;
                self.types = pre_loop_types;
                self.borrows = pre_loop_borrows;
                self.update_borrow_states();
                Ok(())
            }

            Expr::Return { value } => {
                if let Some(val) = value {
                    self.check_expr(val)?;
                    // Escape check
                    if let Some(TrackType::Ref(_)) = self.current_return_type {
                        let prov = self.get_provenance(val);
                        for v in &prov {
                            if !self.current_params.contains(v) {
                                return Err(format!(
                                    "Compile Error: Cannot return reference to local variable '{}' (escapes function scope).",
                                    v
                                ));
                            }
                        }
                    }
                }
                Ok(())
            }

            Expr::Assign { target, value } => {
                self.check_expr(value)?;
                // For simple variable assignment, re-activate the variable
                if let Expr::Variable(name) = target.as_ref() {
                    self.registry.insert(name.clone(), VariableState::Active);
                    if let Some(ty) = self.infer_type(value) {
                        self.types.insert(name.clone(), ty.clone());
                        if matches!(ty, TrackType::Ref(_)) {
                            let prov = self.get_provenance(value);
                            self.borrows.insert(name.clone(), prov);
                        } else {
                            self.borrows.remove(name);
                        }
                    }
                }
                self.update_borrow_states();
                Ok(())
            }

            Expr::FnDef {
                params,
                body,
                return_type,
                ..
            } => {
                // Enter function scope
                let saved_registry = self.registry.clone();
                let saved_types = self.types.clone();
                let saved_borrows = self.borrows.clone();
                let saved_lens = self.lens_locked.clone();
                let saved_params = self.current_params.clone();
                let saved_ret = self.current_return_type.clone();

                self.current_params = params.iter().map(|(n, _)| n.clone()).collect();
                self.current_return_type = return_type.clone();

                for (name, ty) in params {
                    self.declare(name.clone());
                    self.types.insert(name.clone(), ty.clone());
                }

                self.update_borrow_states();

                for stmt in body {
                    self.check_expr(stmt)?;
                    self.update_borrow_states();
                }

                // Escape check for implicit return at the end of function body
                if let Some(TrackType::Ref(_)) = return_type {
                    let has_explicit_return =
                        body.iter().any(|stmt| matches!(stmt, Expr::Return { .. }));
                    if !has_explicit_return {
                        if let Some(last_stmt) = body.last() {
                            let prov = self.get_provenance(last_stmt);
                            for v in &prov {
                                if !self.current_params.contains(v) {
                                    return Err(format!(
                                        "Compile Error: Cannot return reference to local variable '{}' (escapes function scope).",
                                        v
                                    ));
                                }
                            }
                        }
                    }
                }

                // Restore outer scope
                self.registry = saved_registry;
                self.types = saved_types;
                self.borrows = saved_borrows;
                self.lens_locked = saved_lens;
                self.current_params = saved_params;
                self.current_return_type = saved_ret;
                Ok(())
            }

            Expr::Use {
                path,
                imports,
                alias,
            } => {
                let norm_path = path.replace("::", "/");
                let provided = match norm_path.as_str() {
                    "std/io" => vec![
                        ("print".to_string(), Some(TrackType::Void)),
                        ("println".to_string(), Some(TrackType::Void)),
                        ("read".to_string(), Some(TrackType::I64)),
                        ("eprint".to_string(), Some(TrackType::Void)),
                    ],
                    "std/fs" => vec![
                        (
                            "file_open".to_string(),
                            Some(TrackType::Ptr(Box::new(TrackType::Void))),
                        ),
                        ("file_close".to_string(), Some(TrackType::Void)),
                        ("file_exists".to_string(), Some(TrackType::I32)),
                    ],
                    "std/sys" => vec![
                        ("exit".to_string(), Some(TrackType::Void)),
                        ("clock_ms".to_string(), Some(TrackType::I64)),
                    ],
                    "std/mem" => vec![
                        (
                            "alloc".to_string(),
                            Some(TrackType::Ptr(Box::new(TrackType::Void))),
                        ),
                        ("dealloc".to_string(), Some(TrackType::Void)),
                    ],
                    "math/vec" => vec![
                        ("add".to_string(), Some(TrackType::I64)),
                        ("sub".to_string(), Some(TrackType::I64)),
                        ("dot".to_string(), Some(TrackType::I64)),
                        ("cross".to_string(), Some(TrackType::I64)),
                    ],
                    _ => return Err(format!("Compile Error: Unknown module '{}'", path)),
                };

                let default_ns = path.split('/').next_back().unwrap_or(path);

                if let Some(ref alias_name) = alias {
                    if let Some(ref items) = imports {
                        if items.len() == 1 {
                            let item_name = &items[0];
                            if let Some((_, ret_ty)) = provided.iter().find(|(n, _)| n == item_name)
                            {
                                self.functions.insert(alias_name.clone(), ret_ty.clone());
                            } else {
                                return Err(format!(
                                    "Compile Error: Module '{}' does not export '{}'",
                                    path, item_name
                                ));
                            }
                        } else {
                            for item_name in items {
                                if let Some((_, ret_ty)) =
                                    provided.iter().find(|(n, _)| n == item_name)
                                {
                                    self.functions.insert(
                                        format!("{}::{}", alias_name, item_name),
                                        ret_ty.clone(),
                                    );
                                } else {
                                    return Err(format!(
                                        "Compile Error: Module '{}' does not export '{}'",
                                        path, item_name
                                    ));
                                }
                            }
                        }
                    } else {
                        for (func_name, ret_ty) in &provided {
                            self.functions
                                .insert(format!("{}::{}", alias_name, func_name), ret_ty.clone());
                        }
                    }
                } else {
                    if let Some(ref items) = imports {
                        for item_name in items {
                            if let Some((_, ret_ty)) = provided.iter().find(|(n, _)| n == item_name)
                            {
                                self.functions.insert(item_name.clone(), ret_ty.clone());
                            } else {
                                return Err(format!(
                                    "Compile Error: Module '{}' does not export '{}'",
                                    path, item_name
                                ));
                            }
                        }
                    } else {
                        for (func_name, ret_ty) in &provided {
                            self.functions
                                .insert(format!("{}::{}", default_ns, func_name), ret_ty.clone());
                        }
                    }
                }
                Ok(())
            }

            Expr::ConstDef { name, value } => {
                self.check_expr(value)?;
                self.declare(name.clone());
                if let Some(ty) = self.infer_type(value) {
                    self.types.insert(name.clone(), ty);
                }
                self.update_borrow_states();
                Ok(())
            }

            Expr::MacroDef {
                name,
                params,
                return_type,
                body,
            } => {
                self.functions.insert(name.clone(), return_type.clone());

                let saved_registry = self.registry.clone();
                let saved_types = self.types.clone();
                let saved_borrows = self.borrows.clone();
                let saved_lens = self.lens_locked.clone();
                let saved_params = self.current_params.clone();
                let saved_ret = self.current_return_type.clone();

                self.current_params = params.iter().map(|(n, _)| n.clone()).collect();
                self.current_return_type = return_type.clone();

                for (pname, pty) in params {
                    self.declare(pname.clone());
                    self.types.insert(pname.clone(), pty.clone());
                }

                self.update_borrow_states();

                for stmt in body {
                    self.check_expr(stmt)?;
                    self.update_borrow_states();
                }

                self.registry = saved_registry;
                self.types = saved_types;
                self.borrows = saved_borrows;
                self.lens_locked = saved_lens;
                self.current_params = saved_params;
                self.current_return_type = saved_ret;
                Ok(())
            }

            Expr::MacroCall { name, args, body } => {
                if name == "compile_error" {
                    if let Some(Expr::StringLiteral(msg)) = args.first() {
                        return Err(format!("Compile Error: {}", msg));
                    } else {
                        return Err(
                            "Compile Error: @compile_error requires a string message".to_string()
                        );
                    }
                }

                for arg in args {
                    self.check_expr(arg)?;
                }

                if let Some(ref block_body) = body {
                    for stmt in block_body {
                        self.check_expr(stmt)?;
                        self.update_borrow_states();
                    }
                }
                Ok(())
            }

            Expr::EnumDef {
                name,
                underlying_type: _,
                variants,
            } => {
                for (var_name, val_opt) in variants {
                    let fullname = format!("{}::{}", name, var_name);
                    self.types
                        .insert(fullname.clone(), TrackType::Custom(name.clone()));
                    self.declare(fullname);
                    if let Some(ref val) = val_opt {
                        self.check_expr(val)?;
                    }
                }
                Ok(())
            }

            Expr::UnionDef { name, variants } => {
                for (var_name, ty_opt) in variants {
                    let fullname = format!("{}::{}", name, var_name);
                    if ty_opt.is_some() {
                        self.functions
                            .insert(fullname, Some(TrackType::Custom(name.clone())));
                    } else {
                        self.types
                            .insert(fullname.clone(), TrackType::Custom(name.clone()));
                        self.declare(fullname);
                    }
                }
                Ok(())
            }

            Expr::Match { target, arms } => {
                self.check_expr(target)?;
                for arm in arms {
                    let saved_registry = self.registry.clone();
                    let saved_types = self.types.clone();
                    let saved_borrows = self.borrows.clone();
                    let saved_lens = self.lens_locked.clone();

                    if let crate::ast::Pattern::Variant {
                        ref enum_or_union,
                        ref variant,
                        binding: Some(ref bind_var),
                    } = arm.pattern
                    {
                        let bind_ty = match (enum_or_union.as_str(), variant.as_str()) {
                            ("Value", "Int") => TrackType::I32,
                            ("Value", "Float") => TrackType::I64,
                            ("Value", "Bool") => TrackType::Bool,
                            ("Result", "Ok") => TrackType::I64,
                            ("Result", "Err") => TrackType::I64,
                            _ => TrackType::I32,
                        };
                        self.declare(bind_var.clone());
                        self.types.insert(bind_var.clone(), bind_ty);
                    }

                    if let Some(ref guard_expr) = arm.guard {
                        self.check_expr(guard_expr)?;
                    }

                    self.check_expr(&arm.body)?;

                    self.registry = saved_registry;
                    self.types = saved_types;
                    self.borrows = saved_borrows;
                    self.lens_locked = saved_lens;
                }
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
                    None => Err(format!("Compile Error: Undeclared variable '{}'.", name)),
                }
            }
            _ => self.check_expr(expr),
        }
    }
    fn get_provenance(&self, expr: &Expr) -> Vec<String> {
        match expr {
            Expr::AddressOf { target } => match target.as_ref() {
                Expr::Variable(name) => vec![name.clone()],
                Expr::ArrayIndex {
                    target: inner_target,
                    ..
                } => self.get_provenance(inner_target),
                _ => self.get_provenance(target),
            },
            Expr::Variable(name) => {
                if let Some(targets) = self.borrows.get(name) {
                    targets.clone()
                } else if self
                    .types
                    .get(name)
                    .is_some_and(|t| matches!(t, TrackType::Ref(_)))
                {
                    vec![name.clone()]
                } else {
                    Vec::new()
                }
            }
            Expr::FunctionCall { name: _, args } => {
                let mut prov = Vec::new();
                for arg in args {
                    if let Some(ty) = self.infer_type(arg) {
                        if matches!(ty, TrackType::Ref(_)) {
                            prov.extend(self.get_provenance(arg));
                        }
                    }
                }
                prov.sort();
                prov.dedup();
                prov
            }
            Expr::IfElse {
                then_body,
                else_body,
                ..
            } => {
                let mut prov = Vec::new();
                if let Some(last) = then_body.last() {
                    prov.extend(self.get_provenance(last));
                }
                if let Some(last) = else_body.last() {
                    prov.extend(self.get_provenance(last));
                }
                prov.sort();
                prov.dedup();
                prov
            }
            Expr::LensBlock { body, .. } => body
                .last()
                .map_or(Vec::new(), |last| self.get_provenance(last)),
            _ => Vec::new(),
        }
    }
}

fn is_comparison(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Lte | BinOp::Gte
    )
}
