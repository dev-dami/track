//! Cranelift backend for the Rust bootstrap compiler.
//!
//! Every Track value is lowered to a single 64-bit machine word (the "word
//! ABI"): integers and booleans are sign/zero-normalised `i64`s, strings are
//! `char*` pointers, and tuples / tagged-union cells are pointers to arrays of
//! words. All Track functions take and return words, which keeps calls across
//! separately compiled modules well defined and matches the portable C backend
//! of the self-hosted compiler (`compiler/src/cgen.trk`) bit for bit.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use cranelift_codegen::ir::{
    self, AbiParam, InstBuilder, StackSlotData, StackSlotKind, Value, condcodes::IntCC,
};
use cranelift_codegen::isa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::Triple;

use crate::ast::{BinOp, Expr, Pattern, TrackType, UnaryOp};

const W: ir::Type = ir::types::I64;

/// Type knowledge gathered by the checker across a module and everything it
/// imports. Codegen compiles one file at a time, so enum/union layouts and
/// type aliases defined in imported modules arrive through this table.
#[derive(Debug, Clone, Default)]
pub struct TypeInfo {
    /// `Enum::Variant` / `Union::Variant` → discriminant.
    pub variants: HashMap<String, i64>,
    /// Unions with at least one payload variant; their values are heap cells
    /// laid out as `[tag, payload...]`.
    pub payload_unions: HashSet<String>,
    pub aliases: HashMap<String, TrackType>,
    /// Integer constants, including those defined in imported modules.
    pub consts: HashMap<String, i64>,
    /// Struct name → field names in declaration order.
    pub structs: HashMap<String, Vec<String>>,
}

pub struct CodeGen {
    module_name: String,
    module: ObjectModule,
    fn_builder_ctx: FunctionBuilderContext,
    functions: HashMap<String, FuncId>,
    info: TypeInfo,
    consts: HashMap<String, i64>,
}

/// Evaluate a compile-time constant expression (integer literals, negation,
/// and arithmetic over known constants).
pub fn eval_const(consts: &HashMap<String, i64>, expr: &Expr) -> Option<i64> {
    match expr {
        Expr::IntLiteral(v) => Some(*v),
        Expr::BoolLiteral(b) => Some(if *b { 1 } else { 0 }),
        Expr::UnaryOp {
            op: UnaryOp::Neg,
            expr,
        } => eval_const(consts, expr).map(|v| v.wrapping_neg()),
        Expr::BinaryOp { op, left, right } => {
            let l = eval_const(consts, left)?;
            let r = eval_const(consts, right)?;
            match op {
                BinOp::Add => Some(l.wrapping_add(r)),
                BinOp::Sub => Some(l.wrapping_sub(r)),
                BinOp::Mul => Some(l.wrapping_mul(r)),
                BinOp::Div if r != 0 => Some(l.wrapping_div(r)),
                BinOp::Mod if r != 0 => Some(l.wrapping_rem(r)),
                BinOp::BitAnd => Some(l & r),
                BinOp::BitOr => Some(l | r),
                BinOp::Shl => Some(l.wrapping_shl((r & 63) as u32)),
                BinOp::Shr => Some(((l as u64) >> (r & 63)) as i64),
                _ => None,
            }
        }
        Expr::Variable(name) => consts.get(name).copied(),
        _ => None,
    }
}

/// Assign discriminants to enum variants: an explicit `= value` sets the
/// discriminant, otherwise it is one more than the previous variant.
pub fn enum_discriminants(
    consts: &HashMap<String, i64>,
    variants: &[(String, Option<Expr>)],
) -> Vec<(String, i64)> {
    let mut next = 0i64;
    variants
        .iter()
        .map(|(name, value)| {
            let disc = value
                .as_ref()
                .and_then(|v| eval_const(consts, v))
                .unwrap_or(next);
            next = disc.wrapping_add(1);
            (name.clone(), disc)
        })
        .collect()
}

/// Runtime tag stored in word 0 of every struct value: FNV-1a over the
/// name with 64-bit wrapping (identical in the self-hosted compiler).
pub fn struct_tag(name: &str) -> i64 {
    let mut h: i64 = 2166136261;
    for b in name.bytes() {
        h ^= i64::from(b);
        h = h.wrapping_mul(16777619);
    }
    h
}

/// Number of words in a tuple type, resolving aliases.
fn tuple_len(aliases: &HashMap<String, TrackType>, ty: &TrackType) -> Option<usize> {
    let mut current = ty;
    for _ in 0..16 {
        match current {
            TrackType::Tuple(elems) => return Some(elems.len()),
            TrackType::Custom(name) => current = aliases.get(name)?,
            _ => return None,
        }
    }
    None
}

impl CodeGen {
    pub fn create_default_isa() -> Arc<dyn isa::TargetIsa> {
        let mut flag_builder = settings::builder();
        flag_builder.set("is_pic", "false").unwrap();
        flag_builder.set("opt_level", "speed").unwrap();
        // More aggressive opts for near-native speed
        let _ = flag_builder.set("regalloc_algorithm", "backtracking");
        let _ = flag_builder.set("enable_alias_analysis", "true");
        let _ = flag_builder.set("enable_verifier", "false");
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
        let object_builder = ObjectBuilder::new(isa, module_name, default_libcall_names()).unwrap();
        let module = ObjectModule::new(object_builder);

        Self {
            module_name: module_name.to_string(),
            module,
            fn_builder_ctx: FunctionBuilderContext::new(),
            functions: HashMap::new(),
            info: TypeInfo::default(),
            consts: HashMap::new(),
        }
    }

