fn main(){
    let mut sum: i64=0;
    for i in 0..10000000i64 { let t=std::hint::black_box((i,i+1)); let (a,b)=t; sum = std::hint::black_box(sum + a + b); }
    println!("{}", std::hint::black_box(sum));
}
