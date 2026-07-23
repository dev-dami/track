use track::compile_source;

#[test]
fn test_linear_checker_valid_code() {
    let source = r#"
        fn main() -> void {
            let x: i32 = 42;
            let y = x; // Copy type i32
            print(x);
            print(y);
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_ok());
}

#[test]
fn test_linear_checker_linear_type_transfer() {
    let source = r#"
        fn process(v: Vec) -> void {
            // v is consumed here
        }

        fn main() -> void {
            let v: Vec = vec_init(10);
            process(v);
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_ok());
}

#[test]
fn test_linear_checker_use_after_free_error() {
    let source = r#"
        fn consume(v: Vec) -> void {}

        fn main() -> void {
            let v: Vec = vec_init(10);
            consume(v);
            consume(v); // Use after free
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.contains("Use-after-free"));
}

#[test]
fn test_linear_checker_lens_block_valid_and_locked() {
    let source = r#"
        fn main() -> void {
            let mut u = User { age: 30 };
            with u -> user {
                let a = user.age();
            }
            print(u);
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_ok(),
        "Expected lens block valid test to compile, got error: {:?}",
        res.err()
    );
}

#[test]
fn test_linear_checker_lens_block_locked_error() {
    let source = r#"
        fn use_user(u: User) -> void {}

        fn main() -> void {
            let mut u = User { age: 30 };
            with u -> user {
                use_user(u); // Error: u is frozen while in lens block
            }
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.contains("frozen")
            || err.contains("locked")
            || err.contains("Active")
            || err.contains("Locked"),
        "Unexpected error message: {}",
        err
    );
}

#[test]
fn test_linear_checker_borrow_and_escape() {
    let source = r#"
        fn main() -> void {
            let mut v: Vec = vec_init(10);
            let ref_v = &mut v;
            vec_push(ref_v, 1);
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_ok());
}

#[test]
fn test_linear_checker_escape_error() {
    let source = r#"
        fn escape() -> &Vec {
            let v: Vec = vec_init(10);
            return &v; // Error: reference outlives stack frame
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.contains("outlives") || err.contains("Cannot return reference"));
}

#[test]
fn test_linear_checker_enum_and_union_matching() {
    let source = r#"
        enum Color { Red, Green, Blue }
        union Result { Ok(i32), Err }

        fn main() -> void {
            let c = Color::Red;
            match c {
                Color::Red => print(1),
                Color::Green => print(2),
                Color::Blue => print(3),
            }

            let r = Result::Ok(42);
            match r {
                Result::Ok(val) => print(val),
                Result::Err => print(0),
            }
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_ok());
}