    /// Provide enum/union layouts and aliases discovered by the checker
    /// (including imported modules).
    pub fn set_type_info(&mut self, info: TypeInfo) {
        self.consts
            .extend(info.consts.iter().map(|(k, v)| (k.clone(), *v)));
        self.info = info;
    }

    fn signature(
        &self,
        name: &str,
        params: &[(String, TrackType)],
        ret: Option<&TrackType>,
    ) -> ir::Signature {
        let mut sig = self.module.make_signature();
        if name == "main" {
            if params.is_empty() {
                // C entry point: int main(int argc, char **argv)
                sig.params.push(AbiParam::new(ir::types::I32));
                sig.params.push(AbiParam::new(W));
            } else {
                for _ in params {
                    sig.params.push(AbiParam::new(W));
                }
            }
            sig.returns.push(AbiParam::new(ir::types::I32));
            return sig;
        }
        for _ in params {
            sig.params.push(AbiParam::new(W));
        }
        if let Some(rty) = ret
            && *rty != TrackType::Void
        {
            sig.returns.push(AbiParam::new(W));
        }
        sig
    }

    pub fn compile_program(&mut self, program: &[Expr]) {
        // First pass: constants, type layouts, and function declarations.
        for expr in program {
            match expr {
                Expr::ConstDef { name, value } => {
                    if let Some(v) = eval_const(&self.consts, value) {
                        self.consts.insert(name.clone(), v);
                    }
                }
                Expr::TypeAlias { name, target } => {
                    self.info.aliases.insert(name.clone(), target.clone());
                }
                Expr::StructDef { name, fields } => {
                    self.info.structs.insert(
                        name.clone(),
                        fields.iter().map(|(f, _)| f.clone()).collect(),
                    );
                }
                Expr::EnumDef { name, variants, .. } => {
                    for (vname, disc) in enum_discriminants(&self.consts, variants) {
                        self.info
                            .variants
                            .insert(format!("{}::{}", name, vname), disc);
                    }
                }
                Expr::UnionDef { name, variants } => {
                    for (idx, (vname, ty)) in variants.iter().enumerate() {
                        self.info
                            .variants
                            .insert(format!("{}::{}", name, vname), idx as i64);
                        if ty.is_some() {
                            self.info.payload_unions.insert(name.clone());
                        }
                    }
                }
                Expr::FnDef {
                    name,
                    generics,
                    params,
                    return_type,
                    ..
                } => {
                    // Generic templates are never emitted — only their
                    // monomorphized instances (which have empty `generics`).
                    if !generics.is_empty() {
                        continue;
                    }
                    self.declare_track_fn(name, params, return_type.as_ref());
                }
                Expr::MacroDef {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    self.declare_track_fn(name, params, return_type.as_ref());
                }
                _ => {}
            }
        }

        let mut has_main = false;

        // Second pass: define function bodies
        for expr in program {
            match expr {
                Expr::FnDef {
                    name,
                    generics,
                    params,
                    return_type,
                    body,
                } if generics.is_empty() => {
                    has_main |= name == "main";
                    let func_id = self.functions[name.as_str()];
                    self.compile_fn(func_id, name, params, return_type.as_ref(), body, program);
                }
                Expr::MacroDef {
                    name,
                    params,
                    return_type,
                    body,
                } => {
                    has_main |= name == "main";
                    let func_id = self.functions[name.as_str()];
                    self.compile_fn(func_id, name, params, return_type.as_ref(), body, program);
                }
                _ => {}
            }
        }

        // Synthesize C main wrapper if needed (only for entry point modules)
        if !has_main
            && (self.module_name == "main"
                || self.module_name == "track_module"
                || self.module_name.ends_with("main"))
        {
            self.compile_fn_inner(None, "main", &[], None, &[], program);
        }
    }

    fn declare_track_fn(
        &mut self,
        name: &str,
        params: &[(String, TrackType)],
        ret: Option<&TrackType>,
    ) {
        let sig = self.signature(name, params, ret);
        let func_id = self
            .module
            .declare_function(name, Linkage::Export, &sig)
            .unwrap_or_else(|_| self.module.declare_anonymous_function(&sig).unwrap());
        self.functions.insert(name.to_string(), func_id);
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
        self.compile_fn_inner(Some(func_id), name, params, return_type, body, program);
    }

