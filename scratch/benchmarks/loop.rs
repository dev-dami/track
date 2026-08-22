fn main() {
    let mut sum: i64 = 0;
    for i in 0..100000000i64 { sum = std::hint::black_box(sum + (i & 7)); }
    println!("{}", std::hint::black_box(sum));
}
