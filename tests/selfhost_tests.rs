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
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
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
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    let objects = || {
        fs::read_dir(&cache)
            .map(|d| {
                d.filter(|e| {
                    e.as_ref()
                        .unwrap()
                        .path()
                        .extension()
                        .is_some_and(|x| x == "o")
                })
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
        &[
            "build",
            src.to_str().unwrap(),
            "-o",
            c_exe.to_str().unwrap(),
        ],
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
    for name in ["rt_alloc", "rt_memlimit", "rt_memlimit_realloc"] {
        programs.push(repo().join(format!("tests/native/{}.trk", name)));
    }
    programs.sort();
    for program in programs {
        let stem = program.file_stem().unwrap().to_string_lossy().to_string();
        let (c, t) = run_under_both_runtimes(&dir, &stem, &program);
        assert_eq!(c, t, "{}: behaviour differs between runtimes", stem);
    }
    let _ = fs::remove_dir_all(&dir);
}

/// A program without `main` runs its top-level statements under both
/// runtimes (the Track runtime's rt_start calls the synthetic main).
#[test]
fn program_without_main_runs_on_both_runtimes() {
    let dir = scratch("trackrt_nomain");
    let src = dir.join("nomain.trk");
    fs::write(&src, "let x = 41;\nprint(x + 1);\n").unwrap();
    let (c, t) = run_under_both_runtimes(&dir, "nomain", &src);
    assert_eq!(c, ("42\n".to_string(), Some(0)));
    assert_eq!(c, t, "behaviour differs between runtimes");
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
        &[
            "build",
            "compiler/src/main.trk",
            "-o",
            a_exe.to_str().unwrap(),
        ],
        &repo(),
    );
    assert!(out.status.success());
    let b_exe = dir.join("b/trackc");
    let out = Command::new(&rt_trackc)
        .args([
            "build",
            "compiler/src/main.trk",
            "-o",
            b_exe.to_str().unwrap(),
        ])
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

// L2 warnings channel, driven by real lifecycle warnings (L6): two unused
// bindings, so the output also shows that diagnostics are sorted by position.
#[test]
fn warnings_print_sorted_and_only_deny_warnings_fails() {
    let dir = scratch("warnings");
    let src = dir.join("w.trk");
    fs::write(
        &src,
        "fn helper() -> i64 { let a = 1; return 2; }\nfn main() -> void { let b = 3; print(helper()); }\n",
    )
    .unwrap();
    let path = src.to_str().unwrap();
    let at = |stderr: &str, needle: &str| stderr.find(needle).unwrap_or(usize::MAX);

    for args in [
        vec!["check", path],
        vec!["build", path],
        vec!["build", "--native", path],
    ] {
        let out = run_trackc(&args, &dir);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{:?}: {}", args, stderr);
        assert_eq!(
            stderr.matches("warning[W0301]").count(),
            2,
            "{:?}: {}",
            args,
            stderr
        );
        assert!(
            at(&stderr, "w.trk:1:") < at(&stderr, "w.trk:2:"),
            "{:?}: warnings not sorted by position:\n{}",
            args,
            stderr
        );
    }
    let out = run_trackc(&["check", path], &dir);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("(2 warnings)"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );

    let out = run_trackc(&["run", path], &dir);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "2");
    assert!(String::from_utf8_lossy(&out.stderr).contains("warning[W0301]"));

    // --deny-warnings in any position turns warnings into a failure.
    for args in [
        vec!["check", "--deny-warnings", path],
        vec!["check", path, "--deny-warnings"],
        vec!["build", "--native", path, "--deny-warnings"],
        vec!["run", "--deny-warnings", path],
    ] {
        let out = run_trackc(&args, &dir);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{:?}: {}", args, stderr);
        assert!(stderr.contains("warning[W0301]"), "{:?}: {}", args, stderr);
    }

    // `_`-prefixed names are silent, and --deny-warnings then passes.
    let quiet = dir.join("quiet.trk");
    fs::write(
        &quiet,
        "fn helper() -> i64 { let _a = 1; return 2; }\nfn main() -> void { print(helper()); }\n",
    )
    .unwrap();
    let out = run_trackc(&["check", "--deny-warnings", quiet.to_str().unwrap()], &dir);
    assert!(out.status.success());
    assert!(!String::from_utf8_lossy(&out.stderr).contains("warning"));

    // A pending warning and an error print together, sorted by position.
    let bad = dir.join("bad.trk");
    fs::write(
        &bad,
        "fn first() -> i64 { let u = 1; return 0; }\nfn main() -> void {\n    print(nope);\n}\n",
    )
    .unwrap();
    let out = run_trackc(&["check", bad.to_str().unwrap()], &dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{}", stderr);
    let _ = fs::remove_dir_all(&dir);
}

/// Lifecycle warnings W0301-W0305 (card L6, docs/lifecycles.md §2): each
/// docs/lifecycles/wN example gives exactly its warning, with the §2 text,
/// exits 0, and fails under --deny-warnings.
#[test]
fn lifecycle_warnings_match_the_catalogue() {
    let root = repo();
    let cases = [
        (
            "w1_fresh_unread",
            "warning[W0301]: 'origin' is created and never read; remove it or name it '_origin'.",
        ),
        (
            "w2_write_only",
            "warning[W0302]: write to 'p.y' is never read; 'p' ends at line 7.",
        ),
        (
            "w3_mutated_unread",
            "warning[W0303]: 'bump' mutates 'c', but 'c' is never read afterwards.",
        ),
        (
            "w4_discarded_result",
            "warning[W0304]: result of 'vadd' is discarded, and 'vadd' has no other effect (it only reads its parameters and returns a fresh value).",
        ),
        (
            "w5_consume_only_reads",
            "warning[W0305]: 'norm1' takes 'p' by value but only reads it; take '&Point' so callers keep it.",
        ),
    ];
    for (stem, want) in cases {
        let rel = format!("docs/lifecycles/{}.trk", stem);
        let out = run_trackc(&["check", &rel], &root);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{}: {}", rel, stderr);
        assert!(
            stderr.contains(want),
            "{}: want {:?}, got:\n{}",
            rel,
            want,
            stderr
        );
        assert_eq!(stderr.matches("warning[").count(), 1, "{}: {}", rel, stderr);
        let out = run_trackc(&["check", "--deny-warnings", &rel], &root);
        assert_eq!(out.status.code(), Some(1), "{}", rel);
    }
    // Near misses stay warning-free.
    let dir = scratch("lifewarn");
    let clean = [
        // read after the write
        "struct P { x: i64, y: i64 }\nfn main() -> void { let mut p = P { x: 1, y: 2 }; p.y = 9; print(p.y); }\n",
        // mutated, then read
        "struct C { n: i64 }\nfn bump(c: &C) -> i64 { with c -> l { l.n = l.n + 1; } return 0; }\nfn main() -> void { let c = C { n: 0 }; bump(&c); print(c.n); }\n",
        // pure result used
        "struct V { x: i64 }\nfn mk(a: &V) -> V { return V { x: a.x }; }\nfn main() -> void { let a = V { x: 1 }; let b = mk(&a); print(b.x); }\n",
        // by-value parameter handed back
        "struct P { x: i64 }\nfn get(p: &P) -> i64 { return p.x; }\nfn eat(p: P) -> P { return p; }\nfn main() -> void { let p = P { x: 1 }; print(get(&p)); let q = eat(p); print(q.x); }\n",
        // a write read on the next loop iteration
        "struct P { x: i64 }\nfn main() -> void { let mut p = P { x: 0 }; let mut i = 0; while i < 3 { print(p.x); p.x = i; i = i + 1; } }\n",
        // a discarded call with an effect
        "struct V { x: i64 }\nfn show(a: &V) -> V { print(a.x); return V { x: a.x }; }\nfn main() -> void { let a = V { x: 1 }; show(&a); }\n",
    ];
    for (i, src) in clean.iter().enumerate() {
        let f = dir.join(format!("clean{}.trk", i));
        fs::write(&f, src).unwrap();
        let out = run_trackc(&["check", f.to_str().unwrap()], &dir);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "clean{}: {}", i, stderr);
        assert!(!stderr.contains("warning["), "clean{}: {}", i, stderr);
    }
    let _ = fs::remove_dir_all(&dir);
}

/// examples/ is warning-free except two intended demonstrations: an unused
/// allocation that is auto-freed, and a struct pattern match on a by-value
/// parameter.
#[test]
fn examples_warn_only_where_intended() {
    let root = repo();
    let intended = [
        ("linear_auto_free.trk", "warning[W0301]: 'buf'"),
        ("structs_arrays.trk", "warning[W0305]: 'quadrant'"),
    ];
    let mut files: Vec<PathBuf> = fs::read_dir(root.join("examples"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "trk"))
        .collect();
    files.sort();
    for file in files {
        let name = file.file_name().unwrap().to_str().unwrap().to_string();
        let rel = format!("examples/{}", name);
        let out = run_trackc(&["check", &rel], &root);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let want: Vec<&str> = intended
            .iter()
            .filter(|(f, _)| *f == name)
            .map(|(_, w)| *w)
            .collect();
        assert_eq!(
            stderr.matches("warning[").count(),
            want.len(),
            "{}: {}",
            rel,
            stderr
        );
        for w in want {
            assert!(stderr.contains(w), "{}: want {:?}: {}", rel, w, stderr);
        }
    }
}

/// The compiler's own sources stay warning-clean (CI runs the same check).
#[test]
fn compiler_sources_are_warning_clean() {
    let out = run_trackc(
        &["check", "--deny-warnings", "compiler/src/main.trk"],
        &repo(),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// (name, source, accepted) for trackc alone: holes the review found where
/// the Rust checker is also wrong (or differs) and trackc must be right.
const TRACKC_VERDICTS: &[(&str, &str, bool)] = &[
    (
        "generic_param_is_linear",
        "fn dup<T>(x: T) -> (T, T) { return (x, x); } fn main() -> void { let t = dup(str_concat(\"a\", \"b\")); print(t.0); }",
        false,
    ),
    (
        "generic_param_used_once",
        "fn id<T>(x: T) -> T { return x; } fn main() -> void { let s = id(str_concat(\"a\", \"b\")); print(s); print(id(3)); }",
        true,
    ),
    // Generics are checked per instantiation: T is Copy when bound to i64.
    (
        "generic_dup_of_copy_type",
        "fn dup<T>(x: T) -> (T, T) { return (x, x); } fn main() -> void { let t = dup(3); print(t.0); }",
        true,
    ),
    (
        "generic_twice_of_copy_type",
        "fn twice<T>(x: T) -> T { let a = x; let b = x; return b; } fn main() -> void { let n: i64 = 4; print(twice(n)); }",
        true,
    ),
    (
        "generic_twice_of_linear_type",
        "fn twice<T>(x: T) -> T { let a = x; let b = x; return b; } fn main() -> void { let s = str_concat(\"a\", \"b\"); print(twice(s)); }",
        false,
    ),
    (
        "generic_both_instantiations_checked",
        "fn dup<T>(x: T) -> (T, T) { return (x, x); } fn main() -> void { let t = dup(3); print(t.0); let u = dup(str_concat(\"a\", \"b\")); print(u.0); }",
        false,
    ),
    (
        "generic_nested_instantiation_linear",
        "fn dup<T>(x: T) -> (T, T) { return (x, x); } fn wrap<U>(y: U) -> (U, U) { return dup(y); } fn main() -> void { let t = wrap(str_concat(\"a\", \"b\")); print(t.0); }",
        false,
    ),
    (
        "generic_nested_instantiation_copy",
        "fn dup<T>(x: T) -> (T, T) { return (x, x); } fn wrap<U>(y: U) -> (U, U) { return dup(y); } fn main() -> void { let t = wrap(7); print(t.1); }",
        true,
    ),
    (
        "generic_pick_with_early_return",
        "fn pick<T>(c: bool, a: T, b: T) -> T { if c { return a; } return b; } fn main() -> void { let x: i64 = 1; let y: i64 = 2; print(pick(true, x, y)); print(x + y); }",
        true,
    ),
    (
        "generic_max_of_copy_type",
        "fn mx<T>(a: T, b: T) -> T { if a > b { return a; } return b; } fn main() -> void { print(mx(3, 9)); }",
        true,
    ),
    (
        "generic_max_of_linear_type",
        "fn mx<T>(a: T, b: T) -> T { if a > b { return a; } return b; } fn main() -> void { let s = mx(str_concat(\"a\", \"b\"), str_concat(\"c\", \"d\")); print(s); }",
        true,
    ),
    (
        "unused_generic_checked_linear",
        "fn dup<T>(x: T) -> (T, T) { return (x, x); } fn main() -> void { print(1); }",
        false,
    ),
    // A branch that returns does not take part in the state merge.
    (
        "returning_then_branch_skips_merge",
        "fn eat(s: Str) -> void { } fn f(s: Str, c: bool) -> i64 { if c { eat(s); return 1; } eat(s); return 0; } fn main() -> void { print(f(str_concat(\"a\", \"b\"), true)); }",
        true,
    ),
    (
        "returning_else_branch_skips_merge",
        "fn eat(s: Str) -> void { } fn f(s: Str, c: bool) -> i64 { if c { print(1); } else { eat(s); return 0; } eat(s); return 1; } fn main() -> void { print(f(str_concat(\"a\", \"b\"), true)); }",
        true,
    ),
    (
        "returning_else_keeps_then_state",
        "fn eat(s: Str) -> void { } fn f(s: Str, c: bool) -> i64 { if c { eat(s); } else { return 0; } eat(s); return 1; } fn main() -> void { print(f(str_concat(\"a\", \"b\"), true)); }",
        false,
    ),
    (
        "returning_branch_in_loop",
        "fn eat(s: Str) -> void { } fn f(s: Str) -> i64 { let mut i = 0; while i < 3 { if i == 2 { eat(s); return i; } i = i + 1; } eat(s); return 0; } fn main() -> void { print(f(str_concat(\"a\", \"b\"))); }",
        true,
    ),
    (
        "aborting_branch_skips_merge",
        "fn eat(s: Str) -> void { } fn main() -> void { let s = str_concat(\"a\", \"b\"); let c = true; if c { eat(s); } else { abort(\"no\"); } print(1); }",
        true,
    ),
    (
        "returning_match_arm_skips_merge",
        "fn eat(s: Str) -> void { } fn g(s: Str) -> i64 { return 1; } fn f(s: Str, k: i64) -> i64 { match k { 1 => return g(s), _ => print(0), } eat(s); return 0; } fn main() -> void { print(f(str_concat(\"a\", \"b\"), 1)); }",
        true,
    ),
    (
        "const_div_by_zero",
        "const D = 7 / 0; fn main() -> void { print(D); }",
        false,
    ),
    (
        "const_mod_by_zero",
        "const D = 7 % 0; fn main() -> void { print(D); }",
        false,
    ),
    (
        "const_not_constant",
        "fn f() -> i64 { return 1; } const D = f(); fn main() -> void { print(D); }",
        false,
    ),
    (
        "const_cycle",
        "const A = B; const B = A; fn main() -> void { print(A); }",
        false,
    ),
    (
        "arity_too_few",
        "fn f(a: i64, b: i64) -> i64 { return a + b; } fn main() -> void { print(f(1)); }",
        false,
    ),
    (
        "arity_too_many",
        "fn f(a: i64) -> i64 { return a; } fn main() -> void { print(f(1, 2)); }",
        false,
    ),
    (
        "arity_builtin",
        "fn main() -> void { print(str_concat(\"a\")); }",
        false,
    ),
    (
        "destructure_int",
        "fn main() -> void { let (a, b) = 1; print(a); }",
        false,
    ),
    (
        "tuple_index_on_int",
        "fn main() -> void { let x = 1; print(x.0); }",
        false,
    ),
    (
        "index_on_int",
        "fn main() -> void { let x = 1; print(x[0]); }",
        false,
    ),
    (
        "destructure_too_many",
        "fn main() -> void { let t = (1, 2); let (a, b, c) = t; print(c); }",
        false,
    ),
    (
        "consume_in_match_arm",
        "fn eat(s: Str) -> void { } fn main() -> void { let s = str_concat(\"a\", \"b\"); let k = 1; match k { 1 => eat(s), _ => print(0), } print(s); }",
        false,
    ),
    (
        "consume_in_while_cond",
        "fn eat(s: Str) -> bool { return true; } fn main() -> void { let s = str_concat(\"a\", \"b\"); while eat(s) { print(1); return; } }",
        false,
    ),
    (
        "wrong_struct_ref",
        "struct A { x: i64, y: i64 } struct B { y: i64, z: i64, x: i64 } fn gx(p: &A) -> i64 { return p.x; } fn main() -> void { let b = B { y: 20, z: 30, x: 10 }; print(gx(&b)); }",
        false,
    ),
    (
        "right_struct_ref",
        "struct A { x: i64, y: i64 } fn gx(p: &A) -> i64 { return p.x; } fn main() -> void { let a = A { x: 1, y: 2 }; print(gx(&a)); }",
        true,
    ),
    // A guard that consumes runs before the arm body, so its spends count
    // even when the body diverges (and on the fall-through to later arms).
    (
        "guard_consumes_on_diverging_arm",
        "fn eatb(s: Str) -> bool { return true; } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); match k { 1 if eatb(s) => return 1, _ => print(0), } print(s); return 0; } fn main() -> void { print(f(1)); }",
        false,
    ),
    // L5b: a guard's spends happen only where its pattern matched; arm `2` cannot overlap `1`.
    (
        "guard_spend_skips_disjoint_literal_arm",
        "fn eatb(s: Str) -> bool { return true; } fn eatr(s: Str) -> i64 { return 1; } fn readb(s: &Str) -> bool { return true; } fn eat(s: Str) -> void { } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); match k { 1 if eatb(s) => return 1, 2 => return eatr(s), _ => return 0, } } fn main() -> void { print(f(2)); }",
        true,
    ),
    // s is spent on k==1 paths and live on the others.
    (
        "guard_spend_inconsistent_after_match",
        "fn eatb(s: Str) -> bool { return true; } fn eatr(s: Str) -> i64 { return 1; } fn readb(s: &Str) -> bool { return true; } fn eat(s: Str) -> void { } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); match k { 1 if eatb(s) => print(1), _ => print(0), } return 0; } fn main() -> void { print(f(1)); }",
        false,
    ),
    // the guard-failed path reaches `_` with s already spent.
    (
        "guard_spend_wild_arm_use_after_free",
        "fn eatb(s: Str) -> bool { return true; } fn eatr(s: Str) -> i64 { return 1; } fn readb(s: &Str) -> bool { return true; } fn eat(s: Str) -> void { } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); match k { 1 if eatb(s) => return 1, _ => return eatr(s), } } fn main() -> void { print(f(1)); }",
        false,
    ),
    // distinct enum variants cannot overlap.
    (
        "guard_spend_disjoint_enum_variant",
        "fn eatb(s: Str) -> bool { return true; } fn eatr(s: Str) -> i64 { return 1; } fn readb(s: &Str) -> bool { return true; } fn eat(s: Str) -> void { } enum C { R, G, B } fn f(k: C) -> i64 { let s = str_concat(\"a\", \"b\"); match k { C::R if eatb(s) => return 1, C::G => return eatr(s), _ => return 0, } } fn main() -> void { print(f(C::G)); }",
        true,
    ),
    // the same variant overlaps: guard-failed path has s spent.
    (
        "guard_spend_same_enum_variant",
        "fn eatb(s: Str) -> bool { return true; } fn eatr(s: Str) -> i64 { return 1; } fn readb(s: &Str) -> bool { return true; } fn eat(s: Str) -> void { } enum C { R, G, B } fn f(k: C) -> i64 { let s = str_concat(\"a\", \"b\"); match k { C::R if eatb(s) => return 1, C::R => return eatr(s), _ => return 0, } } fn main() -> void { print(f(C::R)); }",
        false,
    ),
    // a guard on an irrefutable pattern runs on every path to later arms,
    // so s is spent on every way out: consistent.
    (
        "guard_spend_irrefutable_always_spent",
        "fn eatb(s: Str) -> bool { return true; } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); match k { x if eatb(s) => return 1, _ => print(0), } return 0; } fn main() -> void { print(f(1)); }",
        true,
    ),
    // ... and s is gone after the match.
    (
        "guard_spend_irrefutable_use_after",
        "fn eatb(s: Str) -> bool { return true; } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); match k { x if eatb(s) => return 1, _ => print(0), } print(s); return 0; } fn main() -> void { print(f(1)); }",
        false,
    ),
    // a binding pattern overlaps everything.
    (
        "guard_spend_binding_then_use",
        "fn eatb(s: Str) -> bool { return true; } fn eatr(s: Str) -> i64 { return 1; } fn readb(s: &Str) -> bool { return true; } fn eat(s: Str) -> void { } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); match k { x if eatb(s) => return x, y => return eatr(s), } } fn main() -> void { print(f(1)); }",
        false,
    ),
    // a guard that only reads spends nothing.
    (
        "guard_read_only_then_use",
        "fn eatb(s: Str) -> bool { return true; } fn eatr(s: Str) -> i64 { return 1; } fn readb(s: &Str) -> bool { return true; } fn eat(s: Str) -> void { } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); match k { 1 if readb(&s) => return 1, _ => return eatr(s), } } fn main() -> void { print(f(1)); }",
        true,
    ),
    // two guards spending different values: arm `3` is disjoint from both.
    (
        "guard_two_spends_disjoint_arms",
        "fn eatb(s: Str) -> bool { return true; } fn eatr(s: Str) -> i64 { return 1; } fn readb(s: &Str) -> bool { return true; } fn eat2(a: Str, b: Str) -> i64 { return 1; } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); let u = str_concat(\"c\", \"d\"); match k { 1 if eatb(s) => return eatr(u), 2 if eatb(u) => return eatr(s), 3 => return eat2(s, u), _ => return 0, } } fn main() -> void { print(f(3)); }",
        true,
    ),
    // ... but `_` overlaps the guard on `1`, which spent s.
    (
        "guard_two_spends_wild_uses_one",
        "fn eatb(s: Str) -> bool { return true; } fn eatr(s: Str) -> i64 { return 1; } fn readb(s: &Str) -> bool { return true; } fn eat2(a: Str, b: Str) -> i64 { return 1; } fn f(k: i64) -> i64 { let s = str_concat(\"a\", \"b\"); let u = str_concat(\"c\", \"d\"); match k { 1 if eatb(s) => return eatr(u), 2 if eatb(u) => return eatr(s), _ => return eatr(s), } } fn main() -> void { print(f(5)); }",
        false,
    ),
    (
        "shadow_in_else_hides_consume",
        "fn eat(s: Str) -> void { } fn main() -> void { let s = str_concat(\"a\", \"b\"); if true { eat(s); } else { let s = 1; } }",
        false,
    ),
];

#[test]
fn trackc_closes_checker_holes() {
    let dir = scratch("trackc_verdicts");
    for (name, source, accepted) in TRACKC_VERDICTS {
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

/// Lifecycle hard errors (docs/lifecycles.md §2, card L5), trackc-only:
/// (program, exact first diagnostic line). Paths are under docs/lifecycles/;
/// other entries are inline sources.
const LIFE_ERRORS: &[(&str, &str)] = &[
    (
        "e1_lent_consumed.trk",
        "error[E0211]: 'take' consumes its parameter 'p', but 'r' is only lent to 'peek'. Pass '&' and make 'p: &Point', or take ownership in 'peek'.",
    ),
    (
        "e2_escape_tuple.trk",
        "error[E0212]: local 'p' escapes 'pair' inside the returned tuple (element 0); it dies when 'pair' returns.",
    ),
    (
        "e3_escape_struct.trk",
        "error[E0212]: local 'p' escapes 'make' through field 'Holder.p'; it dies when 'make' returns.",
    ),
    (
        "e4_outlive_block.trk",
        "error[E0213]: 'r' refers to 'b', which ends at the close of this block (line 9); 'r' is used after it at line 14.",
    ),
    (
        "e5_end_twice.trk",
        "error[E0214]: 'p' would be ended twice: fields 'Bag.a' and 'Bag.b' both take ownership of the borrowed 'r'.",
    ),
    (
        "fn eatb(s: Str) -> bool { return true; }\nfn f(k: i64) -> i64 {\n    let s = str_concat(\"a\", \"b\");\n    match k { 1 if eatb(s) => return 1, _ => print(0), }\n    print(s);\n    return 0;\n}\nfn main() -> void { print(f(1)); }\n",
        "error[E0200]: Resource 's' is spent in some match arms but not in others.",
    ),
    // I2: through a local alias, E0211 names the lent parameter.
    (
        "struct Point { x: i64, y: i64 }\nfn take(p: Point) -> i64 { return p.x; }\nfn peek(r: &Point) -> i64 {\n    let q = r;\n    return take(q);\n}\nfn main() -> void { let p = Point { x: 1, y: 2 }; print(peek(&p)); }\n",
        "error[E0211]: 'take' consumes its parameter 'p', but 'r' is only lent to 'peek'. Pass '&' and make 'p: &Point', or take ownership in 'peek'.",
    ),
];

#[test]
fn lifecycle_hard_errors_print_the_catalogue_texts() {
    let root = repo();
    let dir = scratch("life_errors");
    for (i, (prog, want)) in LIFE_ERRORS.iter().enumerate() {
        let path = if prog.ends_with(".trk") {
            root.join("docs/lifecycles").join(prog)
        } else {
            let file = dir.join(format!("inline{}.trk", i));
            fs::write(&file, prog).unwrap();
            file
        };
        let out = run_trackc(&["check", path.to_str().unwrap()], &dir);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{}: {}", prog, stderr);
        assert_eq!(
            stderr.lines().next().unwrap_or(""),
            *want,
            "{}: unexpected diagnostic:\n{}",
            prog,
            stderr
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// (name, source, expected stdout) of programs built by trackc.
const RUN_OUTPUTS: &[(&str, &str, &str)] = &[
    (
        "const_forward_reference",
        "const B = A * 2; const A = 3; const C = 5 > 2; const S = \"hi\"; fn main() -> void { print(B); print(C); print(S); }",
        "6\n1\nhi\n",
    ),
    (
        "tuple_in_returned_tuple",
        "fn mk(n: i64) -> ((i64, i64), i64) { let inner = (n, n * 2); return (inner, 9); } fn clobber(n: i64) -> i64 { let z = (n, n, n, n); return z.0 + z.3; } fn main() -> void { let t = mk(3); print(clobber(100)); let i = t.0; print(i.1); }",
        "200\n6\n",
    ),
    (
        "tuple_in_returned_struct",
        "struct H { t: (i64, i64) } fn mk(n: i64) -> H { return H { t: (n, n + 1) }; } fn clobber(n: i64) -> i64 { let z = (n, n, n, n); return z.0 + z.3; } fn main() -> void { let h = mk(7); print(clobber(100)); let t = h.t; print(t.1); }",
        "200\n8\n",
    ),
    (
        "tuple_in_returned_union",
        "union U { P((i64, i64)), N } fn mk(n: i64) -> U { return U::P((n, n + 5)); } fn clobber(n: i64) -> i64 { let z = (n, n, n, n); return z.0 + z.3; } fn main() -> void { let u = mk(1); print(clobber(100)); match u { U::P(t) => print(t.1), U::N => print(0) } }",
        "200\n6\n",
    ),
    (
        "tuple_in_returned_array",
        "fn mk(n: i64) -> [](i64, i64) { return [(n, n + 1)]; } fn clobber(n: i64) -> i64 { let z = (n, n, n, n); return z.0 + z.3; } fn main() -> void { let a = mk(4); print(clobber(100)); let t = a[0]; print(t.1); }",
        "200\n5\n",
    ),
    (
        "stack_tuple_reassigned_from_itself",
        "fn main() -> void { let mut p = (1, 2); let mut i = 0; while i < 3 { p = (p.1, p.0 + p.1); i = i + 1; } print(p.0); print(p.1); }",
        "5\n8\n",
    ),
    (
        "stack_tuple_destructured_and_matched",
        "fn main() -> void { let mut s = 0; for i in 0..4 { let t = (i, i * 2); let u = (i, 1); let (a, b) = t; match u { (0, _) => s = s + 100, (x, y) => s = s + x + y, } s = s + a + b; } print(s); }",
        "127\n",
    ),
    (
        "match_in_loop_resets_each_time",
        "fn main() -> void { for i in 0..3 { match i { 0 => print(10), _ => print(i), } } }",
        "10\n1\n2\n",
    ),
    (
        "tuple_copied_out_of_loop_escapes",
        "fn main() -> void { let mut keep = (0, 0); for i in 0..3 { let t = (i, i + 1); if i == 1 { keep = t; } else { let (a, b) = t; } } print(keep.0); print(keep.1); }",
        "1\n2\n",
    ),
    (
        "tuple_literal_in_loop",
        "fn main() -> void { let mut a = [(0, 0), (0, 0), (0, 0)]; for i in 0..3 { a[i] = (i, i * 10); } print(a[0].1); print(a[2].1); }",
        "0\n20\n",
    ),
];

/// A tuple literal that never escapes lives in a stack slot: 20M of them in
/// a loop allocate nothing (heap cells took ~600 MB before), while one that
/// is returned stays a heap cell.
#[test]
fn non_escaping_tuples_use_stack_slots() {
    let dir = scratch("stack_tuples");
    let file = dir.join("t3.trk");
    fs::write(
        &file,
        "fn pair(n: i64) -> (i64, i64) { let r = (n, n + 1); return r; } \
         fn main() -> void { let mut i = 0; let mut acc = 0; while i < 20000000 { let p = (i, 1); acc = acc + p.1; i = i + 1; } print(acc); print(pair(4).1); }",
    )
    .unwrap();
    let exe = dir.join("t3");
    let out = run_trackc(
        &["build", file.to_str().unwrap(), "-o", exe.to_str().unwrap()],
        &dir,
    );
    assert!(
        out.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let c = fs::read_to_string(dir.join("t3.c")).unwrap();
    let main_body = &c[c.find("tk_w tk_main(void) {").expect("tk_main")..];
    let main_body = &main_body[..main_body.find("\n}\n").unwrap()];
    assert!(
        !main_body.contains("alloc("),
        "loop tuple must not allocate:\n{}",
        main_body
    );
    let pair_body = &c[c.find("tk_w tk_pair(tk_w a0) {").expect("tk_pair")..];
    let pair_body = &pair_body[..pair_body.find("\n}\n").unwrap()];
    assert!(
        pair_body.contains("alloc("),
        "returned tuple must be a heap cell:\n{}",
        pair_body
    );
    // 64 MB of address space is far below what 20M heap cells need.
    let run = Command::new("sh")
        .arg("-c")
        .arg(format!("ulimit -v 65536; exec {}", exe.display()))
        .output()
        .unwrap();
    assert!(run.status.success(), "t3 failed under a 64 MB limit");
    assert_eq!(String::from_utf8_lossy(&run.stdout), "20000000\n5\n");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn trackc_programs_print_the_expected_output() {
    let dir = scratch("run_outputs");
    for (name, source, expected) in RUN_OUTPUTS {
        let file = dir.join(format!("{}.trk", name));
        fs::write(&file, source).unwrap();
        let out = run_trackc(&["run", file.to_str().unwrap()], &dir);
        assert!(
            out.status.success(),
            "{}: trackc run failed: {}",
            name,
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            *expected,
            "{}: wrong output",
            name
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// `trackc life` dumps per-function lifecycle summaries (card L3). Each
/// program in docs/lifecycles/, bench/progs/ and tests/life/progs/ has a
/// golden dump in tests/life/<stem>.life. Set `UPDATE_LIFE=1` to rewrite the goldens.
#[test]
fn life_dumps_match_goldens() {
    let root = repo();
    let update = env::var("UPDATE_LIFE").is_ok();
    let mut checked = 0;
    for sub in ["docs/lifecycles", "bench/progs", "tests/life/progs"] {
        let mut files: Vec<PathBuf> = fs::read_dir(root.join(sub))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "trk"))
            .collect();
        files.sort();
        for file in files {
            let rel = file.strip_prefix(&root).unwrap().to_str().unwrap();
            let out = run_trackc(&["life", rel], &root);
            assert!(
                out.status.success(),
                "{}: trackc life failed: {}",
                rel,
                String::from_utf8_lossy(&out.stderr)
            );
            let stem = file.file_stem().unwrap().to_str().unwrap();
            let golden = root.join("tests/life").join(format!("{}.life", stem));
            if update {
                fs::create_dir_all(golden.parent().unwrap()).unwrap();
                fs::write(&golden, &out.stdout).unwrap();
            }
            let want = fs::read_to_string(&golden)
                .unwrap_or_else(|_| panic!("missing golden {}", golden.display()));
            assert_eq!(
                String::from_utf8_lossy(&out.stdout),
                want,
                "{}: lifecycle dump differs from {}",
                rel,
                golden.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 22,
        "expected at least 22 programs, saw {}",
        checked
    );
}

/// `trackc life --place` dumps the placement plan (card L7,
/// docs/lifecycles.md §3.1): one line per allocation site with its line,
/// kind, PLACE and ENDS. Each program in tests/life/place/ has a golden
/// `<stem>.place`; `UPDATE_LIFE=1` rewrites them. place.trk pins one site
/// per rule, so the asserts below name the rule each line checks.
#[test]
fn life_place_dumps_match_goldens() {
    let root = repo();
    let update = env::var("UPDATE_LIFE").is_ok();
    let dir = root.join("tests/life/place");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "trk"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no programs in tests/life/place");
    for file in files {
        let rel = file.strip_prefix(&root).unwrap().to_str().unwrap();
        let out = run_trackc(&["life", "--place", rel], &root);
        assert!(
            out.status.success(),
            "{}: trackc life --place failed: {}",
            rel,
            String::from_utf8_lossy(&out.stderr)
        );
        let golden = file.with_extension("place");
        if update {
            fs::write(&golden, &out.stdout).unwrap();
        }
        let want = fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("missing golden {}", golden.display()));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            want,
            "{}: placement plan differs from {}",
            rel,
            golden.display()
        );
    }
    let got = String::from_utf8_lossy(
        &run_trackc(&["life", "--place", "tests/life/place/place.trk"], &root).stdout,
    )
    .into_owned();
    for (line, why) in [
        (
            "L20 tuple stack ends L21",
            "a tuple only destructured lives on the stack",
        ),
        (
            "L8 tuple ret-slot ends L8",
            "a returned tuple goes to the caller's slot",
        ),
        (
            "L24 struct heap ends -",
            "a value stored into a collection is heap",
        ),
        (
            "L26 call heap ends -",
            "a value passed to an esc param is heap",
        ),
        (
            "L28 str heap-autofree ends L29",
            "a fresh Str that dies in this frame is auto-freed (L13)",
        ),
    ] {
        assert!(
            got.contains(line),
            "{}: missing `{}` in\n{}",
            why,
            line,
            got
        );
    }
}

/// Stack cells in the C backend (card L8, docs/lifecycles.md §3.2):
/// every program in tests/stack/ builds, prints its `<stem>.out`, and
/// plans as its `<stem>.place` golden (`UPDATE_LIFE=1` rewrites them).
/// The asserts name the rule each placement line checks, and the emitted
/// C of the stack programs holds their literals in `tk_w sN[k]` slots.
#[test]
fn stack_cells_run_and_plan() {
    let root = repo();
    let update = env::var("UPDATE_LIFE").is_ok();
    let dir = scratch("stack");
    let mut plans = String::new();
    for stem in ["escapes", "mutate", "read_only", "union_match"] {
        let rel = format!("tests/stack/{}.trk", stem);
        let exe = dir.join(stem);
        let out = run_trackc(&["build", &rel, "-o", exe.to_str().unwrap()], &root);
        assert!(
            out.status.success(),
            "{}: build failed: {}",
            rel,
            String::from_utf8_lossy(&out.stderr)
        );
        let want = fs::read_to_string(root.join(format!("tests/stack/{}.out", stem))).unwrap();
        assert_eq!(run_exe(&exe), (want, Some(0)), "{}: output differs", rel);
        let c = fs::read_to_string(exe.with_extension("c")).unwrap();
        if stem != "escapes" {
            assert!(
                c.contains("tk_w s0["),
                "{}: no stack slot in emitted C",
                rel
            );
        }
        let plan = run_trackc(&["life", "--place", &rel], &root);
        let got = String::from_utf8_lossy(&plan.stdout).into_owned();
        let golden = root.join(format!("tests/stack/{}.place", stem));
        if update {
            fs::write(&golden, &got).unwrap();
        }
        let want = fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("missing golden {}", golden.display()));
        assert_eq!(got, want, "{}: placement plan differs", rel);
        plans.push_str(&format!("{}:\n{}", stem, got));
    }
    for (line, why) in [
        (
            "L23 struct stack ends L25",
            "a struct only lent to a reading callee is stack",
        ),
        (
            "L24 struct stack ends L25",
            "a struct only read through its fields is stack",
        ),
        (
            "L29 array stack ends L31",
            "an array only indexed and lent is stack",
        ),
        ("L8 struct heap ends -", "a returned struct is heap"),
        (
            "L17 struct heap ends -",
            "a struct stored into another literal is heap",
        ),
        (
            "L27 struct heap ends -",
            "a struct passed to a `ret` parameter is heap",
        ),
        (
            "L29 struct heap ends -",
            "a struct inside an array literal is heap",
        ),
        (
            "L17 struct stack ends L23",
            "a struct mutated through the slot is stack",
        ),
        (
            "L21 array stack ends L24",
            "an array written by index is stack",
        ),
        (
            "L13 union stack ends L14",
            "a payload union only matched is stack",
        ),
    ] {
        assert!(
            plans.contains(line),
            "{}: missing `{}` in\n{}",
            why,
            line,
            plans
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Auto-free of heap cells (card L9, docs/lifecycles.md §3.3): every
/// program in tests/autofree/ builds, prints its `<stem>.out`, plans as its
/// `<stem>.place` golden (`UPDATE_LIFE=1` rewrites them) and frees cells in
/// its emitted C. Where gcc has AddressSanitizer, the programs also run
/// with no use after free and no double free.
#[test]
fn autofree_frees_heap_cells() {
    let root = repo();
    let update = env::var("UPDATE_LIFE").is_ok();
    let dir = scratch("autofree");
    let stems = [
        "die_param",
        "early_return",
        "moved_field",
        "nested",
        "returned",
    ];
    let mut plans = String::new();
    for stem in stems {
        let rel = format!("tests/autofree/{}.trk", stem);
        let exe = dir.join(stem);
        let out = run_trackc(&["build", &rel, "-o", exe.to_str().unwrap()], &root);
        assert!(
            out.status.success(),
            "{}: build failed: {}",
            rel,
            String::from_utf8_lossy(&out.stderr)
        );
        let want = fs::read_to_string(root.join(format!("tests/autofree/{}.out", stem))).unwrap();
        assert_eq!(run_exe(&exe), (want, Some(0)), "{}: output differs", rel);
        let c = fs::read_to_string(exe.with_extension("c")).unwrap();
        assert!(c.contains("dealloc(l"), "{}: no free in emitted C", rel);
        let plan = run_trackc(&["life", "--place", &rel], &root);
        let got = String::from_utf8_lossy(&plan.stdout).into_owned();
        let golden = root.join(format!("tests/autofree/{}.place", stem));
        if update {
            fs::write(&golden, &got).unwrap();
        }
        let want = fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("missing golden {}", golden.display()));
        assert_eq!(got, want, "{}: placement plan differs", rel);
        plans.push_str(&format!("{}:\n{}", stem, got));
    }
    for (line, why) in [
        (
            "L23 call heap-autofree deep ends L24",
            "a fresh cell passed to a `die` parameter is freed by the caller",
        ),
        (
            "L21 call stack ends L22",
            "a ret-slot result passed to a `die` parameter is a stack slot (L10)",
        ),
        (
            "L19 struct stack ends L20",
            "a stack cell passed to a `die` parameter stays a stack cell",
        ),
        (
            "L16 call heap-autofree ends L17",
            "a cell whose child is moved out frees only itself",
        ),
        (
            "L22 struct heap-autofree deep ends L27",
            "a nested literal reassigned fresh cells runs drop glue",
        ),
        ("L7 struct heap ends -", "a returned cell is never freed"),
        (
            "L13 struct heap ends -",
            "a cell stored into a returned cell is never freed",
        ),
    ] {
        assert!(
            plans.contains(line),
            "{}: missing `{}` in\n{}",
            why,
            line,
            plans
        );
    }
    let asan = Command::new("gcc")
        .args(["-fsanitize=address", "-x", "c", "-", "-o"])
        .arg(dir.join("probe"))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"int main(void){return 0;}")?;
            child.wait()
        })
        .map(|s| s.success())
        .unwrap_or(false);
    if !asan {
        return;
    }
    for stem in stems {
        let rel = format!("tests/autofree/{}.trk", stem);
        let exe = dir.join(format!("{}_asan", stem));
        let out = Command::new(trackc())
            .args(["build", &rel, "-o", exe.to_str().unwrap()])
            .env("CC", "gcc -fsanitize=address -g")
            .env("TRACKC_CACHE", dir.join("cache"))
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}: ASan build failed", rel);
        let run = Command::new(&exe)
            .env("ASAN_OPTIONS", "detect_leaks=0")
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(
            run.status.success() && !err.contains("AddressSanitizer"),
            "{}: ASan reports an error:\n{}",
            rel,
            err
        );
    }
}

/// Auto-free of fresh `Str` values and runtime handles (card L13,
/// docs/lifecycles.md §3.5): every program in tests/handles/ builds with
/// both backends, prints its `<stem>.out` (whose last line is
/// `sys_get_memory_used()`, so native and C agree on it), plans as its
/// `<stem>.place` golden (`UPDATE_LIFE=1` rewrites them) and frees through
/// the kind's runtime free function. Values that are returned, stored or
/// aliased are never freed. Where gcc has AddressSanitizer, the C builds
/// also run with no memory error.
#[test]
fn handles_free_fresh_values() {
    let root = repo();
    let update = env::var("UPDATE_LIFE").is_ok();
    let dir = scratch("handles");
    let stems = [
        "builder",
        "coll_kept",
        "coll_local",
        "int_loop",
        "str_kept",
        "str_local",
    ];
    let mut plans = String::new();
    for stem in stems {
        let rel = format!("tests/handles/{}.trk", stem);
        let want = fs::read_to_string(root.join(format!("tests/handles/{}.out", stem))).unwrap();
        let exe = dir.join(stem);
        let out = run_trackc(&["build", &rel, "-o", exe.to_str().unwrap()], &root);
        assert!(
            out.status.success(),
            "{}: build failed: {}",
            rel,
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            run_exe(&exe),
            (want.clone(), Some(0)),
            "{}: C output differs",
            rel
        );
        let nat = dir.join(format!("{}_nat", stem));
        let out = run_trackc(
            &["build", "--native", &rel, "-o", nat.to_str().unwrap()],
            &root,
        );
        assert!(out.status.success(), "{}: native build failed", rel);
        assert_eq!(
            run_exe(&nat),
            (want, Some(0)),
            "{}: native output differs",
            rel
        );
        let plan = run_trackc(&["life", "--place", &rel], &root);
        let got = String::from_utf8_lossy(&plan.stdout).into_owned();
        let golden = root.join(format!("tests/handles/{}.place", stem));
        if update {
            fs::write(&golden, &got).unwrap();
        }
        let want = fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("missing golden {}", golden.display()));
        assert_eq!(got, want, "{}: placement plan differs", rel);
        plans.push_str(&format!("{}:\n{}", stem, got));
        let c = fs::read_to_string(exe.with_extension("c")).unwrap();
        for (prog, free) in [
            ("str_local", "track_str_free(l"),
            ("int_loop", "track_str_free(l"),
            ("builder", "track_sb_free(l"),
            ("builder", "track_str_free(l"),
            ("coll_local", "track_i64_collection_free(l"),
            ("coll_local", "vec_free(l"),
        ] {
            if prog == stem {
                assert!(c.contains(free), "{}: no `{}` in emitted C", rel, free);
            }
        }
    }
    for (line, why) in [
        (
            "L4 str heap-autofree ends L6",
            "a str_concat result that dies locally is freed",
        ),
        (
            "L14 str heap-autofree ends L17",
            "a Str reassigned fresh strings is freed",
        ),
        (
            "L7 str heap-autofree ends L10",
            "an int_to_str in a loop is freed each iteration",
        ),
        (
            "L4 builder heap-autofree ends L12",
            "a builder finished into a Str is freed",
        ),
        (
            "L12 str heap-autofree ends L13",
            "the finished Str is freed",
        ),
        (
            "L3 collection heap-autofree ends L12",
            "a collection that dies locally is freed",
        ),
        (
            "L19 vec heap-autofree ends L27",
            "a Vec that dies locally is freed",
        ),
    ] {
        assert!(
            plans.contains(line),
            "{}: missing `{}` in\n{}",
            why,
            line,
            plans
        );
    }
    for (prog, kept) in [
        ("str_kept", "L4 "),
        ("str_kept", "L12 str"),
        ("str_kept", "L14 str"),
        ("coll_kept", "L7 "),
        ("coll_kept", "L11 "),
    ] {
        let block = plans.split(&format!("{}:\n", prog)).nth(1).unwrap_or("");
        let block = block.split(":\n").next().unwrap_or("");
        assert!(
            !block
                .lines()
                .any(|l| l.trim_start().starts_with(kept.trim_end()) && l.contains("autofree")),
            "{}: a returned, stored or aliased value at `{}` must not be freed:\n{}",
            prog,
            kept,
            block
        );
    }
    let asan = Command::new("gcc")
        .args(["-fsanitize=address", "-x", "c", "-", "-o"])
        .arg(dir.join("probe"))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"int main(void){return 0;}")?;
            child.wait()
        })
        .map(|s| s.success())
        .unwrap_or(false);
    if !asan {
        let _ = fs::remove_dir_all(&dir);
        return;
    }
    for stem in stems {
        let rel = format!("tests/handles/{}.trk", stem);
        let exe = dir.join(format!("{}_asan", stem));
        let out = Command::new(trackc())
            .args(["build", &rel, "-o", exe.to_str().unwrap()])
            .env("CC", "gcc -fsanitize=address -g")
            .env("TRACKC_CACHE", dir.join("cache"))
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}: ASan build failed", rel);
        let run = Command::new(&exe)
            .env("ASAN_OPTIONS", "detect_leaks=0")
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(
            run.status.success() && !err.contains("AddressSanitizer"),
            "{}: ASan reports an error:\n{}",
            rel,
            err
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Auto-free of fresh temporaries (card L13b, docs/lifecycles.md §3.5): a
/// fresh Str or handle passed straight to a call that neither keeps, frees
/// nor returns it is freed right after that call; a discarded fresh
/// statement right after itself. Every program in tests/temps/ prints its
/// `<stem>.out` with both backends and plans as its `<stem>.place` golden
/// (`UPDATE_LIFE=1` rewrites them). Where gcc has AddressSanitizer, the C
/// builds run with no memory error, and the programs whose temporaries all
/// die leak nothing.
#[test]
fn temps_free_fresh_temporaries() {
    let root = repo();
    let update = env::var("UPDATE_LIFE").is_ok();
    let dir = scratch("temps");
    let stems = ["alias", "discard", "kept", "nested", "print_loop"];
    let mut plans = String::new();
    for stem in stems {
        let rel = format!("tests/temps/{}.trk", stem);
        let want = fs::read_to_string(root.join(format!("tests/temps/{}.out", stem))).unwrap();
        let exe = dir.join(stem);
        let out = run_trackc(&["build", &rel, "-o", exe.to_str().unwrap()], &root);
        assert!(
            out.status.success(),
            "{}: build failed: {}",
            rel,
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            run_exe(&exe),
            (want.clone(), Some(0)),
            "{}: C output differs",
            rel
        );
        let nat = dir.join(format!("{}_nat", stem));
        let out = run_trackc(
            &["build", "--native", &rel, "-o", nat.to_str().unwrap()],
            &root,
        );
        assert!(out.status.success(), "{}: native build failed", rel);
        assert_eq!(
            run_exe(&nat),
            (want, Some(0)),
            "{}: native output differs",
            rel
        );
        let plan = run_trackc(&["life", "--place", &rel], &root);
        let got = String::from_utf8_lossy(&plan.stdout).into_owned();
        let golden = root.join(format!("tests/temps/{}.place", stem));
        if update {
            fs::write(&golden, &got).unwrap();
        }
        let want = fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("missing golden {}", golden.display()));
        assert_eq!(got, want, "{}: placement plan differs", rel);
        plans.push_str(&format!("{}:\n{}", stem, got));
    }
    let block = |prog: &str| -> String {
        let b = plans.split(&format!("{}:\n", prog)).nth(1).unwrap_or("");
        b.split(":\n").next().unwrap_or("").to_string()
    };
    for (prog, line, count, why) in [
        (
            "print_loop",
            "str temp-autofree",
            2,
            "int_to_str passed to print and str_len is freed",
        ),
        (
            "nested",
            "str temp-autofree",
            6,
            "every nested fresh argument is freed",
        ),
        (
            "discard",
            "str temp-autofree",
            3,
            "discarded results and their fresh arguments are freed",
        ),
        (
            "kept",
            "L12 str temp-autofree ends L12",
            1,
            "int_to_str lent to str_concat is freed",
        ),
    ] {
        let got = block(prog).matches(line).count();
        assert_eq!(got, count, "{}: `{}` x{} in\n{}", why, line, count, plans);
    }
    for (prog, kept) in [("kept", "L11 "), ("alias", "temp-autofree")] {
        assert!(
            !block(prog)
                .lines()
                .any(|l| l.trim_start().starts_with(kept.trim_end()) && l.contains("temp-autofree")),
            "{}: a temporary that is kept or aliased (`{}`) must not be freed:\n{}",
            prog,
            kept,
            plans
        );
    }
    let asan = Command::new("gcc")
        .args(["-fsanitize=address", "-x", "c", "-", "-o"])
        .arg(dir.join("probe"))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"int main(void){return 0;}")?;
            child.wait()
        })
        .map(|s| s.success())
        .unwrap_or(false);
    if !asan {
        let _ = fs::remove_dir_all(&dir);
        return;
    }
    for stem in stems {
        let rel = format!("tests/temps/{}.trk", stem);
        let exe = dir.join(format!("{}_asan", stem));
        let out = Command::new(trackc())
            .args(["build", &rel, "-o", exe.to_str().unwrap()])
            .env("CC", "gcc -fsanitize=address -g")
            .env("TRACKC_CACHE", dir.join("cache"))
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}: ASan build failed", rel);
        let all_die = matches!(stem, "discard" | "nested" | "print_loop");
        let leaks = if all_die {
            "detect_leaks=1"
        } else {
            "detect_leaks=0"
        };
        let run = Command::new(&exe)
            .env("ASAN_OPTIONS", leaks)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(
            run.status.success() && !err.contains("Sanitizer"),
            "{}: ASan reports an error or a leak:\n{}",
            rel,
            err
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Caller return slots in the C backend (card L10, docs/lifecycles.md
/// §3.2): every program in tests/retslot/ builds, prints its `<stem>.out`
/// and plans as its `<stem>.place` golden (`UPDATE_LIFE=1` rewrites them).
/// The ret-slot functions get a `__rs` entry that writes the caller's
/// cell. Where gcc has AddressSanitizer, the programs also run with no
/// memory error.
#[test]
fn ret_slots_write_caller_cells() {
    let root = repo();
    let update = env::var("UPDATE_LIFE").is_ok();
    let dir = scratch("retslot");
    let stems = [
        "alias_ref",
        "alias_val",
        "both",
        "collection",
        "nested",
        "recursion",
    ];
    let mut plans = String::new();
    for stem in stems {
        let rel = format!("tests/retslot/{}.trk", stem);
        let exe = dir.join(stem);
        let out = run_trackc(&["build", &rel, "-o", exe.to_str().unwrap()], &root);
        assert!(
            out.status.success(),
            "{}: build failed: {}",
            rel,
            String::from_utf8_lossy(&out.stderr)
        );
        let want = fs::read_to_string(root.join(format!("tests/retslot/{}.out", stem))).unwrap();
        assert_eq!(run_exe(&exe), (want, Some(0)), "{}: output differs", rel);
        let c = fs::read_to_string(exe.with_extension("c")).unwrap();
        assert!(
            c.contains("__rs("),
            "{}: no ret-slot entry in emitted C",
            rel
        );
        let plan = run_trackc(&["life", "--place", &rel], &root);
        let got = String::from_utf8_lossy(&plan.stdout).into_owned();
        let golden = root.join(format!("tests/retslot/{}.place", stem));
        if update {
            fs::write(&golden, &got).unwrap();
        }
        let want = fs::read_to_string(&golden)
            .unwrap_or_else(|_| panic!("missing golden {}", golden.display()));
        assert_eq!(got, want, "{}: placement plan differs", rel);
        plans.push_str(&format!("{}:\n{}", stem, got));
    }
    for (line, why) in [
        (
            "L12 struct stack ends L22",
            "a local reassigned only ret-slot results is a stack cell",
        ),
        (
            "L16 call stack ends L22",
            "`acc = f(&acc)` writes acc's own cell",
        ),
        (
            "L14 call stack ends L20",
            "`acc = f(acc)` writes acc's own cell",
        ),
        (
            "L13 call heap ends -",
            "a ret-slot result stored into a collection is heap",
        ),
        (
            "L20 call stack ends L22",
            "a ret-slot result in a stack slot",
        ),
        (
            "L21 call heap-autofree deep ends L26",
            "a local also reassigned a heap-only cell stays heap-autofree",
        ),
        (
            "L25 call heap-autofree ends L26",
            "a ret-slot result written into an autofree local's cell",
        ),
        (
            "L12 call heap ends -",
            "a ret-slot result returned onward through a local is heap",
        ),
        (
            "L10 call stack ends L11",
            "a recursive ret-slot call writes a slot of its caller's frame",
        ),
        (
            "L18 call ret-slot ends L18",
            "a `return f(..)` passes the caller's slot through",
        ),
        (
            "L17 call stack ends L18",
            "the outer call of `f(g(x))` writes a stack slot",
        ),
    ] {
        assert!(
            plans.contains(line),
            "{}: missing `{}` in\n{}",
            why,
            line,
            plans
        );
    }
    let asan = Command::new("gcc")
        .args(["-fsanitize=address", "-x", "c", "-", "-o"])
        .arg(dir.join("probe"))
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"int main(void){return 0;}")?;
            child.wait()
        })
        .map(|s| s.success())
        .unwrap_or(false);
    if !asan {
        return;
    }
    for stem in stems {
        let rel = format!("tests/retslot/{}.trk", stem);
        let exe = dir.join(format!("{}_asan", stem));
        let out = Command::new(trackc())
            .args(["build", &rel, "-o", exe.to_str().unwrap()])
            .env("CC", "gcc -fsanitize=address -g")
            .env("TRACKC_CACHE", dir.join("cache"))
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}: ASan build failed", rel);
        let run = Command::new(&exe)
            .env("ASAN_OPTIONS", "detect_leaks=0")
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(
            run.status.success() && !err.contains("AddressSanitizer"),
            "{}: ASan reports an error:\n{}",
            rel,
            err
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// K3a: a user program may define any name a runtime uses internally (the
/// Track runtime's `sum`, `write`, `O_RDONLY`...; the C prelude's `tk_w`,
/// `tk_shl`...). Every runtime, and the native backend, runs it unchanged;
/// the two programs that used to clash with compiler/rt/vec.trk's `sum`
/// build natively.
#[test]
fn user_names_never_clash_with_runtime_names() {
    let root = repo();
    let dir = scratch("names");
    let cases = [
        ("tests/names/rt_clash", vec!["", "--rt=track", "--native"]),
        ("tests/stack/read_only", vec!["--native"]),
        ("tests/autofree/die_param", vec!["--native"]),
    ];
    for (stem, modes) in cases {
        let rel = format!("{}.trk", stem);
        let want = fs::read_to_string(root.join(format!("{}.out", stem))).unwrap();
        for mode in modes {
            let exe = dir.join(format!("{}{}", stem.replace('/', "_"), mode));
            let mut args = vec!["build"];
            if !mode.is_empty() {
                args.push(mode);
            }
            args.extend([rel.as_str(), "-o", exe.to_str().unwrap()]);
            let out = run_trackc(&args, &root);
            assert!(
                out.status.success(),
                "{} {}: build failed: {}",
                rel,
                mode,
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(
                run_exe(&exe),
                (want.clone(), Some(0)),
                "{} {}: output differs",
                rel,
                mode
            );
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// K3a: two definitions of one function are E0201 (trackc), whether in one
/// file or across imports.
/// ngen's inline allocator reads the runtime-state layout constants by
/// name, so a program may not redefine them under `--native`: a user
/// `RT_BASE` (and friends) is E0201, not a binary that segfaults.
#[test]
fn runtime_layout_consts_stay_reserved() {
    let dir = scratch("layoutconst");
    for name in [
        "RT_BASE",
        "RS_USED",
        "RS_HEAP",
        "RS_LIMIT",
        "RS_HEAP_END",
        "RS_FREE",
    ] {
        let file = format!("{}.trk", name);
        fs::write(
            dir.join(&file),
            format!(
                "const {} = 5;\nstruct P {{ x: i64 }}\nfn mk(i: i64) -> P {{ return P {{ x: i }}; }}\nfn main() -> i64 {{ let p = mk(3); print(p.x); return 0; }}\n",
                name
            ),
        )
        .unwrap();
        let exe = dir.join(name);
        let out = run_trackc(
            &["build", "--native", &file, "-o", exe.to_str().unwrap()],
            &dir,
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success() && err.contains("E0201"),
            "{}: user const accepted under --native: {}",
            name,
            err
        );
    }
}

#[test]
fn duplicate_functions_are_rejected() {
    let dir = scratch("dupfn");
    fs::write(
        dir.join("other.trk"),
        "fn place_root() -> i64 { return 7; }\n",
    )
    .unwrap();
    fs::write(
        dir.join("one.trk"),
        "fn place_root() -> i64 { return 1; }\nfn place_root() -> i64 { return 2; }\nfn main() -> i64 { print(place_root()); return 0; }\n",
    )
    .unwrap();
    fs::write(
        dir.join("two.trk"),
        "import \"other\";\nfn place_root() -> i64 { return 2; }\nfn main() -> i64 { print(place_root()); return 0; }\n",
    )
    .unwrap();
    for file in ["one.trk", "two.trk"] {
        for mode in ["check", "build"] {
            let out = run_trackc(&[mode, file], &dir);
            let err = String::from_utf8_lossy(&out.stderr);
            assert!(
                !out.status.success()
                    && err.contains("error[E0201]: duplicate definition of function 'place_root'"),
                "{} {}: duplicate accepted: {}",
                mode,
                file,
                err
            );
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// K3a: strings are pointers (SPEC §11) and concatenation is
/// `str_concat(&a, &b)` (docs/stdlib.md); `a + b` on two strings would add
/// the pointers, so trackc rejects it.
#[test]
fn string_plus_string_is_a_type_error() {
    let dir = scratch("strplus");
    let bad = [
        "fn main() -> i64 { let a = \"ab\"; let b = \"cd\"; let c = a + b; print(c); return 0; }\n",
        "fn main() -> i64 { print(\"ab\" + \"cd\"); return 0; }\n",
        "fn main() -> i64 { let a = \"ab\"; let s = str_concat(&a, &a); let t = s + a; print(t); return 0; }\n",
    ];
    for (i, src) in bad.iter().enumerate() {
        let file = format!("bad{}.trk", i);
        fs::write(dir.join(&file), src).unwrap();
        let out = run_trackc(&["check", &file], &dir);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success() && err.contains("str_concat"),
            "{}: string + string accepted: {}",
            src,
            err
        );
    }
    // Pointer arithmetic (string + integer, string - string) stays legal.
    fs::write(
        dir.join("ok.trk"),
        "fn main() -> i64 { let a = \"abcd\"; let n = 2; let p = a + n; print(p - a); return 0; }\n",
    )
    .unwrap();
    let out = run_trackc(&["check", "ok.trk"], &dir);
    assert!(
        out.status.success(),
        "string + int rejected: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = fs::remove_dir_all(&dir);
}

/// K3b: `for x in a` binds the elements of array `a` (`&a` borrows it),
/// `for i in s..e` counts; an empty array runs the body zero times. On
/// both runtimes, `exit(3)` and a division-by-zero trap keep what was
/// printed. A scalar iterable is a type error.
#[test]
fn for_in_exit_and_div_trap_on_both_runtimes() {
    use std::os::unix::process::ExitStatusExt;
    let dir = scratch("k3b");
    let (c, t) = run_under_both_runtimes(&dir, "for_in", &repo().join("tests/native/for_in.trk"));
    let want = "10\n20\n30\n3\n2\n3\n4\n10\n20\n30\n110\n120\n130\nab\ncde\n60\n11\n21\n31\n";
    assert_eq!(c, (want.to_string(), Some(0)), "for_in: C runtime");
    assert_eq!(t, c, "for_in: Track runtime differs");
    let (c, t) = run_under_both_runtimes(
        &dir,
        "exit_flush",
        &repo().join("tests/native/exit_flush.trk"),
    );
    assert_eq!(c, ("1\n".to_string(), Some(3)), "exit_flush: C runtime");
    assert_eq!(t, c, "exit_flush: Track runtime differs");
    run_under_both_runtimes(
        &dir,
        "divzero_flush",
        &repo().join("tests/native/divzero_flush.trk"),
    );
    for exe in ["divzero_flush_crt", "divzero_flush_trt"] {
        let out = Command::new(dir.join(exe)).output().unwrap();
        assert_eq!(
            (
                String::from_utf8_lossy(&out.stdout).to_string(),
                out.status.signal()
            ),
            ("1\n".to_string(), Some(8)),
            "{}: output lost on SIGFPE",
            exe
        );
    }
    fs::write(
        dir.join("forint.trk"),
        "fn main() -> i64 { let n = 3; for i in n { print(i); } return 0; }\n",
    )
    .unwrap();
    let out = run_trackc(&["check", "forint.trk"], &dir);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success() && err.contains("`for` iterates over a range or an array"),
        "for over an integer accepted: {}",
        err
    );
    let _ = fs::remove_dir_all(&dir);
}

/// K3c: `INT64_MIN / -1` (and `% -1`) traps like a zero divisor on every
/// backend and runtime: what was printed is flushed, then SIGFPE. Native
/// once took the hardware idiv fault and lost the buffered output.
#[test]
fn int64_min_div_minus_one_flushes_on_every_backend() {
    use std::os::unix::process::ExitStatusExt;
    let dir = scratch("k3c_divmin");
    for (stem, want) in [
        ("divmin_flush", "-7\n0\n-9223372036854775807\n"),
        ("divmin_mod_flush", "0\n"),
    ] {
        let src = repo().join(format!("tests/native/{}.trk", stem));
        for mode in ["", "--rt=track", "--native"] {
            let exe = dir.join(format!("{}{}", stem, mode.replace(['-', '='], "_")));
            let mut args = vec!["build"];
            if !mode.is_empty() {
                args.push(mode);
            }
            args.extend([src.to_str().unwrap(), "-o", exe.to_str().unwrap()]);
            let out = run_trackc(&args, &dir);
            assert!(out.status.success(), "{} {}: build failed", stem, mode);
            let out = Command::new(&exe).output().unwrap();
            assert_eq!(
                (
                    String::from_utf8_lossy(&out.stdout).to_string(),
                    out.status.signal()
                ),
                (want.to_string(), Some(8)),
                "{} {}: output lost or wrong signal",
                stem,
                mode
            );
        }
    }
    let _ = fs::remove_dir_all(&dir);
}
