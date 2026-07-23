use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

use track::{build_file, compile_source, yard};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "build" => {
            let file = require_file_arg(&args, 2);
            cmd_build(&file);
        }
        "run" => {
            let file = require_file_arg(&args, 2);
            cmd_run(&file);
        }
        "check" => {
            let file = require_file_arg(&args, 2);
            cmd_check(&file);
        }
        "yard" => {
            let yard_args: Vec<String> = args[2..].to_vec();
            match yard::run(&yard_args) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("yard error: {}", e);
                    process::exit(1);
                }
            }
        }
        "--help" | "-h" | "help" => {
            print_usage();
        }
        // Legacy: if argument ends with .trk, treat as `build`
        arg if arg.ends_with(".trk") => {
            cmd_build(arg);
        }
        other => {
            eprintln!("Unknown command: '{}'\n", other);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Track — Systems programming with linear types\n");
    eprintln!("USAGE:");
    eprintln!("    track build <file.trk>    Compile to native executable");
    eprintln!("    track run <file.trk>      Compile and run");
    eprintln!("    track check <file.trk>    Type-check only (no codegen)");
    eprintln!("    track yard <command>      Package manager");
    eprintln!("    track <file.trk>          Same as 'track build'");
    eprintln!("    track help                Show this help");
}

fn require_file_arg(args: &[String], idx: usize) -> String {
    args.get(idx).cloned().unwrap_or_else(|| {
        eprintln!("Error: expected a .trk file path");
        process::exit(1);
    })
}

// ── commands ─────────────────────────────────────────────────────────

fn cmd_build(filename: &str) -> PathBuf {
    match build_file(filename) {
        Ok(exe) => {
            println!("✓ Built: {}", exe.display());
            exe
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

fn cmd_run(filename: &str) {
    let exe = cmd_build(filename);
    let exe_path = if exe.is_absolute() {
        exe
    } else {
        env::current_dir().unwrap().join(exe)
    };

    let status = process::Command::new(&exe_path)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Failed to run '{}': {}", exe_path.display(), e);
            process::exit(1);
        });

    process::exit(status.code().unwrap_or(1));
}

fn cmd_check(filename: &str) {
    let source = fs::read_to_string(filename).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {}", filename, e);
        process::exit(1);
    });

    match compile_source(&source) {
        Ok(_) => println!("✓ Check passed: {}", filename),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
