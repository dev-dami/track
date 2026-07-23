use std::env;
use std::fs;
use std::process::Command;

fn get_track_binary() -> String {
    let target_dir = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    let bin_path = format!("{}/debug/track", target_dir);
    if std::path::Path::new(&bin_path).exists() {
        return bin_path;
    }
    // Fallback to cargo run --bin track
    "cargo".to_string()
}

fn run_track_cmd(args: &[&str]) -> std::process::Output {
    let bin = get_track_binary();
    if bin == "cargo" {
        let mut full_args = vec!["run", "--bin", "track", "--"];
        full_args.extend_from_slice(args);
        Command::new("cargo")
            .args(&full_args)
            .output()
            .unwrap()
    } else {
        Command::new(bin)
            .args(args)
            .output()
            .unwrap()
    }
}

#[test]
fn test_cli_help() {
    let output = run_track_cmd(&["--help"]);
    assert!(output.status.success() || output.stderr.len() > 0);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("USAGE:") || combined.contains("Track"));
}

#[test]
fn test_cli_check_command() {
    let temp_dir = env::temp_dir().join(format!("track_cli_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let trk_file = temp_dir.join("test.trk");
    fs::write(&trk_file, "fn main() -> void { print(42); }").unwrap();

    let output = run_track_cmd(&["check", trk_file.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Check passed"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_invalid_command() {
    let output = run_track_cmd(&["nonexistent_command"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown command"));
}
