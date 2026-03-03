use std::io::{self, BufRead, Write};
use std::process;

use colored::Colorize;
use ferrite_core::errors::{CallFrame, FerriError};
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
                eprintln!("{} 'run' requires a file argument", "error:".red().bold());
                process::exit(2);
            });
            run_file(file);
        }
        Some("repl") => {
            run_repl();
        }
        Some("test") => {
            let file = args.get(2).unwrap_or_else(|| {
                eprintln!("{} 'test' requires a file argument", "error:".red().bold());
                process::exit(2);
            });
            run_test_file(file);
        }
        Some("--dump-tokens") => {
            let file = args.get(2).unwrap_or_else(|| {
                eprintln!(
                    "{} --dump-tokens requires a file argument",
                    "error:".red().bold()
                );
                process::exit(2);
            });
            dump_tokens(file);
        }
        Some("--dump-ast") => {
            let file = args.get(2).unwrap_or_else(|| {
                eprintln!(
                    "{} --dump-ast requires a file argument",
                    "error:".red().bold()
                );
                process::exit(2);
            });
            dump_ast(file);
        }
        Some(cmd) => {
            eprintln!("{} unknown command '{cmd}'", "error:".red().bold());
            eprintln!("Run 'ferrite --help' for usage information.");
            process::exit(2);
        }
    }
}

fn run_file(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "{} could not read file '{path}': {e}",
                "error:".red().bold()
            );
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
        Err(runtime_err) => {
            display_error(&runtime_err.error, &source, &runtime_err.call_stack);
            process::exit(1);
        }
    }
}

