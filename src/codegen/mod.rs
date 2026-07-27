use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use cranelift_codegen::ir::{self, AbiParam, InstBuilder, Value};
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{default_libcall_names, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;

use crate::ast::{BinOp, Expr, Pattern, TrackType, UnaryOp};

pub struct CodeGen {
    module_name: String,
    module: ObjectModule,
    fn_builder_ctx: FunctionBuilderContext,
    functions: HashMap<String, FuncId>,
    variant_map: HashMap<String, i64>,
}

impl CodeGen {
    pub fn create_default_isa() -> Arc<dyn isa::TargetIsa> {
        let mut flag_builder = settings::builder();
        flag_builder.set("is_pic", "false").unwrap();
        flag_builder.set("opt_level", "speed").unwrap();
        let isa_flags = settings::Flags::new(flag_builder);

        isa::lookup(Triple::host())
            .unwrap()
            .finish(isa_flags)
            .unwrap()
    }

    pub fn new(module_name: &str) -> Self {
        Self::new_with_isa(module_name, Self::create_default_isa())
    }

    pub fn new_with_isa(module_name: &str, isa: Arc<dyn isa::TargetIsa>) -> Self {
        let object_builder =
            ObjectBuilder::new(isa, module_name, default_libcall_names()).unwrap();
        let module = ObjectModule::new(object_builder);

        let mut variant_map = HashMap::new();
        variant_map.insert("Red".to_string(), 0);
        variant_map.insert("Green".to_string(), 1);
        variant_map.insert("Blue".to_string(), 2);
        variant_map.insert("Active".to_string(), 0);
        variant_map.insert("Locked".to_string(), 1);
        variant_map.insert("Spent".to_string(), 2);
        variant_map.insert("Int".to_string(), 0);
        variant_map.insert("Float".to_string(), 1);
        variant_map.insert("Bool".to_string(), 2);
        variant_map.insert("Ok".to_string(), 0);
        variant_map.insert("Err".to_string(), 1);

        Self {
            module_name: module_name.to_string(),
            module,
            fn_builder_ctx: FunctionBuilderContext::new(),
            functions: HashMap::new(),
            variant_map,
        }
    }

    pub fn compile_program(&mut self, program: &[Expr]) {
        // First pass: declare all functions, macros, enums, and unions
        for expr in program {
            match expr {
                Expr::EnumDef { name, variants, .. } => {
                    for (idx, (vname, _)) in variants.iter().enumerate() {
                        self.variant_map.insert(vname.clone(), idx as i64);
                        self.variant_map.insert(format!("{}::{}", name, vname), idx as i64);
                    }
                }
                Expr::UnionDef { name, variants } => {
                    for (idx, (vname, _)) in variants.iter().enumerate() {
                        self.variant_map.insert(vname.clone(), idx as i64);
                        self.variant_map.insert(format!("{}::{}", name, vname), idx as i64);
                    }
                }
                Expr::FnDef {
                    name,
                    params,
                    return_type,
                    ..
                }
                | Expr::MacroDef {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let mut sig = self.module.make_signature();
                    for (_, pty) in params {
                        sig.params.push(AbiParam::new(track_type_to_cl(pty)));
                    }
                    if name == "main" {
                        sig.returns.push(AbiParam::new(ir::types::I32));
                    } else if let Some(rty) = return_type
                        && *rty != TrackType::Void {
                            sig.returns.push(AbiParam::new(track_type_to_cl(rty)));
                        }

                    let func_id = self
                        .module
                        .declare_function(name, Linkage::Export, &sig)
                        .unwrap_or_else(|_| self.module.declare_anonymous_function(&sig).unwrap());
                    self.functions.insert(name.clone(), func_id);
                }
                _ => {}
            }
        }

        // Declare built-in print / printf if needed
        let mut print_sig = self.module.make_signature();
        print_sig.params.push(AbiParam::new(ir::types::I64));
        let print_id = self
            .module
            .declare_function("print", Linkage::Import, &print_sig)
            .unwrap_or_else(|_| self.module.declare_anonymous_function(&print_sig).unwrap());
        self.functions.entry("print".to_string()).or_insert(print_id);

        let mut has_main = false;

        // Second pass: define function bodies
        for expr in program {
            if let Expr::FnDef {
                name,
                params,
                return_type,
                body,
            }
            | Expr::MacroDef {
                name,
                params,
                return_type,
                body,
            } = expr
            {
                if name == "main" {
                    has_main = true;
                }
                let func_id = *self.functions.get(name).unwrap();
                self.compile_fn(func_id, name, params, return_type.as_ref(), body, program);
            }
        }

        // Synthesize C main wrapper if needed (only for entry point modules)
        if !has_main
            && (self.module_name == "main"
                || self.module_name == "track_module"
                || self.module_name.ends_with("main"))
        {
            self.synthesize_main(program);
        }
    }

    fn compile_fn(
        &mut self,
        func_id: FuncId,
        name: &str,
        params: &[(String, TrackType)],
        return_type: Option<&TrackType>,
        body: &[Expr],
        program: &[Expr],
    ) {
        let is_main = name == "main";
        let mut ctx = self.module.make_context();
        let mut sig = self.module.make_signature();
        for (_, pty) in params {
            sig.params.push(AbiParam::new(track_type_to_cl(pty)));
        }
        if is_main {
            sig.returns.push(AbiParam::new(ir::types::I32));
        } else if let Some(rty) = return_type
            && *rty != TrackType::Void {
                sig.returns.push(AbiParam::new(track_type_to_cl(rty)));
            }
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.fn_builder_ctx);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);

        let mut var_map: HashMap<String, Variable> = HashMap::new();
        let mut var_counter = 0u32;

        for (idx, (pname, pty)) in params.iter().enumerate() {
            let var = Variable::from_u32(var_counter);
            var_counter += 1;
            builder.declare_var(var, track_type_to_cl(pty));
            let val = builder.block_params(entry_block)[idx];
            builder.def_var(var, val);
            var_map.insert(pname.clone(), var);
        }

        let mut fn_ctx = FnContext {
            var_map,
            var_counter,
            functions: &mut self.functions,
            variant_map: &self.variant_map,
        };

        // If compiling main, compile top-level statements (global variables/consts) into entry block first
        if is_main {
            for top_stmt in program {
                if !matches!(top_stmt, Expr::FnDef { .. } | Expr::MacroDef { .. }) {
                    fn_ctx.compile_expr(&mut builder, &mut self.module, top_stmt);
                }
            }
        }

        let mut last_val = None;
        let mut has_return_stmt = false;
        for stmt in body {
            if builder.is_unreachable() {
                break;
            }
            if matches!(stmt, Expr::Return { .. }) {
                has_return_stmt = true;
            }
            last_val = fn_ctx.compile_expr(&mut builder, &mut self.module, stmt);
        }

        if !builder.is_unreachable() && !has_return_stmt {
            if is_main {
                let zero = builder.ins().iconst(ir::types::I32, 0);
                builder.ins().return_(&[zero]);
            } else if return_type.is_none() || return_type == Some(&TrackType::Void) {
                builder.ins().return_(&[]);
            } else if let Some(v) = last_val {
                builder.ins().return_(&[v]);
            } else {
                let default_ret = builder.ins().iconst(ir::types::I64, 0);
                builder.ins().return_(&[default_ret]);
            }
        }

        builder.seal_all_blocks();
        builder.finalize();
        self.module.define_function(func_id, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
    }

    fn synthesize_main(&mut self, program: &[Expr]) {
        let mut sig = self.module.make_signature();
        sig.returns.push(AbiParam::new(ir::types::I32));

        let main_id = self
            .module
            .declare_function("main", Linkage::Export, &sig)
            .unwrap();

        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.fn_builder_ctx);
        let entry_block = builder.create_block();
        builder.switch_to_block(entry_block);

        let mut fn_ctx = FnContext {
            var_map: HashMap::new(),
            var_counter: 0,
            functions: &mut self.functions,
            variant_map: &self.variant_map,
        };

        let mut has_return_stmt = false;
        for stmt in program {
            if !matches!(stmt, Expr::FnDef { .. } | Expr::MacroDef { .. }) {
                if matches!(stmt, Expr::Return { .. }) {
                    has_return_stmt = true;
                }
                fn_ctx.compile_expr(&mut builder, &mut self.module, stmt);
            }
        }

        if !builder.is_unreachable() && !has_return_stmt {
            let ret_code = builder.ins().iconst(ir::types::I32, 0);
            builder.ins().return_(&[ret_code]);
        }

        builder.seal_all_blocks();
        builder.finalize();
        self.module.define_function(main_id, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
    }

    pub fn write_object_file(self, output_path: &Path) -> Result<(), String> {
        let product = self.module.finish();
        let bytes = product
            .emit()
            .map_err(|e| format!("Failed to emit Cranelift object module: {}", e))?;
        fs::write(output_path, bytes)
            .map_err(|e| format!("Failed to write object file '{}': {}", output_path.display(), e))
    }
}

