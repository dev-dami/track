use track::compile_source;

#[test]
fn test_soundness_loop_consumed_variable_use_rejected() {
    let source = r#"
        fn main() -> void {
            let mut v = vec_init(16);
            while true {
                let x = v;
            }
            let y = v;
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_err(),
        "Expected rejection for use of loop-consumed variable after loop"
    );
    let err = res.unwrap_err();
    assert!(
        err.contains("v"),
        "Error message should mention variable 'v': {}",
        err
    );
}

#[test]
fn test_soundness_loop_reinitialization_accepted() {
    let source = r#"
        fn main() -> void {
            let mut v = vec_init(16);
            while true {
                let tmp = v;
                v = vec_init(16);
            }
            let end = v;
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_ok(),
        "Expected valid compilation for loop reinitialization, got: {:?}",
        res.err()
    );
}

#[test]
fn test_soundness_lens_escape_rejected() {
    let source = r#"
        struct User { age: i32 }
        fn main() -> void {
            let mut u = User { age: 30 };
            with u -> user {
                let leaked = user;
            }
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_err(), "Expected rejection for lens escape attempt");
}

#[test]
fn test_soundness_inconsistent_branch_move_rejected() {
    let source = r#"
        fn main() -> void {
            let v = vec_init(16);
            let cond = true;
            if cond {
                let x = v;
            } else {
                let y = 10;
            }
            let z = v;
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_err(),
        "Expected rejection for inconsistent ownership branch move"
    );
    let err = res.unwrap_err();
    assert!(
        err.contains("inconsistent state"),
        "Error message should mention inconsistent state: {}",
        err
    );
}

#[test]
fn test_soundness_double_move_rejected() {
    let source = r#"
        fn main() -> void {
            let v = vec_init(16);
            let x = v;
            let y = v;
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_err(), "Expected rejection for double move");
}