fn run_repl() {
    println!("{}", ferrite_core::version_string());
    println!(
        "Type {} for help, {} to exit.\n",
        ":help".cyan(),
        ":quit".cyan()
    );

    let mut interp = Interpreter::new();

    // Try to use rustyline for a better REPL experience
    let history_path = dirs_home().map(|h| format!("{}/.ferrite_history", h));
    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(mut editor) => {
            if let Some(ref path) = history_path {
                let _ = editor.load_history(path);
            }
            Some(editor)
        }
        Err(_) => None,
    };

    loop {
        let line = if let Some(ref mut editor) = rl {
            match editor.readline("fe> ") {
                Ok(line) => {
                    let _ = editor.add_history_entry(&line);
                    line
                }
                Err(rustyline::error::ReadlineError::Interrupted) => continue,
                Err(rustyline::error::ReadlineError::Eof) => break,
                Err(_) => break,
            }
        } else {
            // Fallback to raw stdin
            print!("fe> ");
            io::stdout().lock().flush().unwrap();
            let mut buf = String::new();
            match io::stdin().lock().read_line(&mut buf) {
                Ok(0) => break,
                Ok(_) => buf,
                Err(_) => break,
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match trimmed {
            ":quit" | ":q" => break,
            ":help" | ":h" => {
                println!("{}:", "Commands".bold());
                println!("  {}    Show this help", ":help, :h".cyan());
                println!("  {}    Exit the REPL", ":quit, :q".cyan());
                println!();
                println!("Enter Ferrite code directly. Items (fn, struct, enum, impl,");
                println!("trait, type, const, use, mod, pub, async, #[...]) are");
                println!("registered and persist. Expressions and statements execute");
                println!("immediately. Multi-line input continues with '...' prompt");
                println!("until braces are balanced.");
                continue;
            }
            _ => {}
        }

        // Try to parse as item definition
        let is_item = trimmed.starts_with("fn ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("trait ")
            || trimmed.starts_with("pub ")
            || trimmed.starts_with("async ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("use ")
            || trimmed.starts_with("mod ")
            || trimmed.starts_with("#[");
        if is_item {
            // Accumulate multi-line input for definitions
            let mut input = line.clone();
            while !balanced_braces(&input) {
                let more = if let Some(ref mut editor) = rl {
                    match editor.readline("... ") {
                        Ok(l) => l + "\n",
                        Err(_) => break,
                    }
                } else {
                    print!("... ");
                    io::stdout().lock().flush().unwrap();
                    let mut buf = String::new();
                    match io::stdin().lock().read_line(&mut buf) {
                        Ok(0) => break,
                        Ok(_) => buf,
                        Err(_) => break,
                    }
                };
                input.push_str(&more);
            }

            match ferrite_core::parser::parse(&input) {
                Ok(program) => {
                    for item in &program.items {
                        if let Err(e) = interp.register_item(item) {
                            display_error(&e, &input, &[]);
                        }
                    }
                }
                Err(e) => display_error(&e, &input, &[]),
            }
            continue;
        }

        // Otherwise, try as expression/statement in the persistent REPL scope
        let wrapped = format!("fn __repl__() {{ {trimmed} }}");
        match ferrite_core::parser::parse(&wrapped) {
            Ok(program) => {
                if let Some(ferrite_core::ast::Item::Function(f)) = program.items.first() {
                    for stmt in &f.body.stmts {
                        match interp.execute_stmt(stmt) {
                            Ok(val) => {
                                if val != ferrite_core::types::Value::Unit {
                                    println!("{val}");
                                }
                            }
                            Err(e) => {
                                display_error(&e, trimmed, &[]);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // If wrapping as expression failed, try parsing as item directly
                // (handles edge cases like multi-line let + fn combos)
                match ferrite_core::parser::parse(trimmed) {
                    Ok(program) => {
                        for item in &program.items {
                            if let Err(e) = interp.register_item(item) {
                                display_error(&e, trimmed, &[]);
                            }
                        }
                    }
                    Err(e) => display_error(&e, trimmed, &[]),
                }
            }
        }
    }

    // Save history on exit
    if let (Some(ref mut editor), Some(ref path)) = (&mut rl, &history_path) {
        let _ = editor.save_history(path);
    }
}

/// Get the user's home directory.
fn dirs_home() -> Option<String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
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

fn run_test_file(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "{} could not read file '{path}': {e}",
                "error:".red().bold()
            );
            process::exit(1);
        }
    };

    println!("\n{} tests in {}\n", "running".bold(), path.cyan());

    match ferrite_core::interpreter::run_tests(path, &source) {
        Ok(results) => {
            let mut passed = 0;
            let mut failed = 0;

            for result in &results {
                if result.passed {
                    println!("  {} ... {}", result.name, "ok".green().bold());
                    passed += 1;
                } else {
                    println!("  {} ... {}", result.name, "FAILED".red().bold());
                    if let Some(ref err) = result.error {
                        println!("    {}", err.red());
                    }
                    failed += 1;
                }
            }

            println!();
            if failed > 0 {
                println!(
                    "{}: {} passed, {} failed",
                    "test result: FAILED".red().bold(),
                    passed,
                    failed
                );
                process::exit(1);
            } else if passed == 0 {
                println!("{}", "no tests found".yellow());
            } else {
                println!("{}: {} passed", "test result: ok".green().bold(), passed);
            }
        }
        Err(e) => {
            display_error(&e, &source, &[]);
            process::exit(1);
        }
    }
}

fn dump_tokens(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "{} could not read file '{path}': {e}",
                "error:".red().bold()
            );
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
            display_error(&e, &source, &[]);
            process::exit(1);
        }
    }
}

fn dump_ast(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "{} could not read file '{path}': {e}",
                "error:".red().bold()
            );
            process::exit(1);
        }
    };

    match ferrite_core::parser::parse(&source) {
        Ok(program) => {
            print!("{}", program.pretty_print());
        }
        Err(e) => {
            display_error(&e, &source, &[]);
            process::exit(1);
        }
    }
}

fn print_help() {
    println!("{}", ferrite_core::version_string());
    println!("{}\n", "Rust syntax, scripting freedom.".italic());
    println!("{} ferrite [command] [options]\n", "Usage:".bold());
    println!("{}:", "Commands".bold());
    println!(
        "  {}        Execute a Ferrite source file",
        "run <file.fe>".cyan()
    );
    println!(
        "  {}       Run #[test] functions in a file",
        "test <file.fe>".cyan()
    );
    println!(
        "  {}                 Start the interactive REPL\n",
        "repl".cyan()
    );
    println!("{}:", "Options".bold());
    println!(
        "  {} Dump token stream for a file",
        "--dump-tokens <file>".cyan()
    );
    println!("  {}    Dump AST for a file", "--dump-ast <file>".cyan());
    println!(
        "  {}           Print version information",
        "-V, --version".cyan()
    );
    println!(
        "  {}              Print this help message",
        "-h, --help".cyan()
    );
}

