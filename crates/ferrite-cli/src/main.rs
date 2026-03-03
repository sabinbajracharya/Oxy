use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("--version" | "-V") => {
            println!("{}", ferrite_core::version_string());
        }
        Some("--help" | "-h") | None => {
            print_help();
        }
        Some(cmd) => {
            eprintln!("error: unknown command '{cmd}'");
            eprintln!("Run 'ferrite --help' for usage information.");
            process::exit(2);
        }
    }
}

fn print_help() {
    println!("{}", ferrite_core::version_string());
    println!("Rust syntax, scripting freedom.\n");
    println!("Usage: ferrite [command] [options]\n");
    println!("Commands:");
    println!("  run <file.fe>    Execute a Ferrite source file");
    println!("  repl             Start the interactive REPL\n");
    println!("Options:");
    println!("  -V, --version    Print version information");
    println!("  -h, --help       Print this help message");
}
