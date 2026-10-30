pub mod ast;
pub mod checker;
pub mod codegen;
pub mod lexer;
pub mod lsp;
pub mod mono;
pub mod parser;
pub mod yard;

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

/// The Track runtime, shared with the self-hosted compiler (word ABI).
pub const RUNTIME_C_SOURCE: &str = include_str!("../runtime/track_runtime.c");

/// Compile source string through Lexer -> Parser -> Monomorphizer -> LinearChecker pipeline.
pub fn compile_source(source: &str) -> Result<Vec<ast::Expr>, String> {
    compile_source_with_info(source).map(|(program, _)| program)
}

/// Like [`compile_source`], also returning the type layouts codegen needs
/// (enum discriminants, payload unions, aliases — including imports).
pub fn compile_source_with_info(
    source: &str,
) -> Result<(Vec<ast::Expr>, codegen::TypeInfo), String> {
    let tokens = lexer::Lexer::tokenize(source)?;
    let mut p = parser::Parser::new(tokens, source.to_string());
    let program = p.parse_program()?;
    let mut program = program;
    mono::monomorphize(&mut program)?;
    let mut chk = checker::LinearChecker::new();
    chk.check_program(&program)?;
    Ok((program, chk.type_info))
}

/// Full build: source -> object file -> linked executable in specified directory.
pub fn build_file_in_dir(filename: &str, out_dir: &Path) -> Result<PathBuf, String> {
    let source =
        fs::read_to_string(filename).map_err(|e| format!("Error reading '{}': {}", filename, e))?;

    let (program, info) = compile_source_with_info(&source)?;

    let mut cg = codegen::CodeGen::new("track_module");
    cg.set_type_info(info);
    cg.compile_program(&program);

    let stem = Path::new(filename)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let obj_path = out_dir.join(format!("{}.o", stem));
    let runtime_path = out_dir.join(format!("_track_runtime_{}.c", stem));
    let exe_path = out_dir.join(&stem);

    cg.write_object_file(&obj_path)?;
    fs::write(&runtime_path, RUNTIME_C_SOURCE)
        .map_err(|e| format!("Failed to write runtime helper: {}", e))?;

    let status = process::Command::new("cc")
        .arg("-O3")
        .arg(&obj_path)
        .arg(&runtime_path)
        .arg("-o")
        .arg(&exe_path)
        .arg("-lm")
        .arg("-no-pie")
        .status()
        .map_err(|e| format!("Linker error: {}", e))?;

    let _ = fs::remove_file(&obj_path);
    let _ = fs::remove_file(&runtime_path);

    if !status.success() {
        return Err(format!("Linker failed with exit code: {:?}", status.code()));
    }

    Ok(exe_path)
}

/// Full build placing executable in current directory.
pub fn build_file(filename: &str) -> Result<PathBuf, String> {
    build_file_in_dir(filename, Path::new("."))
}