/// Display a rich error with colored output, source context, and optional stack trace.
fn display_error(err: &FerriError, source: &str, call_stack: &[CallFrame]) {
    let is_tty = atty_stderr();

    match err {
        FerriError::Runtime {
            message,
            line,
            column,
        } => {
            print_error_header("runtime error", message, *line, *column, is_tty);
            if *line > 0 {
                print_source_context(source, *line, *column, is_tty);
            }
            // Print "did you mean?" as a separate help line if embedded in message
            if message.contains("did you mean") {
                // Already in the message, extract and format as help
            }
            // Print stack trace
            if !call_stack.is_empty() {
                eprintln!();
                if is_tty {
                    eprint!("{}", "stack trace".blue().bold());
                    eprintln!("{}", " (most recent call last):".blue());
                } else {
                    eprintln!("stack trace (most recent call last):");
                }
                for frame in call_stack.iter().rev() {
                    if is_tty {
                        eprintln!(
                            "  {} `{}` {} {}:{}",
                            "in".dimmed(),
                            frame.name.yellow(),
                            "at".dimmed(),
                            frame.line,
                            frame.column,
                        );
                    } else {
                        eprintln!("{frame}");
                    }
                }
            }
        }
        FerriError::Parser {
            message,
            line,
            column,
        } => {
            print_error_header("parse error", message, *line, *column, is_tty);
            if *line > 0 {
                print_source_context(source, *line, *column, is_tty);
            }
        }
        FerriError::Lexer {
            message,
            line,
            column,
        } => {
            print_error_header("lex error", message, *line, *column, is_tty);
            if *line > 0 {
                print_source_context(source, *line, *column, is_tty);
            }
        }
        _ => {
            if is_tty {
                eprintln!("{} {err}", "error:".red().bold());
            } else {
                eprintln!("error: {err}");
            }
        }
    }
}

/// Print the error header line: `error[kind]: message`
fn print_error_header(kind: &str, message: &str, line: usize, column: usize, is_tty: bool) {
    if is_tty {
        eprintln!(
            "{}{}{}{} {}",
            "error".red().bold(),
            "[".dimmed(),
            kind.red(),
            "]".dimmed(),
            message.bold(),
        );
    } else {
        eprintln!("error[{kind}]: {message}");
    }

    if line > 0 {
        if is_tty {
            eprintln!(" {} line {}:{}", "-->".blue().bold(), line, column,);
        } else {
            eprintln!("  --> line {line}:{column}");
        }
    }
}

/// Print source context: the error line ± 1, with line numbers and an underline.
fn print_source_context(source: &str, line: usize, column: usize, is_tty: bool) {
    let lines: Vec<&str> = source.lines().collect();
    let gutter_width = format!("{}", (line + 1).min(lines.len())).len();

    // Empty gutter separator
    let gutter_sep = if is_tty {
        format!("{:>gutter_width$} {}", "", "|".blue().bold())
    } else {
        format!("{:>gutter_width$} |", "")
    };

    eprintln!("{gutter_sep}");

    // Show line before (context)
    if line >= 2 {
        if let Some(prev) = lines.get(line - 2) {
            let ln = line - 1;
            if is_tty {
                eprintln!(
                    "{:>gutter_width$} {} {}",
                    ln.to_string().dimmed(),
                    "|".blue().bold(),
                    prev.dimmed(),
                );
            } else {
                eprintln!("{ln:>gutter_width$} | {prev}");
            }
        }
    }

    // Show the error line
    if let Some(src_line) = lines.get(line - 1) {
        if is_tty {
            eprintln!(
                "{:>gutter_width$} {} {}",
                line.to_string().bold(),
                "|".blue().bold(),
                src_line,
            );
        } else {
            eprintln!("{line:>gutter_width$} | {src_line}");
        }

        // Underline / caret
        if column > 0 {
            let padding = " ".repeat(column - 1);
            if is_tty {
                eprintln!(
                    "{:>gutter_width$} {} {}{}",
                    "",
                    "|".blue().bold(),
                    padding,
                    "^-- here".cyan().bold(),
                );
            } else {
                eprintln!("{:>gutter_width$} | {padding}^-- here", "");
            }
        }
    }

    // Show line after (context)
    if let Some(next) = lines.get(line) {
        let ln = line + 1;
        if is_tty {
            eprintln!(
                "{:>gutter_width$} {} {}",
                ln.to_string().dimmed(),
                "|".blue().bold(),
                next.dimmed(),
            );
        } else {
            eprintln!("{ln:>gutter_width$} | {next}");
        }
    }

    eprintln!("{gutter_sep}");
}

/// Check if stderr is a TTY (for colored output).
fn atty_stderr() -> bool {
    use std::io::IsTerminal;
    io::stderr().is_terminal()
}
