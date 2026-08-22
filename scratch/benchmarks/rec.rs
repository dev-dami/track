fn fib(n: i64) -> i64 { let n = std::hint::black_box(n); if n<=1 {return n;} std::hint::black_box(fib(n-1)+fib(n-2)) }
fn main(){ let r = std::hint::black_box(fib(38)); println!("{}", r); }