struct FnContext<'a> {
    var_map: HashMap<String, Variable>,
    var_counter: u32,
    functions: &'a mut HashMap<String, FuncId>,
    variant_map: &'a HashMap<String, i64>,
}

impl<'a> FnContext<'a> {
    fn new_var(&mut self) -> Variable {
        let var = Variable::from_u32(self.var_counter);
        self.var_counter += 1;
        var
    }

    fn compile_expr(
        &mut self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        expr: &Expr,
    ) -> Option<Value> {
        match expr {
            Expr::IntLiteral(val) => Some(builder.ins().iconst(ir::types::I64, *val)),

            Expr::BoolLiteral(val) => Some(builder.ins().iconst(ir::types::I8, if *val { 1 } else { 0 })),

            Expr::StringLiteral(s) => {
                let mut data_ctx = cranelift_module::DataDescription::new();
                let mut bytes = s.as_bytes().to_vec();
                bytes.push(0); // null terminate
                data_ctx.define(bytes.into_boxed_slice());

                let data_id = module
                    .declare_anonymous_data(true, false)
                    .unwrap();
                module.define_data(data_id, &data_ctx).unwrap();

                let local_data = module.declare_data_in_func(data_id, builder.func);
                let ptr = builder.ins().symbol_value(ir::types::I64, local_data);
                Some(ptr)
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
                    Some(builder.ins().iconst(ir::types::I64, disc))
                } else if let Some(&var) = self.var_map.get(name) {
                    Some(builder.use_var(var))
                } else {
                    None
                }
            }

            Expr::LetDef { name, value, .. } => {
                let val = self.compile_expr(builder, module, value)?;
                let var = self.new_var();
                let ty = builder.func.dfg.value_type(val);
                builder.declare_var(var, ty);
                builder.def_var(var, val);
                self.var_map.insert(name.clone(), var);
                Some(val)
            }

            Expr::Assign { target, value } => {
                let val = self.compile_expr(builder, module, value)?;
                if let Expr::Variable(name) = &**target
                    && let Some(&var) = self.var_map.get(name) {
                        builder.def_var(var, val);
                    }
                Some(val)
            }

            Expr::BinaryOp { op, left, right } => {
                let lhs = self.compile_expr(builder, module, left)?;
                let rhs = self.compile_expr(builder, module, right)?;

                let res = match op {
                    BinOp::Add => builder.ins().iadd(lhs, rhs),
                    BinOp::Sub => builder.ins().isub(lhs, rhs),
                    BinOp::Mul => builder.ins().imul(lhs, rhs),
                    BinOp::Div => builder.ins().sdiv(lhs, rhs),
                    BinOp::Mod => builder.ins().srem(lhs, rhs),
                    BinOp::Eq => builder.ins().icmp(ir::condcodes::IntCC::Equal, lhs, rhs),
                    BinOp::Neq => builder.ins().icmp(ir::condcodes::IntCC::NotEqual, lhs, rhs),
                    BinOp::Lt => builder.ins().icmp(ir::condcodes::IntCC::SignedLessThan, lhs, rhs),
                    BinOp::Gt => builder.ins().icmp(ir::condcodes::IntCC::SignedGreaterThan, lhs, rhs),
                    BinOp::Lte => builder.ins().icmp(ir::condcodes::IntCC::SignedLessThanOrEqual, lhs, rhs),
                    BinOp::Gte => builder.ins().icmp(ir::condcodes::IntCC::SignedGreaterThanOrEqual, lhs, rhs),
                    BinOp::And => builder.ins().band(lhs, rhs),
                    BinOp::Or => builder.ins().bor(lhs, rhs),
                    BinOp::BitAnd => builder.ins().band(lhs, rhs),
                    BinOp::BitOr => builder.ins().bor(lhs, rhs),
                    BinOp::Shl => builder.ins().ishl(lhs, rhs),
                    BinOp::Shr => builder.ins().ushr(lhs, rhs),
                };
                Some(res)
            }

            Expr::UnaryOp { op, expr } => {
                let val = self.compile_expr(builder, module, expr)?;
                match op {
                    UnaryOp::Neg => Some(builder.ins().ineg(val)),
                    UnaryOp::Not => Some(builder.ins().bnot(val)),
                    UnaryOp::Deref => Some(val),
                }
            }

            Expr::AddressOf { target } => self.compile_expr(builder, module, target),

            Expr::FunctionCall { name, args } | Expr::MacroCall { name, args, .. } => {
                let mut arg_vals = Vec::new();
                for arg in args {
                    let v = self.compile_expr(builder, module, arg).unwrap_or_else(|| {
                        builder.ins().iconst(ir::types::I64, 0)
                    });
                    arg_vals.push(v);
                }

                let clean_name = name.trim_start_matches('@');
                let target_name = if clean_name.contains("::") {
                    clean_name.split("::").last().unwrap()
                } else {
                    clean_name
                };

                if let Some(&disc) = self
                    .variant_map
                    .get(name)
                    .or_else(|| self.variant_map.get(target_name))
                {
                    return arg_vals.first().copied().or_else(|| {
                        Some(builder.ins().iconst(ir::types::I64, disc))
                    });
                }

                let func_id = if let Some(&fid) = self.functions.get(name) {
                    fid
                } else if let Some(&fid) = self.functions.get(clean_name) {
                    fid
                } else if let Some(&fid) = self.functions.get(target_name) {
                    fid
                } else {
                    let mut sig = module.make_signature();
                    for arg_val in &arg_vals {
                        let ty = builder.func.dfg.value_type(*arg_val);
                        sig.params.push(AbiParam::new(ty));
                    }
                    sig.returns.push(AbiParam::new(ir::types::I64));
                    let fid = module
                        .declare_function(target_name, Linkage::Import, &sig)
                        .unwrap_or_else(|_| module.declare_anonymous_function(&sig).unwrap());
                    self.functions.insert(name.clone(), fid);
                    fid
                };

                let local_func = module.declare_func_in_func(func_id, builder.func);
                let sig_ref = builder.func.dfg.ext_funcs[local_func].signature;

                let mut fixed_args = Vec::new();
                for (idx, &arg_val) in arg_vals.iter().enumerate() {
                    let expected_ty_opt = builder.func.dfg.signatures[sig_ref].params.get(idx).map(|p| p.value_type);
                    if let Some(expected_ty) = expected_ty_opt {
                        let actual_ty = builder.func.dfg.value_type(arg_val);
                        if actual_ty != expected_ty {
                            if actual_ty.bytes() > expected_ty.bytes() {
                                fixed_args.push(builder.ins().ireduce(expected_ty, arg_val));
                            } else if actual_ty.bytes() < expected_ty.bytes() {
                                fixed_args.push(builder.ins().uextend(expected_ty, arg_val));
                            } else {
                                fixed_args.push(arg_val);
                            }
                        } else {
                            fixed_args.push(arg_val);
                        }
                    } else {
                        fixed_args.push(arg_val);
                    }
                }

                let call_inst = builder.ins().call(local_func, &fixed_args);
                let results = builder.inst_results(call_inst);
                if let Some(&res) = results.first() {
                    let res_ty = builder.func.dfg.value_type(res);
                    if res_ty == ir::types::I32 {
                        Some(builder.ins().uextend(ir::types::I64, res))
                    } else {
                        Some(res)
                    }
                } else {
                    None
                }
            }

            Expr::IfElse {
                condition,
                then_body,
                else_body,
            } => {
                let cond_val = self.compile_expr(builder, module, condition)?;

                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();

                builder.ins().brif(cond_val, then_block, &[], else_block, &[]);

                // Compile then block
                builder.switch_to_block(then_block);
                let mut then_val = None;
                let then_returns = then_body.iter().any(|s| matches!(s, Expr::Return { .. }));
                for stmt in then_body {
                    then_val = self.compile_expr(builder, module, stmt);
                }
                if !then_returns {
                    builder.ins().jump(merge_block, &[]);
                }

                // Compile else block
                builder.switch_to_block(else_block);
                let mut else_val = None;
                let else_returns = else_body.iter().any(|s| matches!(s, Expr::Return { .. }));
                for stmt in else_body {
                    else_val = self.compile_expr(builder, module, stmt);
                }
                if !else_returns {
                    builder.ins().jump(merge_block, &[]);
                }

                // Merge block
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);

                then_val.or(else_val)
            }

