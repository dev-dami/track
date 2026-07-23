pub mod ast;
pub mod checker;
pub mod codegen;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod yard;

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const RUNTIME_C_SOURCE: &str = r#"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void* alloc(size_t size) { return malloc(size); }
typedef struct { int* data; int len; int cap; } Vec;
Vec vec_init(int cap) {
    Vec v;
    v.data = (int*)malloc((size_t)cap * sizeof(int));
    v.len = 0;
    v.cap = cap;
    return v;
}
void vec_push(Vec* v, int val) {
    if (v && v->data && v->len < v->cap) {
        v->data[v->len++] = val;
    }
}
int vec_get(const Vec* v, int idx) {
    if (v && v->data && idx >= 0 && idx < v->len) {
        return v->data[idx];
    }
    return 0;
}
void vec_set(Vec* v, int idx, int val) {
    if (v && v->data && idx >= 0 && idx < v->len) {
        v->data[idx] = val;
    }
}
int vec_pop(Vec* v) {
    if (v && v->len > 0) {
        return v->data[--v->len];
    }
    return 0;
}
void vec_free(Vec v) {
    if (v.data) {
        free(v.data);
    }
}
typedef struct { char* data; int len; } Str;
void str_free(Str s) {
    if (s.data) {
        free(s.data);
    }
}
void file_close(void* f) {
    if (f) {
        fclose((FILE*)f);
    }
}
"#;

/// Compile source string through Lexer -> Parser -> LinearChecker pipeline.
pub fn compile_source(source: &str) -> Result<Vec<ast::Expr>, String> {
    let tokens = lexer::Lexer::tokenize(source)?;
    let mut p = parser::Parser::new(tokens, source.to_string());
    let program = p.parse_program()?;
    let mut chk = checker::LinearChecker::new();
    chk.check_program(&program)?;
    Ok(program)
}

/// Full build: source -> object file -> linked executable in specified directory.
pub fn build_file_in_dir(filename: &str, out_dir: &Path) -> Result<PathBuf, String> {
    let source = fs::read_to_string(filename)
        .map_err(|e| format!("Error reading '{}': {}", filename, e))?;

    let program = compile_source(&source)?;

    let context = inkwell::context::Context::create();
    let mut cg = codegen::CodeGen::new(&context, "track_module");
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
