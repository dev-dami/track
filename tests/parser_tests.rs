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
            generics: Vec::new(),
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
            mutable: true,
            ty: Some(TrackType::I64),
            value: Box::new(Expr::IntLiteral(100)),
        }]
    );
}

#[test]
fn test_parse_slice_type_and_indexing() {
    let source = "let s: []u8 = arr[0..5];";
    let ast = parse(source);
    assert_eq!(
        ast,
        vec![Expr::LetDef {
            name: "s".to_string(),
            mutable: false,
            ty: Some(TrackType::Slice(Box::new(TrackType::U8))),
            value: Box::new(Expr::SliceIndex {
                target: Box::new(Expr::Variable("arr".to_string())),
                start: Some(Box::new(Expr::IntLiteral(0))),
                end: Some(Box::new(Expr::IntLiteral(5))),
            }),
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
            mutable: false,
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
                        bindings: vec![Pattern::Ident("n".to_string())],
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
            mutable: false,
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
    let source = "import \"std/math\" as m :: { abs, max }; const PI = 3;";
    let ast = parse(source);
    assert_eq!(ast.len(), 2);
    assert_eq!(
        ast[0],
        Expr::Use {
            path: "std/math".to_string(),
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
            mutable: false,
            ty: Some(TrackType::Array(Box::new(TrackType::I32), 3)),
            value: Box::new(Expr::ArrayLiteral {
                elements: vec![
                    Expr::IntLiteral(1),
                    Expr::IntLiteral(2),
                    Expr::IntLiteral(3)
                ]
            })
        }
    );
    assert_eq!(
        ast[1],
        Expr::LetDef {
            name: "x".to_string(),
            mutable: false,
            ty: None,
            value: Box::new(Expr::ArrayIndex {
                target: Box::new(Expr::Variable("arr".to_string())),
                index: Box::new(Expr::IntLiteral(0))
            })
        }
    );
}

#[test]
fn test_parse_type_alias() {
    let source = "type ByteBuf = []u8;";
    let ast = parse(source);
    assert_eq!(
        ast,
        vec![Expr::TypeAlias {
            name: "ByteBuf".to_string(),
            target: TrackType::Slice(Box::new(TrackType::U8)),
        }]
    );
}

#[test]
fn test_parse_for_in_loop() {
    let source = "for i in 0..10 { print(i); }";
    let ast = parse(source);
    assert_eq!(ast.len(), 1);
    assert!(matches!(ast[0], Expr::ForIn { .. }));
}

#[test]
fn test_parse_use_statement() {
    let source = "use \"std/io\" as io::{print, read_line};";
    let ast = parse(source);
    assert_eq!(ast.len(), 1);
    assert!(matches!(ast[0], Expr::Use { .. }));
}

#[test]
fn test_parse_generic_fn_single_param() {
    let source = "fn identity<T>(x: T) -> T { return x; }";
    let ast = parse(source);
    assert_eq!(
        ast,
        vec![Expr::FnDef {
            name: "identity".to_string(),
            generics: vec!["T".to_string()],
            params: vec![("x".to_string(), TrackType::Custom("T".to_string()))],
            return_type: Some(TrackType::Custom("T".to_string())),
            body: vec![Expr::Return {
                value: Some(Box::new(Expr::Variable("x".to_string())))
            }]
        }]
    );
}

#[test]
fn test_parse_generic_fn_multi_params() {
    let source = "fn pair<T, U>(a: T, b: U) -> (T, U) { return (a, b); }";
    let ast = parse(source);
    assert_eq!(ast.len(), 1);
    if let Expr::FnDef {
        name,
        generics,
        params,
        return_type,
        ..
    } = &ast[0]
    {
        assert_eq!(name, "pair");
        assert_eq!(generics, &vec!["T".to_string(), "U".to_string()]);
        assert_eq!(params.len(), 2);
        assert!(matches!(return_type, Some(TrackType::Tuple(_))));
    } else {
        panic!("expected FnDef");
    }
}

#[test]
fn test_parse_generic_fn_no_return() {
    let source = "fn log<T>(x: T) { print(x); }";
    let ast = parse(source);
    assert!(matches!(&ast[0], Expr::FnDef { generics, .. } if generics == &vec!["T".to_string()]));
}
