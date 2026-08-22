//! Compile-time monomorphization pass (v0.6.0).
//!
//! Runs between the parser and the linear checker. Generic function
//! definitions (`fn name<T, U>(...)`) are treated as templates:
//!
//! 1. Templates are extracted into a table keyed by name.
//! 2. Every call site whose target is a template gets rewritten to a
//!    mangled instance name (`identity__i32`), with type parameters
//!    inferred from the argument expressions (with let/param scope
//!    tracking; annotate with `let x: T = ...` when inference is
//!    ambiguous).
//! 3. A concrete clone of the template (types substituted, body walked
//!    recursively so nested generic calls also specialize) is appended to
//!    the program. The checker and codegen then see only ordinary,
//!    fully concrete functions.
//!
//! Substitution rules: a `TrackType::Custom(name)` is replaced when `name`
//! is one of the template's declared type parameters; composite types
//! (Ptr, Ref, Slice, Array, Tuple) substitute recursively.

use std::collections::HashMap;

use crate::ast::{Expr, TrackType};

struct Template {
    type_params: Vec<String>,
    params: Vec<(String, TrackType)>,
    return_type: Option<TrackType>,
    body: Vec<Expr>,
}

type Env = HashMap<String, TrackType>;

pub fn monomorphize(program: &mut Vec<Expr>) -> Result<(), String> {
    let mut templates: HashMap<String, Template> = HashMap::new();
    for stmt in program.iter() {
        if let Expr::FnDef {
            name,
            generics,
            params,
            return_type,
            body,
        } = stmt
        {
            if generics.is_empty() {
                continue;
            }
            if templates
                .insert(
                    name.clone(),
                    Template {
                        type_params: generics.clone(),
                        params: params.clone(),
                        return_type: return_type.clone(),
                        body: body.clone(),
                    },
                )
                .is_some()
            {
                return Err(format!(
                    "Compile Error: duplicate generic function definition '{}'",
                    name
                ));
            }
        }
    }

    if templates.is_empty() {
        return Ok(());
    }

    // Map of concrete (non-generic) function return types for inference.
    let mut concrete_sigs: HashMap<String, Option<TrackType>> = HashMap::new();
    for stmt in program.iter() {
        if let Expr::FnDef {
            name,
            generics,
            return_type,
            ..
        } = stmt
            && generics.is_empty() {
                concrete_sigs.insert(name.clone(), return_type.clone());
            }
    }

    // Generated instances, keyed by mangled name to guarantee one emission each.
    let mut generated: HashMap<String, Expr> = HashMap::new();
    // In-progress instantiations guard against runaway recursion.
    let mut in_progress: Vec<String> = Vec::new();

    // Walk until fixpoint. Generated instances are collected separately;
    // their bodies were already rewritten recursively during generation.
    let mut top_env: Env = HashMap::new();
    let mut i = 0;
    while i < program.len() {
        let is_template = matches!(&program[i], Expr::FnDef { generics, .. } if !generics.is_empty());
        if is_template {
            // Templates stay symbolic — their bodies are never emitted as-is.
            i += 1;
            continue;
        }
        walk(
            &mut program[i],
            &mut top_env,
            &templates,
            &concrete_sigs,
            &mut generated,
            &mut in_progress,
        )?;
        i += 1;
    }

    let instances: Vec<Expr> = generated.into_values().collect();
    program.extend(instances);
    Ok(())
}

