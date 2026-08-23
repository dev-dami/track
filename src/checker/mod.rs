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
    pub lens_aliases: std::collections::HashSet<String>,
    pub mutables: std::collections::HashSet<String>,
    pub unions: HashMap<String, Vec<(String, Option<TrackType>)>>,
    pub type_aliases: HashMap<String, TrackType>,
    pub current_params: std::collections::HashSet<String>,
    pub current_return_type: Option<TrackType>,
    pub barebones: bool,
    pub loaded_modules: std::collections::HashSet<String>,
}

impl LinearChecker {
    pub fn new_barebones() -> Self {
        let mut c = Self::new();
        c.barebones = true;
        c.functions.clear();
        c
    }
}

fn is_copy_type(ty: &TrackType) -> bool {
    matches!(
        ty,
        TrackType::U8
            | TrackType::I8
            | TrackType::I32
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
            lens_aliases: std::collections::HashSet::new(),
            mutables: std::collections::HashSet::new(),
            unions: HashMap::new(),
            type_aliases: HashMap::new(),
            current_params: std::collections::HashSet::new(),
            current_return_type: None,
            barebones: false,
            loaded_modules: std::collections::HashSet::new(),
        }
    }

    pub fn declare(&mut self, name: String) {
        self.registry.insert(name, VariableState::Active);
    }

    pub fn is_copy_var(&self, name: &str) -> bool {
        // Enum/union variant constructors (e.g. `Color::Red`, `Token::Import`) are
        // semantically value constants — reusing the same variant must not
        // consume a linear resource. Treat any `::`-qualified name that was
        // introduced as an enum/union variant as copy so `read_or_move`
        // remains `Active` across uses and CFG merges.
        if name.contains("::") {
            return true;
        }
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
                if !self.is_copy_var(name) && !self.lens_aliases.contains(name) {
                    self.registry.insert(name.to_string(), VariableState::Spent);
                }
                Ok(())
            }
            None => Err(format!("Compile Error: Undeclared variable '{}'.", name)),
        }
    }

    pub fn update_borrow_states(&mut self) {
        if self.borrows.is_empty() && self.lens_locked.is_empty() {
            for state in self.registry.values_mut() {
                if *state == VariableState::Locked {
                    *state = VariableState::Active;
                }
            }
            return;
        }

        // Collect all variables that are borrowed by currently Active reference variables
        let mut borrowed_vars = std::collections::HashSet::new();
        for (ref_name, provs) in &self.borrows {
            if self.registry.get(ref_name) == Some(&VariableState::Active) {
                for p in provs {
                    borrowed_vars.insert(p.clone());
                }
            }
        }

        // Update registry states
        for (name, state) in self.registry.iter_mut() {
            let is_locked = self.lens_locked.contains(name) || borrowed_vars.contains(name);
            if is_locked && *state == VariableState::Active {
                *state = VariableState::Locked;
            } else if !is_locked && *state == VariableState::Locked {
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
        if !self.barebones {
            // print is built-in and returns Void — barebones disables the implicit prelude
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
            self.functions
                .insert("math_clamp".to_string(), Some(TrackType::I64));
            self.functions
                .insert("math_random".to_string(), Some(TrackType::U64));

            // Extended String & File & Sys functions
            self.functions
                .insert("str_find".to_string(), Some(TrackType::I64));
            self.functions
                .insert("str_to_int".to_string(), Some(TrackType::I64));
            self.functions.insert(
                "int_to_str".to_string(),
                Some(TrackType::Custom("Str".to_string())),
            );
            self.functions
                .insert("file_remove".to_string(), Some(TrackType::I32));
            self.functions
                .insert("file_size".to_string(), Some(TrackType::I64));
            self.functions
                .insert("sys_exec".to_string(), Some(TrackType::I32));
            self.functions
                .insert("sys_set_memory_limit".to_string(), Some(TrackType::Void));
            self.functions
                .insert("sys_get_memory_used".to_string(), Some(TrackType::I64));
            self.functions.insert(
                "env_get".to_string(),
                Some(TrackType::Custom("Str".to_string())),
            );

            // Math Extensions
            self.functions
                .insert("math_abs".to_string(), Some(TrackType::I64));
            self.functions
                .insert("math_min".to_string(), Some(TrackType::I64));
            self.functions
                .insert("math_max".to_string(), Some(TrackType::I64));
            self.functions
                .insert("math_pow".to_string(), Some(TrackType::I64));
            self.functions
                .insert("math_sqrt".to_string(), Some(TrackType::I64));
            self.functions
                .insert("math_floor".to_string(), Some(TrackType::I64));
            self.functions
                .insert("math_ceil".to_string(), Some(TrackType::I64));
            self.functions
                .insert("math_round".to_string(), Some(TrackType::I64));

            // Extra String & IO Extensions
            self.functions
                .insert("print_err".to_string(), Some(TrackType::Void));
            self.functions
                .insert("eprint".to_string(), Some(TrackType::Void));
            self.functions
                .insert("str_contains".to_string(), Some(TrackType::I64));
            self.functions
                .insert("str_is_int".to_string(), Some(TrackType::I32));

            // Explicit error handling primitives (v0.5)
            self.functions
                .insert("abort".to_string(), Some(TrackType::Void));
            self.functions
                .insert("env_exists".to_string(), Some(TrackType::I32));

            // TCP Socket Net API
            self.functions
                .insert("net_socket_tcp_listen".to_string(), Some(TrackType::I32));
            self.functions
                .insert("net_socket_accept".to_string(), Some(TrackType::I32));
            self.functions
                .insert("net_socket_connect".to_string(), Some(TrackType::I32));
            self.functions
                .insert("net_socket_send".to_string(), Some(TrackType::I64));
            self.functions
                .insert("net_socket_recv".to_string(), Some(TrackType::I64));
            self.functions
                .insert("net_socket_close".to_string(), Some(TrackType::Void));

            // OS & FS Extensions (v0.2.0)
            self.functions
                .insert("os_args_count".to_string(), Some(TrackType::I32));
            self.functions.insert(
                "os_arg".to_string(),
                Some(TrackType::Custom("Str".to_string())),
            );
            self.functions
                .insert("dir_exists".to_string(), Some(TrackType::Bool));
            self.functions
                .insert("file_copy".to_string(), Some(TrackType::I32));
            self.functions
                .insert("process_spawn".to_string(), Some(TrackType::I32));

            // Char & Byte Operations
            self.functions
                .insert("char_is_digit".to_string(), Some(TrackType::Bool));
            self.functions
                .insert("char_is_alpha".to_string(), Some(TrackType::Bool));
            self.functions
                .insert("char_is_alphanumeric".to_string(), Some(TrackType::Bool));
            self.functions
                .insert("char_is_space".to_string(), Some(TrackType::Bool));
            self.functions
                .insert("char_to_upper".to_string(), Some(TrackType::U8));
            self.functions
                .insert("char_to_lower".to_string(), Some(TrackType::U8));

            // Extended String & Memory
            self.functions
                .insert("str_starts_with".to_string(), Some(TrackType::Bool));
            self.functions
                .insert("str_ends_with".to_string(), Some(TrackType::Bool));
            self.functions.insert(
                "str_substr".to_string(),
                Some(TrackType::Custom("Str".to_string())),
            );
            self.functions.insert(
                "str_trim".to_string(),
                Some(TrackType::Custom("Str".to_string())),
            );
            self.functions
                .insert("str_char_at".to_string(), Some(TrackType::U8));
            self.functions.insert(
                "mem_realloc".to_string(),
                Some(TrackType::Ptr(Box::new(TrackType::Custom(
                    "u8".to_string(),
                )))),
            );
            self.functions
                .insert("vec_reserve".to_string(), Some(TrackType::Void));
            self.functions
                .insert("vec_clear".to_string(), Some(TrackType::Void));
            self.functions
                .insert("vec_len".to_string(), Some(TrackType::I32));

            // Path Operations
            self.functions.insert(
                "path_basename".to_string(),
                Some(TrackType::Custom("Str".to_string())),
            );
            self.functions.insert(
                "path_ext".to_string(),
                Some(TrackType::Custom("Str".to_string())),
            );
            self.functions.insert(
                "path_join".to_string(),
                Some(TrackType::Custom("Str".to_string())),
            );
        } // end if !barebones — std package (import "std") still works via Use handling

        for stmt in program {
            match stmt {
                Expr::TypeAlias { name, target } => {
                    self.type_aliases.insert(name.clone(), target.clone());
                }
                Expr::EnumDef { name, variants, .. } => {
                    for (var_name, _) in variants {
                        let fullname = format!("{}::{}", name, var_name);
                        self.types
                            .insert(fullname.clone(), TrackType::Custom(name.clone()));
                        self.declare(fullname);
                    }
                }
                Expr::UnionDef { name, variants } => {
                    self.unions.insert(name.clone(), variants.clone());
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
                }
                Expr::FnDef {
                    name,
                    generics,
                    return_type,
                    ..
                } => {
                    if !generics.is_empty() {
                        // Generic templates are checked via their instances.
                        continue;
                    }
                    self.functions.insert(name.clone(), return_type.clone());
                }
                _ => {}
            }
        }
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
            Expr::TupleLiteral { elements } => {
                let elem_types = elements
                    .iter()
                    .map(|e| self.infer_type(e).unwrap_or(TrackType::I32))
                    .collect();
                Some(TrackType::Tuple(elem_types))
            }
            Expr::TupleIndex { target, index } => match self.infer_type(target) {
                Some(TrackType::Tuple(types)) => types.get(*index).cloned(),
                Some(TrackType::Ref(inner)) => match *inner {
                    TrackType::Tuple(types) => types.get(*index).cloned(),
                    _ => None,
                },
                _ => None,
            },
            Expr::ArrayIndex { target, .. } => match self.infer_type(target) {
                Some(TrackType::Array(inner, _)) => Some(*inner),
                Some(TrackType::Ptr(inner)) => Some(*inner),
                Some(TrackType::Slice(inner)) => Some(*inner),
                Some(TrackType::Ref(inner)) => match *inner {
                    TrackType::Array(elem, _) => Some(*elem),
                    TrackType::Ptr(elem) => Some(*elem),
                    TrackType::Slice(elem) => Some(*elem),
                    other => Some(other),
                },
                _ => None,
            },
            Expr::SliceIndex { target, .. } => match self.infer_type(target) {
                Some(TrackType::Array(inner, _)) => Some(TrackType::Slice(inner)),
                Some(TrackType::Slice(inner)) => Some(TrackType::Slice(inner)),
                Some(TrackType::Ptr(inner)) => Some(TrackType::Slice(inner)),
                Some(TrackType::Ref(inner)) => match *inner {
                    TrackType::Array(elem, _) => Some(TrackType::Slice(elem)),
                    TrackType::Slice(elem) => Some(TrackType::Slice(elem)),
                    TrackType::Ptr(elem) => Some(TrackType::Slice(elem)),
                    _ => None,
                },
                _ => None,
            },
            Expr::Range { .. } => Some(TrackType::Void),
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
            Expr::LetDef { .. } | Expr::LetDestructure { .. } => Some(TrackType::Void),
            Expr::EnumDef { .. } => Some(TrackType::Void),
            Expr::UnionDef { .. } => Some(TrackType::Void),
            Expr::TypeAlias { .. } => Some(TrackType::Void),
            Expr::ForIn { .. } => Some(TrackType::Void),
            Expr::Match { arms, .. } => arms.first().and_then(|arm| self.infer_type(&arm.body)),
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::IntLiteral(_)
            | Expr::StringLiteral(_)
            | Expr::BoolLiteral(_)
            | Expr::TypeAlias { .. } => Ok(()),

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

            Expr::TupleLiteral { elements } => {
                for elem in elements {
                    self.check_expr(elem)?;
                }
                Ok(())
            }

            Expr::TupleIndex { target, index } => {
                self.check_borrow(target)?;
                if let Some(ty) = self.infer_type(target) {
                    let tuple_types = match ty {
                        TrackType::Tuple(types) => Some(types),
                        TrackType::Ref(inner) => match *inner {
                            TrackType::Tuple(types) => Some(types),
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(types) = tuple_types
                        && *index >= types.len()
                    {
                        return Err(format!(
                            "Compile Error: Tuple index {} out of bounds for tuple of length {}",
                            index,
                            types.len()
                        ));
                    }
                }
                Ok(())
            }

            Expr::ArrayIndex { target, index } => {
                // Array index borrows both target and index
                self.check_borrow(target)?;
                self.check_borrow(index)?;
                Ok(())
            }

            Expr::SliceIndex { target, start, end } => {
                self.check_borrow(target)?;
                if let Some(s) = start {
                    self.check_borrow(s)?;
                }
                if let Some(e) = end {
                    self.check_borrow(e)?;
                }
                Ok(())
            }

            Expr::Range { start, end } => {
                self.check_borrow(start)?;
                self.check_borrow(end)?;
                Ok(())
            }

            Expr::AddressOf { target } => {
                // &expr borrows but doesn't consume — check without moving
                self.check_borrow(target)?;
                Ok(())
            }

            Expr::StructInitialization { fields, .. } => {
                for (_, fval) in fields {
                    self.reject_lens_escape(fval)?;
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

                let clean_name = name.trim_start_matches('@');
                let target_name = if clean_name.contains("::") {
                    clean_name.split("::").last().unwrap()
                } else {
                    clean_name
                };

                if !self.functions.contains_key(name)
                    && !self.functions.contains_key(clean_name)
                    && !self.functions.contains_key(target_name)
                {
                    let mut best_match = None;
                    let mut min_dist = usize::MAX;
                    for candidate in self.functions.keys() {
                        let dist = levenshtein_distance(target_name, candidate);
                        if dist <= 2 && dist < min_dist {
                            min_dist = dist;
                            best_match = Some(candidate.clone());
                        }
                    }
                    if let Some(suggestion) = best_match {
                        return Err(format!(
                            "Compile Error: Undefined function '{}'. Did you mean '{}'?",
                            name, suggestion
                        ));
                    }
                }

                for arg in args {
                    self.check_expr(arg)?;
                }
                Ok(())
            }

            Expr::LetDef {
                name,
                mutable,
                ty,
                value,
            } => {
                self.reject_lens_escape(value)?;
                self.check_expr(value)?;
                let inferred = self.infer_type(value);
                let final_ty = if let Some(annotated_ty) = ty {
                    annotated_ty.clone()
                } else {
                    inferred.unwrap_or(TrackType::Void)
                };

                self.declare(name.clone());
                if *mutable {
                    self.mutables.insert(name.clone());
                }
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
                self.lens_aliases.insert(lens_name.clone());
                if let Some(ty) = self.types.get(target).cloned() {
                    self.types.insert(lens_name.clone(), ty);
                }
                for expr in body {
                    self.check_expr(expr)?;
                }
                self.exit_lens(target);
                self.registry.remove(lens_name);
                self.types.remove(lens_name);
                self.borrows.remove(lens_name);
                self.lens_aliases.remove(lens_name);
                Ok(())
            }

            Expr::IfElse {
                condition,
                then_body,
                else_body,
            } => {
                self.check_expr(condition)?;

                // Clone state for each branch
                // Snapshot pre-if state
                let pre_if = self.registry.clone();
                let pre_if_types = self.types.clone();
                let pre_if_borrows = self.borrows.clone();

                // Check then branch directly
                for stmt in then_body {
                    self.check_expr(stmt)?;
                    self.update_borrow_states();
                }
                let then_end = std::mem::replace(&mut self.registry, pre_if.clone());
                let _then_types = std::mem::replace(&mut self.types, pre_if_types.clone());
                let then_end_borrows = std::mem::replace(&mut self.borrows, pre_if_borrows.clone());

                // Check else branch directly
                for stmt in else_body {
                    self.check_expr(stmt)?;
                    self.update_borrow_states();
                }
                let else_end = std::mem::take(&mut self.registry);
                let _else_types = std::mem::take(&mut self.types);
                let else_end_borrows = std::mem::take(&mut self.borrows);

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
                let pre_loop = self.registry.clone();
                let pre_loop_types = self.types.clone();
                let pre_loop_borrows = self.borrows.clone();

                for stmt in body {
                    self.check_expr(stmt)?;
                    self.update_borrow_states();
                }

                // Reject linear resources defined outside the loop being consumed inside the loop
                for (name, pre_state) in &pre_loop {
                    if *pre_state == VariableState::Active
                        && !self.is_copy_var(name)
                        && let Some(post_state) = self.registry.get(name)
                        && *post_state == VariableState::Spent
                    {
                        return Err(format!(
                            "Compile Error: Cannot consume linear resource '{}' inside a loop. It was declared outside the loop and would be double-freed/moved on subsequent iterations.",
                            name
                        ));
                    }
                }

                self.registry = pre_loop;
                self.types = pre_loop_types;
                self.borrows = pre_loop_borrows;
                self.update_borrow_states();
                Ok(())
            }

            Expr::ForIn { var, iter, body } => {
                self.check_expr(iter)?;
                let elem_ty = self.infer_type(iter).map_or(TrackType::I32, |ty| match ty {
                    TrackType::Slice(inner) => *inner,
                    TrackType::Array(inner, _) => *inner,
                    _ => TrackType::I32,
                });

                let pre_loop = self.registry.clone();
                let pre_loop_types = self.types.clone();
                let pre_loop_borrows = self.borrows.clone();

                self.declare(var.clone());
                self.types.insert(var.clone(), elem_ty);

                for stmt in body {
                    self.check_expr(stmt)?;
                    self.update_borrow_states();
                }

                for (name, pre_state) in &pre_loop {
                    if *pre_state == VariableState::Active
                        && !self.is_copy_var(name)
                        && let Some(post_state) = self.registry.get(name)
                        && *post_state == VariableState::Spent
                    {
                        return Err(format!(
                            "Compile Error: Cannot consume linear resource '{}' inside a loop.",
                            name
                        ));
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
                    self.reject_lens_escape(val)?;
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
                self.reject_lens_escape(value)?;
                self.check_expr(value)?;
                // For simple variable assignment, check mutability and re-activate variable
                if let Expr::Variable(name) = target.as_ref() {
                    if !self.mutables.contains(name) && self.registry.contains_key(name) {
                        return Err(format!(
                            "Compile Error: Cannot mutate immutable variable '{}'. Use 'let mut {}' to declare a mutable variable.",
                            name, name
                        ));
                    }
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
                generics,
                params,
                body,
                return_type,
                ..
            } => {
                if !generics.is_empty() {
                    return Ok(());
                }
                // Enter function scope (preserve global module declarations)
                let saved_registry = self.registry.clone();
                let saved_types = self.types.clone();
                let saved_borrows = std::mem::take(&mut self.borrows);
                let saved_lens = std::mem::take(&mut self.lens_locked);
                let saved_lens_aliases = std::mem::take(&mut self.lens_aliases);
                let saved_params = std::mem::take(&mut self.current_params);
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
                    if !has_explicit_return && let Some(last_stmt) = body.last() {
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

                // Restore outer scope
                self.registry = saved_registry;
                self.types = saved_types;
                self.borrows = saved_borrows;
                self.lens_locked = saved_lens;
                self.lens_aliases = saved_lens_aliases;
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
                    // Unified std package: `import "std"` → std::print, std::alloc, std::io::print, …
                    "std" => vec![
                        ("print".to_string(), Some(TrackType::Void)),
                        ("println".to_string(), Some(TrackType::Void)),
                        ("eprint".to_string(), Some(TrackType::Void)),
                        ("read".to_string(), Some(TrackType::I64)),
                        (
                            "alloc".to_string(),
                            Some(TrackType::Ptr(Box::new(TrackType::Void))),
                        ),
                        ("dealloc".to_string(), Some(TrackType::Void)),
                        ("exit".to_string(), Some(TrackType::Void)),
                        ("abort".to_string(), Some(TrackType::Void)),
                        ("clock_ms".to_string(), Some(TrackType::I64)),
                        // sub-namespaces under std::
                        ("io::print".to_string(), Some(TrackType::Void)),
                        ("io::println".to_string(), Some(TrackType::Void)),
                        ("io::read".to_string(), Some(TrackType::I64)),
                        ("io::eprint".to_string(), Some(TrackType::Void)),
                        (
                            "fs::file_open".to_string(),
                            Some(TrackType::Ptr(Box::new(TrackType::Void))),
                        ),
                        ("fs::file_close".to_string(), Some(TrackType::Void)),
                        ("fs::file_exists".to_string(), Some(TrackType::I32)),
                        ("fs::file_copy".to_string(), Some(TrackType::I32)),
                        ("fs::file_size".to_string(), Some(TrackType::I64)),
                        ("fs::dir_exists".to_string(), Some(TrackType::Bool)),
                        (
                            "mem::alloc".to_string(),
                            Some(TrackType::Ptr(Box::new(TrackType::Void))),
                        ),
                        ("mem::dealloc".to_string(), Some(TrackType::Void)),
                        ("sys::exit".to_string(), Some(TrackType::Void)),
                        ("sys::abort".to_string(), Some(TrackType::Void)),
                        ("sys::clock_ms".to_string(), Some(TrackType::I64)),
                        ("char::char_is_digit".to_string(), Some(TrackType::Bool)),
                        ("str::str_find".to_string(), Some(TrackType::I64)),
                        ("process::process_spawn".to_string(), Some(TrackType::I32)),
                        (
                            "net::net_socket_tcp_listen".to_string(),
                            Some(TrackType::I32),
                        ),
                    ],
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
                        ("abort".to_string(), Some(TrackType::Void)),
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
                    _ => {
                        // Local file import fallback — try to resolve `path` as a
                        // `.trk` file relative to the project. Candidates mirror
                        // yard's `src/` layout and the `compiler/` self-hosting
                        // directory.
                        let candidates = vec![
                            format!("src/{}.trk", norm_path),
                            format!("{}.trk", norm_path),
                            format!("compiler/src/{}.trk", norm_path),
                            format!("compiler/{}.trk", norm_path),
                            format!("src/{}/mod.trk", norm_path),
                        ];
                        let mut found: Option<String> = None;
                        for cand in &candidates {
                            if std::path::Path::new(cand).exists() {
                                found = Some(cand.clone());
                                break;
                            }
                        }
                        // Also try relative to current compiler package when
                        // checking from repository root: the import `token`
                        // inside `compiler/src/main.trk` should resolve to
                        // `compiler/src/token.trk` even when the candidate
                        // `src/token.trk` is tried first.
                        if found.is_none() {
                            for cand in &[
                                format!("compiler/src/{}.trk", path),
                                format!("compiler/{}.trk", path),
                            ] {
                                if std::path::Path::new(cand).exists() {
                                    found = Some(cand.clone());
                                    break;
                                }
                            }
                        }
                        if let Some(cand) = found {
                            if self.loaded_modules.contains(&cand) {
                                return Ok(());
                            }
                            self.loaded_modules.insert(cand.clone());
                            let src = std::fs::read_to_string(&cand).map_err(|e| {
                                format!("Failed to read module '{}' ({}): {}", path, cand, e)
                            })?;
                            let tokens = crate::lexer::Lexer::tokenize(&src)
                                .map_err(|e| format!("Lexer error in '{}': {}", cand, e))?;
                            let mut p = crate::parser::Parser::new(tokens, src.clone());
                            let prog = p
                                .parse_program()
                                .map_err(|e| format!("Parse error in '{}': {}", cand, e))?;
                            // Run monomorphization then checker on the imported program
                            // so its types/functions become visible to the importer.
                            let mut prog_owned = prog;
                            crate::mono::monomorphize(&mut prog_owned)
                                .map_err(|e| format!("Monomorphize error in '{}': {}", cand, e))?;
                            self.check_program(&prog_owned)?;
                            return Ok(());
                        }
                        return Err(format!("Compile Error: Unknown module '{}'", path));
                    }
                };

                let default_ns = path.split('/').next_back().unwrap_or(path);

                if let Some(alias_name) = alias {
                    if let Some(items) = imports {
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
                    if let Some(items) = imports {
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

                let saved_registry = std::mem::take(&mut self.registry);
                let saved_types = std::mem::take(&mut self.types);
                let saved_borrows = std::mem::take(&mut self.borrows);
                let saved_lens = std::mem::take(&mut self.lens_locked);
                let saved_lens_aliases = std::mem::take(&mut self.lens_aliases);
                let saved_params = std::mem::take(&mut self.current_params);
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
                self.lens_aliases = saved_lens_aliases;
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

                if let Some(block_body) = body {
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
                    if let Some(val) = val_opt {
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

            Expr::LetDestructure {
                pattern,
                mutable,
                value,
            } => {
                self.reject_lens_escape(value)?;
                self.check_expr(value)?;
                let val_ty = self.infer_type(value).unwrap_or(TrackType::I32);
                self.bind_pattern_variables(pattern, &val_ty, *mutable)?;
                self.update_borrow_states();
                Ok(())
            }

            Expr::Match { target, arms } => {
                self.check_expr(target)?;
                let target_ty = self.infer_type(target).unwrap_or(TrackType::I32);
                for arm in arms {
                    let saved_registry = self.registry.clone();
                    let saved_types = self.types.clone();
                    let saved_borrows = self.borrows.clone();
                    let saved_lens = self.lens_locked.clone();
                    let saved_lens_aliases = self.lens_aliases.clone();

                    self.bind_pattern_variables(&arm.pattern, &target_ty, false)?;

                    if let Some(ref guard_expr) = arm.guard {
                        self.check_expr(guard_expr)?;
                    }

                    self.check_expr(&arm.body)?;

                    self.registry = saved_registry;
                    self.types = saved_types;
                    self.borrows = saved_borrows;
                    self.lens_locked = saved_lens;
                    self.lens_aliases = saved_lens_aliases;
                }
                Ok(())
            }
        }
    }

    fn bind_pattern_variables(
        &mut self,
        pattern: &crate::ast::Pattern,
        target_type: &TrackType,
        mutable: bool,
    ) -> Result<(), String> {
        match pattern {
            crate::ast::Pattern::Ident(name) => {
                self.declare(name.clone());
                self.types.insert(name.clone(), target_type.clone());
                if mutable {
                    self.mutables.insert(name.clone());
                }
            }
            crate::ast::Pattern::Wildcard | crate::ast::Pattern::Literal(_) => {}
            crate::ast::Pattern::Tuple(pats) => {
                let elem_types = match target_type {
                    TrackType::Tuple(types) => types.clone(),
                    _ => vec![TrackType::I32; pats.len()],
                };
                for (i, p) in pats.iter().enumerate() {
                    let ty = elem_types.get(i).cloned().unwrap_or(TrackType::I32);
                    self.bind_pattern_variables(p, &ty, mutable)?;
                }
            }
            crate::ast::Pattern::Variant {
                enum_or_union,
                variant,
                bindings,
            } => {
                for (i, p) in bindings.iter().enumerate() {
                    let bind_ty = match (enum_or_union.as_str(), variant.as_str(), i) {
                        ("Value", "Int", _) => TrackType::I32,
                        ("Value", "Float", _) => TrackType::I64,
                        ("Value", "Bool", _) => TrackType::Bool,
                        ("Result", "Ok", _) => TrackType::I64,
                        ("Result", "Err", _) => TrackType::I64,
                        _ => TrackType::I32,
                    };
                    self.bind_pattern_variables(p, &bind_ty, mutable)?;
                }
            }
            crate::ast::Pattern::Struct { fields, .. } => {
                for (_fname, p) in fields {
                    self.bind_pattern_variables(p, &TrackType::I32, mutable)?;
                }
            }
        }
        Ok(())
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
                    if let Some(ty) = self.infer_type(arg)
                        && matches!(ty, TrackType::Ref(_))
                    {
                        prov.extend(self.get_provenance(arg));
                    }
                }
                if prov.len() > 1 {
                    prov.sort();
                    prov.dedup();
                }
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
                if prov.len() > 1 {
                    prov.sort();
                    prov.dedup();
                }
                prov
            }
            Expr::LensBlock { body, .. } => body
                .last()
                .map_or(Vec::new(), |last| self.get_provenance(last)),
            _ => Vec::new(),
        }
    }

    fn reject_lens_escape(&self, expr: &Expr) -> Result<(), String> {
        if let Some(name) = self.find_lens_alias(expr) {
            Err(format!(
                "Compile Error: Lens '{}' cannot be moved, stored, returned, or escape its with block.",
                name
            ))
        } else {
            Ok(())
        }
    }

    fn find_lens_alias(&self, expr: &Expr) -> Option<String> {
        if self.lens_aliases.is_empty() {
            return None;
        }
        match expr {
            Expr::Variable(name) if self.lens_aliases.contains(name) => Some(name.clone()),
            Expr::BinaryOp { left, right, .. } => self
                .find_lens_alias(left)
                .or_else(|| self.find_lens_alias(right)),
            Expr::UnaryOp { expr, .. } | Expr::AddressOf { target: expr } => {
                self.find_lens_alias(expr)
            }
            Expr::ArrayLiteral { elements } => {
                elements.iter().find_map(|e| self.find_lens_alias(e))
            }
            Expr::ArrayIndex { target, index } => self
                .find_lens_alias(target)
                .or_else(|| self.find_lens_alias(index)),
            Expr::SliceIndex { target, start, end } => self
                .find_lens_alias(target)
                .or_else(|| start.as_deref().and_then(|s| self.find_lens_alias(s)))
                .or_else(|| end.as_deref().and_then(|e| self.find_lens_alias(e))),
            Expr::Range { start, end } => self
                .find_lens_alias(start)
                .or_else(|| self.find_lens_alias(end)),
            Expr::StructInitialization { fields, .. } => fields
                .iter()
                .find_map(|(_, value)| self.find_lens_alias(value)),
            Expr::IfElse {
                condition,
                then_body,
                else_body,
            } => self
                .find_lens_alias(condition)
                .or_else(|| then_body.iter().find_map(|e| self.find_lens_alias(e)))
                .or_else(|| else_body.iter().find_map(|e| self.find_lens_alias(e))),
            Expr::WhileLoop { condition, body } => self
                .find_lens_alias(condition)
                .or_else(|| body.iter().find_map(|e| self.find_lens_alias(e))),
            Expr::Return { value } => value.as_deref().and_then(|v| self.find_lens_alias(v)),
            Expr::Assign { target, value } => self
                .find_lens_alias(target)
                .or_else(|| self.find_lens_alias(value)),
            Expr::LetDef { value, .. }
            | Expr::ConstDef { value, .. }
            | Expr::LetDestructure { value, .. } => self.find_lens_alias(value),
            Expr::TupleLiteral { elements } => {
                elements.iter().find_map(|e| self.find_lens_alias(e))
            }
            Expr::TupleIndex { target, .. } => self.find_lens_alias(target),
            Expr::FunctionCall { .. } => None,
            Expr::MacroCall { args, body, .. } => args
                .iter()
                .find_map(|arg| self.find_lens_alias(arg))
                .or_else(|| {
                    body.as_ref()
                        .and_then(|body| body.iter().find_map(|e| self.find_lens_alias(e)))
                }),
            Expr::LensBlock { body, .. } => body.iter().find_map(|e| self.find_lens_alias(e)),
            Expr::Match { target, arms } => self.find_lens_alias(target).or_else(|| {
                arms.iter().find_map(|arm| {
                    arm.guard
                        .as_ref()
                        .and_then(|guard| self.find_lens_alias(guard))
                        .or_else(|| self.find_lens_alias(&arm.body))
                })
            }),
            Expr::IntLiteral(_)
            | Expr::StringLiteral(_)
            | Expr::BoolLiteral(_)
            | Expr::FnDef { .. }
            | Expr::Use { .. }
            | Expr::MacroDef { .. }
            | Expr::EnumDef { .. }
            | Expr::UnionDef { .. }
            | Expr::TypeAlias { .. }
            | Expr::ForIn { .. }
            | Expr::Variable(_) => None,
        }
    }
}

fn is_comparison(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::Lte | BinOp::Gte
    )
}

#[allow(clippy::needless_range_loop)]
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut distances = vec![vec![0; b_chars.len() + 1]; a_chars.len() + 1];

    for i in 0..=a_chars.len() {
        distances[i][0] = i;
    }
    for j in 0..=b_chars.len() {
        distances[0][j] = j;
    }

    for i in 1..=a_chars.len() {
        for j in 1..=b_chars.len() {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            distances[i][j] = (distances[i - 1][j] + 1)
                .min(distances[i][j - 1] + 1)
                .min(distances[i - 1][j - 1] + cost);
        }
    }

    distances[a_chars.len()][b_chars.len()]
}