    fn compile_fn_inner(
        &mut self,
        func_id: Option<FuncId>,
        name: &str,
        params: &[(String, TrackType)],
        return_type: Option<&TrackType>,
        body: &[Expr],
        program: &[Expr],
    ) {
        let is_main = name == "main";
        let sig = self.signature(name, params, return_type);
        let func_id = match func_id {
            Some(id) => id,
            None => self
                .module
                .declare_function("main", Linkage::Export, &sig)
                .unwrap(),
        };
        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;
        let returns_value = is_main || return_type.is_some_and(|rty| *rty != TrackType::Void);

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.fn_builder_ctx);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);

        let mut fn_ctx = FnContext {
            var_map: HashMap::new(),
            var_counter: 0,
            functions: &mut self.functions,
            info: &self.info,
            consts: &self.consts,
            filled: false,
            is_main,
            returns_value,
            stack_tuples: HashMap::new(),
            tuple_params: HashMap::new(),
        };

        if is_main && params.is_empty() {
            // Publish argc/argv to the runtime before any user code runs.
            let argc = builder.block_params(entry_block)[0];
            let argv = builder.block_params(entry_block)[1];
            let argc = builder.ins().sextend(W, argc);
            fn_ctx.call_named(
                &mut builder,
                &mut self.module,
                "track_runtime_init",
                &[argc, argv],
            );
        } else {
            for (idx, (pname, pty)) in params.iter().enumerate() {
                let var = fn_ctx.new_var();
                builder.declare_var(var, W);
                let val = builder.block_params(entry_block)[idx];
                builder.def_var(var, val);
                fn_ctx.var_map.insert(pname.clone(), var);
                if let Some(n) = tuple_len(&self.info.aliases, pty) {
                    fn_ctx.tuple_params.insert(pname.clone(), n);
                }
            }
        }

        // Top-level statements (global lets) run at the start of main.
        if is_main {
            for top_stmt in program {
                if fn_ctx.filled {
                    break;
                }
                if !matches!(top_stmt, Expr::FnDef { .. } | Expr::MacroDef { .. }) {
                    fn_ctx.compile_expr(&mut builder, &mut self.module, top_stmt);
                }
            }
        }

        let last_val = fn_ctx.compile_block(&mut builder, &mut self.module, body);

        if !fn_ctx.filled {
            if is_main {
                let zero = builder.ins().iconst(ir::types::I32, 0);
                builder.ins().return_(&[zero]);
            } else if returns_value {
                let v = last_val.unwrap_or_else(|| builder.ins().iconst(W, 0));
                builder.ins().return_(&[v]);
            } else {
                builder.ins().return_(&[]);
            }
        }

        builder.seal_all_blocks();
        builder.finalize();
        self.module.define_function(func_id, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
    }

    pub fn write_object_file(self, output_path: &Path) -> Result<(), String> {
        let product = self.module.finish();
        let bytes = product
            .emit()
            .map_err(|e| format!("Failed to emit Cranelift object module: {}", e))?;
        fs::write(output_path, bytes).map_err(|e| {
            format!(
                "Failed to write object file '{}': {}",
                output_path.display(),
                e
            )
        })
    }
}

struct Scope {
    vars: HashMap<String, Variable>,
    stack_tuples: HashMap<String, usize>,
    tuple_params: HashMap<String, usize>,
}

struct FnContext<'a> {
    var_map: HashMap<String, Variable>,
    var_counter: u32,
    functions: &'a mut HashMap<String, FuncId>,
    info: &'a TypeInfo,
    consts: &'a HashMap<String, i64>,
    /// The current block already ends in a terminator; further statements in
    /// the same lexical block are unreachable and skipped.
    filled: bool,
    is_main: bool,
    returns_value: bool,
    /// Locals bound directly to a tuple literal live in a stack slot and must
    /// be copied to the heap when returned.
    stack_tuples: HashMap<String, usize>,
    tuple_params: HashMap<String, usize>,
}

impl<'a> FnContext<'a> {
    fn new_var(&mut self) -> Variable {
        let var = Variable::from_u32(self.var_counter);
        self.var_counter += 1;
        var
    }

    fn switch_to(&mut self, builder: &mut FunctionBuilder, block: ir::Block) {
        builder.switch_to_block(block);
        self.filled = false;
    }

    fn jump(&mut self, builder: &mut FunctionBuilder, block: ir::Block, args: &[Value]) {
        if !self.filled {
            builder.ins().jump(block, args);
            self.filled = true;
        }
    }

    /// Normalise a value to a full word (booleans and narrow parameters).
    fn word(&self, builder: &mut FunctionBuilder, v: Value) -> Value {
        let ty = builder.func.dfg.value_type(v);
        if ty == W {
            v
        } else if ty == ir::types::I32 {
            builder.ins().sextend(W, v)
        } else {
            builder.ins().uextend(W, v)
        }
    }

    fn bool_word(&self, builder: &mut FunctionBuilder, cc: IntCC, a: Value, b: Value) -> Value {
        let c = builder.ins().icmp(cc, a, b);
        builder.ins().uextend(W, c)
    }

    fn zero(&self, builder: &mut FunctionBuilder) -> Value {
        builder.ins().iconst(W, 0)
    }

    fn func_ref(
        &mut self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        name: &str,
        argc: usize,
    ) -> ir::FuncRef {
        let func_id = if let Some(&fid) = self.functions.get(name) {
            fid
        } else {
            let mut sig = module.make_signature();
            for _ in 0..argc {
                sig.params.push(AbiParam::new(W));
            }
            sig.returns.push(AbiParam::new(W));
            let fid = module
                .declare_function(name, Linkage::Import, &sig)
                .unwrap_or_else(|_| module.declare_anonymous_function(&sig).unwrap());
            self.functions.insert(name.to_string(), fid);
            fid
        };
        module.declare_func_in_func(func_id, builder.func)
    }

    /// Call a function by symbol name with word arguments, adapting to the
    /// declared signature of locally defined functions.
    fn call_named(
        &mut self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        name: &str,
        args: &[Value],
    ) -> Option<Value> {
        let local_func = self.func_ref(builder, module, name, args.len());
        let sig_ref = builder.func.dfg.ext_funcs[local_func].signature;
        let expected: Vec<ir::Type> = builder.func.dfg.signatures[sig_ref]
            .params
            .iter()
            .map(|p| p.value_type)
            .collect();
        let mut fixed_args = Vec::with_capacity(expected.len());
        for (idx, &ty) in expected.iter().enumerate() {
            let arg = match args.get(idx) {
                Some(&a) => self.word(builder, a),
                None => self.zero(builder),
            };
            fixed_args.push(if ty == W {
                arg
            } else {
                builder.ins().ireduce(ty, arg)
            });
        }
        let call_inst = builder.ins().call(local_func, &fixed_args);
        let results = builder.inst_results(call_inst);
        results.first().copied().map(|r| self.word(builder, r))
    }

    fn alloc_words(
        &mut self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        words: &[Value],
    ) -> Value {
        let size = builder.ins().iconst(W, (words.len().max(1) * 8) as i64);
        let ptr = self
            .call_named(builder, module, "alloc", &[size])
            .unwrap_or_else(|| builder.ins().iconst(W, 0));
        for (i, &val) in words.iter().enumerate() {
            builder
                .ins()
                .store(ir::MemFlags::new(), val, ptr, (i * 8) as i32);
        }
        ptr
    }

