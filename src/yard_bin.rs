use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match track::yard::run(&args) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("yard error: {}", e);
            process::exit(1);
        }
    }
}
