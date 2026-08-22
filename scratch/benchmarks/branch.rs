fn main(){
    let mut sum: i64=0;
    for i in 0..50000000i64 { if i & 1 == 0 { sum+=i; } else { sum-=i; } }
    println!("{}", sum);
}
