mod ast;
mod checker;
mod codegen;
mod lexer;
mod parser;

use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: track <file.trk>");
        std::process::exit(1);
    });

    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {}", filename, e);
        std::process::exit(1);
    });

    // Step 1: Lex
    let tokens = match lexer::Lexer::tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if cfg!(debug_assertions) {
        println!("=== Tokens ({}) ===", tokens.len());
        for (tok, span) in &tokens {
            println!("  {:?} @ {:?}", tok, source[span.clone()].trim());
        }
    }

    // Step 2: Parse
    let mut parser = parser::Parser::new(tokens, source.clone());
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if cfg!(debug_assertions) {
        println!("\n=== AST ({}) statements ===", program.len());
        for stmt in &program {
            println!("  {:#?}", stmt);
        }
    }

    // Step 3: Linear check
    let mut checker = checker::LinearChecker::new();
    match checker.check_program(&program) {
        Ok(()) => {
            if cfg!(debug_assertions) {
                println!("\n=== Linear Check: PASSED ===");
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    // Step 4: Codegen with inkwell
    if cfg!(debug_assertions) {
        println!("\n=== Codegen (inkwell) ===");
    }
    let context = inkwell::context::Context::create();
    let codegen = codegen::CodeGen::new(&context, "track_module");
    codegen.compile_program(&program);

    if cfg!(debug_assertions) {
        println!("\n=== Compilation complete ===");
    }
}
