// Runtime semantics of the Cranelift backend. Each case builds a program,
// runs it, and compares stdout. These cover miscompilations found while
// bootstrapping the self-hosted compiler.

use std::env;
use std::fs;
use std::process::Command;
use track::build_file_in_dir;

fn run(name: &str, source: &str, args: &[&str]) -> String {
    let temp_dir = env::temp_dir().join(format!("track_sem_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();
    let src_file = temp_dir.join(format!("{}.trk", name));
    fs::write(&src_file, source).unwrap();
    let exe = build_file_in_dir(src_file.to_str().unwrap(), &temp_dir).unwrap();
    let output = Command::new(&exe).args(args).output().unwrap();
    let _ = fs::remove_dir_all(&temp_dir);
    assert!(
        output.status.success(),
        "{} exited with {:?}: {}",
        name,
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// K3b: executables are PIE (ELF type ET_DYN). Non-PIE put the brk heap a
/// random 0..32 MiB above the image, so `print(count)` sometimes found a
/// heap string at address `count` and printed it (bench/progs/tokenize.trk
/// printed `9036 = bar + 1814332 * (baz - 1);` for 25988585 in ~1 run in 40).
#[test]
fn executables_are_position_independent() {
    let temp_dir = env::temp_dir().join(format!("track_sem_pie_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();
    let src = temp_dir.join("pie.trk");
    fs::write(&src, "fn main() -> i64 { print(25988585); return 0; }\n").unwrap();
    let exe = build_file_in_dir(src.to_str().unwrap(), &temp_dir).unwrap();
    let image = fs::read(&exe).unwrap();
    let out = Command::new(&exe).output().unwrap();
    let _ = fs::remove_dir_all(&temp_dir);
    assert_eq!(u16::from_le_bytes([image[16], image[17]]), 3, "not a PIE");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "25988585\n");
}

/// K3b: `for x in a` binds the elements of array `a`, not 0, 1, 2, ...
/// forever; ranges count, empty arrays skip the body, loops nest.
#[test]
fn for_in_binds_array_elements() {
    let source = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/native/for_in.trk"),
    )
    .unwrap();
    let out = run("for_in", &source, &[]);
    assert_eq!(
        out,
        "10\n20\n30\n3\n2\n3\n4\n10\n20\n30\n110\n120\n130\nab\ncde\n60\n11\n21\n31"
    );
}

#[test]
fn program_arguments_reach_os_args() {
    let out = run(
        "argv",
        r#"
        fn main() -> void {
            print(os_args_count());
            let first = os_arg(1);
            print(first);
        }
        "#,
        &["hello"],
    );
    assert_eq!(out, "2\nhello");
}

#[test]
fn logical_operators_short_circuit_and_normalise() {
    let out = run(
        "logic",
        r#"
        fn loud(v: i64) -> i64 {
            print(v);
            return v;
        }
        fn main() -> void {
            let x: i64 = 1;
            let y: i64 = 2;
            if x && y { print(10); } else { print(11); }
            if loud(0) && loud(5) { print(12); }
            if loud(3) || loud(6) { print(13); }
            let z = x && y;
            print(z);
        }
        "#,
        &[],
    );
    assert_eq!(out, "10\n0\n3\n13\n1");
}

#[test]
fn return_inside_match_while_and_nested_if() {
    let out = run(
        "returns",
        r#"
        fn pick(x: i64) -> i64 {
            match x {
                1 => return 10,
                _ => return 20,
            }
            return 0;
        }
        fn first_over(limit: i64) -> i64 {
            let mut i: i64 = 0;
            while i < 100 {
                if i * i > limit {
                    return i;
                }
                i = i + 1;
            }
            return -1;
        }
        fn nested(x: i64) -> i64 {
            if x > 0 {
                if x > 5 { return 7; } else { return 8; }
            }
            return 9;
        }
        fn loop_return(x: i64) -> i64 {
            while x > 0 {
                return 5;
            }
            return 6;
        }
        fn main() -> void {
            print(pick(1)); print(pick(4));
            print(first_over(50));
            print(nested(10)); print(nested(2)); print(nested(-1));
            print(loop_return(1)); print(loop_return(0));
        }
        "#,
        &[],
    );
    assert_eq!(out, "10\n20\n8\n7\n8\n9\n5\n6");
}

#[test]
fn if_and_match_produce_values() {
    let out = run(
        "values",
        r#"
        fn sign(x: i64) -> i64 {
            let s = if x < 0 { -1 } else if x == 0 { 0 } else { 1 };
            return s;
        }
        fn name(x: i64) -> i64 {
            let v = match x { 1 => 100, 2 => 200, _ => 300, };
            return v;
        }
        fn main() -> void {
            print(sign(-4)); print(sign(0)); print(sign(9));
            print(name(2)); print(name(7));
        }
        "#,
        &[],
    );
    assert_eq!(out, "-1\n0\n1\n200\n300");
}

#[test]
fn returned_tuple_variables_do_not_dangle() {
    let out = run(
        "tuples",
        r#"
        type Pair = (i64, i64);
        fn make(a: i64) -> Pair {
            let t = (a, a + 1);
            return t;
        }
        fn clobber() -> i64 {
            let z = (9, 9, 9, 9, 9, 9, 9, 9);
            return z.0;
        }
        fn main() -> void {
            let p = make(5);
            let q = clobber();
            print(p.0); print(p.1); print(q);
        }
        "#,
        &[],
    );
    assert_eq!(out, "5\n6\n9");
}

#[test]
fn string_patterns_compare_contents() {
    let out = run(
        "strpat",
        r#"
        fn main() -> void {
            let s = str_concat("he", "llo");
            match s {
                "world" => print(0),
                "hello" => print(1),
                _ => print(2),
            }
        }
        "#,
        &[],
    );
    assert_eq!(out, "1");
}

#[test]
fn union_payloads_and_enum_discriminants() {
    let out = run(
        "unions",
        r#"
        union Value { Int(i64), Flag(bool), Nothing }
        enum Code { A = 5, B, C = 10 }
        fn describe(v: Value) -> i64 {
            let mut out: i64 = -1;
            match v {
                Value::Int(n) => out = n,
                Value::Flag(f) => out = 1000 + f,
                Value::Nothing => out = 0,
            }
            return out;
        }
        fn main() -> void {
            print(describe(Value::Int(42)));
            print(describe(Value::Flag(true)));
            print(describe(Value::Nothing));
            print(Code::A); print(Code::B); print(Code::C);
        }
        "#,
        &[],
    );
    assert_eq!(out, "42\n1001\n0\n5\n6\n10");
}

/// Stage0 folds constants like symbols.trk const_eval: comparisons, `!`,
/// `&&` and `||` give 0/1, and a unit variant is a constant to later ones.
#[test]
fn constants_fold_comparisons_and_variants() {
    let out = run(
        "constfold",
        r#"
        const ONE = 1 < 2;
        enum E { A = ONE, B }
        const Q = E::B + 10;
        const N = !ONE || (3 >= 3);
        fn main() -> void {
            print(ONE); print(E::A); print(E::B); print(Q); print(N);
            print(7 / (1 < 2));
        }
        "#,
        &[],
    );
    assert_eq!(out, "1\n1\n2\n12\n1\n7");
}

#[test]
fn negative_runtime_results_keep_their_sign() {
    let out = run(
        "signs",
        r#"
        fn main() -> void {
            let r = file_remove("/definitely/not/here/x");
            print(r);
            let c = str_char_at("abc", 1);
            print(c);
        }
        "#,
        &[],
    );
    assert_eq!(out, "-1\n98");
}

#[test]
fn block_bindings_do_not_leak_into_the_enclosing_scope() {
    let out = run(
        "scopes",
        r#"
        fn main() -> void {
            let x = 10;
            if true {
                let x = 20;
                print(x);
            }
            print(x);
            match 1 {
                x => print(x),
            }
            print(x);
        }
        "#,
        &[],
    );
    assert_eq!(out, "20\n10\n1\n10");
}

#[test]
fn structs_and_arrays_have_runtime_values() {
    let out = run(
        "structs",
        r#"
        struct Point { x: i64, y: i64 }
        struct Size { w: i64, x: i64 }
        fn make(n: i64) -> Point { let p = Point { y: n + 1, x: n }; return p; }
        fn main() -> void {
            let mut p = make(3);
            print(p.x); print(p.y);
            p.x = 10;
            let s = Size { x: 8, w: 7 };
            print(p.x + s.x);
            match p { Point { x: 10, y } => print(y), _ => print(0), }
            let mut a = [1, 2, 3, 4];
            a[3] = 40;
            let tail = a[2..];
            print(array_len(&tail)); print(tail[1]);
            let mut t = (1, 2);
            t.1 = 9;
            print(t.1);
        }
        "#,
        &[],
    );
    assert_eq!(out, "3\n4\n18\n4\n2\n40\n9");
}

#[test]
fn array_index_out_of_bounds_aborts() {
    let temp_dir = env::temp_dir().join(format!("track_sem_bounds_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();
    let src = temp_dir.join("bounds.trk");
    fs::write(&src, "fn main() -> void { let a = [1, 2]; print(a[2]); }").unwrap();
    let exe = build_file_in_dir(src.to_str().unwrap(), &temp_dir).unwrap();
    let output = Command::new(&exe).output().unwrap();
    let _ = fs::remove_dir_all(&temp_dir);
    assert_eq!(output.status.code(), Some(134));
    assert!(String::from_utf8_lossy(&output.stderr).contains("array index out of bounds"));
}

/// Build and run `source`; return (stdout, terminating signal).
fn run_signal(name: &str, source: &str) -> (String, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let temp_dir = env::temp_dir().join(format!("track_sem_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();
    let src_file = temp_dir.join(format!("{}.trk", name));
    fs::write(&src_file, source).unwrap();
    let exe = build_file_in_dir(src_file.to_str().unwrap(), &temp_dir).unwrap();
    let output = Command::new(&exe).output().unwrap();
    let _ = fs::remove_dir_all(&temp_dir);
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        output.status.signal(),
    )
}

/// K3c: a zero divisor (`/` or `%`, variable or constant) flushes stdout
/// and raises SIGFPE through `track_trap_div`, like trackc's C and native
/// builds, instead of losing the output to Cranelift's SIGILL trap.
#[test]
fn division_by_zero_flushes_and_raises_sigfpe() {
    let cases = [
        (
            "divz",
            "fn main()->i64{ print(1); let mut i=3; while i>=0 { print(100/i); i=i-1; } return 0; }",
            "1\n33\n50\n100\n",
        ),
        (
            "modz",
            "fn main()->i64{ print(7); let z = 0; print(5 % z); return 0; }",
            "7\n",
        ),
        (
            "constz",
            "fn main()->i64{ print(2); print(9 / 0); return 0; }",
            "2\n",
        ),
        (
            // A parameter shadowing a constant is the run-time divisor.
            "shadowz",
            "const TWO = 2; const ONE = 1 < 2; fn d(TWO: i64) -> i64 { return 7 / TWO; } fn e(ONE: i64) -> i64 { return 7 % (ONE + 1); } fn main()->i64{ print(3); print(e(1)); print(d(0)); return 0; }",
            "3\n1\n",
        ),
    ];
    for (name, src, want) in cases {
        let (out, sig) = run_signal(name, src);
        assert_eq!(out, want, "{}", name);
        assert_eq!(sig, Some(8), "{}: expected SIGFPE", name);
    }
}

/// K3c: `INT64_MIN / -1` and `INT64_MIN % -1` trap like a zero divisor
/// (flush, SIGFPE), matching trackc's `tk_div`/`tk_mod`; other -1 divisions
/// still compute.
#[test]
fn int64_min_div_minus_one_flushes_and_raises_sigfpe() {
    let ok = run(
        "negone",
        "fn main()->i64{ let m = -1; print(7 / m); print(7 % m); print(-9223372036854775807 / m); return 0; }",
        &[],
    );
    assert_eq!(ok, "-7\n0\n9223372036854775807");
    for (name, op) in [("mindiv", "/"), ("minmod", "%")] {
        let src = format!(
            "fn main()->i64{{ print(3); let m = -1; let a = -9223372036854775807 - 1; print(a {} m); return 0; }}",
            op
        );
        let (out, sig) = run_signal(name, &src);
        assert_eq!(out, "3\n", "{}", name);
        assert_eq!(sig, Some(8), "{}: expected SIGFPE", name);
    }
}
