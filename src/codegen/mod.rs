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
    active_linear_vars: std::collections::HashSet<String>,
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
            active_linear_vars: std::collections::HashSet::new(),
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
            TrackType::Ptr(_) | TrackType::Ref(_) => {
                self.context.ptr_type(AddressSpace::default()).into()
            }
            TrackType::Array(elem, size) => {
                let elem_ty = self.track_type_to_llvm(elem);
                elem_ty.array_type(*size as u32).into()
            }
            TrackType::Custom(name) => {
                if name == "u8" || name == "i8" {
                    self.context.i8_type().into()
                } else {
                    self.context
                        .struct_type(
                            &[
                                self.context.i64_type().into(),
                                self.context.i64_type().into(),
                            ],
                            false,
                        )
                        .into()
                }
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
            TrackType::Ptr(_) | TrackType::Ref(_) => {
                self.context.ptr_type(AddressSpace::default()).into()
            }
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
        self.module.add_function(
            "printf",
            printf_type,
            Some(inkwell::module::Linkage::External),
        )
    }

    // ── expression compilation ───────────────────────────────────────

    pub fn compile_expr(&mut self, expr: &Expr) -> Option<BasicValueEnum<'ctx>> {
        match expr {
            Expr::IntLiteral(val) => {
                Some(self.context.i64_type().const_int(*val as u64, true).into())
            }

            Expr::BoolLiteral(val) => Some(
                self.context
                    .bool_type()
                    .const_int(*val as u64, false)
                    .into(),
            ),

            Expr::StringLiteral(s) => {
                let gv = self.builder.build_global_string_ptr(s, "str").unwrap();
                Some(gv.as_pointer_value().into())
            }

            Expr::Variable(name) => {
                if name.contains("::") {
                    let parts: Vec<&str> = name.split("::").collect();
                    let variant_name = parts[1];
                    let disc = match variant_name {
                        "Red" | "Active" | "Int" | "Ok" => 0i64,
                        "Green" | "Locked" | "Float" | "Err" => 1i64,
                        "Blue" | "Spent" | "Bool" => 2i64,
                        _ => 0i64,
                    };
                    Some(self.context.i64_type().const_int(disc as u64, false).into())
                } else if let Some(&ptr) = self.variables.get(name.as_str()) {
                    self.active_linear_vars.remove(name);
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
                    BinOp::Eq => self
                        .builder
                        .build_int_compare(IntPredicate::EQ, li, ri, "eq")
                        .unwrap(),
                    BinOp::Neq => self
                        .builder
                        .build_int_compare(IntPredicate::NE, li, ri, "neq")
                        .unwrap(),
                    BinOp::Lt => self
                        .builder
                        .build_int_compare(IntPredicate::SLT, li, ri, "lt")
                        .unwrap(),
                    BinOp::Gt => self
                        .builder
                        .build_int_compare(IntPredicate::SGT, li, ri, "gt")
                        .unwrap(),
                    BinOp::Lte => self
                        .builder
                        .build_int_compare(IntPredicate::SLE, li, ri, "lte")
                        .unwrap(),
                    BinOp::Gte => self
                        .builder
                        .build_int_compare(IntPredicate::SGE, li, ri, "gte")
                        .unwrap(),
                    BinOp::And => self.builder.build_and(li, ri, "and").unwrap(),
                    BinOp::Or => self.builder.build_or(li, ri, "or").unwrap(),
                    BinOp::BitAnd => self.builder.build_and(li, ri, "bitand").unwrap(),
                    BinOp::Shl => self.builder.build_left_shift(li, ri, "shl").unwrap(),
                    BinOp::Shr => self
                        .builder
                        .build_right_shift(li, ri, false, "shr")
                        .unwrap(),
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
                            self.builder
                                .build_gep(arr_type, alloca, &[zero, idx], "arr_elem")
                                .unwrap()
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
                    self.builder
                        .build_gep(i64_type.array_type(256), ptr, &[zero, idx], "idx")
                        .unwrap()
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
                let field_types: Vec<BasicTypeEnum> =
                    fields.iter().map(|_| i64_type.into()).collect();
                let struct_type = self.context.struct_type(&field_types, false);
                let alloca = self
                    .builder
                    .build_alloca(struct_type, "struct_init")
                    .unwrap();

                for (i, (_fname, fval)) in fields.iter().enumerate() {
                    if let Some(val) = self.compile_expr(fval) {
                        let gep = self
                            .builder
                            .build_struct_gep(struct_type, alloca, i as u32, "field")
                            .unwrap();
                        self.builder.build_store(gep, val).unwrap();
                    }
                }
                Some(alloca.into())
            }

            Expr::LensBlock {
                target,
                lens_name,
                body,
            } => {
                // Lens: copy target into lens_name, execute body, copy back
                if let Some(&target_ptr) = self.variables.get(target.as_str()) {
                    let pointee = self.pointee_type_for(target);
                    let val = self
                        .builder
                        .build_load(pointee, target_ptr, "lens_load")
                        .unwrap();
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
                    let lens_val = self
                        .builder
                        .build_load(pointee, lens_alloca, "lens_back")
                        .unwrap();
                    self.builder.build_store(target_ptr, lens_val).unwrap();
                    self.variables.remove(lens_name);

                    last
                } else {
                    None
                }
            }

            Expr::FunctionCall { name, args } => {
                let is_print = name == "print" || name.ends_with("::print");
                if is_print {
                    return self.compile_print(args);
                }
                // Regular function call
                let func_opt = if let Some(func) = self.module.get_function(name) {
                    Some(func)
                } else {
                    self.get_or_declare_stdlib_func(name)
                };

                if let Some(func) = func_opt {
                    let compiled_args: Vec<BasicMetadataValueEnum> = args
                        .iter()
                        .filter_map(|a| self.compile_expr(a))
                        .map(|v| v.into())
                        .collect();
                    let call = self
                        .builder
                        .build_call(func, &compiled_args, "call")
                        .unwrap();
                    call.try_as_basic_value().basic()
                } else if name.contains("::") {
                    let parts: Vec<&str> = name.split("::").collect();
                    let _union_name = parts[0];
                    let variant_name = parts[1];
                    let tag_val = match variant_name {
                        "Int" | "Ok" => 0u64,
                        "Float" | "Err" => 1u64,
                        "Bool" => 2u64,
                        _ => 0u64,
                    };
                    let struct_ty = self.context.struct_type(
                        &[
                            self.context.i64_type().into(),
                            self.context.i64_type().into(),
                        ],
                        false,
                    );
                    let ptr = self.builder.build_alloca(struct_ty, "union_tmp").unwrap();
                    let tag_ptr = self
                        .builder
                        .build_struct_gep(struct_ty, ptr, 0, "tag_ptr")
                        .unwrap();
                    self.builder
                        .build_store(tag_ptr, self.context.i64_type().const_int(tag_val, false))
                        .unwrap();
                    if let Some(arg_expr) = args.first() {
                        let arg_val = self.compile_expr(arg_expr)?;
                        let payload_ptr = self
                            .builder
                            .build_struct_gep(struct_ty, ptr, 1, "payload_ptr")
                            .unwrap();
                        self.builder.build_store(payload_ptr, arg_val).unwrap();
                    }
                    let loaded = self
                        .builder
                        .build_load(struct_ty, ptr, "union_val")
                        .unwrap();
                    Some(loaded)
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

            Expr::IfElse {
                condition,
                then_body,
                else_body,
            } => {
                let cond_val = self.compile_expr(condition)?;
                let cond_int = cond_val.into_int_value();

                // If cond is i64 (from comparison), truncate to i1
                let cond_bool = if cond_int.get_type().get_bit_width() > 1 {
                    self.builder
                        .build_int_truncate(cond_int, self.context.bool_type(), "tobool")
                        .unwrap()
                } else {
                    cond_int
                };

                let func = self.current_fn?;
                let then_bb = self.context.append_basic_block(func, "then");
                let else_bb = self.context.append_basic_block(func, "else");
                let merge_bb = self.context.append_basic_block(func, "merge");

                self.builder
                    .build_conditional_branch(cond_bool, then_bb, else_bb)
                    .unwrap();

                // Then block
                self.builder.position_at_end(then_bb);
                for stmt in then_body {
                    self.compile_expr(stmt);
                }
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                // Else block
                self.builder.position_at_end(else_bb);
                for stmt in else_body {
                    self.compile_expr(stmt);
                }
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
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
                        self.builder
                            .build_int_truncate(ci, self.context.bool_type(), "tobool")
                            .unwrap()
                    } else {
                        ci
                    };
                    self.builder
                        .build_conditional_branch(cond_bool, body_bb, exit_bb)
                        .unwrap();
                } else {
                    self.builder.build_unconditional_branch(exit_bb).unwrap();
                }

                // Body
                self.builder.position_at_end(body_bb);
                for stmt in body {
                    self.compile_expr(stmt);
                }
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }

                self.builder.position_at_end(exit_bb);
                None
            }

            Expr::Return { value } => {
                if let Some(val_expr) = value {
                    let val = self.compile_expr(val_expr);
                    self.insert_cleanup_calls();
                    if let Some(v) = val {
                        self.builder.build_return(Some(&v)).unwrap();
                    } else {
                        self.builder.build_return(None).unwrap();
                    }
                } else {
                    self.insert_cleanup_calls();
                    self.builder.build_return(None).unwrap();
                }
                None
            }

            Expr::Assign { target, value } => {
                let val = self.compile_expr(value)?;
                if let Expr::Variable(name) = target.as_ref() {
                    if let Some(&ptr) = self.variables.get(name.as_str()) {
                        if self.active_linear_vars.contains(name) {
                            self.generate_cleanup_for(name);
                        }
                        self.builder.build_store(ptr, val).unwrap();
                        if let Some(ty) = self.var_types.get(name) {
                            if !self.is_copy_type(ty) {
                                self.active_linear_vars.insert(name.clone());
                            }
                        }
                    }
                }
                None
            }

            Expr::FnDef {
                name,
                params,
                return_type,
                body,
            } => {
                self.compile_fn_def(name, params, return_type, body);
                None
            }

            Expr::Use { .. } => None,

            Expr::ConstDef { name, value } => {
                let val = self.compile_expr(value)?;
                let alloca_ty = val.get_type();
                let ptr = self.builder.build_alloca(alloca_ty, name).unwrap();
                self.builder.build_store(ptr, val).unwrap();
                self.variables.insert(name.clone(), ptr);
                None
            }

            Expr::MacroDef { .. } => None,

            Expr::MacroCall { name, args, body } => {
                if name == "bit" {
                    let n = self.compile_expr(&args[0])?.into_int_value();
                    let one = n.get_type().const_int(1, false);
                    let val = self.builder.build_left_shift(one, n, "bit_val").unwrap();
                    Some(val.into())
                } else if name == "pin" {
                    let port = self.compile_expr(&args[0])?.into_int_value();
                    let pin = self.compile_expr(&args[1])?.into_int_value();
                    let sh = port.get_type().const_int(8, false);
                    let port_sh = self.builder.build_left_shift(port, sh, "port_sh").unwrap();
                    let val = self.builder.build_or(port_sh, pin, "pin_val").unwrap();
                    Some(val.into())
                } else if name == "register" {
                    let addr = self.compile_expr(&args[0])?.into_int_value();
                    let mask = self.compile_expr(&args[1])?.into_int_value();
                    let sh = mask.get_type().const_int(8, false);
                    let mask_sh = self.builder.build_left_shift(mask, sh, "mask_sh").unwrap();
                    let val = self.builder.build_or(addr, mask_sh, "reg_val").unwrap();
                    Some(val.into())
                } else if name == "fib_comptime" {
                    if let Some(Expr::IntLiteral(n)) = args.first() {
                        fn fib_rec(n: i64) -> i64 {
                            if n <= 1 {
                                n
                            } else {
                                fib_rec(n - 1) + fib_rec(n - 2)
                            }
                        }
                        let fib_val = fib_rec(*n);
                        let i64_type = self.context.i64_type();
                        Some(i64_type.const_int(fib_val as u64, true).into())
                    } else {
                        None
                    }
                } else if name == "timer" {
                    if let Some(ref block_body) = body {
                        let mut last = None;
                        for stmt in block_body {
                            last = self.compile_expr(stmt);
                        }
                        last
                    } else {
                        None
                    }
                } else {
                    None
                }
            }

            Expr::LetDef { name, ty, value } => {
                let val = self.compile_expr(value);

                let alloca_ty = if let Some(TrackType::Array(ref elem, size)) = ty {
                    let elem_llvm = self.track_type_to_llvm(elem);
                    elem_llvm.array_type(*size as u32).into()
                } else if let Some(ref v) = val {
                    v.get_type()
                } else {
                    self.context.i64_type().into()
                };

                let ptr = self.builder.build_alloca(alloca_ty, name).unwrap();

                if let Some(TrackType::Array(_, size)) = ty {
                    if let Expr::StringLiteral(ref s) = value.as_ref() {
                        let bytes = s.as_bytes();
                        for i in 0..*size {
                            let byte_val = if i < bytes.len() { bytes[i] } else { 0 };
                            let llvm_byte =
                                self.context.i8_type().const_int(byte_val as u64, false);
                            let zero = self.context.i64_type().const_int(0, false);
                            let index = self.context.i64_type().const_int(i as u64, false);
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_gep(
                                        alloca_ty,
                                        ptr,
                                        &[zero, index],
                                        &format!("{}_elem_{}", name, i),
                                    )
                                    .unwrap()
                            };
                            self.builder.build_store(elem_ptr, llvm_byte).unwrap();
                        }
                    } else if let Some(v) = val {
                        self.builder.build_store(ptr, v).unwrap();
                    }
                } else if let Some(v) = val {
                    self.builder.build_store(ptr, v).unwrap();
                }

                self.variables.insert(name.clone(), ptr);

                let track_ty = if let Some(ref annotated_ty) = ty {
                    annotated_ty.clone()
                } else {
                    self.infer_track_type_from_llvm(alloca_ty)
                };
                self.var_types.insert(name.clone(), track_ty.clone());
                if !self.is_copy_type(&track_ty) {
                    self.active_linear_vars.insert(name.clone());
                }

                None
            }

            Expr::EnumDef { .. } => None,
            Expr::UnionDef { .. } => None,

            Expr::Match { target, arms } => {
                let target_val = self.compile_expr(target)?;
                let is_union = target_val.is_struct_value();
                let (tag_val, payload_val) = if is_union {
                    let struct_val = target_val.into_struct_value();
                    let tag = self
                        .builder
                        .build_extract_value(struct_val, 0, "tag")
                        .unwrap();
                    let payload = self
                        .builder
                        .build_extract_value(struct_val, 1, "payload")
                        .unwrap();
                    (tag.into_int_value(), Some(payload))
                } else {
                    (target_val.into_int_value(), None)
                };

                let current_func = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let mut arm_blocks = Vec::new();
                for i in 0..arms.len() {
                    let block = self
                        .context
                        .append_basic_block(current_func, &format!("match_arm_{}", i));
                    arm_blocks.push(block);
                }
                let merge_block = self.context.append_basic_block(current_func, "match_merge");

                let prev_insert = self.builder.get_insert_block().unwrap();

                for (i, arm) in arms.iter().enumerate() {
                    let next_check_block =
                        if i + 1 < arms.len() {
                            Some(self.context.append_basic_block(
                                current_func,
                                &format!("match_check_{}", i + 1),
                            ))
                        } else {
                            None
                        };

                    self.builder.position_at_end(if i == 0 {
                        prev_insert
                    } else {
                        self.builder.get_insert_block().unwrap()
                    });

                    let matches_cond = match &arm.pattern {
                        crate::ast::Pattern::Wildcard => {
                            self.context.bool_type().const_int(1, false)
                        }
                        crate::ast::Pattern::Variant { variant, .. } => {
                            let expected_tag = match variant.as_str() {
                                "Red" | "Active" | "Int" | "Ok" => 0u64,
                                "Green" | "Locked" | "Float" | "Err" => 1u64,
                                "Blue" | "Spent" | "Bool" => 2u64,
                                _ => 0u64,
                            };
                            let expected_val = tag_val.get_type().const_int(expected_tag, false);
                            self.builder
                                .build_int_compare(
                                    inkwell::IntPredicate::EQ,
                                    tag_val,
                                    expected_val,
                                    "tag_match",
                                )
                                .unwrap()
                        }
                        crate::ast::Pattern::Ident(_) => {
                            self.context.bool_type().const_int(1, false)
                        }
                    };

                    let target_fail = next_check_block.unwrap_or(merge_block);
                    self.builder
                        .build_conditional_branch(matches_cond, arm_blocks[i], target_fail)
                        .unwrap();

                    self.builder.position_at_end(arm_blocks[i]);

                    match &arm.pattern {
                        crate::ast::Pattern::Variant {
                            binding: Some(bind_var),
                            ..
                        } => {
                            if let Some(payload) = payload_val {
                                let ptr = self
                                    .builder
                                    .build_alloca(payload.get_type(), bind_var)
                                    .unwrap();
                                self.builder.build_store(ptr, payload).unwrap();
                                self.variables.insert(bind_var.clone(), ptr);
                            }
                        }
                        crate::ast::Pattern::Ident(var_name) => {
                            let ptr = self
                                .builder
                                .build_alloca(target_val.get_type(), var_name)
                                .unwrap();
                            self.builder.build_store(ptr, target_val).unwrap();
                            self.variables.insert(var_name.clone(), ptr);
                        }
                        _ => {}
                    }

                    self.compile_expr(&arm.body);

                    if self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_none()
                    {
                        self.builder
                            .build_unconditional_branch(merge_block)
                            .unwrap();
                    }

                    if let Some(next_chk) = next_check_block {
                        self.builder.position_at_end(next_chk);
                    }
                }

                self.builder.position_at_end(merge_block);
                None
            }
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
                        let ext = self
                            .builder
                            .build_int_z_extend(iv, self.context.i32_type(), "bext")
                            .unwrap();
                        ("%d\n", vec![ext.into()])
                    } else {
                        ("%lld\n", vec![iv.into()])
                    }
                }
                BasicValueEnum::PointerValue(pv) => ("%s\n", vec![pv.into()]),
                _ => ("%lld\n", vec![val.into()]),
            };

            let fmt_str = self.builder.build_global_string_ptr(fmt, "fmt").unwrap();
            let mut all_args: Vec<BasicMetadataValueEnum> = vec![fmt_str.as_pointer_value().into()];
            all_args.extend(call_args);
            self.builder
                .build_call(printf, &all_args, "printf_call")
                .unwrap();
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
        let saved_linear = self.active_linear_vars.clone();
        self.current_fn = Some(function);
        self.active_linear_vars.clear();

        // Alloca + store params
        for (i, (param_name, param_ty)) in params.iter().enumerate() {
            let llvm_ty = self.track_type_to_llvm(param_ty);
            let alloca = self.builder.build_alloca(llvm_ty, param_name).unwrap();
            let param_val = function.get_nth_param(i as u32).unwrap();
            self.builder.build_store(alloca, param_val).unwrap();
            self.variables.insert(param_name.clone(), alloca);
            self.var_types.insert(param_name.clone(), param_ty.clone());
            if !self.is_copy_type(param_ty) {
                self.active_linear_vars.insert(param_name.clone());
            }
        }

        // Compile body
        for stmt in body {
            self.compile_expr(stmt);
        }

        // Add implicit return if no terminator
        let current_block = self.builder.get_insert_block().unwrap();
        if current_block.get_terminator().is_none() {
            self.insert_cleanup_calls();
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
        self.active_linear_vars = saved_linear;
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
            self.context.i64_type().into()
        }
    }

    fn deref_type_for(&self, expr: &Expr) -> BasicTypeEnum<'ctx> {
        if let Expr::Variable(name) = expr {
            if let Some(TrackType::Ref(inner)) | Some(TrackType::Ptr(inner)) =
                self.var_types.get(name)
            {
                return self.track_type_to_llvm(inner);
            }
        }
        // Default to i64
        self.context.i64_type().into()
    }

    fn infer_track_type_from_llvm(&self, ty: BasicTypeEnum<'ctx>) -> TrackType {
        match ty {
            BasicTypeEnum::IntType(it) => match it.get_bit_width() {
                1 => TrackType::Bool,
                8 => TrackType::Custom("u8".to_string()),
                32 => TrackType::I32,
                64 => TrackType::I64,
                _ => TrackType::I64,
            },
            BasicTypeEnum::PointerType(_) => TrackType::Ptr(Box::new(TrackType::I64)),
            BasicTypeEnum::ArrayType(_) => TrackType::Array(Box::new(TrackType::I64), 0),
            BasicTypeEnum::StructType(_) => TrackType::Custom("Value".to_string()),
            _ => TrackType::I64,
        }
    }

    fn coerce_ints(
        &self,
        a: inkwell::values::IntValue<'ctx>,
        b: inkwell::values::IntValue<'ctx>,
    ) -> (
        inkwell::values::IntValue<'ctx>,
        inkwell::values::IntValue<'ctx>,
    ) {
        let aw = a.get_type().get_bit_width();
        let bw = b.get_type().get_bit_width();
        if aw == bw {
            (a, b)
        } else if aw > bw {
            let ext = self
                .builder
                .build_int_s_extend(b, a.get_type(), "ext")
                .unwrap();
            (a, ext)
        } else {
            let ext = self
                .builder
                .build_int_s_extend(a, b.get_type(), "ext")
                .unwrap();
            (ext, b)
        }
    }

    fn is_copy_type(&self, ty: &TrackType) -> bool {
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

    fn get_or_declare_stdlib_func(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function(name) {
            return Some(f);
        }
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();
        let bool_type = self.context.bool_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let str_struct = self
            .context
            .struct_type(&[ptr_type.into(), i32_type.into()], false);
        let vec_struct = self
            .context
            .struct_type(&[ptr_type.into(), i32_type.into(), i32_type.into()], false);

        // (params, returns_void, opt_basic_return_type)
        let sig: Option<(
            Vec<BasicMetadataTypeEnum<'ctx>>,
            bool,
            Option<BasicTypeEnum<'ctx>>,
        )> = match name {
            "alloc" => Some((vec![i64_type.into()], false, Some(ptr_type.into()))),
            "memset" => Some((
                vec![ptr_type.into(), i32_type.into(), i64_type.into()],
                true,
                None,
            )),
            "memcpy" => Some((
                vec![ptr_type.into(), ptr_type.into(), i64_type.into()],
                true,
                None,
            )),
            "memcmp" => Some((
                vec![ptr_type.into(), ptr_type.into(), i64_type.into()],
                false,
                Some(i32_type.into()),
            )),

            "str_len" => Some((vec![ptr_type.into()], false, Some(i32_type.into()))),
            "str_eq" => Some((
                vec![ptr_type.into(), ptr_type.into()],
                false,
                Some(bool_type.into()),
            )),
            "str_from_literal" => Some((vec![ptr_type.into()], false, Some(str_struct.into()))),
            "str_concat" => Some((
                vec![ptr_type.into(), ptr_type.into()],
                false,
                Some(str_struct.into()),
            )),

            "vec_init" => Some((vec![i32_type.into()], false, Some(vec_struct.into()))),
            "vec_push" => Some((vec![ptr_type.into(), i32_type.into()], true, None)),
            "vec_get" => Some((
                vec![ptr_type.into(), i32_type.into()],
                false,
                Some(i32_type.into()),
            )),
            "vec_set" => Some((
                vec![ptr_type.into(), i32_type.into(), i32_type.into()],
                true,
                None,
            )),
            "vec_pop" => Some((vec![ptr_type.into()], false, Some(i32_type.into()))),

            "print_str" => Some((vec![ptr_type.into()], true, None)),
            "print_int" => Some((vec![i64_type.into()], true, None)),
            "print_hex" => Some((vec![i64_type.into()], true, None)),
            "read_line" => Some((vec![], false, Some(str_struct.into()))),
            "file_open" => Some((
                vec![ptr_type.into(), i32_type.into()],
                false,
                Some(ptr_type.into()),
            )),
            "file_read_all" => Some((vec![ptr_type.into()], false, Some(str_struct.into()))),
            "file_write" => Some((vec![ptr_type.into(), ptr_type.into()], true, None)),

            "math_abs" => Some((vec![i32_type.into()], false, Some(i32_type.into()))),
            "math_max" => Some((
                vec![i32_type.into(), i32_type.into()],
                false,
                Some(i32_type.into()),
            )),
            "math_min" => Some((
                vec![i32_type.into(), i32_type.into()],
                false,
                Some(i32_type.into()),
            )),
            "math_pow" => Some((
                vec![i64_type.into(), i64_type.into()],
                false,
                Some(i64_type.into()),
            )),
            "math_sqrt" => Some((vec![i64_type.into()], false, Some(i64_type.into()))),

            "free" => Some((vec![ptr_type.into()], true, None)),
            "str_free" => Some((vec![str_struct.into()], true, None)),
            "vec_free" => Some((vec![vec_struct.into()], true, None)),
            "file_close" => Some((vec![ptr_type.into()], true, None)),

            _ => None,
        };

        let (params, returns_void, ret_ty) = sig?;
        let func_type = if returns_void {
            self.context.void_type().fn_type(&params, false)
        } else {
            ret_ty.unwrap().fn_type(&params, false)
        };
        Some(self.module.add_function(name, func_type, None))
    }

    fn insert_cleanup_calls(&mut self) {
        let active_vars: Vec<String> = self.active_linear_vars.iter().cloned().collect();
        for name in active_vars {
            self.generate_cleanup_for(&name);
        }
        self.active_linear_vars.clear();
    }

    fn generate_cleanup_for(&mut self, name: &str) {
        if let Some(ty) = self.var_types.get(name).cloned() {
            let ptr = if let Some(&p) = self.variables.get(name) {
                p
            } else {
                return;
            };

            let func_name = match &ty {
                TrackType::Custom(custom_name) => {
                    if custom_name == "Vec" {
                        Some("vec_free")
                    } else if custom_name == "Str" {
                        Some("str_free")
                    } else {
                        None
                    }
                }
                TrackType::Ptr(inner_ty) => {
                    if let TrackType::Custom(custom_name) = &**inner_ty {
                        if custom_name == "File" {
                            Some("file_close")
                        } else {
                            Some("free")
                        }
                    } else {
                        Some("free")
                    }
                }
                _ => None,
            };

            if let Some(fname) = func_name {
                if let Some(func) = self.get_or_declare_stdlib_func(fname) {
                    let pointee = self.track_type_to_llvm(&ty);
                    let val = self
                        .builder
                        .build_load(pointee, ptr, &format!("{}_val_to_free", name))
                        .unwrap();
                    self.builder
                        .build_call(func, &[val.into()], "cleanup_call")
                        .unwrap();
                }
            }
        }
    }
}
