fn main(){
    let mut sum: i64=0;
    for i in 0..50000000i64 { if i & 1 == 0 { sum = std::hint::black_box(sum + i); } else { sum = std::hint::black_box(sum - i); } }
    println!("{}", std::hint::black_box(sum));
}
