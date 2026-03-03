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
        Some("--dump-tokens") => {
            let file = args.get(2).unwrap_or_else(|| {
                eprintln!("error: --dump-tokens requires a file argument");
                process::exit(2);
            });
            dump_tokens(file);
        }
        Some(cmd) => {
            eprintln!("error: unknown command '{cmd}'");
            eprintln!("Run 'ferrite --help' for usage information.");
            process::exit(2);
        }
    }
}

fn dump_tokens(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read file '{path}': {e}");
            process::exit(1);
        }
    };

    match ferrite_core::lexer::tokenize(&source) {
        Ok(tokens) => {
            for token in &tokens {
                println!(
                    "{:>4}:{:<3} {:?}",
                    token.span.line, token.span.column, token.kind
                );
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    }
}

fn print_help() {
    println!("{}", ferrite_core::version_string());
    println!("Rust syntax, scripting freedom.\n");
    println!("Usage: ferrite [command] [options]\n");
    println!("Commands:");
    println!("  run <file.fe>        Execute a Ferrite source file");
    println!("  repl                 Start the interactive REPL\n");
    println!("Options:");
    println!("  --dump-tokens <file> Dump token stream for a file");
    println!("  -V, --version        Print version information");
    println!("  -h, --help           Print this help message");
}
