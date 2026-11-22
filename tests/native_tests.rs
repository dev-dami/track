// Native x86-64 backend (`trackc build --native`, compiler/src/ngen.trk).
//
// trackc is built once with `yard` (stage 1). The tests check that native
// executables need no C toolchain, behave like the C backend's, and that
// the natively built compiler reaches a byte-identical fixed point.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch(name: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("track_native_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn trackc() -> &'static Path {
    static TRACKC: OnceLock<PathBuf> = OnceLock::new();
    TRACKC.get_or_init(|| {
        let status = Command::new(env!("CARGO_BIN_EXE_yard"))
            .arg("build")
            .current_dir(repo().join("compiler"))
            .output()
            .expect("failed to run yard");
        assert!(status.status.success(), "yard build of compiler/ failed");
        let copy = scratch("stage1").join("trackc");
        fs::copy(repo().join("compiler/target/trackc"), &copy).unwrap();
        copy
    })
}

fn run(exe: &Path, args: &[&str], path_env: Option<&str>) -> Output {
    let mut cmd = Command::new(exe);
    cmd.args(args).current_dir(repo());
    if let Some(p) = path_env {
        cmd.env("PATH", p);
    }
    cmd.output().expect("failed to run")
}

fn observe(exe: &Path) -> (String, String, Option<i32>) {
    let out = Command::new(exe).output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code(),
    )
}

