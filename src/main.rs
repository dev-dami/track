mod ast;
mod checker;
mod codegen;
mod lexer;
mod parser;
mod yard;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "build" => {
            let file = require_file_arg(&args, 2);
            cmd_build(&file);
        }
        "run" => {
            let file = require_file_arg(&args, 2);
            cmd_run(&file);
        }
        "check" => {
            let file = require_file_arg(&args, 2);
            cmd_check(&file);
        }
        "yard" => {
            let yard_args: Vec<String> = args[2..].to_vec();
            match yard::run(&yard_args) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("yard error: {}", e);
                    process::exit(1);
                }
            }
        }
        "--help" | "-h" | "help" => {
            print_usage();
        }
        // Legacy: if argument ends with .trk, treat as `build`
        arg if arg.ends_with(".trk") => {
            cmd_build(arg);
        }
        other => {
            eprintln!("Unknown command: '{}'\n", other);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Track — Systems programming with linear types\n");
    eprintln!("USAGE:");
    eprintln!("    track build <file.trk>    Compile to native executable");
    eprintln!("    track run <file.trk>      Compile and run");
    eprintln!("    track check <file.trk>    Type-check only (no codegen)");
    eprintln!("    track yard <command>      Package manager");
    eprintln!("    track <file.trk>          Same as 'track build'");
    eprintln!("    track help                Show this help");
}

fn require_file_arg(args: &[String], idx: usize) -> String {
    args.get(idx)
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("Error: expected a .trk file path");
            process::exit(1);
        })
}

// ── compile pipeline ─────────────────────────────────────────────────

fn compile_source(source: &str) -> Vec<ast::Expr> {
    // Step 1: Lex
    let tokens = match lexer::Lexer::tokenize(source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Step 2: Parse
    let mut p = parser::Parser::new(tokens, source.to_string());
    let program = match p.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // Step 3: Linear check
    let mut chk = checker::LinearChecker::new();
    match chk.check_program(&program) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }

    program
}

/// Full build: source → object file → linked executable. Returns the exe path.
pub fn build_file(filename: &str) -> PathBuf {
    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {}", filename, e);
        process::exit(1);
    });

    let program = compile_source(&source);

    // Step 4: Codegen
    let context = inkwell::context::Context::create();
    let mut cg = codegen::CodeGen::new(&context, "track_module");
    cg.compile_program(&program);

    // Derive output paths
    let stem = Path::new(filename)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let obj_path = format!("{}.o", stem);
    let exe_path = stem.clone();

    // Emit object file
    cg.write_object_file(Path::new(&obj_path)).unwrap_or_else(|e| {
        eprintln!("Codegen error: {}", e);
        process::exit(1);
    });

    // Link
    let status = process::Command::new("cc")
        .arg(&obj_path)
        .arg("-o")
        .arg(&exe_path)
        .arg("-lm")
        .arg("-no-pie")
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Linker error: {}", e);
            process::exit(1);
        });

    if !status.success() {
        eprintln!("Linker failed with exit code: {:?}", status.code());
        process::exit(1);
    }

    // Clean up object file
    let _ = fs::remove_file(&obj_path);

    println!("✓ Built: {}", exe_path);
    PathBuf::from(exe_path)
}

// ── commands ─────────────────────────────────────────────────────────

fn cmd_build(filename: &str) {
    build_file(filename);
}

fn cmd_run(filename: &str) {
    let exe = build_file(filename);
    let exe_path = if exe.is_absolute() {
        exe
    } else {
        env::current_dir().unwrap().join(exe)
    };

    let status = process::Command::new(&exe_path)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Failed to run '{}': {}", exe_path.display(), e);
            process::exit(1);
        });

    process::exit(status.code().unwrap_or(1));
}

fn cmd_check(filename: &str) {
    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {}", filename, e);
        process::exit(1);
    });

    compile_source(&source);
    println!("✓ Check passed: {}", filename);
}
