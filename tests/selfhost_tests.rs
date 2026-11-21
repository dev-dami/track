// Self-hosted compiler (compiler/) against the Rust bootstrap compiler.
//
// trackc is built once with `yard` (stage 1). The tests then check the
// bootstrap fixed point, that both compilers produce programs with the same
// observable behaviour, and that both checkers accept and reject the same
// programs.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("track_selfhost_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Stage 1: trackc built by the Rust compiler.
fn trackc() -> &'static Path {
    static TRACKC: OnceLock<PathBuf> = OnceLock::new();
    TRACKC.get_or_init(|| {
        let status = Command::new(env!("CARGO_BIN_EXE_yard"))
            .arg("build")
            .current_dir(repo().join("compiler"))
            .output()
            .expect("failed to run yard");
        assert!(
            status.status.success(),
            "yard build of compiler/ failed:\n{}{}",
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr)
        );
        let built = repo().join("compiler/target/trackc");
        let copy = scratch("stage1").join("trackc");
        fs::copy(&built, &copy).unwrap();
        copy
    })
}

fn run_trackc(args: &[&str], cwd: &Path) -> Output {
    Command::new(trackc())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run trackc")
}

#[test]
fn build_creates_missing_output_directories() {
    let dir = scratch("newdir");
    let src = dir.join("x.trk");
    fs::write(&src, "fn main() -> void { print(7); }").unwrap();
    let exe = dir.join("some/new/dir/x");
    let out = run_trackc(
        &["build", src.to_str().unwrap(), "-o", exe.to_str().unwrap()],
        &dir,
    );
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(run_exe(&exe).0.trim(), "7");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_object_is_cached_and_builds_stay_identical() {
    let dir = scratch("rtcache");
    let cache = dir.join("cache");
    let a = dir.join("a.trk");
    let b = dir.join("b.trk");
    fs::write(&a, "fn main() -> void { print(1); }").unwrap();
    fs::write(&b, "fn main() -> void { print(1); }").unwrap();
    let build = |src: &Path, exe: &Path| {
        let out = Command::new(trackc())
            .args(["build", src.to_str().unwrap(), "-o", exe.to_str().unwrap()])
            .env("TRACKC_CACHE", &cache)
            .current_dir(&dir)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    };
    let objects = || {
        fs::read_dir(&cache)
            .map(|d| {
                d.filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "o"))
                    .count()
            })
            .unwrap_or(0)
    };
    build(&a, &dir.join("out1/a"));
    assert_eq!(objects(), 1, "first build should store one runtime object");
    build(&a, &dir.join("out2/a"));
    assert_eq!(objects(), 1, "second build should reuse it");
    assert!(fs::read(dir.join("out1/a")).unwrap() == fs::read(dir.join("out2/a")).unwrap());
    build(&b, &dir.join("out3/b"));
    assert_eq!(objects(), 1);
    assert_eq!(run_exe(&dir.join("out3/b")).0.trim(), "1");
    let _ = fs::remove_dir_all(&dir);
}

fn run_exe(exe: &Path) -> (String, Option<i32>) {
    let out = Command::new(exe).output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        out.status.code(),
    )
}

