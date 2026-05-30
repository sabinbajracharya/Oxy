use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process;

use colored::Colorize;
use oxy_core::errors::{CallFrame, FerriError};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("--version" | "-V") => {
            println!("{}", oxy_core::version_string());
        }
        Some("--help" | "-h") | None => {
            print_help();
        }
        Some("run") => {
            let (externs, file, script_args) = match parse_subcmd_args(&args[2..], "run") {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{} {}", "error:".red().bold(), e);
                    process::exit(2);
                }
            };
            let mut script_argv = vec![file.clone()];
            script_argv.extend(script_args);
            oxy_core::stdlib::env::set_cli_args(script_argv);
            run_file(&file, externs);
        }
        Some("repl") => {
            run_repl();
        }
        Some("test") => {
            let (externs, file, _) = match parse_subcmd_args(&args[2..], "test") {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("{} {}", "error:".red().bold(), e);
                    process::exit(2);
                }
            };
            run_test_file(&file, externs);
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
        // `--dump-bytecode` is the legacy name kept as a hidden alias; Oxy
        // compiles through a register IR, not bytecode.
        Some(flag @ ("--dump-ir" | "--dump-bytecode")) => {
            let file = args.get(2).unwrap_or_else(|| {
                eprintln!("{} {flag} requires a file argument", "error:".red().bold());
                process::exit(2);
            });
            dump_ir(file);
        }
        Some(cmd) => {
            eprintln!("{} unknown command '{cmd}'", "error:".red().bold());
            eprintln!("Run 'oxy --help' for usage information.");
            process::exit(2);
        }
    }
}

fn run_file(path: &str, externs: HashMap<String, PathBuf>) {
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

    // Run on the bytecode VM. If `main` is declared as `-> Result<_, _>`
    // (or `-> Option<_>`) and returns `Err(_)` / `None`, surface that to
    // the user — otherwise `?` in main would silently exit 0.
    match oxy_core::vm::run_compiled_with_options(&source, Some(path), externs) {
        Ok(v) => {
            if let Some(msg) = main_error_message(&v) {
                eprintln!("{} {}", "error:".red().bold(), msg);
                process::exit(1);
            }
        }
        Err(e) => {
            display_error(&e, &source, &[]);
            process::exit(1);
        }
    }
}

type ParsedSubcmd = (HashMap<String, PathBuf>, String, Vec<String>);

/// Parse `[--extern name=path]... <file> [-- args...]` and return
/// `(externs, file, script_args)`. Errors if no file argument is given.
fn parse_subcmd_args(args: &[String], cmd: &str) -> Result<ParsedSubcmd, String> {
    let mut externs: HashMap<String, PathBuf> = HashMap::new();
    let mut file: Option<String> = None;
    let mut script_args: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--extern" {
            let kv = args
                .get(i + 1)
                .ok_or_else(|| "--extern requires a name=path argument".to_string())?;
            let (name, path) = kv
                .split_once('=')
                .ok_or_else(|| format!("--extern expects name=path, got '{kv}'"))?;
            if name.is_empty() {
                return Err("--extern name cannot be empty".to_string());
            }
            externs.insert(name.to_string(), PathBuf::from(path));
            i += 2;
            continue;
        }
        if let Some(rest) = a.strip_prefix("--extern=") {
            let (name, path) = rest
                .split_once('=')
                .ok_or_else(|| format!("--extern expects name=path, got '{rest}'"))?;
            if name.is_empty() {
                return Err("--extern name cannot be empty".to_string());
            }
            externs.insert(name.to_string(), PathBuf::from(path));
            i += 1;
            continue;
        }
        // First non-flag argument is the file; everything after goes to the script.
        if file.is_none() {
            file = Some(a.clone());
            script_args = args[i + 1..].to_vec();
            break;
        }
        i += 1;
    }
    let file = file.ok_or_else(|| format!("'{cmd}' requires a file argument"))?;
    Ok((externs, file, script_args))
}

