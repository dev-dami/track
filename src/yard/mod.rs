pub mod builder;
pub mod cache;
pub mod commands;
pub mod linker;
pub mod manifest;
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
        "lint" => commands::lint(&args[1..]),
        "clean" => commands::clean(&args[1..]),
        "--version" | "-v" | "version" => {
            println!("yard {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("Unknown yard command: '{}'", other)),
    }
}

fn print_usage() {
    eprintln!("Yard - Track Package Manager");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    yard <command> [options]");
    eprintln!();
    eprintln!("COMMANDS:");
    eprintln!("    init <name>     Create a new Track project");
    eprintln!("    build           Build the current project");
    eprintln!("    run             Build and run the current project");
    eprintln!("    add <pkg>       Add a dependency");
    eprintln!("    check           Check the project for errors");
    eprintln!("    lint            Lint the project without building");
    eprintln!("    clean           Clean target build directory");
    eprintln!("    help            Show this help");
}
