use track::ast::*;
use track::lexer::Lexer;
use track::parser::Parser;

fn parse(source: &str) -> Vec<Expr> {
    let tokens = Lexer::tokenize(source).unwrap();
    let mut parser = Parser::new(tokens, source.to_string());
    parser.parse_program().unwrap()
}

#[test]
fn test_parse_fn_def() {
    let source = "fn add(a: i32, b: i32) -> i32 { return a + b; }";
    let ast = parse(source);
    assert_eq!(
        ast,
        vec![Expr::FnDef {
            name: "add".to_string(),
            params: vec![
                ("a".to_string(), TrackType::I32),
                ("b".to_string(), TrackType::I32)
            ],
            return_type: Some(TrackType::I32),
            body: vec![Expr::Return {
                value: Some(Box::new(Expr::BinaryOp {
                    op: BinOp::Add,
                    left: Box::new(Expr::Variable("a".to_string())),
                    right: Box::new(Expr::Variable("b".to_string())),
                }))
            }]
        }]
    );
}

#[test]
fn test_parse_let_def() {
    let source = "let mut x: i64 = 100;";
    let ast = parse(source);
    assert_eq!(
        ast,
        vec![Expr::LetDef {
            name: "x".to_string(),
            ty: Some(TrackType::I64),
            value: Box::new(Expr::IntLiteral(100)),
        }]
    );
}

#[test]
fn test_parse_struct_and_lens() {
    let source = "let u = User { age: 30 }; with u -> user { user.set_age(31); }";
    let ast = parse(source);
    assert_eq!(ast.len(), 2);
    assert_eq!(
        ast[0],
        Expr::LetDef {
            name: "u".to_string(),
            ty: None,
            value: Box::new(Expr::StructInitialization {
                ty_name: "User".to_string(),
                fields: vec![("age".to_string(), Expr::IntLiteral(30))]
            })
        }
    );
    assert_eq!(
        ast[1],
        Expr::LensBlock {
            target: "u".to_string(),
            lens_name: "user".to_string(),
            body: vec![Expr::FunctionCall {
                name: "set_age".to_string(),
                args: vec![Expr::Variable("user".to_string()), Expr::IntLiteral(31)]
            }]
        }
    );
}


#[test]
fn test_parse_enum_and_union() {
    let source = "enum Status: i32 { Active = 1, Inactive = 0 } union Value { Int(i32), Empty }";
    let ast = parse(source);
    assert_eq!(ast.len(), 2);
    assert_eq!(
        ast[0],
        Expr::EnumDef {
            name: "Status".to_string(),
            underlying_type: Some(TrackType::I32),
            variants: vec![
                ("Active".to_string(), Some(Expr::IntLiteral(1))),
                ("Inactive".to_string(), Some(Expr::IntLiteral(0)))
            ]
        }
    );
    assert_eq!(
        ast[1],
        Expr::UnionDef {
            name: "Value".to_string(),
            variants: vec![
                ("Int".to_string(), Some(TrackType::I32)),
                ("Empty".to_string(), None)
            ]
        }
    );
}

#[test]
fn test_parse_match() {
    let source = "match val { Value::Int(n) if n > 0 => print(n), _ => print(0) }";
    let ast = parse(source);
    assert_eq!(
        ast,
        vec![Expr::Match {
            target: Box::new(Expr::Variable("val".to_string())),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Variant {
                        enum_or_union: "Value".to_string(),
                        variant: "Int".to_string(),
                        binding: Some("n".to_string()),
                    },
                    guard: Some(Expr::BinaryOp {
                        op: BinOp::Gt,
                        left: Box::new(Expr::Variable("n".to_string())),
                        right: Box::new(Expr::IntLiteral(0)),
                    }),
                    body: Expr::FunctionCall {
                        name: "print".to_string(),
                        args: vec![Expr::Variable("n".to_string())]
                    }
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Expr::FunctionCall {
                        name: "print".to_string(),
                        args: vec![Expr::IntLiteral(0)]
                    }
                }
            ]
        }]
    );
}

#[test]
fn test_parse_macro_def_and_call() {
    let source = "@macro bit(n: u32) -> u32 { return 1 << n; } let val = @bit(5);";
    let ast = parse(source);
    assert_eq!(ast.len(), 2);
    assert_eq!(
        ast[0],
        Expr::MacroDef {
            name: "bit".to_string(),
            params: vec![("n".to_string(), TrackType::U32)],
            return_type: Some(TrackType::U32),
            body: vec![Expr::Return {
                value: Some(Box::new(Expr::BinaryOp {
                    op: BinOp::Shl,
                    left: Box::new(Expr::IntLiteral(1)),
                    right: Box::new(Expr::Variable("n".to_string()))
                }))
            }]
        }
    );
    assert_eq!(
        ast[1],
        Expr::LetDef {
            name: "val".to_string(),
            ty: None,
            value: Box::new(Expr::MacroCall {
                name: "bit".to_string(),
                args: vec![Expr::IntLiteral(5)],
                body: None,
            })
        }
    );
}

#[test]
fn test_parse_use_and_const() {
    let source = "@use(\"std::math::{abs, max}\") as m; const PI = 3;";
    let ast = parse(source);
    assert_eq!(ast.len(), 2);
    assert_eq!(
        ast[0],
        Expr::Use {
            path: "std::math".to_string(),
            imports: Some(vec!["abs".to_string(), "max".to_string()]),
            alias: Some("m".to_string()),
        }
    );
    assert_eq!(
        ast[1],
        Expr::ConstDef {
            name: "PI".to_string(),
            value: Box::new(Expr::IntLiteral(3))
        }
    );
}

#[test]
fn test_parse_array_type_and_literal() {
    let source = "let arr: [i32; 3] = [1, 2, 3]; let x = arr[0];";
    let ast = parse(source);
    assert_eq!(ast.len(), 2);
    assert_eq!(
        ast[0],
        Expr::LetDef {
            name: "arr".to_string(),
            ty: Some(TrackType::Array(Box::new(TrackType::I32), 3)),
            value: Box::new(Expr::ArrayLiteral {
                elements: vec![Expr::IntLiteral(1), Expr::IntLiteral(2), Expr::IntLiteral(3)]
            })
        }
    );
    assert_eq!(
        ast[1],
        Expr::LetDef {
            name: "x".to_string(),
            ty: None,
            value: Box::new(Expr::ArrayIndex {
                target: Box::new(Expr::Variable("arr".to_string())),
                index: Box::new(Expr::IntLiteral(0))
            })
        }
    );
}
