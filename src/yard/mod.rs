pub mod manifest;
pub mod commands;
pub mod resolver;

pub fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_usage();
        return Ok(());
    }
    match args[0].as_str() {
        "init" => commands::init(&args[1..]),
        "build" => commands::build(&args[1..]),
        "run" => commands::run_cmd(&args[1..]),
        "add" => commands::add(&args[1..]),
        "check" => commands::check(&args[1..]),
        "--help" | "-h" | "help" => { print_usage(); Ok(()) },
        other => Err(format!("Unknown yard command: '{}'", other)),
    }
}

fn print_usage() {
    eprintln!("Yard - Track Package Manager");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    track yard <command> [options]");
    eprintln!();
    eprintln!("COMMANDS:");
    eprintln!("    init <name>     Create a new Track project");
    eprintln!("    build           Build the current project");
    eprintln!("    run             Build and run the current project");
    eprintln!("    add <pkg>       Add a dependency");
    eprintln!("    check           Check the project for errors");
    eprintln!("    help            Show this help");
}
