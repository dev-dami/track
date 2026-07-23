use std::time::Instant;
use track::lexer::Lexer;
use track::parser::Parser;
use track::compile_source;

#[test]
fn test_performance_lexer_throughput() {
    let statement = "let mut x: i64 = 100 + 200 * 300 / 400;\n";
    let large_source = statement.repeat(5000); // ~200,000 characters, ~75,000 tokens

    let start = Instant::now();
    let tokens = Lexer::tokenize(&large_source).unwrap();
    let elapsed = start.elapsed();

    println!(
        "Lexed {} tokens in {:?} ({:.2} million tokens/sec)",
        tokens.len(),
        elapsed,
        (tokens.len() as f64 / 1_000_000.0) / elapsed.as_secs_f64()
    );

    assert!(tokens.len() >= 60_000);
    assert!(elapsed.as_secs() < 5, "Lexer took too long!");
}

#[test]
fn test_performance_parser_throughput() {
    let fn_def = "fn add(a: i32, b: i32) -> i32 { return a + b; }\n";
    let large_source = fn_def.repeat(1000); // 1,000 function definitions

    let tokens = Lexer::tokenize(&large_source).unwrap();
    let mut parser = Parser::new(tokens, large_source);

    let start = Instant::now();
    let program = parser.parse_program().unwrap();
    let elapsed = start.elapsed();

    println!(
        "Parsed {} functions in {:?} ({:.2} functions/sec)",
        program.len(),
        elapsed,
        program.len() as f64 / elapsed.as_secs_f64()
    );

    assert_eq!(program.len(), 1000);
    assert!(elapsed.as_secs() < 5, "Parser took too long!");
}

#[test]
fn test_performance_full_pipeline_throughput() {
    let fn_def = "fn compute(x: i32) -> i32 { let y = x + 1; return y * 2; }\n";
    let source = fn_def.repeat(500);

    let start = Instant::now();
    let ast = compile_source(&source).unwrap();
    let elapsed = start.elapsed();

    println!(
        "Compiled and checked AST of {} statements in {:?}",
        ast.len(),
        elapsed
    );

    assert_eq!(ast.len(), 500);
    assert!(elapsed.as_secs() < 5, "Pipeline took too long!");
}