    fn copy_words_to_heap(
        &mut self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        src: Value,
        count: usize,
    ) -> Value {
        let words: Vec<Value> = (0..count)
            .map(|i| {
                builder
                    .ins()
                    .load(W, ir::MemFlags::new(), src, (i * 8) as i32)
            })
            .collect();
        self.alloc_words(builder, module, &words)
    }

    /// (tag, field index) of every struct declaring `field`, by struct name.
    fn field_candidates(&self, field: &str) -> Vec<(i64, usize)> {
        let mut names: Vec<&String> = self.info.structs.keys().collect();
        names.sort();
        names
            .into_iter()
            .filter_map(|name| {
                self.info.structs[name]
                    .iter()
                    .position(|f| f == field)
                    .map(|idx| (struct_tag(name), idx))
            })
            .collect()
    }

    /// Load (`store == None`) or store a struct field. A field name shared
    /// by several structs dispatches on the tag in word 0.
    fn field_access(
        &mut self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        ptr: Value,
        field: &str,
        store: Option<Value>,
    ) -> Value {
        let candidates = self.field_candidates(field);
        let access = |builder: &mut FunctionBuilder, idx: usize| -> Value {
            let offset = ((idx + 1) * 8) as i32;
            match store {
                Some(v) => {
                    builder.ins().store(ir::MemFlags::new(), v, ptr, offset);
                    v
                }
                None => builder.ins().load(W, ir::MemFlags::new(), ptr, offset),
            }
        };
        if candidates.len() == 1 {
            return access(builder, candidates[0].1);
        }
        let merge = builder.create_block();
        let result = builder.append_block_param(merge, W);
        let tag = builder.ins().load(W, ir::MemFlags::new(), ptr, 0);
        for (expected, idx) in candidates {
            let hit = builder.create_block();
            let miss = builder.create_block();
            let want = builder.ins().iconst(W, expected);
            let is_hit = builder.ins().icmp(IntCC::Equal, tag, want);
            builder.ins().brif(is_hit, hit, &[], miss, &[]);
            self.filled = true;
            self.switch_to(builder, hit);
            let v = access(builder, idx);
            self.jump(builder, merge, &[v]);
            self.switch_to(builder, miss);
        }
        let bad = self
            .call_named(builder, module, "track_bad_field", &[])
            .unwrap_or_else(|| builder.ins().iconst(W, 0));
        self.jump(builder, merge, &[bad]);
        self.switch_to(builder, merge);
        result
    }

    fn compile_block(
        &mut self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        stmts: &[Expr],
    ) -> Option<Value> {
        // Bindings introduced inside a block go out of scope at its end.
        let scope = self.enter_scope();
        let mut last = None;
        for stmt in stmts {
            if self.filled {
                break;
            }
            last = self.compile_expr(builder, module, stmt);
        }
        self.exit_scope(scope);
        last
    }

    fn enter_scope(&self) -> Scope {
        Scope {
            vars: self.var_map.clone(),
            stack_tuples: self.stack_tuples.clone(),
            tuple_params: self.tuple_params.clone(),
        }
    }

    fn exit_scope(&mut self, scope: Scope) {
        self.var_map = scope.vars;
        self.stack_tuples = scope.stack_tuples;
        self.tuple_params = scope.tuple_params;
    }

    fn variant_key(&self, name: &str) -> Option<i64> {
        self.info.variants.get(name).copied()
    }

    fn is_payload_variant(&self, name: &str) -> bool {
        name.split("::")
            .next()
            .is_some_and(|u| self.info.payload_unions.contains(u))
    }

