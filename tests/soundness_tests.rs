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
        fn main() -> void {
            let mut v = vec_init(16);
            with v -> lens {
                let leaked = lens;
            }
        }
    "#;
    let res = compile_source(source);
    assert!(res.is_err(), "Expected rejection for lens escape attempt");
    let err = res.unwrap_err();
    assert!(
        err.contains("Lens") && err.contains("escape"),
        "Error message should identify lens escape: {}",
        err
    );
}

#[test]
fn test_soundness_lens_alias_not_visible_after_block() {
    let source = r#"
        fn main() -> void {
            let mut v = vec_init(16);
            with v -> lens {
                vec_push(lens, 1);
            }
            vec_push(lens, 2);
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_err(),
        "Expected rejection when lens alias is used after its with block"
    );
}

#[test]
fn test_soundness_nested_lens_same_target_rejected() {
    let source = r#"
        fn main() -> void {
            let mut v = vec_init(16);
            with v -> a {
                with v -> b {
                    vec_push(b, 1);
                }
            }
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_err(),
        "Expected rejection for nested lens over already locked target"
    );
}

#[test]
fn test_soundness_lens_released_on_early_return() {
    let source = r#"
        fn f(cond: bool) -> void {
            let mut v = vec_init(16);
            with v -> lens {
                if cond {
                    return;
                } else {
                    vec_push(lens, 1);
                }
            }
            let x = v;
        }
    "#;
    let res = compile_source(source);
    assert!(
        res.is_ok(),
        "Expected checker to release lens after early-return branch, got: {:?}",
        res.err()
    );
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
