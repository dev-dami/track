use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use track::{build_file_in_dir, compile_source};

#[test]
fn test_valid_examples_compilation_and_execution() {
    let temp_dir = env::temp_dir().join(format!("track_test_examples_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let examples = vec![
        "hello.trk",
        "arithmetic.trk",
        "borrow.trk",
        "linear_auto_free.trk",
        "macro_test.trk",
        "union_enum_test.trk",
        "use_test.trk",
    ];

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    for example in examples {
        println!("Testing example: {}", example);
        let example_path = examples_dir.join(example);
        assert!(
            example_path.exists(),
            "Example file not found: {}",
            example_path.display()
        );

        let exe_path = build_file_in_dir(example_path.to_str().unwrap(), &temp_dir)
            .unwrap_or_else(|e| panic!("Failed to build {}: {}", example, e));

        assert!(
            exe_path.exists(),
            "Executable path does not exist: {}",
            exe_path.display()
        );

        let output = Command::new(&exe_path)
            .output()
            .unwrap_or_else(|e| panic!("Failed to run {}: {}", example, e));

        assert!(
            output.status.success(),
            "Execution of {} failed with code {:?}. Stderr:\n{}",
            example,
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_memory_boundary_exceeded_crash() {
    let source = r#"
        fn main() -> void {
            sys_set_memory_limit(1000);
            let buf = alloc(2000);
        }
    "#;
    let temp_dir = env::temp_dir().join(format!("track_mem_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let src_file = temp_dir.join("mem_test.trk");
    fs::write(&src_file, source).unwrap();

    let exe_path = build_file_in_dir(src_file.to_str().unwrap(), &temp_dir).unwrap();
    let output = Command::new(&exe_path).output().unwrap();

    assert!(!output.status.success(), "Memory boundary exceeded program should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Process memory boundary limit exceeded"),
        "Expected memory boundary message in stderr, got: {}",
        stderr
    );
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_invalid_examples_rejection() {
    let invalid_examples = vec![
        ("borrow_lock_err.trk", "frozen"),
        ("escape_err.trk", "escapes"),
        ("manual_free_err.trk", "managed automatically"),
    ];

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let examples_dir = Path::new(&manifest_dir).join("examples");

    for (example, expected_err) in invalid_examples {
        let example_path = examples_dir.join(example);
        let source = fs::read_to_string(&example_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", example, e));

        let res = compile_source(&source);
        assert!(
            res.is_err(),
            "Expected compilation error for {}, but it succeeded",
            example
        );

        let err_msg = res.unwrap_err();
        assert!(
            err_msg.contains(expected_err),
            "Expected error for {} to contain '{}', got:\n{}",
            example,
            expected_err,
            err_msg
        );
    }
}

#[test]
fn test_for_in_loop_execution() {
    let source = r#"
        import "std/io";
        fn main() -> void {
            for i in 0..3 {
                print(i);
            }
        }
    "#;
    let temp_dir = env::temp_dir().join(format!("track_for_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let src_file = temp_dir.join("for_test.trk");
    fs::write(&src_file, source).unwrap();

    let exe_path = build_file_in_dir(src_file.to_str().unwrap(), &temp_dir).unwrap();
    let output = Command::new(&exe_path).output().unwrap();

    assert!(output.status.success(), "For loop execution failed");
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_explicit_error_handling_convention() {
    // v0.5.0: (value, err) tuple returns, explicit propagation, abort(msg)
    let source = r#"
        const ERR_NOT_FOUND = 1;

        fn file_len(path: &ptr<u8>) -> (i64, i32) {
            let ok = file_exists(path);
            if (!ok) {
                return (-1, ERR_NOT_FOUND);
            }
            return (file_size(path), 0);
        }

        fn total(path: &ptr<u8>, fallback: i64) -> i64 {
            let (size, err) = file_len(path);
            if (err == ERR_NOT_FOUND) {
                print_err("missing input, using fallback");
                return fallback;
            }
            if (err != 0) {
                abort("fatal: unexpected error");
            }
            return size;
        }

        fn main() -> void {
            print(total("/etc/hostname", -7));
            print(total("/definitely/not/here", 7));
        }
    "#;
    let temp_dir = env::temp_dir().join(format!("track_err_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let src_file = temp_dir.join("error_handling.trk");
    fs::write(&src_file, source).unwrap();

    let exe_path = build_file_in_dir(src_file.to_str().unwrap(), &temp_dir).unwrap();
    let output = Command::new(&exe_path).output().unwrap();

    assert!(output.status.success(), "Error handling program failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() >= 2 && lines[0].trim() != "-1" && !lines[0].is_empty(),
        "Expected real size for existing file, got: {}",
        stdout
    );
    assert!(
        lines[1].trim() == "7",
        "Expected fallback value 7 for missing file, got: {}",
        stdout
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("missing input"),
        "Expected stderr diagnostic, got: {}",
        stderr
    );
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_logical_not_operator() {
    // Regression: unary ! was compiled as bitwise NOT (bnot),
    // making `if (!flag)` with flag == 1 take the wrong branch.
    let source = r#"
        fn main() -> void {
            let ok = file_exists("/etc/hostname");
            if (!ok) {
                print(999);
                return;
            }
            let missing = file_exists("/definitely/not/here");
            if (!missing) {
                print(42);
            }
        }
    "#;
    let temp_dir = env::temp_dir().join(format!("track_lnot_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let src_file = temp_dir.join("lnot.trk");
    fs::write(&src_file, source).unwrap();

    let exe_path = build_file_in_dir(src_file.to_str().unwrap(), &temp_dir).unwrap();
    let output = Command::new(&exe_path).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "42", "Logical NOT miscompiled; got: {}", stdout);
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_const_resolution_across_functions() {
    // Regression: top-level consts were invisible to non-main functions
    // and silently evaluated to 0 at codegen.
    let source = r#"
        const BASE = 100;
        const OFFSET = BASE + 23;

        fn get_offset() -> i64 {
            return OFFSET;
        }

        fn main() -> void {
            print(BASE);
            print(get_offset());
        }
    "#;
    let temp_dir = env::temp_dir().join(format!("track_const_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let src_file = temp_dir.join("consts.trk");
    fs::write(&src_file, source).unwrap();

    let exe_path = build_file_in_dir(src_file.to_str().unwrap(), &temp_dir).unwrap();
    let output = Command::new(&exe_path).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.first().copied().unwrap_or(""), "100", "got: {}", stdout);
    assert_eq!(lines.get(1).copied().unwrap_or(""), "123", "got: {}", stdout);
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_error_predicates_disambiguate_sentinels() {
    // str_is_int / env_exists disambiguate ambiguous sentinel returns
    let source = r#"
        fn main() -> void {
            if (str_is_int("00042")) {
                print(str_to_int("00042"));
            }
            if (!str_is_int("junk")) {
                print(111);
            }
            if (env_exists("PATH")) {
                print(222);
            }
            if (!env_exists("TRACK_DEFINITELY_NOT_SET_9x7")) {
                print(333);
            }
        }
    "#;
    let temp_dir = env::temp_dir().join(format!("track_pred_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let src_file = temp_dir.join("pred.trk");
    fs::write(&src_file, source).unwrap();

    let exe_path = build_file_in_dir(src_file.to_str().unwrap(), &temp_dir).unwrap();
    let output = Command::new(&exe_path).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.first().copied().unwrap_or(""), "42", "got: {}", stdout);
    assert!(lines.contains(&"111"), "got: {}", stdout);
    assert!(lines.contains(&"222"), "got: {}", stdout);
    assert!(lines.contains(&"333"), "got: {}", stdout);
    assert!(!lines.contains(&"999"), "got: {}", stdout);
    let _ = fs::remove_dir_all(&temp_dir);
}
