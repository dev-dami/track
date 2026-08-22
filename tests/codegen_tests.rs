use std::env;
use std::fs;
use track::ast::*;
use track::codegen::CodeGen;
use track::compile_source;

#[test]
fn test_codegen_simple_function() {
    let mut cg = CodeGen::new("test_module");

    let program = vec![Expr::FnDef {
        name: "add".to_string(),
        generics: Vec::new(),
        params: vec![
            ("a".to_string(), TrackType::I32),
            ("b".to_string(), TrackType::I32),
        ],
        return_type: Some(TrackType::I32),
        body: vec![Expr::Return {
            value: Some(Box::new(Expr::BinaryOp {
                op: BinOp::Add,
                left: Box::new(Expr::Variable("a".to_string())),
                right: Box::new(Expr::Variable("b".to_string())),
            })),
        }],
    }];

    cg.compile_program(&program);

    let temp_dir = env::temp_dir().join(format!("track_cg_simple_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let obj_path = temp_dir.join("output.o");

    let res = cg.write_object_file(&obj_path);
    assert!(res.is_ok(), "Object file emission failed: {:?}", res.err());
    assert!(obj_path.exists());

    let bytes = fs::read(&obj_path).unwrap();
    assert!(!bytes.is_empty());
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_codegen_object_file_emission() {
    let temp_dir = env::temp_dir().join(format!("track_codegen_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let obj_path = temp_dir.join("output.o");

    let mut cg = CodeGen::new("test_module");

    let source = "fn main() -> void { print(42); }";
    let ast = compile_source(source).unwrap();
    cg.compile_program(&ast);

    let res = cg.write_object_file(&obj_path);
    assert!(res.is_ok(), "Object file emission failed: {:?}", res.err());
    assert!(obj_path.exists());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_codegen_string_literals_and_print() {
    let mut cg = CodeGen::new("test_module");

    let source = r#"
        fn main() -> void {
            print("Hello, Cranelift Codegen!");
        }
    "#;
    let ast = compile_source(source).unwrap();
    cg.compile_program(&ast);

    let temp_dir = env::temp_dir().join(format!("track_cg_str_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let obj_path = temp_dir.join("output.o");

    let res = cg.write_object_file(&obj_path);
    assert!(res.is_ok(), "Object file emission failed: {:?}", res.err());
    assert!(obj_path.exists());

    let _ = fs::remove_dir_all(&temp_dir);
}
