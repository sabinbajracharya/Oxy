use std::io::{self, BufRead, Write};
use std::process;

use ferrite_core::interpreter::Interpreter;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("--version" | "-V") => {
            println!("{}", ferrite_core::version_string());
        }
        Some("--help" | "-h") | None => {
            print_help();
        }
        Some("run") => {
            let file = args.get(2).unwrap_or_else(|| {
                eprintln!("error: 'run' requires a file argument");
                process::exit(2);
            });
            run_file(file);
        }
        Some("repl") => {
            run_repl();
        }
        Some("--dump-tokens") => {
            let file = args.get(2).unwrap_or_else(|| {
                eprintln!("error: --dump-tokens requires a file argument");
                process::exit(2);
            });
            dump_tokens(file);
        }
        Some("--dump-ast") => {
            let file = args.get(2).unwrap_or_else(|| {
                eprintln!("error: --dump-ast requires a file argument");
                process::exit(2);
            });
            dump_ast(file);
        }
        Some(cmd) => {
            eprintln!("error: unknown command '{cmd}'");
            eprintln!("Run 'ferrite --help' for usage information.");
            process::exit(2);
        }
    }
}

fn run_file(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read file '{path}': {e}");
            process::exit(1);
        }
    };

    // Collect program args: everything after the file argument
    let all_args: Vec<String> = std::env::args().collect();
    let cli_args: Vec<String> = if all_args.len() > 3 {
        all_args[3..].to_vec()
    } else {
        vec![]
    };
    // Prepend program name
    let mut program_args = vec![path.to_string()];
    program_args.extend(cli_args);

    match ferrite_core::interpreter::run_file_with_args(path, &source, program_args) {
        Ok(_) => {}
        Err(e) => {
            display_error(&e, &source);
            process::exit(1);
        }
    }
}

fn run_repl() {
    println!("{}", ferrite_core::version_string());
    println!("Type :help for help, :quit to exit.\n");

    let mut interp = Interpreter::new();
    let stdin = io::stdin();
    let stdout = io::stdout();

    loop {
        print!("fe> ");
        stdout.lock().flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("error reading input: {e}");
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match trimmed {
            ":quit" | ":q" => break,
            ":help" | ":h" => {
                println!("Commands:");
                println!("  :help, :h    Show this help");
                println!("  :quit, :q    Exit the REPL");
                println!();
                println!("Enter expressions or function definitions.");
                continue;
            }
            _ => {}
        }

        // Try to parse as function definition
        if trimmed.starts_with("fn ") {
            // Accumulate multi-line input for function definitions
            let mut input = line.clone();
            while !balanced_braces(&input) {
                print!("... ");
                stdout.lock().flush().unwrap();
                let mut more = String::new();
                match stdin.lock().read_line(&mut more) {
                    Ok(0) => break,
                    Ok(_) => input.push_str(&more),
                    Err(_) => break,
                }
            }

            match ferrite_core::parser::parse(&input) {
                Ok(program) => {
                    for item in &program.items {
                        if let Err(e) = interp.register_item(item) {
                            eprintln!("error: {e}");
                        }
                    }
                }
                Err(e) => eprintln!("error: {e}"),
            }
            continue;
        }

        // Otherwise, wrap as expression/statement in a synthetic function
        let wrapped = format!("fn __repl__() {{ {trimmed} }}");
        match ferrite_core::parser::parse(&wrapped) {
            Ok(program) => {
                // Extract the body of __repl__ and execute statements directly
                if let Some(ferrite_core::ast::Item::Function(f)) = program.items.first() {
                    for stmt in &f.body.stmts {
                        match interp.execute_stmt(stmt) {
                            Ok(val) => {
                                if val != ferrite_core::types::Value::Unit {
                                    println!("{val}");
                                }
                            }
                            Err(e) => {
                                eprintln!("error: {e}");
                            }
                        }
                    }
                }
            }
            Err(e) => eprintln!("error: {e}"),
        }
    }
}

/// Check if braces are balanced (simple heuristic for multi-line input).
fn balanced_braces(s: &str) -> bool {
    let mut depth = 0i32;
    for ch in s.chars() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
    }
    depth <= 0
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

fn dump_ast(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read file '{path}': {e}");
            process::exit(1);
        }
    };

    match ferrite_core::parser::parse(&source) {
        Ok(program) => {
            print!("{}", program.pretty_print());
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
    println!("  --dump-ast <file>    Dump AST for a file");
    println!("  -V, --version        Print version information");
    println!("  -h, --help           Print this help message");
}

/// Display an error with source context when possible.
fn display_error(err: &ferrite_core::errors::FerriError, source: &str) {
    use ferrite_core::errors::FerriError;
    match err {
        FerriError::Runtime { line, column, .. }
        | FerriError::Parser { line, column, .. }
        | FerriError::Lexer { line, column, .. } => {
            eprintln!("error: {err}");
            if *line > 0 {
                let lines: Vec<&str> = source.lines().collect();
                if let Some(src_line) = lines.get(line - 1) {
                    eprintln!("  --> line {line}:{column}");
                    eprintln!("   |");
                    eprintln!("{line:>4} | {src_line}");
                    if *column > 0 {
                        let caret = " ".repeat(*column - 1);
                        eprintln!("   | {caret}^-- here");
                    }
                }
            }
        }
        _ => eprintln!("error: {err}"),
    }
}
