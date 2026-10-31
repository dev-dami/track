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