fn walk(
    expr: &mut Expr,
    env: &mut Env,
    templates: &HashMap<String, Template>,
    concrete_sigs: &HashMap<String, Option<TrackType>>,
    generated: &mut HashMap<String, Expr>,
    in_progress: &mut Vec<String>,
) -> Result<(), String> {
    match expr {
        Expr::FnDef { params, body, .. } | Expr::MacroDef { params, body, .. } => {
            // Fresh scope for nested definitions.
            let mut inner = env.clone();
            for (n, ty) in params.iter() {
                inner.insert(n.clone(), ty.clone());
            }
            for stmt in body.iter_mut() {
                walk(stmt, &mut inner, templates, concrete_sigs, generated, in_progress)?;
            }
        }
        Expr::LetDef { name, ty, value, .. } => {
            walk(value, env, templates, concrete_sigs, generated, in_progress)?;
            let inferred = infer_shallow(value, env, templates, concrete_sigs, generated);
            let final_ty = ty.clone().or(inferred);
            if let Some(t) = final_ty {
                env.insert(name.clone(), t);
            }
        }
        Expr::LetDestructure { pattern, value, .. } => {
            walk(value, env, templates, concrete_sigs, generated, in_progress)?;
            if let Ok(TrackType::Tuple(elems)) = infer_tuple_shape(value, env, templates, concrete_sigs, generated) {
                bind_pattern(pattern, &elems, env);
            }
        }
        Expr::Assign { value, .. } => {
            walk(value, env, templates, concrete_sigs, generated, in_progress)?;
        }
        Expr::Return { value } => {
            if let Some(v) = value.as_mut() {
                walk(v, env, templates, concrete_sigs, generated, in_progress)?;
            }
        }
        Expr::IfElse {
            condition,
            then_body,
            else_body,
        } => {
            walk(condition, env, templates, concrete_sigs, generated, in_progress)?;
            for stmt in then_body.iter_mut() {
                let mut branch = env.clone();
                walk(stmt, &mut branch, templates, concrete_sigs, generated, in_progress)?;
            }
            for stmt in else_body.iter_mut() {
                let mut branch = env.clone();
                walk(stmt, &mut branch, templates, concrete_sigs, generated, in_progress)?;
            }
        }
        Expr::WhileLoop { condition, body } => {
            walk(condition, env, templates, concrete_sigs, generated, in_progress)?;
            for stmt in body.iter_mut() {
                let mut branch = env.clone();
                walk(stmt, &mut branch, templates, concrete_sigs, generated, in_progress)?;
            }
        }
        Expr::ForIn { var, iter, body } => {
            walk(iter, env, templates, concrete_sigs, generated, in_progress)?;
            let mut inner = env.clone();
            if let Some(TrackType::Array(elem, _)) | Some(TrackType::Slice(elem)) =
                infer_shallow(iter, env, templates, concrete_sigs, generated)
            {
                inner.insert(var.clone(), *elem);
            } else {
                inner.insert(var.clone(), TrackType::I64);
            }
            for stmt in body.iter_mut() {
                walk(stmt, &mut inner, templates, concrete_sigs, generated, in_progress)?;
            }
        }
        Expr::LensBlock { body, .. } => {
            for stmt in body.iter_mut() {
                let mut branch = env.clone();
                walk(stmt, &mut branch, templates, concrete_sigs, generated, in_progress)?;
            }
        }
        Expr::Match { target, arms } => {
            walk(target, env, templates, concrete_sigs, generated, in_progress)?;
            for arm in arms.iter_mut() {
                if let Some(guard) = arm.guard.as_mut() {
                    walk(guard, env, templates, concrete_sigs, generated, in_progress)?;
                }
                let mut branch = env.clone();
                walk(&mut arm.body, &mut branch, templates, concrete_sigs, generated, in_progress)?;
            }
        }
        Expr::FunctionCall { name, args } => {
            for arg in args.iter_mut() {
                walk(arg, env, templates, concrete_sigs, generated, in_progress)?;
            }
            specialize_call(name, args, env, templates, concrete_sigs, generated, in_progress)?;
        }
        Expr::MacroCall { args, body, .. } => {
            for arg in args.iter_mut() {
                walk(arg, env, templates, concrete_sigs, generated, in_progress)?;
            }
            if let Some(b) = body.as_mut() {
                for stmt in b.iter_mut() {
                    let mut branch = env.clone();
                    walk(stmt, &mut branch, templates, concrete_sigs, generated, in_progress)?;
                }
            }
        }
        Expr::UnaryOp { expr, .. }
        | Expr::AddressOf { target: expr }
        | Expr::ArrayIndex { target: expr, .. }
        | Expr::SliceIndex { target: expr, .. }
        | Expr::TupleIndex { target: expr, .. } => {
            walk(expr, env, templates, concrete_sigs, generated, in_progress)?;
        }
        Expr::BinaryOp { left, right, .. } => {
            walk(left, env, templates, concrete_sigs, generated, in_progress)?;
            walk(right, env, templates, concrete_sigs, generated, in_progress)?;
        }
        Expr::ArrayLiteral { elements } | Expr::TupleLiteral { elements } => {
            for e in elements.iter_mut() {
                walk(e, env, templates, concrete_sigs, generated, in_progress)?;
            }
        }
        Expr::StructInitialization { fields, .. } => {
            for (_, e) in fields.iter_mut() {
                walk(e, env, templates, concrete_sigs, generated, in_progress)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Resolve the base name of a call (`Stack::push` -> `push`).
fn base_name(name: &str) -> &str {
    if let Some(idx) = name.rfind("::") {
        &name[idx + 2..]
    } else {
        name
    }
}

fn specialize_call(
    name: &mut String,
    args: &[Expr],
    env: &Env,
    templates: &HashMap<String, Template>,
    concrete_sigs: &HashMap<String, Option<TrackType>>,
    generated: &mut HashMap<String, Expr>,
    in_progress: &mut Vec<String>,
) -> Result<(), String> {
    let tpl_name = base_name(name).to_string();
    let Some(tpl) = templates.get(&tpl_name) else {
        return Ok(());
    };
    if *name != tpl_name {
        // Already specialized at this site.
        return Ok(());
    }

    if args.len() != tpl.params.len() {
        return Err(format!(
            "Compile Error: generic function '{}' expects {} argument(s), got {}",
            tpl_name,
            tpl.params.len(),
            args.len()
        ));
    }

    // Unify inferred argument types against declared parameter types.
    let mut subst: HashMap<String, TrackType> = HashMap::new();
    for (arg, (_, pty)) in args.iter().zip(tpl.params.iter()) {
        let Some(mut concrete) = infer_shallow(arg, env, templates, concrete_sigs, generated) else {
            return Err(format!(
                "Compile Error: cannot infer type parameter for an argument of '{}' — annotate it first, e.g. `let x: i64 = ...`",
                tpl_name
            ));
        };
        substitute_ty(&mut concrete, &subst);
        unify(pty, &concrete, &tpl.type_params, &mut subst, &tpl_name)?;
    }

    for tp in &tpl.type_params {
        if !subst.contains_key(tp) {
            return Err(format!(
                "Compile Error: cannot infer type parameter '{}' in call to '{}'",
                tp, tpl_name
            ));
        }
    }

    let mangled = mangle(&tpl_name, &subst);
    *name = mangled.clone();

    if generated.contains_key(&mangled) || in_progress.contains(&mangled) {
        return Ok(());
    }

    // Substitute the signature and generate a concrete instance body.
    let mut inst_params = tpl.params.clone();
    for (_, ty) in inst_params.iter_mut() {
        substitute_ty(ty, &subst);
    }
    let mut inst_ret = tpl.return_type.clone();
    if let Some(rt) = inst_ret.as_mut() {
        substitute_ty(rt, &subst);
    }

    in_progress.push(mangled.clone());
    let mut inst_body = tpl.body.clone();

    let result = (|| -> Result<(), String> {
        let mut inst_env: Env = HashMap::new();
        for (n, ty) in inst_params.iter() {
            inst_env.insert(n.clone(), ty.clone());
        }
        for stmt in inst_body.iter_mut() {
            substitute_stmt_types(stmt, &subst);
            walk(stmt, &mut inst_env, templates, concrete_sigs, generated, in_progress)?;
        }
        Ok(())
    })();
    in_progress.pop();
    result?;

    generated.insert(
        mangled.clone(),
        Expr::FnDef {
            name: mangled,
            generics: Vec::new(),
            params: inst_params,
            return_type: inst_ret,
            body: inst_body,
        },
    );
    Ok(())
}

fn mangle(base: &str, subst: &HashMap<String, TrackType>) -> String {
    let mut parts: Vec<String> = subst
        .iter()
        .map(|(k, v)| format!("{}{}", k, type_tag(v)))
        .collect();
    parts.sort();
    format!("{}__{}", base, parts.join("_"))
}

fn type_tag(ty: &TrackType) -> String {
    match ty {
        TrackType::U8 => "u8".into(),
        TrackType::I8 => "i8".into(),
        TrackType::I32 => "i32".into(),
        TrackType::U32 => "u32".into(),
        TrackType::I64 => "i64".into(),
        TrackType::U64 => "u64".into(),
        TrackType::Bool => "bool".into(),
        TrackType::Void => "void".into(),
        TrackType::Ptr(inner) => format!("p{}", type_tag(inner)),
        TrackType::Ref(inner) => format!("r{}", type_tag(inner)),
        TrackType::Slice(inner) => format!("s{}", type_tag(inner)),
        TrackType::Array(inner, n) => format!("a{}_{}", type_tag(inner), n),
        TrackType::Tuple(elems) => {
            let tags: Vec<String> = elems.iter().map(type_tag).collect();
            format!("t{}", tags.join("."))
        }
        TrackType::Custom(name) => sanitize(name),
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn unify(
    param: &TrackType,
    concrete: &TrackType,
    type_params: &[String],
    subst: &mut HashMap<String, TrackType>,
    ctx: &str,
) -> Result<(), String> {
    match param {
        TrackType::Custom(name) => {
            if type_params.iter().any(|tp| tp == name) {
                if let Some(existing) = subst.get(name) {
                    if existing != concrete {
                        return Err(format!(
                            "Compile Error: conflicting types for '{}' in '{}': {:?} vs {:?}",
                            name, ctx, existing, concrete
                        ));
                    }
                } else {
                    subst.insert(name.clone(), concrete.clone());
                }
            }
            Ok(())
        }
        TrackType::Ptr(inner) => {
            if let TrackType::Ptr(c) = concrete {
                unify(inner, c, type_params, subst, ctx)
            } else {
                Ok(())
            }
        }
        TrackType::Ref(inner) => {
            if let TrackType::Ref(c) = concrete {
                unify(inner, c, type_params, subst, ctx)
            } else {
                Ok(())
            }
        }
        TrackType::Slice(inner) => {
            if let TrackType::Slice(c) = concrete {
                unify(inner, c, type_params, subst, ctx)
            } else {
                Ok(())
            }
        }
        TrackType::Array(inner, _) => {
            if let TrackType::Array(c, _) = concrete {
                unify(inner, c, type_params, subst, ctx)
            } else {
                Ok(())
            }
        }
        TrackType::Tuple(elems) => {
            if let TrackType::Tuple(cs) = concrete {
                for (p, c) in elems.iter().zip(cs.iter()) {
                    unify(p, c, type_params, subst, ctx)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn substitute_ty(ty: &mut TrackType, subst: &HashMap<String, TrackType>) {
    match ty {
        TrackType::Custom(name) => {
            if let Some(concrete) = subst.get(name) {
                *ty = concrete.clone();
            }
        }
        TrackType::Ptr(inner) => substitute_ty(inner, subst),
        TrackType::Ref(inner) => substitute_ty(inner, subst),
        TrackType::Slice(inner) => substitute_ty(inner, subst),
        TrackType::Array(inner, _) => substitute_ty(inner, subst),
        TrackType::Tuple(elems) => {
            for e in elems {
                substitute_ty(e, subst);
            }
        }
        _ => {}
    }
}

/// Rewrite type annotations inside an instantiated statement tree.
fn substitute_stmt_types(expr: &mut Expr, subst: &HashMap<String, TrackType>) {
    match expr {
        Expr::LetDef { ty, value, .. } => {
            if let Some(t) = ty.as_mut() {
                substitute_ty(t, subst);
            }
            substitute_stmt_types(value, subst);
        }
        Expr::Return { value } => {
            if let Some(v) = value.as_mut() {
                substitute_stmt_types(v, subst);
            }
        }
        Expr::IfElse {
            then_body,
            else_body,
            ..
        } => {
            for s in then_body.iter_mut().chain(else_body.iter_mut()) {
                substitute_stmt_types(s, subst);
            }
        }
        Expr::WhileLoop { body, condition } => {
            substitute_stmt_types(condition, subst);
            for s in body.iter_mut() {
                substitute_stmt_types(s, subst);
            }
        }
        Expr::ForIn { iter, body, .. } => {
            substitute_stmt_types(iter, subst);
            for s in body.iter_mut() {
                substitute_stmt_types(s, subst);
            }
        }
        Expr::LensBlock { body, .. } => {
            for s in body.iter_mut() {
                substitute_stmt_types(s, subst);
            }
        }
        Expr::Match { target, arms } => {
            substitute_stmt_types(target, subst);
            for arm in arms.iter_mut() {
                if let Some(guard) = arm.guard.as_mut() {
                    substitute_stmt_types(guard, subst);
                }
                substitute_stmt_types(&mut arm.body, subst);
            }
        }
        Expr::FunctionCall { args, .. } | Expr::MacroCall { args, .. } => {
            for a in args.iter_mut() {
                substitute_stmt_types(a, subst);
            }
        }
        Expr::Assign { value, target, .. } => {
            substitute_stmt_types(value, subst);
            substitute_stmt_types(target, subst);
        }
        _ => {}
    }
}

fn bind_pattern(pattern: &crate::ast::Pattern, elems: &[TrackType], env: &mut Env) {
    use crate::ast::Pattern::*;
    match pattern {
        Tuple(pats) => {
            for (p, t) in pats.iter().zip(elems.iter()) {
                bind_pattern(p, std::slice::from_ref(t), env);
            }
        }
        Ident(name) => {
            if let Some(t) = elems.first() {
                env.insert(name.clone(), t.clone());
            }
        }
        _ => {}
    }
}

fn infer_tuple_shape(expr: &Expr, env: &Env, templates: &HashMap<String, Template>, concrete_sigs: &HashMap<String, Option<TrackType>>, generated: &HashMap<String, Expr>) -> Result<TrackType, ()> {
    infer_shallow(expr, env, templates, concrete_sigs, generated).ok_or(())
}

/// Lightweight structural type inference sufficient for choosing a
/// monomorphization substitution. Deliberately conservative: returns None
/// when unsure rather than guessing wrong.
fn infer_shallow(
    expr: &Expr,
    env: &Env,
    templates: &HashMap<String, Template>,
    concrete_sigs: &HashMap<String, Option<TrackType>>,
    generated: &HashMap<String, Expr>,
) -> Option<TrackType> {
    match expr {
        Expr::IntLiteral(_) => Some(TrackType::I32),
        Expr::BoolLiteral(_) => Some(TrackType::Bool),
        Expr::StringLiteral(_) => Some(TrackType::Ptr(Box::new(TrackType::I32))),
        Expr::Variable(name) => env.get(name).cloned(),
        Expr::BinaryOp { op, left, .. } => {
            use crate::ast::BinOp::*;
            if matches!(op, Eq | Neq | Lt | Gt | Lte | Gte | And | Or) {
                Some(TrackType::Bool)
            } else {
                infer_shallow(left, env, templates, concrete_sigs, generated)
            }
        }
        Expr::UnaryOp { op, expr } => match op {
            crate::ast::UnaryOp::Not => Some(TrackType::Bool),
            crate::ast::UnaryOp::Neg => infer_shallow(expr, env, templates, concrete_sigs, generated),
            crate::ast::UnaryOp::Deref => match infer_shallow(expr, env, templates, concrete_sigs, generated) {
                Some(TrackType::Ptr(inner)) | Some(TrackType::Ref(inner)) => Some(*inner),
                _ => None,
            },
        },
        Expr::AddressOf { target } => {
            infer_shallow(target, env, templates, concrete_sigs, generated).map(|inner| TrackType::Ref(Box::new(inner)))
        }
        Expr::ArrayLiteral { elements } => {
            let elem = elements.first().and_then(|e| infer_shallow(e, env, templates, concrete_sigs, generated))?;
            Some(TrackType::Array(Box::new(elem), elements.len()))
        }
        Expr::TupleLiteral { elements } => {
            let mut tys = Vec::new();
            for e in elements {
                tys.push(infer_shallow(e, env, templates, concrete_sigs, generated)?);
            }
            Some(TrackType::Tuple(tys))
        }
        Expr::TupleIndex { target, index } => match infer_shallow(target, env, templates, concrete_sigs, generated)? {
            TrackType::Tuple(mut elems) => {
                if *index < elems.len() {
                    Some(elems.swap_remove(*index))
                } else {
                    None
                }
            }
            _ => None,
        },
        Expr::ArrayIndex { target, .. } => match infer_shallow(target, env, templates, concrete_sigs, generated)? {
            TrackType::Array(inner, _) | TrackType::Slice(inner) => Some(*inner),
            _ => None,
        },
        Expr::StructInitialization { ty_name, .. } => Some(TrackType::Custom(ty_name.clone())),
        Expr::LetDef { value, .. } => infer_shallow(value, env, templates, concrete_sigs, generated),
        Expr::IfElse { then_body, .. } => then_body.last().and_then(|e| infer_shallow(e, env, templates, concrete_sigs, generated)),
        Expr::FunctionCall { name, args } => {
            // Already-specialized instance?
            if let Expr::FnDef { return_type, .. } = generated.get(name)? {
                return return_type.clone();
            }
            if let Some(rt) = concrete_sigs.get(name) {
                return rt.clone();
            }
            // Generic template not yet specialized — infer its return via
            // argument-driven substitution (best-effort, falls back to Custom).
            let base = base_name(name);
            let tpl = templates.get(base)?;
            let mut subst: HashMap<String, TrackType> = HashMap::new();
            for (arg, (_, pty)) in args.iter().zip(tpl.params.iter()) {
                let concrete = infer_shallow(arg, env, templates, concrete_sigs, generated)?;
                unify(pty, &concrete, &tpl.type_params, &mut subst, base).ok()?;
            }
            let mut ret = tpl.return_type.clone()?;
            substitute_ty(&mut ret, &subst);
            Some(ret)
        }
        _ => None,
    }
}