    fn compile_expr(
        &mut self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        expr: &Expr,
    ) -> Option<Value> {
        if self.filled {
            return None;
        }
        match expr {
            Expr::IntLiteral(val) => Some(builder.ins().iconst(W, *val)),

            Expr::BoolLiteral(val) => Some(builder.ins().iconst(W, if *val { 1 } else { 0 })),

            Expr::StringLiteral(s) => {
                let mut data_ctx = cranelift_module::DataDescription::new();
                let mut bytes = s.as_bytes().to_vec();
                bytes.push(0); // null terminate
                data_ctx.define(bytes.into_boxed_slice());

                let data_id = module.declare_anonymous_data(true, false).unwrap();
                module.define_data(data_id, &data_ctx).unwrap();

                let local_data = module.declare_data_in_func(data_id, builder.func);
                Some(builder.ins().symbol_value(W, local_data))
            }

            Expr::Variable(name) => {
                if let Some(&var) = self.var_map.get(name) {
                    Some(builder.use_var(var))
                } else if name.contains("::") {
                    let disc = self.variant_key(name).unwrap_or(0);
                    let tag = builder.ins().iconst(W, disc);
                    if self.is_payload_variant(name) {
                        let zero = self.zero(builder);
                        Some(self.alloc_words(builder, module, &[tag, zero]))
                    } else {
                        Some(tag)
                    }
                } else if let Some(&c) = self.consts.get(name) {
                    Some(builder.ins().iconst(W, c))
                } else {
                    None
                }
            }

            Expr::LetDef { name, value, .. } => {
                let val = self
                    .compile_expr(builder, module, value)
                    .unwrap_or_else(|| builder.ins().iconst(W, 0));
                if self.filled {
                    return None;
                }
                let var = self.new_var();
                builder.declare_var(var, W);
                builder.def_var(var, val);
                self.var_map.insert(name.clone(), var);
                self.track_tuple_binding(name, value);
                Some(val)
            }

            Expr::Assign { target, value } => {
                let val = self.compile_expr(builder, module, value)?;
                let val = self.word(builder, val);
                match &**target {
                    Expr::Variable(name) => {
                        if let Some(&var) = self.var_map.get(name) {
                            builder.def_var(var, val);
                            self.track_tuple_binding(name, value);
                        }
                    }
                    Expr::FieldAccess { target, field } => {
                        let ptr = self.compile_expr(builder, module, target)?;
                        self.field_access(builder, module, ptr, field, Some(val));
                    }
                    Expr::TupleIndex { target, index } => {
                        let ptr = self.compile_expr(builder, module, target)?;
                        builder
                            .ins()
                            .store(ir::MemFlags::new(), val, ptr, (*index * 8) as i32);
                    }
                    Expr::ArrayIndex { target, index } => {
                        let arr = self.compile_expr(builder, module, target)?;
                        let idx = self.compile_expr(builder, module, index)?;
                        self.call_named(builder, module, "track_array_set", &[arr, idx, val]);
                    }
                    _ => {}
                }
                Some(val)
            }

            Expr::BinaryOp {
                op: op @ (BinOp::And | BinOp::Or),
                left,
                right,
            } => {
                // Short-circuit evaluation; result is a normalised 0/1 word.
                let lhs = self.compile_expr(builder, module, left)?;
                let zero = self.zero(builder);
                let lhs_bool = self.bool_word(builder, IntCC::NotEqual, lhs, zero);
                let rhs_block = builder.create_block();
                let merge = builder.create_block();
                let result = builder.append_block_param(merge, W);
                if *op == BinOp::And {
                    builder
                        .ins()
                        .brif(lhs_bool, rhs_block, &[], merge, &[lhs_bool]);
                } else {
                    builder
                        .ins()
                        .brif(lhs_bool, merge, &[lhs_bool], rhs_block, &[]);
                }
                self.filled = true;
                self.switch_to(builder, rhs_block);
                let rhs = self
                    .compile_expr(builder, module, right)
                    .unwrap_or_else(|| builder.ins().iconst(W, 0));
                if !self.filled {
                    let zero = self.zero(builder);
                    let rhs_bool = self.bool_word(builder, IntCC::NotEqual, rhs, zero);
                    self.jump(builder, merge, &[rhs_bool]);
                }
                self.switch_to(builder, merge);
                Some(result)
            }

            Expr::BinaryOp { op, left, right } => {
                let lhs = self.compile_expr(builder, module, left)?;
                let rhs = self.compile_expr(builder, module, right)?;
                let lhs = self.word(builder, lhs);
                let rhs = self.word(builder, rhs);

                let res = match op {
                    BinOp::Add => builder.ins().iadd(lhs, rhs),
                    BinOp::Sub => builder.ins().isub(lhs, rhs),
                    BinOp::Mul => builder.ins().imul(lhs, rhs),
                    BinOp::Div => builder.ins().sdiv(lhs, rhs),
                    BinOp::Mod => builder.ins().srem(lhs, rhs),
                    BinOp::Eq => self.bool_word(builder, IntCC::Equal, lhs, rhs),
                    BinOp::Neq => self.bool_word(builder, IntCC::NotEqual, lhs, rhs),
                    BinOp::Lt => self.bool_word(builder, IntCC::SignedLessThan, lhs, rhs),
                    BinOp::Gt => self.bool_word(builder, IntCC::SignedGreaterThan, lhs, rhs),
                    BinOp::Lte => self.bool_word(builder, IntCC::SignedLessThanOrEqual, lhs, rhs),
                    BinOp::Gte => {
                        self.bool_word(builder, IntCC::SignedGreaterThanOrEqual, lhs, rhs)
                    }
                    BinOp::BitAnd => builder.ins().band(lhs, rhs),
                    BinOp::BitOr => builder.ins().bor(lhs, rhs),
                    BinOp::Shl => builder.ins().ishl(lhs, rhs),
                    BinOp::Shr => builder.ins().ushr(lhs, rhs),
                    BinOp::And | BinOp::Or => unreachable!(),
                };
                Some(res)
            }

            Expr::UnaryOp { op, expr } => {
                let val = self.compile_expr(builder, module, expr)?;
                let val = self.word(builder, val);
                match op {
                    UnaryOp::Neg => Some(builder.ins().ineg(val)),
                    // Logical NOT: (val == 0). Bitwise bnot would turn 1 into -2,
                    // which is still truthy and inverts branch semantics.
                    UnaryOp::Not => {
                        let zero = self.zero(builder);
                        Some(self.bool_word(builder, IntCC::Equal, val, zero))
                    }
                    UnaryOp::Deref => Some(val),
                }
            }

            Expr::AddressOf { target } => self.compile_expr(builder, module, target),

            Expr::FunctionCall { name, args } | Expr::MacroCall { name, args, .. } => {
                let mut arg_vals = Vec::new();
                for arg in args {
                    let v = self
                        .compile_expr(builder, module, arg)
                        .unwrap_or_else(|| builder.ins().iconst(W, 0));
                    if self.filled {
                        return None;
                    }
                    arg_vals.push(v);
                }

                let clean_name = name.trim_start_matches('@');

                // Variant constructors: `Union::Variant(payload...)`.
                if clean_name.contains("::")
                    && let Some(disc) = self.variant_key(clean_name)
                {
                    let tag = builder.ins().iconst(W, disc);
                    if self.is_payload_variant(clean_name) {
                        let mut words = vec![tag];
                        words.extend(arg_vals.iter().map(|&v| self.word(builder, v)));
                        return Some(self.alloc_words(builder, module, &words));
                    }
                    return Some(tag);
                }

                let mut target_name = if clean_name.contains("::") {
                    clean_name.rsplit("::").next().unwrap()
                } else {
                    clean_name
                };
                // Map Track-level builtin names to their C runtime symbols.
                if target_name == "abort" {
                    target_name = "track_abort";
                }
                let symbol = if self.functions.contains_key(name) {
                    name.as_str()
                } else if self.functions.contains_key(clean_name) {
                    clean_name
                } else {
                    target_name
                };
                self.call_named(builder, module, symbol, &arg_vals)
            }

            Expr::IfElse {
                condition,
                then_body,
                else_body,
            } => {
                let cond_val = self.compile_expr(builder, module, condition)?;
                let cond_val = self.word(builder, cond_val);

                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();
                let result = builder.append_block_param(merge_block, W);

                builder
                    .ins()
                    .brif(cond_val, then_block, &[], else_block, &[]);
                self.filled = true;

                self.switch_to(builder, then_block);
                let then_val = self.compile_block(builder, module, then_body);
                if !self.filled {
                    let v = match then_val {
                        Some(v) => self.word(builder, v),
                        None => self.zero(builder),
                    };
                    self.jump(builder, merge_block, &[v]);
                }

                self.switch_to(builder, else_block);
                let else_val = self.compile_block(builder, module, else_body);
                if !self.filled {
                    let v = match else_val {
                        Some(v) => self.word(builder, v),
                        None => self.zero(builder),
                    };
                    self.jump(builder, merge_block, &[v]);
                }

                self.switch_to(builder, merge_block);
                Some(result)
            }

            Expr::WhileLoop { condition, body } => {
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                self.jump(builder, header_block, &[]);

                self.switch_to(builder, header_block);
                let cond_val = self
                    .compile_expr(builder, module, condition)
                    .unwrap_or_else(|| builder.ins().iconst(W, 0));
                if !self.filled {
                    let cond_val = self.word(builder, cond_val);
                    builder
                        .ins()
                        .brif(cond_val, body_block, &[], exit_block, &[]);
                    self.filled = true;
                }

                self.switch_to(builder, body_block);
                self.compile_block(builder, module, body);
                self.jump(builder, header_block, &[]);

                self.switch_to(builder, exit_block);
                None
            }

            Expr::ForIn { var, iter, body } => {
                let (start_val, end_val) = match iter.as_ref() {
                    Expr::Range { start, end } => {
                        let s = self
                            .compile_expr(builder, module, start)
                            .unwrap_or_else(|| builder.ins().iconst(W, 0));
                        let e = self
                            .compile_expr(builder, module, end)
                            .unwrap_or_else(|| builder.ins().iconst(W, 0));
                        (s, e)
                    }
                    _ => {
                        let s = builder.ins().iconst(W, 0);
                        let e = self
                            .compile_expr(builder, module, iter)
                            .unwrap_or_else(|| builder.ins().iconst(W, 0));
                        (s, e)
                    }
                };
                if self.filled {
                    return None;
                }
                let start_val = self.word(builder, start_val);
                let end_val = self.word(builder, end_val);

                let loop_scope = self.enter_scope();
                let loop_var = self.new_var();
                builder.declare_var(loop_var, W);
                builder.def_var(loop_var, start_val);
                self.var_map.insert(var.clone(), loop_var);

                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                self.jump(builder, header_block, &[]);

                self.switch_to(builder, header_block);
                let cur_val = builder.use_var(loop_var);
                let cond_val = builder.ins().icmp(IntCC::SignedLessThan, cur_val, end_val);
                builder
                    .ins()
                    .brif(cond_val, body_block, &[], exit_block, &[]);
                self.filled = true;

                self.switch_to(builder, body_block);
                self.compile_block(builder, module, body);
                if !self.filled {
                    let next_val = builder.ins().iadd_imm(cur_val, 1);
                    builder.def_var(loop_var, next_val);
                    self.jump(builder, header_block, &[]);
                }

                self.switch_to(builder, exit_block);
                self.exit_scope(loop_scope);
                None
            }

            Expr::Return { value } => {
                let v = match value.as_deref() {
                    // Returning a tuple literal must heap-allocate (a stack slot would dangle).
                    Some(Expr::TupleLiteral { elements }) => {
                        let mut words = Vec::new();
                        for elem in elements {
                            let v = self
                                .compile_expr(builder, module, elem)
                                .unwrap_or_else(|| builder.ins().iconst(W, 0));
                            if self.filled {
                                return None;
                            }
                            words.push(self.word(builder, v));
                        }
                        Some(self.alloc_words(builder, module, &words))
                    }
                    Some(Expr::Variable(name))
                        if self.stack_tuples.contains_key(name)
                            || self.tuple_params.contains_key(name) =>
                    {
                        let count = self
                            .stack_tuples
                            .get(name)
                            .or_else(|| self.tuple_params.get(name))
                            .copied()
                            .unwrap();
                        let src = builder.use_var(self.var_map[name.as_str()]);
                        Some(self.copy_words_to_heap(builder, module, src, count))
                    }
                    Some(v_expr) => self.compile_expr(builder, module, v_expr),
                    None => None,
                };
                if self.filled {
                    return None;
                }
                if self.is_main {
                    let code = match v {
                        Some(v) => {
                            let v = self.word(builder, v);
                            builder.ins().ireduce(ir::types::I32, v)
                        }
                        None => builder.ins().iconst(ir::types::I32, 0),
                    };
                    builder.ins().return_(&[code]);
                } else if self.returns_value {
                    let v = match v {
                        Some(v) => self.word(builder, v),
                        None => self.zero(builder),
                    };
                    builder.ins().return_(&[v]);
                } else {
                    builder.ins().return_(&[]);
                }
                self.filled = true;
                None
            }

            Expr::LensBlock {
                target,
                lens_name,
                body,
            } => {
                let lens_scope = self.enter_scope();
                if let Some(&var) = self.var_map.get(target) {
                    self.var_map.insert(lens_name.clone(), var);
                }
                let value = self.compile_block(builder, module, body);
                self.exit_scope(lens_scope);
                value
            }

            Expr::TupleLiteral { elements } => {
                // Fast, stack-allocated tuples for non-escaping locals.
                // Escaping tuples are copied to the heap by `Return`.
                let mut words = Vec::new();
                for elem in elements {
                    let v = self
                        .compile_expr(builder, module, elem)
                        .unwrap_or_else(|| builder.ins().iconst(W, 0));
                    if self.filled {
                        return None;
                    }
                    words.push(self.word(builder, v));
                }
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    (elements.len().max(1) * 8) as u32,
                ));
                let ptr = builder.ins().stack_addr(W, slot, 0);
                for (i, val) in words.into_iter().enumerate() {
                    builder
                        .ins()
                        .store(ir::MemFlags::new(), val, ptr, (i * 8) as i32);
                }
                Some(ptr)
            }

