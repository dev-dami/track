use track::ast::{BinOp, Expr, TrackType};
use track::checker::LinearChecker;
use track::lexer::Lexer;
use track::parser::Parser;

fn parse(source: &str) -> Vec<Expr> {
    let tokens = Lexer::tokenize(source).expect("Tokenization failed");
    let mut parser = Parser::new(tokens, source.to_string());
    parser.parse_program().expect("Parsing failed")
}

fn check(source: &str) -> Result<(), String> {
    let program = parse(source);
    let mut checker = LinearChecker::new();
    checker.check_program(&program)
}

#[test]
fn test_strict_mutability_reassignment_rejection() {
    let source = r#"
        fn main() -> void {
            let x = 10;
            x = 20;
        }
    "#;
    let res = check(source);
    assert!(
        res.is_err(),
        "Expected mutability check failure when reassigning non-mut variable"
    );
    assert!(
        res.unwrap_err()
            .contains("Cannot mutate immutable variable 'x'"),
        "Error message must specify immutable variable mutation"
    );
}

#[test]
fn test_strict_mutable_variable_reassignment_allowed() {
    let source = r#"
        fn main() -> void {
            let mut x = 10;
            x = 20;
            print(x);
        }
    "#;
    assert!(
        check(source).is_ok(),
        "Reassigning mut variable should be allowed"
    );
}

#[test]
fn test_strict_bitwise_or_vs_logical_or_ast_precedence() {
    let source = "fn main() -> void { let res = a | b || c & d; }";
    let ast = parse(source);

    if let Expr::FnDef { body, .. } = &ast[0]
        && let Expr::LetDef { value, .. } = &body[0]
    {
        if let Expr::BinaryOp { op, .. } = value.as_ref() {
            assert_eq!(
                *op,
                BinOp::Or,
                "Top-level binary operator should be Logical OR (||)"
            );
        } else {
            panic!("Expected BinaryOp for bitwise vs logical precedence test");
        }
    }
}

#[test]
fn test_strict_double_move_in_loop_rejection() {
    let source = r#"
        struct Resource { id: i32 }

        fn consume(r: Resource) -> void {
            print(1);
        }

        fn main() -> void {
            let r = Resource { id: 1 };
            let mut i = 0;
            while i < 5 {
                let consumed = r;
                i = i + 1;
            }
        }
    "#;
    let res = check(source);
    assert!(
        res.is_err(),
        "Expected error when moving linear resource inside loop without reinitialization"
    );
}

#[test]
fn test_strict_unresolved_type_alias_detection() {
    let source = r#"
        type Buffer = UnknownType;
        fn main() -> void {
            let buf = 10;
        }
    "#;
    let ast = parse(source);
    assert_eq!(
        ast[0],
        Expr::TypeAlias {
            name: "Buffer".to_string(),
            target: TrackType::Custom("UnknownType".to_string())
        }
    );
}

#[test]
fn test_strict_borrow_escape_from_lens_block() {
    let source = r#"
        struct System { state: i32 }

        fn main() -> void {
            let mut sys = System { state: 1 };
            with sys -> s {
                let val = 10;
            }
            sys = System { state: 2 };
        }
    "#;
    assert!(
        check(source).is_ok(),
        "Lens block should cleanly unlock target after end"
    );
}