/// If main returned `Result::Err(_)` or `Option::None`, format the carried
/// value as a user-facing error message. `Ok(_)` / `Some(_)` / any other
/// return value yields `None` (nothing to report).
fn main_error_message(v: &oxy_core::types::Value) -> Option<String> {
    use oxy_core::types::Value;
    match v {
        Value::EnumVariant {
            enum_name,
            variant,
            data,
        } if enum_name == "Result" && variant == "Err" => {
            let inner = data.first().map(|d| d.to_string()).unwrap_or_default();
            Some(format!("main returned Err({inner})"))
        }
        Value::EnumVariant {
            enum_name, variant, ..
        } if enum_name == "Option" && variant == "None" => Some("main returned None".to_string()),
        _ => None,
    }
}

fn run_repl() {
    println!("{}", oxy_core::version_string());
    println!(
        "Type {} for help, {} to exit.\n",
        ":help".cyan(),
        ":quit".cyan()
    );

    // Accumulate items (fn, struct, etc.) in a persistent source buffer
    let mut source_buffer = String::new();

    // Try to use rustyline for a better REPL experience
    let history_path = dirs_home().map(|h| format!("{}/.oxy_history", h));
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
            match editor.readline("ox> ") {
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
            print!("ox> ");
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
                println!("Enter Oxy code directly. Items (fn, struct, enum, impl,");
                println!("trait, type, const, use, mod, pub, #[...]) accumulate in a");
                println!("persistent source buffer. Expressions execute immediately");
                println!("via the bytecode VM.");
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

            // Validate by parsing
            match oxy_core::parser::parse(&input) {
                Ok(_) => {
                    source_buffer.push_str(&input);
                    source_buffer.push('\n');
                    println!("  (registered)");
                }
                Err(e) => display_error(&e, &input, &[]),
            }
            continue;
        }

        // Expression/statement: compile and run via VM
        let source = format!("{}fn main() {{ {trimmed} }}\n", source_buffer);
        match oxy_core::vm::run_compiled(&source) {
            Ok(val) => {
                if val != oxy_core::types::Value::Unit {
                    println!("{val}");
                }
            }
            Err(e) => display_error(&e, trimmed, &[]),
        };
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

fn run_test_file(path: &str, externs: HashMap<String, PathBuf>) {
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

    match oxy_core::vm::run_tests_with_options(path, &source, externs) {
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

    match oxy_core::lexer::tokenize(&source) {
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

    match oxy_core::parser::parse(&source) {
        Ok(program) => {
            print!("{}", program.pretty_print());
        }
        Err(e) => {
            display_error(&e, &source, &[]);
            process::exit(1);
        }
    }
}

fn dump_ir(path: &str) {
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

    match oxy_core::vm::disassemble_source(path, &source) {
        Ok(output) => print!("{output}"),
        Err(e) => {
            display_error(&e, &source, &[]);
            process::exit(1);
        }
    }
}

fn print_help() {
    println!("{}", oxy_core::version_string());
    println!("{}\n", "Rust syntax, scripting freedom.".italic());
    println!("{} oxy [command] [options]\n", "Usage:".bold());
    println!("{}:", "Commands".bold());
    println!(
        "  {}   Execute an Oxy source file",
        "run [--extern N=P]... <file.ox> [args...]".cyan()
    );
    println!(
        "  {}            Run #[test] functions in a file",
        "test [--extern N=P]... <file.ox>".cyan()
    );
    println!(
        "  {}                                  Start the interactive REPL\n",
        "repl".cyan()
    );
    println!(
        "{} package management lives in {}.\n",
        "Note:".dimmed(),
        "tug".cyan()
    );
    println!("{}:", "Options".bold());
    println!(
        "  {}             Inject a module by name (mirrors rustc's --extern)",
        "--extern <name>=<path>".cyan()
    );
    println!(
        "  {}                   Dump token stream for a file",
        "--dump-tokens <file>".cyan()
    );
    println!("  {}    Dump AST for a file", "--dump-ast <file>".cyan());
    println!(
        "  {}       Dump the lowered register IR for a file",
        "--dump-ir <file>".cyan()
    );
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
