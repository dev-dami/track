use track::compile_source;

#[test]
fn test_regression_nested_expressions() {
    let source = r#"
        fn main() -> void {
            let x: i64 = ((10 + 20) * (30 - 5)) / (2 % 1 + 5);
            print(x);
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_ok(),
        "Failed to compile nested expression: {:?}",
        res.err()
    );
}

#[test]
fn test_regression_nested_if_else_chain() {
    let source = r#"
        fn classify(n: i32) -> i32 {
            if n < 0 {
                return -1;
            } else if n == 0 {
                return 0;
            } else if n < 10 {
                return 1;
            } else {
                return 2;
            }
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_ok(),
        "Failed to compile nested if-else: {:?}",
        res.err()
    );
}

#[test]
fn test_regression_multiple_macro_calls() {
    let source = r#"
        @macro add_one(x: i32) -> i32 {
            return x + 1;
        }

        fn main() -> void {
            let a = @add_one(10);
            let b = @add_one(@add_one(20));
            print(a + b);
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_ok(), "Failed macro regression test: {:?}", res.err());
}

#[test]
fn test_regression_complex_match_guards() {
    let source = r#"
        union Number {
            Int(i32),
            Big(i64),
            Zero,
        }

        fn check_num(n: Number) -> i32 {
            match n {
                Number::Int(val) if val > 100 => print(val),
                Number::Int(val) => print(val),
                Number::Big(val) => print(val),
                Number::Zero => print(0),
                _ => print(-1),
            }
            return 0;
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_ok(),
        "Failed match guards regression test: {:?}",
        res.err()
    );
}

#[test]
fn test_regression_nested_lens_blocks() {
    let source = r#"
        fn main() -> void {
            let mut outer = Outer { inner: Inner { value: 42 } };
            with outer -> o {
                with o -> i {
                    let v = i.value();
                }
            }
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_ok(),
        "Failed nested lens regression test: {:?}",
        res.err()
    );
}

#[test]
fn test_regression_shadowed_variables() {
    let source = r#"
        fn main() -> void {
            let x = 10;
            if true {
                let x = 20;
                print(x);
            }
            print(x);
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_ok(),
        "Failed scope shadowing regression test: {:?}",
        res.err()
    );
}