#[test]
fn native_build_needs_no_c_toolchain() {
    let dir = scratch("hello");
    let exe = dir.join("hello");
    let out = run(
        trackc(),
        &[
            "build",
            "--native",
            "tests/native_raw/hello_syscall.trk",
            "-o",
            exe.to_str().unwrap(),
        ],
        Some("/nonexistent"),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let image = fs::read(&exe).unwrap();
    assert_eq!(&image[..4], b"\x7fELF");
    let (stdout, _, code) = observe(&exe);
    assert_eq!(stdout, "Hello, native world!\n");
    assert_eq!(code, Some(42));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn native_programs_match_the_c_backend() {
    let dir = scratch("parity");
    let mut inputs: Vec<PathBuf> = Vec::new();
    for sub in ["examples", "tests/native", "bench/progs"] {
        for e in fs::read_dir(repo().join(sub)).unwrap() {
            let p = e.unwrap().path();
            let stem = p.file_stem().unwrap().to_string_lossy().to_string();
            if p.extension().is_some_and(|x| x == "trk") && !stem.ends_with("_err") {
                inputs.push(p);
            }
        }
    }
    inputs.sort();
    assert!(inputs.len() > 10);
    for p in inputs {
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        let c_exe = dir.join(format!("{}_c", stem));
        let n_exe = dir.join(format!("{}_n", stem));
        let src = p.to_str().unwrap();
        let c = run(
            trackc(),
            &["build", src, "-o", c_exe.to_str().unwrap()],
            None,
        );
        assert!(c.status.success(), "{}: C build failed", stem);
        let n = run(
            trackc(),
            &["build", "--native", src, "-o", n_exe.to_str().unwrap()],
            None,
        );
        assert!(
            n.status.success(),
            "{}: native build failed: {}",
            stem,
            String::from_utf8_lossy(&n.stderr)
        );
        let (c_out, _, c_code) = observe(&c_exe);
        let (n_out, _, n_code) = observe(&n_exe);
        assert_eq!(
            (c_out, c_code),
            (n_out, n_code),
            "{}: behaviour differs",
            stem
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn native_compiler_reaches_a_fixed_point() {
    let dir = scratch("fixpoint");
    let n2 = dir.join("trackc2");
    let n3 = dir.join("trackc3");
    let out = run(
        trackc(),
        &[
            "build",
            "--native",
            "compiler/src/main.trk",
            "-o",
            n2.to_str().unwrap(),
        ],
        None,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run(
        &n2,
        &[
            "build",
            "--native",
            "compiler/src/main.trk",
            "-o",
            n3.to_str().unwrap(),
        ],
        Some("/nonexistent"),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        fs::read(&n2).unwrap() == fs::read(&n3).unwrap(),
        "native stage2 and stage3 differ"
    );
    let _ = fs::remove_dir_all(&dir);
}

fn trk_files(sub: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(repo().join(sub))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "trk"))
        .collect();
    v.sort();
    v
}

// Every program `trackc check` rejects must be rejected by the native build
// and by `--rt=track`: the embedded runtime must not change how the user
// program is checked (its plain-i64 definitions of str_concat, vec_push...
// used to replace the builtin signatures the ownership checker relies on).
#[test]
fn native_and_track_rt_reject_what_check_rejects() {
    let dir = scratch("reject");
    let exe = dir.join("x");
    let exe = exe.to_str().unwrap();
    let mut inputs = trk_files("tests/native_reject");
    inputs.extend(
        trk_files("examples")
            .into_iter()
            .filter(|p| p.to_string_lossy().ends_with("_err.trk")),
    );
    assert!(inputs.len() >= 12);
    for p in inputs {
        let src = p.to_str().unwrap();
        assert!(
            !run(trackc(), &["check", src], None).status.success(),
            "{}: check accepted it",
            src
        );
        assert!(
            !run(trackc(), &["build", "--native", src, "-o", exe], None)
                .status
                .success(),
            "{}: --native accepted it",
            src
        );
        assert!(
            !run(trackc(), &["build", "--rt=track", src, "-o", exe], None)
                .status
                .success(),
            "{}: --rt=track accepted it",
            src
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

// Tuples stored in a struct, array, union or tuple, returned, or built in
// a loop outlive the frame that made them (docs/bootstrap.md "Value model").
#[test]
fn native_tuples_outlive_their_frame() {
    let dir = scratch("tuples");
    let expect = [
        ("tup_arr", "200\n5\n"),
        ("tup_loop", "0\n20\n"),
        ("tup_nested", "200\n6\n"),
        ("tup_struct", "200\n8\n"),
        ("tup_struct2", "30\nxy\n8\n"),
        ("tup_union", "200\n6\n"),
    ];
    for (name, want) in expect {
        let exe = dir.join(name);
        let src = format!("tests/native_tuples/{}.trk", name);
        let out = run(
            trackc(),
            &["build", "--native", &src, "-o", exe.to_str().unwrap()],
            None,
        );
        assert!(
            out.status.success(),
            "{}: {}",
            name,
            String::from_utf8_lossy(&out.stderr)
        );
        let (stdout, _, code) = observe(&exe);
        assert_eq!((stdout.as_str(), code), (want, Some(0)), "{}", name);
    }
    let _ = fs::remove_dir_all(&dir);
}

// Identical string literals are one object, as in the C backend.
#[test]
fn native_identical_string_literals_compare_equal() {
    let dir = scratch("strlit");
    let exe = dir.join("s");
    let out = run(
        trackc(),
        &[
            "build",
            "--native",
            "tests/native/strlit_eq.trk",
            "-o",
            exe.to_str().unwrap(),
        ],
        None,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let (stdout, _, _) = observe(&exe);
    assert!(stdout.starts_with("1\n"), "{:?}", stdout);
    let _ = fs::remove_dir_all(&dir);
}

// A program with no `main` builds and exits 0, as on the C path.
#[test]
fn native_program_without_main_builds() {
    let dir = scratch("nomain");
    for name in ["nomain", "nomain_lib"] {
        let exe = dir.join(name);
        let src = format!("tests/native/{}.trk", name);
        let out = run(
            trackc(),
            &["build", "--native", &src, "-o", exe.to_str().unwrap()],
            None,
        );
        assert!(
            out.status.success(),
            "{}: {}",
            name,
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            observe(&exe),
            (String::new(), String::new(), Some(0)),
            "{}",
            name
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

// `--native` works after the file and after `-o` too; with no C compiler on
// PATH only the native backend can succeed.
#[test]
fn native_flag_is_honoured_in_any_position() {
    let dir = scratch("flagpos");
    let exe = dir.join("h");
    let exe = exe.to_str().unwrap();
    let orders: [&[&str]; 3] = [
        &[
            "build",
            "tests/native_raw/hello_syscall.trk",
            "--native",
            "-o",
            exe,
        ],
        &[
            "build",
            "tests/native_raw/hello_syscall.trk",
            "-o",
            exe,
            "--native",
        ],
        &[
            "build",
            "-o",
            exe,
            "--native",
            "tests/native_raw/hello_syscall.trk",
        ],
    ];
    for args in orders {
        let _ = fs::remove_file(exe);
        let out = run(trackc(), args, Some("/nonexistent"));
        assert!(
            out.status.success(),
            "{:?}: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(observe(Path::new(exe)).2, Some(42), "{:?}", args);
    }
    let _ = fs::remove_dir_all(&dir);
}

// Native executables carry PT_GNU_STACK (read/write, not executable).
#[test]
fn native_executables_have_a_non_executable_stack() {
    let dir = scratch("gnustack");
    let exe = dir.join("h");
    let out = run(
        trackc(),
        &[
            "build",
            "--native",
            "tests/native_raw/hello_syscall.trk",
            "-o",
            exe.to_str().unwrap(),
        ],
        None,
    );
    assert!(out.status.success());
    let img = fs::read(&exe).unwrap();
    let u16_at = |o: usize| u16::from_le_bytes([img[o], img[o + 1]]) as usize;
    let u32_at = |o: usize| u32::from_le_bytes(img[o..o + 4].try_into().unwrap());
    let phoff = u64::from_le_bytes(img[32..40].try_into().unwrap()) as usize;
    let (entsize, count) = (u16_at(54), u16_at(56));
    let stack = (0..count)
        .map(|i| phoff + i * entsize)
        .find(|&h| u32_at(h) == 0x6474_e551)
        .expect("no PT_GNU_STACK program header");
    assert_eq!(u32_at(stack + 4), 6, "PT_GNU_STACK flags must be RW");
    let _ = fs::remove_dir_all(&dir);
}

// K3b: programs both backends once got wrong the same way, so parity alone
// missed them. Each must print `stdout` and end with `code` (or be killed
// by signal `sig`) under the C backend and natively.
const K3B_PROGRAMS: [(&str, &str, Option<i32>, Option<i32>); 5] = [
    (
        "for_in",
        "10\n20\n30\n3\n2\n3\n4\n10\n20\n30\n110\n120\n130\nab\ncde\n60\n11\n21\n31\n",
        Some(0),
        None,
    ),
    ("exit_flush", "1\n", Some(3), None),
    ("divzero_flush", "1\n", None, Some(8)),
    // K3c: INT64_MIN / -1 and % -1 trap like a zero divisor (flush, SIGFPE).
    (
        "divmin_flush",
        "-7\n0\n-9223372036854775807\n",
        None,
        Some(8),
    ),
    ("divmin_mod_flush", "0\n", None, Some(8)),
];

#[test]
fn native_and_c_print_the_expected_output() {
    use std::os::unix::process::ExitStatusExt;
    let dir = scratch("k3b");
    for (stem, stdout, code, sig) in K3B_PROGRAMS {
        let src = format!("tests/native/{}.trk", stem);
        for mode in ["c", "native"] {
            let exe = dir.join(format!("{}_{}", stem, mode));
            let mut args = vec!["build"];
            if mode == "native" {
                args.push("--native");
            }
            args.extend([src.as_str(), "-o", exe.to_str().unwrap()]);
            let out = run(trackc(), &args, None);
            assert!(
                out.status.success(),
                "{} {}: build failed: {}",
                stem,
                mode,
                String::from_utf8_lossy(&out.stderr)
            );
            let out = Command::new(&exe).output().unwrap();
            assert_eq!(
                (
                    String::from_utf8_lossy(&out.stdout).to_string(),
                    out.status.code(),
                    out.status.signal()
                ),
                (stdout.to_string(), code, sig),
                "{} {}: wrong behaviour",
                stem,
                mode
            );
        }
    }
    let _ = fs::remove_dir_all(&dir);
}