#[test]
fn bootstrap_reaches_a_fixed_point() {
    let dir = scratch("bootstrap");
    for stage in ["stage2", "stage3"] {
        fs::create_dir_all(dir.join(stage)).unwrap();
    }
    let stage2 = dir.join("stage2/trackc");
    let stage3 = dir.join("stage3/trackc");
    let out = run_trackc(
        &[
            "build",
            "compiler/src/main.trk",
            "-o",
            stage2.to_str().unwrap(),
        ],
        &repo(),
    );
    assert!(
        out.status.success(),
        "stage1 failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = Command::new(&stage2)
        .args([
            "build",
            "compiler/src/main.trk",
            "-o",
            stage3.to_str().unwrap(),
        ])
        .current_dir(repo())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stage2 failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let c2 = fs::read(dir.join("stage2/trackc.c")).unwrap();
    let c3 = fs::read(dir.join("stage3/trackc.c")).unwrap();
    assert!(c2 == c3, "stage1 and stage2 emitted different C");
    let b2 = fs::read(&stage2).unwrap();
    let b3 = fs::read(&stage3).unwrap();
    assert!(b2 == b3, "stage2 and stage3 executables differ");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn examples_behave_the_same_under_both_compilers() {
    let dir = scratch("examples");
    let mut examples: Vec<PathBuf> = fs::read_dir(repo().join("examples"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "trk"))
        .collect();
    examples.sort();
    assert!(!examples.is_empty());
    for example in examples {
        let stem = example.file_stem().unwrap().to_string_lossy().to_string();
        let host = track::build_file_in_dir(example.to_str().unwrap(), &dir);
        let native_exe = dir.join(format!("{}_native", stem));
        let native = run_trackc(
            &[
                "build",
                example.to_str().unwrap(),
                "-o",
                native_exe.to_str().unwrap(),
            ],
            &dir,
        );
        if stem.ends_with("_err") {
            assert!(
                host.is_err(),
                "{}: host compiler accepted a rejection test",
                stem
            );
            assert!(
                !native.status.success(),
                "{}: trackc accepted a rejection test",
                stem
            );
            continue;
        }
        let host_exe = host.unwrap_or_else(|e| panic!("{}: host build failed: {}", stem, e));
        assert!(
            native.status.success(),
            "{}: trackc build failed: {}",
            stem,
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(
            run_exe(&host_exe),
            run_exe(&native_exe),
            "{}: behaviour differs",
            stem
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// (name, source, accepted). Both checkers must agree with the expectation.
const VERDICTS: &[(&str, &str, bool)] = &[
    (
        "move_then_use",
        "fn main() -> void { let s = str_concat(\"a\", \"b\"); let t = s; print(s); }",
        false,
    ),
    (
        "copy_ints",
        "fn main() -> void { let a: i64 = 1; let b = a; print(a); print(b); }",
        true,
    ),
    (
        "borrow_does_not_move",
        "fn f(x: &Str) -> i64 { return 0; } fn main() -> void { let s = str_concat(\"a\", \"b\"); f(&s); print(s); }",
        true,
    ),
    (
        "frozen_by_reference",
        "fn main() -> void { let s = str_concat(\"a\", \"b\"); let r = &s; let t = s; }",
        false,
    ),
    (
        "lens_locks_target",
        "fn main() -> void { let mut v = vec_init(4); with v -> l { let w = v; } }",
        false,
    ),
    (
        "lens_cannot_escape",
        "fn main() -> void { let mut v = vec_init(4); with v -> l { let w = l; } }",
        false,
    ),
    (
        "inconsistent_if",
        "fn eat(s: Str) -> void { } fn main() -> void { let s = str_concat(\"a\", \"b\"); if true { eat(s); } else { } }",
        false,
    ),
    (
        "consistent_if",
        "fn eat(s: Str) -> void { } fn main() -> void { let s = str_concat(\"a\", \"b\"); if true { eat(s); } else { eat(s); } }",
        true,
    ),
    (
        "consume_in_loop",
        "fn eat(s: Str) -> void { } fn main() -> void { let s = str_concat(\"a\", \"b\"); let mut i: i64 = 0; while i < 3 { eat(s); i = i + 1; } }",
        false,
    ),
    (
        "return_local_ref",
        "fn f() -> &i64 { let x = 5; return &x; }",
        false,
    ),
    (
        "return_param_ref",
        "fn f(x: &i64) -> &i64 { return x; }",
        true,
    ),
    (
        "immutable_assign",
        "fn main() -> void { let x = 1; x = 2; }",
        false,
    ),
    (
        "mutable_assign",
        "fn main() -> void { let mut x = 1; x = 2; print(x); }",
        true,
    ),
    (
        "mut_does_not_leak_across_functions",
        "fn a() -> void { let mut x = 1; x = 2; } fn b() -> void { let x = 1; x = 3; }",
        false,
    ),
    (
        "shadowed_binding_is_immutable",
        "fn main() -> void { let mut x = 1; let x = 2; x = 3; }",
        false,
    ),
    ("assign_undeclared", "fn main() -> void { y = 3; }", false),
    (
        "undeclared_use",
        "fn main() -> void { print(nope); }",
        false,
    ),
    (
        "forbidden_free",
        "fn main() -> void { let s = str_concat(\"a\", \"b\"); str_free(s); }",
        false,
    ),
    (
        "typo_suggestion",
        "fn compute() -> i64 { return 1; } fn main() -> void { print(computee()); }",
        false,
    ),
    (
        "tuple_index_bounds",
        "fn main() -> void { let t = (1, 2); print(t.2); }",
        false,
    ),
    (
        "const_before_use",
        "fn main() -> void { print(LIMIT); } const LIMIT = 10;",
        true,
    ),
    (
        "compile_error_macro",
        "fn main() -> void { @compile_error(\"stop\"); }",
        false,
    ),
    (
        "struct_field_update",
        "struct P { x: i64 } fn main() -> void { let mut p = P { x: 1 }; p.x = 2; print(p.x); }",
        true,
    ),
    (
        "immutable_field_update",
        "struct P { x: i64 } fn main() -> void { let p = P { x: 1 }; p.x = 2; }",
        false,
    ),
    (
        "lens_field_update",
        "struct P { x: i64 } fn main() -> void { let mut p = P { x: 1 }; with p -> l { l.x = 5; } print(p.x); }",
        true,
    ),
    (
        "unknown_struct_field",
        "struct P { x: i64 } fn main() -> void { let p = P { z: 1 }; }",
        false,
    ),
    (
        "unknown_field_access",
        "struct P { x: i64 } fn main() -> void { let p = P { x: 1 }; print(p.q); }",
        false,
    ),
    (
        "array_moved_by_value",
        "fn main() -> void { let a = [1, 2]; let n = array_len(a); print(a[0]); }",
        false,
    ),
    (
        "array_index_update",
        "fn main() -> void { let mut a = [1, 2]; a[1] = 7; print(a[1]); }",
        true,
    ),
    // Branch merges only revisit slots a branch wrote (undo-log checker).
    (
        "shadow_in_then_keeps_outer",
        "fn eat(s: Str) -> void { } fn main() -> void { let s = str_concat(\"a\", \"b\"); if true { let s = 1; } else { eat(s); } }",
        false,
    ),
    (
        "if_inside_loop_consumes",
        "fn eat(s: Str) -> void { } fn main() -> void { let s = str_concat(\"a\", \"b\"); let mut i: i64 = 0; while i < 3 { if i == 1 { eat(s); } else { eat(s); } i = i + 1; } }",
        false,
    ),
    (
        "nested_if_second_resource",
        "fn eat(s: Str) -> void { } fn main() -> void { let a = str_concat(\"a\", \"b\"); let b = str_concat(\"a\", \"b\"); if true { eat(b); eat(a); } else { if false { eat(a); } else { eat(a); } } }",
        false,
    ),
    (
        "global_const_spent_in_branch",
        "const K = \"hi\"; fn main() -> void { if true { let a = K; } else { } }",
        false,
    ),
    (
        "reassign_after_move_in_loop",
        "fn eat(s: Str) -> void { } fn main() -> void { let mut s = str_concat(\"a\", \"b\"); let mut i: i64 = 0; while i < 2 { eat(s); s = str_concat(\"x\", \"y\"); i = i + 1; } }",
        true,
    ),
];

#[test]
fn checkers_agree_on_accepted_and_rejected_programs() {
    let dir = scratch("verdicts");
    for (name, source, accepted) in VERDICTS {
        let host = track::compile_source(source);
        assert_eq!(
            host.is_ok(),
            *accepted,
            "{}: host verdict: {:?}",
            name,
            host.err()
        );
        let file = dir.join(format!("{}.trk", name));
        fs::write(&file, source).unwrap();
        let native = run_trackc(&["check", file.to_str().unwrap()], &dir);
        assert_eq!(
            native.status.success(),
            *accepted,
            "{}: trackc verdict: {}",
            name,
            String::from_utf8_lossy(&native.stderr)
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn diagnostics_point_at_the_source() {
    let dir = scratch("diagnostics");
    let file = dir.join("moved.trk");
    fs::write(
        &file,
        "fn main() -> void {\n    let s = str_concat(\"a\", \"b\");\n    let t = s;\n    print(s);\n}\n",
    )
    .unwrap();
    let out = run_trackc(&["check", "moved.trk"], &dir);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.starts_with(
            "error[E0200]: Use-after-free! Resource 's' is already spent.\n --> moved.trk:4:11\n"
        ),
        "unexpected diagnostic:\n{}",
        stderr
    );
    assert!(
        stderr.contains("4 |     print(s);"),
        "missing source frame:\n{}",
        stderr
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn native_test_suite_passes_under_trackc() {
    let compiler = repo().join("compiler");
    let mut tests: Vec<PathBuf> = fs::read_dir(compiler.join("tests"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "trk"))
        .collect();
    tests.sort();
    for test in tests {
        let rel = test
            .strip_prefix(&compiler)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let out = run_trackc(&["run", &rel], &compiler);
        assert!(
            out.status.success(),
            "{} failed under trackc:\n{}{}",
            rel,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// Build `src` with trackc once per runtime and return both runs.
fn run_under_both_runtimes(
    dir: &Path,
    stem: &str,
    src: &Path,
) -> ((String, Option<i32>), (String, Option<i32>)) {
    let c_exe = dir.join(format!("{}_crt", stem));
    let t_exe = dir.join(format!("{}_trt", stem));
    let out = run_trackc(
        &["build", src.to_str().unwrap(), "-o", c_exe.to_str().unwrap()],
        dir,
    );
    assert!(
        out.status.success(),
        "{}: build failed: {}",
        stem,
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run_trackc(
        &[
            "build",
            "--rt=track",
            src.to_str().unwrap(),
            "-o",
            t_exe.to_str().unwrap(),
        ],
        dir,
    );
    assert!(
        out.status.success(),
        "{}: --rt=track build failed: {}{}",
        stem,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (run_exe(&c_exe), run_exe(&t_exe))
}

/// The Track runtime (compiler/rt/*.trk) links without libc and behaves
/// like runtime/track_runtime.c on the examples and the runtime smoke test.
#[test]
fn track_runtime_matches_the_c_runtime() {
    let dir = scratch("trackrt");
    let mut programs: Vec<PathBuf> = fs::read_dir(repo().join("examples"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "trk"))
        .filter(|p| {
            let s = p.file_stem().unwrap().to_string_lossy().to_string();
            !s.ends_with("_err") && s != "net_demo"
        })
        .collect();
    programs.push(repo().join("compiler/rt/tests/rt_smoke.trk"));
    programs.sort();
    for program in programs {
        let stem = program.file_stem().unwrap().to_string_lossy().to_string();
        let (c, t) = run_under_both_runtimes(&dir, &stem, &program);
        assert_eq!(c, t, "{}: behaviour differs between runtimes", stem);
    }
    let _ = fs::remove_dir_all(&dir);
}

/// trackc linked against the Track runtime (no libc) compiles the compiler
/// to the same C as trackc on the C runtime.
#[test]
fn trackc_on_the_track_runtime_self_compiles() {
    let dir = scratch("trackrt_self");
    for stage in ["rt", "a", "b"] {
        fs::create_dir_all(dir.join(stage)).unwrap();
    }
    let rt_trackc = dir.join("rt/trackc");
    let out = run_trackc(
        &[
            "build",
            "--rt=track",
            "compiler/src/main.trk",
            "-o",
            rt_trackc.to_str().unwrap(),
        ],
        &repo(),
    );
    assert!(
        out.status.success(),
        "--rt=track build of trackc failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let elf = fs::read(&rt_trackc).unwrap();
    assert!(
        !elf.windows(7).any(|w| w == b"libc.so"),
        "trackc on the Track runtime must not link libc"
    );
    let a_exe = dir.join("a/trackc");
    let out = run_trackc(
        &["build", "compiler/src/main.trk", "-o", a_exe.to_str().unwrap()],
        &repo(),
    );
    assert!(out.status.success());
    let b_exe = dir.join("b/trackc");
    let out = Command::new(&rt_trackc)
        .args(["build", "compiler/src/main.trk", "-o", b_exe.to_str().unwrap()])
        .current_dir(repo())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "trackc on the Track runtime failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let a = fs::read(dir.join("a/trackc.c")).unwrap();
    let b = fs::read(dir.join("b/trackc.c")).unwrap();
    assert!(a == b, "trackc on the Track runtime emitted different C");
    let _ = fs::remove_dir_all(&dir);
}
