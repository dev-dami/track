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