            Expr::WhileLoop { condition, body } => {
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                builder.ins().jump(header_block, &[]);

                // Header block
                builder.switch_to_block(header_block);
                let cond_val = self.compile_expr(builder, module, condition)?;
                builder.ins().brif(cond_val, body_block, &[], exit_block, &[]);

                // Body block
                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                for stmt in body {
                    self.compile_expr(builder, module, stmt);
                }
                builder.ins().jump(header_block, &[]);

                builder.seal_block(header_block);

                // Exit block
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);

                None
            }

            Expr::Return { value } => {
                let ret_ty = builder.func.signature.returns.first().map(|p| p.value_type);
                if let Some(v_expr) = value {
                    if let Some(mut v) = self.compile_expr(builder, module, v_expr) {
                        if let Some(expected_ty) = ret_ty {
                            let actual_ty = builder.func.dfg.value_type(v);
                            if actual_ty != expected_ty {
                                if actual_ty.bytes() > expected_ty.bytes() {
                                    v = builder.ins().ireduce(expected_ty, v);
                                } else if actual_ty.bytes() < expected_ty.bytes() {
                                    v = builder.ins().uextend(expected_ty, v);
                                }
                            }
                        }
                        builder.ins().return_(&[v]);
                    } else if let Some(ty) = ret_ty {
                        let default_v = builder.ins().iconst(ty, 0);
                        builder.ins().return_(&[default_v]);
                    } else {
                        builder.ins().return_(&[]);
                    }
                } else if let Some(ty) = ret_ty {
                    let default_v = builder.ins().iconst(ty, 0);
                    builder.ins().return_(&[default_v]);
                } else {
                    builder.ins().return_(&[]);
                }
                None
            }

            Expr::LensBlock {
                target,
                lens_name,
                body,
            } => {
                if let Some(&var) = self.var_map.get(target) {
                    self.var_map.insert(lens_name.clone(), var);
                }
                let mut last = None;
                for stmt in body {
                    last = self.compile_expr(builder, module, stmt);
                }
                last
            }

            Expr::Match { target, arms } => {
                let disc = self.compile_expr(builder, module, target)?;
                let merge_block = builder.create_block();
                let mut last_val = None;

                for arm in arms {
                    let arm_block = builder.create_block();
                    let next_arm_block = builder.create_block();

                    let matched = match &arm.pattern {
                        Pattern::Ident(name) => {
                            let var = self.new_var();
                            builder.declare_var(var, ir::types::I64);
                            builder.def_var(var, disc);
                            self.var_map.insert(name.clone(), var);
                            builder.ins().iconst(ir::types::I8, 1)
                        }
                        Pattern::Variant { variant, binding, .. } => {
                            let target_disc = self.variant_map.get(variant).copied().unwrap_or(0i64);
                            let expected = builder.ins().iconst(ir::types::I64, target_disc);
                            let cond = builder.ins().icmp(ir::condcodes::IntCC::Equal, disc, expected);
                            if let Some(bname) = binding {
                                let var = self.new_var();
                                builder.declare_var(var, ir::types::I64);
                                builder.def_var(var, disc);
                                self.var_map.insert(bname.clone(), var);
                            }
                            cond
                        }
                        Pattern::Wildcard => builder.ins().iconst(ir::types::I8, 1),
                    };

                    builder.ins().brif(matched, arm_block, &[], next_arm_block, &[]);

                    builder.switch_to_block(arm_block);
                    builder.seal_block(arm_block);
                    last_val = self.compile_expr(builder, module, &arm.body);
                    builder.ins().jump(merge_block, &[]);

                    builder.switch_to_block(next_arm_block);
                    builder.seal_block(next_arm_block);
                }

                builder.ins().jump(merge_block, &[]);
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);

                last_val
            }

            Expr::SliceIndex { target, start, .. } => {
                let ptr = self.compile_expr(builder, module, target)?;
                if let Some(s) = start
                    && let Some(start_val) = self.compile_expr(builder, module, s) {
                        return Some(builder.ins().iadd(ptr, start_val));
                    }
                Some(ptr)
            }

            Expr::Range { start, end } => {
                let s = self.compile_expr(builder, module, start)?;
                let e = self.compile_expr(builder, module, end)?;
                Some(builder.ins().isub(e, s))
            }

            _ => None,
        }
    }
}

fn track_type_to_cl(ty: &TrackType) -> ir::Type {
    match ty {
        TrackType::U8 | TrackType::I8 => ir::types::I8,
        TrackType::I32 | TrackType::U32 => ir::types::I32,
        TrackType::I64 | TrackType::U64 => ir::types::I64,
        TrackType::Bool => ir::types::I8,
        TrackType::Void => ir::types::I32,
        TrackType::Ptr(_) | TrackType::Ref(_) | TrackType::Slice(_) => ir::types::I64,
        TrackType::Array(_, _) => ir::types::I64,
        TrackType::Custom(_) => ir::types::I64,
    }
}
