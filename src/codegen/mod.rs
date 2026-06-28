use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::BasicValueEnum;
use crate::ast::Expr;

macro_rules! log_debug {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    };
}

#[allow(dead_code)]
pub struct CodeGen<'ctx> {
    pub context: &'ctx Context,
    module: Module<'ctx>,
    builder: inkwell::builder::Builder<'ctx>,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
        }
    }

    pub fn compile_expr(&self, expr: &Expr) -> Option<BasicValueEnum<'ctx>> {
        match expr {
            Expr::IntLiteral(val) => {
                let i64_type = self.context.i64_type();
                Some(i64_type.const_int(*val as u64, false).into())
            }
            Expr::StringLiteral(s) => {
                log_debug!("  [codegen] string \"{}\"", s);
                None
            }
            Expr::BoolLiteral(val) => {
                let bool_type = self.context.bool_type();
                Some(bool_type.const_int(*val as u64, false).into())
            }
            Expr::Variable(name) => {
                log_debug!("  [codegen] load '%{}'", name);
                None
            }
            Expr::BinaryOp { op, left, right } => {
                let l = self.compile_expr(left);
                let r = self.compile_expr(right);
                log_debug!("  [codegen] {:?} {:?} {:?}", l.is_some(), op, r.is_some());
                None
            }
            Expr::UnaryOp { op, expr } => {
                self.compile_expr(expr);
                log_debug!("  [codegen] unary {:?}", op);
                None
            }
            Expr::ArrayLiteral { elements } => {
                log_debug!("  [codegen] array literal ({} elements)", elements.len());
                for elem in elements {
                    self.compile_expr(elem);
                }
                None
            }
            Expr::ArrayIndex { target, index } => {
                self.compile_expr(target);
                self.compile_expr(index);
                log_debug!("  [codegen] GEP (array index)");
                None
            }
            Expr::AddressOf { target } => {
                self.compile_expr(target);
                log_debug!("  [codegen] address-of");
                None
            }
            Expr::FunctionCall { name, args } => {
                for arg in args {
                    self.compile_expr(arg);
                }
                log_debug!("  [codegen] call @{}()", name);
                None
            }
            Expr::LensBlock {
                target,
                lens_name,
                body,
            } => {
                log_debug!("  [codegen] lens '{}' on '{}'", lens_name, target);
                for expr in body {
                    self.compile_expr(expr);
                }
                log_debug!("  [codegen] exit lens '{}'", lens_name);
                None
            }
            Expr::IfElse {
                condition,
                then_body,
                else_body,
            } => {
                self.compile_expr(condition);
                log_debug!("  [codegen] if {{");
                for stmt in then_body {
                    self.compile_expr(stmt);
                }
                if !else_body.is_empty() {
                    log_debug!("  [codegen] }} else {{");
                    for stmt in else_body {
                        self.compile_expr(stmt);
                    }
                }
                log_debug!("  [codegen] }}");
                None
            }
            Expr::WhileLoop { condition, body } => {
                log_debug!("  [codegen] while ({{");
                self.compile_expr(condition);
                log_debug!("  [codegen] ) {{");
                for stmt in body {
                    self.compile_expr(stmt);
                }
                log_debug!("  [codegen] }}");
                None
            }
            Expr::Return { value } => {
                if let Some(val) = value {
                    let v = self.compile_expr(val);
                    log_debug!("  [codegen] ret {:?}", v.is_some());
                } else {
                    log_debug!("  [codegen] ret void");
                }
                None
            }
            Expr::Assign { target, value } => {
                self.compile_expr(value);
                log_debug!("  [codegen] store -> {:?}", target);
                None
            }
            Expr::FnDef {
                name,
                params,
                return_type,
                body,
            } => {
                log_debug!(
                    "  [codegen] fn {}({}) -> {:?} {{",
                    name,
                    params
                        .iter()
                        .map(|(n, t)| format!("{}: {:?}", n, t))
                        .collect::<Vec<_>>()
                        .join(", "),
                    return_type
                );
                for stmt in body {
                    self.compile_expr(stmt);
                }
                log_debug!("  [codegen] }}");
                None
            }
            Expr::StructInitialization { ty_name, fields } => {
                log_debug!(
                    "  [codegen] init struct '{}' ({} fields)",
                    ty_name,
                    fields.len()
                );
                for (fname, fval) in fields {
                    log_debug!("  [codegen]   field '{}': {:?}", fname, fval);
                    self.compile_expr(fval);
                }
                None
            }
        }
    }

    pub fn compile_program(&self, program: &[Expr]) {
        for stmt in program {
            self.compile_expr(stmt);
        }
    }
}
