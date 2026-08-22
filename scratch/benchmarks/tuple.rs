fn main(){
    let mut sum: i64=0;
    for i in 0..10000000i64 { let t=(i,i+1); let (a,b)=t; sum += a + b; }
    println!("{}", sum);
}
