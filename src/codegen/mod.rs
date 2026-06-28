use std::collections::HashMap;
use std::path::Path;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};

use crate::ast::{BinOp, Expr, TrackType, UnaryOp};

pub struct CodeGen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, PointerValue<'ctx>>,
    var_types: HashMap<String, TrackType>,
    current_fn: Option<FunctionValue<'ctx>>,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            var_types: HashMap::new(),
            current_fn: None,
        }
    }

    // ── type conversion ──────────────────────────────────────────────

    fn track_type_to_llvm(&self, ty: &TrackType) -> BasicTypeEnum<'ctx> {
        match ty {
            TrackType::I32 => self.context.i32_type().into(),
            TrackType::U32 => self.context.i32_type().into(),
            TrackType::I64 => self.context.i64_type().into(),
            TrackType::U64 => self.context.i64_type().into(),
            TrackType::Bool => self.context.bool_type().into(),
            TrackType::Void => self.context.i8_type().into(), // void has no BasicType; use i8 placeholder
            TrackType::Ptr(_) | TrackType::Ref(_) => self.context.ptr_type(AddressSpace::default()).into(),
            TrackType::Array(elem, size) => {
                let elem_ty = self.track_type_to_llvm(elem);
                elem_ty.array_type(*size as u32).into()
            }
            TrackType::Custom(_name) => {
                // For now treat custom structs as an opaque i64
                self.context.i64_type().into()
            }
        }
    }

    fn track_type_to_metadata(&self, ty: &TrackType) -> BasicMetadataTypeEnum<'ctx> {
        match ty {
            TrackType::I32 => self.context.i32_type().into(),
            TrackType::U32 => self.context.i32_type().into(),
            TrackType::I64 => self.context.i64_type().into(),
            TrackType::U64 => self.context.i64_type().into(),
            TrackType::Bool => self.context.bool_type().into(),
            TrackType::Void => self.context.i8_type().into(),
            TrackType::Ptr(_) | TrackType::Ref(_) => self.context.ptr_type(AddressSpace::default()).into(),
            TrackType::Array(elem, size) => {
                let elem_ty = self.track_type_to_llvm(elem);
                elem_ty.array_type(*size as u32).into()
            }
            TrackType::Custom(_) => self.context.i64_type().into(),
        }
    }

    // ── printf helper ────────────────────────────────────────────────

    fn get_or_declare_printf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("printf") {
            return f;
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let printf_type = i32_type.fn_type(&[ptr_type.into()], true);
        self.module.add_function("printf", printf_type, Some(inkwell::module::Linkage::External))
    }

    // ── expression compilation ───────────────────────────────────────

    pub fn compile_expr(&mut self, expr: &Expr) -> Option<BasicValueEnum<'ctx>> {
        match expr {
            Expr::IntLiteral(val) => {
                Some(self.context.i64_type().const_int(*val as u64, true).into())
            }

            Expr::BoolLiteral(val) => {
                Some(self.context.bool_type().const_int(*val as u64, false).into())
            }

            Expr::StringLiteral(s) => {
                let gv = self.builder.build_global_string_ptr(s, "str").unwrap();
                Some(gv.as_pointer_value().into())
            }

            Expr::Variable(name) => {
                if let Some(&ptr) = self.variables.get(name.as_str()) {
                    // Determine the pointee type
                    let pointee = self.pointee_type_for(name);
                    let loaded = self.builder.build_load(pointee, ptr, name).unwrap();
                    Some(loaded)
                } else {
                    None
                }
            }

            Expr::BinaryOp { op, left, right } => {
                let lv = self.compile_expr(left)?;
                let rv = self.compile_expr(right)?;
                let li = lv.into_int_value();
                let ri = rv.into_int_value();

                // Widen bool to i64 if mixing types
                let (li, ri) = self.coerce_ints(li, ri);

                let result = match op {
                    BinOp::Add => self.builder.build_int_add(li, ri, "add").unwrap(),
                    BinOp::Sub => self.builder.build_int_sub(li, ri, "sub").unwrap(),
                    BinOp::Mul => self.builder.build_int_mul(li, ri, "mul").unwrap(),
                    BinOp::Div => self.builder.build_int_signed_div(li, ri, "div").unwrap(),
                    BinOp::Mod => self.builder.build_int_signed_rem(li, ri, "rem").unwrap(),
                    BinOp::Eq => self.builder.build_int_compare(IntPredicate::EQ, li, ri, "eq").unwrap(),
                    BinOp::Neq => self.builder.build_int_compare(IntPredicate::NE, li, ri, "neq").unwrap(),
                    BinOp::Lt => self.builder.build_int_compare(IntPredicate::SLT, li, ri, "lt").unwrap(),
                    BinOp::Gt => self.builder.build_int_compare(IntPredicate::SGT, li, ri, "gt").unwrap(),
                    BinOp::Lte => self.builder.build_int_compare(IntPredicate::SLE, li, ri, "lte").unwrap(),
                    BinOp::Gte => self.builder.build_int_compare(IntPredicate::SGE, li, ri, "gte").unwrap(),
                    BinOp::And => self.builder.build_and(li, ri, "and").unwrap(),
                    BinOp::Or => self.builder.build_or(li, ri, "or").unwrap(),
                    BinOp::BitAnd => self.builder.build_and(li, ri, "bitand").unwrap(),
                    BinOp::Shl => self.builder.build_left_shift(li, ri, "shl").unwrap(),
                    BinOp::Shr => self.builder.build_right_shift(li, ri, false, "shr").unwrap(),
                };
                Some(result.into())
            }

            Expr::UnaryOp { op, expr } => {
                let val = self.compile_expr(expr)?;
                let result = match op {
                    UnaryOp::Neg => {
                        let iv = val.into_int_value();
                        self.builder.build_int_neg(iv, "neg").unwrap().into()
                    }
                    UnaryOp::Not => {
                        let iv = val.into_int_value();
                        self.builder.build_not(iv, "not").unwrap().into()
                    }
                    UnaryOp::Deref => {
                        let ptr = val.into_pointer_value();
                        let load_ty = self.deref_type_for(expr);
                        let loaded = self.builder.build_load(load_ty, ptr, "deref").unwrap();
                        return Some(loaded);
                    }
                };
                Some(result)
            }

            Expr::ArrayLiteral { elements } => {
                let i64_type = self.context.i64_type();
                let arr_type = i64_type.array_type(elements.len() as u32);
                let alloca = self.builder.build_alloca(arr_type, "arr").unwrap();

                for (i, elem) in elements.iter().enumerate() {
                    if let Some(val) = self.compile_expr(elem) {
                        let idx = self.context.i64_type().const_int(i as u64, false);
                        let zero = self.context.i64_type().const_int(0, false);
                        let gep = unsafe {
                            self.builder.build_gep(arr_type, alloca, &[zero, idx], "arr_elem").unwrap()
                        };
                        self.builder.build_store(gep, val.into_int_value()).unwrap();
                    }
                }
                Some(alloca.into())
            }

            Expr::ArrayIndex { target, index } => {
                let target_val = self.compile_expr(target)?;
                let idx_val = self.compile_expr(index)?;
                let ptr = target_val.into_pointer_value();
                let idx = idx_val.into_int_value();
                let i64_type = self.context.i64_type();
                let zero = i64_type.const_int(0, false);
                // Assume array of i64
                let arr_type = i64_type.array_type(0); // opaque; GEP doesn't need real size
                let gep = unsafe {
                    self.builder.build_gep(i64_type.array_type(256), ptr, &[zero, idx], "idx").unwrap()
                };
                let _ = arr_type;
                let loaded = self.builder.build_load(i64_type, gep, "elem").unwrap();
                Some(loaded)
            }

            Expr::AddressOf { target } => {
                // Return the alloca pointer without loading
                if let Expr::Variable(name) = target.as_ref() {
                    if let Some(&ptr) = self.variables.get(name.as_str()) {
                        return Some(ptr.into());
                    }
                }
                // Fall back to compiling and returning
                self.compile_expr(target)
            }

            Expr::StructInitialization { ty_name: _, fields } => {
                // Allocate struct as a sequence of i64 fields
                let i64_type = self.context.i64_type();
                let field_types: Vec<BasicTypeEnum> = fields.iter().map(|_| i64_type.into()).collect();
                let struct_type = self.context.struct_type(&field_types, false);
                let alloca = self.builder.build_alloca(struct_type, "struct_init").unwrap();

                for (i, (_fname, fval)) in fields.iter().enumerate() {
                    if let Some(val) = self.compile_expr(fval) {
                        let gep = self.builder.build_struct_gep(struct_type, alloca, i as u32, "field").unwrap();
                        self.builder.build_store(gep, val).unwrap();
                    }
                }
                Some(alloca.into())
            }

            Expr::LensBlock { target, lens_name, body } => {
                // Lens: copy target into lens_name, execute body, copy back
                if let Some(&target_ptr) = self.variables.get(target.as_str()) {
                    let pointee = self.pointee_type_for(target);
                    let val = self.builder.build_load(pointee, target_ptr, "lens_load").unwrap();
                    let lens_alloca = self.builder.build_alloca(pointee, lens_name).unwrap();
                    self.builder.build_store(lens_alloca, val).unwrap();
                    self.variables.insert(lens_name.clone(), lens_alloca);
                    if let Some(ty) = self.var_types.get(target).cloned() {
                        self.var_types.insert(lens_name.clone(), ty);
                    }

                    let mut last = None;
                    for stmt in body {
                        last = self.compile_expr(stmt);
                    }

                    // Copy back
                    let lens_val = self.builder.build_load(pointee, lens_alloca, "lens_back").unwrap();
                    self.builder.build_store(target_ptr, lens_val).unwrap();
                    self.variables.remove(lens_name);

                    last
                } else {
                    None
                }
            }

            Expr::FunctionCall { name, args } => {
                if name == "__assign" {
                    return self.compile_assign_call(args);
                }
                let is_print = name == "print" || name.ends_with("::print");
                if is_print {
                    return self.compile_print(args);
                }
                // Regular function call
                if let Some(func) = self.module.get_function(name) {
                    let compiled_args: Vec<BasicMetadataValueEnum> = args
                        .iter()
                        .filter_map(|a| self.compile_expr(a))
                        .map(|v| v.into())
                        .collect();
                    let call = self.builder.build_call(func, &compiled_args, "call").unwrap();
                    call.try_as_basic_value().basic()
                } else if name.ends_with("::add") || name == "add" || name == "sum" {
                    let left = self.compile_expr(&args[0])?.into_int_value();
                    let right = self.compile_expr(&args[1])?.into_int_value();
                    let sum_val = self.builder.build_int_add(left, right, "add_tmp").unwrap();
                    Some(sum_val.into())
                } else if name.ends_with("::sub") || name == "sub" {
                    let left = self.compile_expr(&args[0])?.into_int_value();
                    let right = self.compile_expr(&args[1])?.into_int_value();
                    let sub_val = self.builder.build_int_sub(left, right, "sub_tmp").unwrap();
                    Some(sub_val.into())
                } else {
                    None
                }
            }

            Expr::IfElse { condition, then_body, else_body } => {
                let cond_val = self.compile_expr(condition)?;
                let cond_int = cond_val.into_int_value();

                // If cond is i64 (from comparison), truncate to i1
                let cond_bool = if cond_int.get_type().get_bit_width() > 1 {
                    self.builder.build_int_truncate(cond_int, self.context.bool_type(), "tobool").unwrap()
                } else {
                    cond_int
                };

                let func = self.current_fn?;
                let then_bb = self.context.append_basic_block(func, "then");
                let else_bb = self.context.append_basic_block(func, "else");
                let merge_bb = self.context.append_basic_block(func, "merge");

                self.builder.build_conditional_branch(cond_bool, then_bb, else_bb).unwrap();

                // Then block
                self.builder.position_at_end(then_bb);
                for stmt in then_body {
                    self.compile_expr(stmt);
                }
                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                // Else block
                self.builder.position_at_end(else_bb);
                for stmt in else_body {
                    self.compile_expr(stmt);
                }
                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                self.builder.position_at_end(merge_bb);
                None
            }

            Expr::WhileLoop { condition, body } => {
                let func = self.current_fn?;
                let cond_bb = self.context.append_basic_block(func, "while_cond");
                let body_bb = self.context.append_basic_block(func, "while_body");
                let exit_bb = self.context.append_basic_block(func, "while_exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();

                // Condition
                self.builder.position_at_end(cond_bb);
                let cond_val = self.compile_expr(condition);
                if let Some(cv) = cond_val {
                    let ci = cv.into_int_value();
                    let cond_bool = if ci.get_type().get_bit_width() > 1 {
                        self.builder.build_int_truncate(ci, self.context.bool_type(), "tobool").unwrap()
                    } else {
                        ci
                    };
                    self.builder.build_conditional_branch(cond_bool, body_bb, exit_bb).unwrap();
                } else {
                    self.builder.build_unconditional_branch(exit_bb).unwrap();
                }

                // Body
                self.builder.position_at_end(body_bb);
                for stmt in body {
                    self.compile_expr(stmt);
                }
                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }

                self.builder.position_at_end(exit_bb);
                None
            }

            Expr::Return { value } => {
                if let Some(val_expr) = value {
                    if let Some(val) = self.compile_expr(val_expr) {
                        self.builder.build_return(Some(&val)).unwrap();
                    } else {
                        self.builder.build_return(None).unwrap();
                    }
                } else {
                    self.builder.build_return(None).unwrap();
                }
                None
            }

            Expr::Assign { target, value } => {
                let val = self.compile_expr(value)?;
                if let Expr::Variable(name) = target.as_ref() {
                    if let Some(&ptr) = self.variables.get(name.as_str()) {
                        self.builder.build_store(ptr, val).unwrap();
                    }
                }
                None
            }

            Expr::FnDef { name, params, return_type, body } => {
                self.compile_fn_def(name, params, return_type, body);
                None
            }

            Expr::Use { .. } => None,
        }
    }

    // ── __assign (let binding) ───────────────────────────────────────

    fn compile_assign_call(&mut self, args: &[Expr]) -> Option<BasicValueEnum<'ctx>> {
        if let Some(Expr::Variable(name)) = args.first() {
            let val = if args.len() > 1 {
                self.compile_expr(&args[1])
            } else {
                None
            };

            let i64_type = self.context.i64_type();
            let alloca_ty: BasicTypeEnum = if let Some(ref v) = val {
                v.get_type()
            } else {
                i64_type.into()
            };

            let alloca = self.builder.build_alloca(alloca_ty, name).unwrap();
            if let Some(v) = val {
                self.builder.build_store(alloca, v).unwrap();
            }
            self.variables.insert(name.clone(), alloca);

            // Track the type for later loads
            let track_ty = self.infer_track_type_from_llvm(alloca_ty);
            self.var_types.insert(name.clone(), track_ty);

            None
        } else {
            None
        }
    }

    // ── print built-in ───────────────────────────────────────────────

    fn compile_print(&mut self, args: &[Expr]) -> Option<BasicValueEnum<'ctx>> {
        let printf = self.get_or_declare_printf();

        if let Some(arg) = args.first() {
            let val = self.compile_expr(arg)?;

            let (fmt, call_args): (&str, Vec<BasicMetadataValueEnum>) = match val {
                BasicValueEnum::IntValue(iv) => {
                    let bits = iv.get_type().get_bit_width();
                    if bits == 1 {
                        // Bool: extend to i32 for printf
                        let ext = self.builder.build_int_z_extend(iv, self.context.i32_type(), "bext").unwrap();
                        ("%d\n", vec![ext.into()])
                    } else {
                        ("%lld\n", vec![iv.into()])
                    }
                }
                BasicValueEnum::PointerValue(pv) => {
                    ("%s\n", vec![pv.into()])
                }
                _ => ("%lld\n", vec![val.into()]),
            };

            let fmt_str = self.builder.build_global_string_ptr(fmt, "fmt").unwrap();
            let mut all_args: Vec<BasicMetadataValueEnum> = vec![fmt_str.as_pointer_value().into()];
            all_args.extend(call_args);
            self.builder.build_call(printf, &all_args, "printf_call").unwrap();
        }
        None
    }

    // ── function definition ──────────────────────────────────────────

    fn compile_fn_def(
        &mut self,
        name: &str,
        params: &[(String, TrackType)],
        return_type: &Option<TrackType>,
        body: &[Expr],
    ) {
        let param_types: Vec<BasicMetadataTypeEnum> = params
            .iter()
            .map(|(_, ty)| self.track_type_to_metadata(ty))
            .collect();

        // For 'main', C ABI requires i32 return type
        let is_main = name == "main";
        let fn_type = if is_main {
            let i32_type = self.context.i32_type();
            i32_type.fn_type(&param_types, false)
        } else {
            match return_type {
                Some(TrackType::Void) | None => {
                    self.context.void_type().fn_type(&param_types, false)
                }
                Some(ty) => {
                    let ret_ty = self.track_type_to_llvm(ty);
                    ret_ty.fn_type(&param_types, false)
                }
            }
        };

        let function = self.module.add_function(name, fn_type, None);
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Save and restore state
        let saved_vars = self.variables.clone();
        let saved_types = self.var_types.clone();
        let saved_fn = self.current_fn;
        self.current_fn = Some(function);

        // Alloca + store params
        for (i, (param_name, param_ty)) in params.iter().enumerate() {
            let llvm_ty = self.track_type_to_llvm(param_ty);
            let alloca = self.builder.build_alloca(llvm_ty, param_name).unwrap();
            let param_val = function.get_nth_param(i as u32).unwrap();
            self.builder.build_store(alloca, param_val).unwrap();
            self.variables.insert(param_name.clone(), alloca);
            self.var_types.insert(param_name.clone(), param_ty.clone());
        }

        // Compile body
        for stmt in body {
            self.compile_expr(stmt);
        }

        // Add implicit return if no terminator
        let current_block = self.builder.get_insert_block().unwrap();
        if current_block.get_terminator().is_none() {
            if is_main {
                let zero = self.context.i32_type().const_int(0, false);
                self.builder.build_return(Some(&zero)).unwrap();
            } else if matches!(return_type, Some(TrackType::Void) | None) {
                self.builder.build_return(None).unwrap();
            }
        }

        // Restore state
        self.variables = saved_vars;
        self.var_types = saved_types;
        self.current_fn = saved_fn;
    }

    // ── compile program ──────────────────────────────────────────────

    pub fn compile_program(&mut self, program: &[Expr]) {
        // First pass: compile all function definitions
        let mut top_level_stmts = Vec::new();
        for stmt in program {
            if let Expr::FnDef { .. } = stmt {
                self.compile_expr(stmt);
            } else {
                top_level_stmts.push(stmt);
            }
        }

        // If there are top-level statements and no user-defined main, wrap them
        if !top_level_stmts.is_empty() && self.module.get_function("main").is_none() {
            let i32_type = self.context.i32_type();
            let fn_type = i32_type.fn_type(&[], false);
            let main_fn = self.module.add_function("main", fn_type, None);
            let entry = self.context.append_basic_block(main_fn, "entry");
            self.builder.position_at_end(entry);
            self.current_fn = Some(main_fn);

            for stmt in &top_level_stmts {
                self.compile_expr(stmt);
            }

            let current_block = self.builder.get_insert_block().unwrap();
            if current_block.get_terminator().is_none() {
                let zero = i32_type.const_int(0, false);
                self.builder.build_return(Some(&zero)).unwrap();
            }
        }
    }

    // ── object file emission ─────────────────────────────────────────

    pub fn write_object_file(&self, path: &Path) -> Result<(), String> {
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| format!("Failed to initialize native target: {}", e))?;

        let triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&triple)
            .map_err(|e| format!("Failed to get target from triple: {}", e))?;

        let machine = target
            .create_target_machine(
                &triple,
                "generic",
                "",
                OptimizationLevel::Default,
                RelocMode::PIC,
                CodeModel::Default,
            )
            .ok_or_else(|| "Failed to create target machine".to_string())?;

        machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| format!("Failed to write object file: {}", e))
    }



    // ── helpers ──────────────────────────────────────────────────────

    fn pointee_type_for(&self, name: &str) -> BasicTypeEnum<'ctx> {
        if let Some(ty) = self.var_types.get(name) {
            self.track_type_to_llvm(ty)
        } else {
            // Default to i64
            self.context.i64_type().into()
        }
    }

    fn deref_type_for(&self, expr: &Expr) -> BasicTypeEnum<'ctx> {
        if let Expr::Variable(name) = expr {
            if let Some(TrackType::Ref(inner)) | Some(TrackType::Ptr(inner)) = self.var_types.get(name) {
                return self.track_type_to_llvm(inner);
            }
        }
        // Default to i64
        self.context.i64_type().into()
    }

    fn infer_track_type_from_llvm(&self, ty: BasicTypeEnum<'ctx>) -> TrackType {
        match ty {
            BasicTypeEnum::IntType(it) => {
                match it.get_bit_width() {
                    1 => TrackType::Bool,
                    32 => TrackType::I32,
                    64 => TrackType::I64,
                    _ => TrackType::I64,
                }
            }
            BasicTypeEnum::PointerType(_) => TrackType::Ptr(Box::new(TrackType::I64)),
            BasicTypeEnum::ArrayType(_) => TrackType::Array(Box::new(TrackType::I64), 0),
            _ => TrackType::I64,
        }
    }

    fn coerce_ints(
        &self,
        a: inkwell::values::IntValue<'ctx>,
        b: inkwell::values::IntValue<'ctx>,
    ) -> (inkwell::values::IntValue<'ctx>, inkwell::values::IntValue<'ctx>) {
        let aw = a.get_type().get_bit_width();
        let bw = b.get_type().get_bit_width();
        if aw == bw {
            (a, b)
        } else if aw > bw {
            let ext = self.builder.build_int_s_extend(b, a.get_type(), "ext").unwrap();
            (a, ext)
        } else {
            let ext = self.builder.build_int_s_extend(a, b.get_type(), "ext").unwrap();
            (ext, b)
        }
    }
}