            Expr::TupleIndex { target, index } => {
                let ptr = self.compile_expr(builder, module, target)?;
                let offset = (*index * 8) as i32;
                Some(builder.ins().load(W, ir::MemFlags::new(), ptr, offset))
            }

            Expr::LetDestructure { pattern, value, .. } => {
                // Fast path: let (a,b) = (x, y) without allocating a tuple.
                if let Expr::TupleLiteral { elements } = value.as_ref()
                    && let Pattern::Tuple(pats) = pattern
                    && pats.len() == elements.len()
                {
                    let mut vals = Vec::new();
                    for elem in elements {
                        let v = self
                            .compile_expr(builder, module, elem)
                            .unwrap_or_else(|| builder.ins().iconst(W, 0));
                        if self.filled {
                            return None;
                        }
                        vals.push(v);
                    }
                    for (pat, val) in pats.iter().zip(vals) {
                        self.bind_pattern(builder, pat, val);
                    }
                    return None;
                }
                if let Some(val) = self.compile_expr(builder, module, value)
                    && !self.filled
                {
                    self.bind_pattern(builder, pattern, val);
                }
                None
            }

            Expr::Match { target, arms } => {
                let disc = self.compile_expr(builder, module, target)?;
                let disc = self.word(builder, disc);
                let merge_block = builder.create_block();
                let result = builder.append_block_param(merge_block, W);

                for arm in arms {
                    let arm_scope = self.enter_scope();
                    let arm_block = builder.create_block();
                    let next_arm_block = builder.create_block();

                    let mut matched =
                        self.compile_pattern_match(builder, module, &arm.pattern, disc);

                    if let Some(ref guard) = arm.guard {
                        let guard_block = builder.create_block();
                        builder
                            .ins()
                            .brif(matched, guard_block, &[], next_arm_block, &[]);
                        self.filled = true;
                        self.switch_to(builder, guard_block);

                        self.bind_pattern(builder, &arm.pattern, disc);
                        matched = match self.compile_expr(builder, module, guard) {
                            Some(g) => self.word(builder, g),
                            None => builder.ins().iconst(W, 1),
                        };
                    } else {
                        self.bind_pattern(builder, &arm.pattern, disc);
                    }

                    builder
                        .ins()
                        .brif(matched, arm_block, &[], next_arm_block, &[]);
                    self.filled = true;

                    self.switch_to(builder, arm_block);
                    let arm_val = self.compile_expr(builder, module, &arm.body);
                    if !self.filled {
                        let v = match arm_val {
                            Some(v) => self.word(builder, v),
                            None => self.zero(builder),
                        };
                        self.jump(builder, merge_block, &[v]);
                    }

                    self.switch_to(builder, next_arm_block);
                    self.exit_scope(arm_scope);
                }

                let zero = self.zero(builder);
                self.jump(builder, merge_block, &[zero]);
                self.switch_to(builder, merge_block);
                Some(result)
            }

