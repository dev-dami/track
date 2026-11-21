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
        &["build", "--native", "tests/native_raw/hello_syscall.trk", "-o", exe.to_str().unwrap()],
        Some("/nonexistent"),
    );
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
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
    for sub in ["examples", "tests/native"] {
        for e in fs::read_dir(repo().join(sub)).unwrap() {
            let p = e.unwrap().path();
            let stem = p.file_stem().unwrap().to_string_lossy().to_string();
            // net_demo needs sockets, which the stub runtime lacks.
            if p.extension().is_some_and(|x| x == "trk") && !stem.ends_with("_err") && stem != "net_demo" {
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
        let c = run(trackc(), &["build", src, "-o", c_exe.to_str().unwrap()], None);
        assert!(c.status.success(), "{}: C build failed", stem);
        let n = run(trackc(), &["build", "--native", src, "-o", n_exe.to_str().unwrap()], None);
        assert!(n.status.success(), "{}: native build failed: {}", stem, String::from_utf8_lossy(&n.stderr));
        let (c_out, _, c_code) = observe(&c_exe);
        let (n_out, _, n_code) = observe(&n_exe);
        assert_eq!((c_out, c_code), (n_out, n_code), "{}: behaviour differs", stem);
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn native_compiler_reaches_a_fixed_point() {
    let dir = scratch("fixpoint");
    let n2 = dir.join("trackc2");
    let n3 = dir.join("trackc3");
    let out = run(trackc(), &["build", "--native", "compiler/src/main.trk", "-o", n2.to_str().unwrap()], None);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let out = run(&n2, &["build", "--native", "compiler/src/main.trk", "-o", n3.to_str().unwrap()], Some("/nonexistent"));
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(fs::read(&n2).unwrap() == fs::read(&n3).unwrap(), "native stage2 and stage3 differ");
    let _ = fs::remove_dir_all(&dir);
}
