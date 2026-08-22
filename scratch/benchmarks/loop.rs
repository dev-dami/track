fn main() {
    let mut sum: i64 = 0;
    for i in 0..100000000i64 { sum += i & 7; }
    println!("{}", sum);
}