            Expr::StructInitialization { ty_name, fields } => {
                // Struct values are heap cells: [tag, fields in declaration order].
                let mut values = Vec::new();
                for (fname, fexpr) in fields {
                    let v = self
                        .compile_expr(builder, module, fexpr)
                        .unwrap_or_else(|| builder.ins().iconst(W, 0));
                    if self.filled {
                        return None;
                    }
                    values.push((fname.clone(), self.word(builder, v)));
                }
                let order: Vec<String> = match self.info.structs.get(ty_name) {
                    Some(decl) => decl.clone(),
                    None => fields.iter().map(|(f, _)| f.clone()).collect(),
                };
                let mut words = vec![builder.ins().iconst(W, struct_tag(ty_name))];
                for fname in &order {
                    let v = match values.iter().find(|(f, _)| f == fname) {
                        Some((_, v)) => *v,
                        None => builder.ins().iconst(W, 0),
                    };
                    words.push(v);
                }
                Some(self.alloc_words(builder, module, &words))
            }

            Expr::FieldAccess { target, field } => {
                let ptr = self.compile_expr(builder, module, target)?;
                Some(self.field_access(builder, module, ptr, field, None))
            }

            Expr::ArrayLiteral { elements } => {
                // Arrays are heap cells: [length, elements...].
                let mut words = vec![builder.ins().iconst(W, elements.len() as i64)];
                for elem in elements {
                    let v = self
                        .compile_expr(builder, module, elem)
                        .unwrap_or_else(|| builder.ins().iconst(W, 0));
                    if self.filled {
                        return None;
                    }
                    words.push(self.word(builder, v));
                }
                Some(self.alloc_words(builder, module, &words))
            }

            Expr::ArrayIndex { target, index } => {
                let arr = self.compile_expr(builder, module, target)?;
                let idx = self.compile_expr(builder, module, index)?;
                self.call_named(builder, module, "track_array_get", &[arr, idx])
            }

            Expr::SliceIndex { target, start, end } => {
                let arr = self.compile_expr(builder, module, target)?;
                let from = match start {
                    Some(s) => self.compile_expr(builder, module, s)?,
                    None => builder.ins().iconst(W, 0),
                };
                let to = match end {
                    Some(e) => self.compile_expr(builder, module, e)?,
                    None => builder.ins().iconst(W, -1),
                };
                self.call_named(builder, module, "track_array_slice", &[arr, from, to])
            }

            Expr::Range { start, end } => {
                let s = self.compile_expr(builder, module, start)?;
                let e = self.compile_expr(builder, module, end)?;
                Some(builder.ins().isub(e, s))
            }

            _ => None,
        }
    }

    fn track_tuple_binding(&mut self, name: &str, value: &Expr) {
        if let Expr::TupleLiteral { elements } = value {
            self.stack_tuples.insert(name.to_string(), elements.len());
        } else {
            self.stack_tuples.remove(name);
        }
        self.tuple_params.remove(name);
    }

    fn bind_pattern(&mut self, builder: &mut FunctionBuilder, pattern: &Pattern, val: Value) {
        match pattern {
            Pattern::Ident(name) => {
                let var = self.new_var();
                builder.declare_var(var, W);
                builder.def_var(var, val);
                self.var_map.insert(name.clone(), var);
                self.stack_tuples.remove(name);
                self.tuple_params.remove(name);
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
            Pattern::Tuple(pats) => {
                for (i, p) in pats.iter().enumerate() {
                    let elem_val = builder
                        .ins()
                        .load(W, ir::MemFlags::new(), val, (i * 8) as i32);
                    self.bind_pattern(builder, p, elem_val);
                }
            }
            Pattern::Variant {
                enum_or_union,
                bindings,
                ..
            } => {
                if self.info.payload_unions.contains(enum_or_union) {
                    for (i, p) in bindings.iter().enumerate() {
                        let payload =
                            builder
                                .ins()
                                .load(W, ir::MemFlags::new(), val, ((i + 1) * 8) as i32);
                        self.bind_pattern(builder, p, payload);
                    }
                } else {
                    for p in bindings {
                        self.bind_pattern(builder, p, val);
                    }
                }
            }
            Pattern::Struct { name, fields } => {
                let decl = self.info.structs.get(name).cloned().unwrap_or_default();
                for (fname, p) in fields {
                    if let Some(idx) = decl.iter().position(|f| f == fname) {
                        let field =
                            builder
                                .ins()
                                .load(W, ir::MemFlags::new(), val, ((idx + 1) * 8) as i32);
                        self.bind_pattern(builder, p, field);
                    }
                }
            }
        }
    }

    /// Returns a 0/1 word that is non-zero when `pattern` matches `val`.
    fn compile_pattern_match(
        &mut self,
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        pattern: &Pattern,
        val: Value,
    ) -> Value {
        match pattern {
            Pattern::Ident(_) | Pattern::Wildcard => builder.ins().iconst(W, 1),
            Pattern::Struct { name, fields } => {
                let Some(decl) = self.info.structs.get(name).cloned() else {
                    return builder.ins().iconst(W, 1);
                };
                let tag = builder.ins().load(W, ir::MemFlags::new(), val, 0);
                let want = builder.ins().iconst(W, struct_tag(name));
                let mut cond = self.bool_word(builder, IntCC::Equal, tag, want);
                for (fname, p) in fields {
                    if let Some(idx) = decl.iter().position(|f| f == fname) {
                        let field =
                            builder
                                .ins()
                                .load(W, ir::MemFlags::new(), val, ((idx + 1) * 8) as i32);
                        let sub = self.compile_pattern_match(builder, module, p, field);
                        cond = builder.ins().band(cond, sub);
                    }
                }
                cond
            }
            Pattern::Literal(lit_expr) => {
                if let Expr::StringLiteral(_) = lit_expr.as_ref() {
                    let lit = self.compile_expr(builder, module, lit_expr).unwrap();
                    let eq = self
                        .call_named(builder, module, "str_eq", &[val, lit])
                        .unwrap();
                    let zero = self.zero(builder);
                    return self.bool_word(builder, IntCC::NotEqual, eq, zero);
                }
                match self.compile_expr(builder, module, lit_expr) {
                    Some(lit_val) => {
                        let lit_val = self.word(builder, lit_val);
                        self.bool_word(builder, IntCC::Equal, val, lit_val)
                    }
                    None => builder.ins().iconst(W, 1),
                }
            }
            Pattern::Variant {
                enum_or_union,
                variant,
                ..
            } => {
                let key = format!("{}::{}", enum_or_union, variant);
                let target_disc = self.variant_key(&key).unwrap_or(0);
                let expected = builder.ins().iconst(W, target_disc);
                let tag = if self.info.payload_unions.contains(enum_or_union) {
                    builder.ins().load(W, ir::MemFlags::new(), val, 0)
                } else {
                    val
                };
                self.bool_word(builder, IntCC::Equal, tag, expected)
            }
            Pattern::Tuple(pats) => {
                let mut cond = builder.ins().iconst(W, 1);
                for (i, p) in pats.iter().enumerate() {
                    let elem_val = builder
                        .ins()
                        .load(W, ir::MemFlags::new(), val, (i * 8) as i32);
                    let elem_cond = self.compile_pattern_match(builder, module, p, elem_val);
                    cond = builder.ins().band(cond, elem_cond);
                }
                cond
            }
        }
    }
}
