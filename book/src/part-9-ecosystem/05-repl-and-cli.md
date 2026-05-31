# The REPL and CLI

The CLI is the face of Oxy — `oxy run`, `oxy test`, `oxy repl` — the command a person actually
types. And like the VS Code extension, it does much less than its prominence suggests: its whole
job is to route a subcommand to the right entry point in `oxy-core` and surface whatever comes back,
errors in red. It is glue, intentionally thin. The REPL is the other half of this file and a
different mood entirely — it's where you play, trying a line of Oxy and seeing the result
immediately, accumulating multi-line input until it parses. The CLI is glue; the REPL is play; both
live in `crates/oxy-cli/src/main.rs`, and we'll read each in turn.

**File:** `crates/oxy-cli/src/main.rs`

---

## The CLI commands

```bash
oxy run file.ox           # compile and run a program
oxy run --extern mod=path file.ox  # with external module
oxy test file.ox          # run #[test] functions
oxy repl                  # interactive mode
oxy --dump-tokens file.ox # print the token stream
oxy --dump-ast file.ox    # print the parsed AST
oxy --dump-ir file.ox     # print the register IR
oxy --version             # print version
```

The dispatcher in `main.rs`:

```rust
match args.get(1).map(|s| s.as_str()) {
    Some("run") => run_file(&file, externs),
    Some("test") => run_test_file(&file, externs),
    Some("repl") => run_repl(),
    Some("--dump-tokens") => dump_tokens(file),
    Some("--dump-ast") => dump_ast(file),
    Some("--dump-ir") => dump_ir(file),
    _ => print_help(),
}
```

Each branch calls a function that delegates to `oxy-core`. The CLI does minimal work itself.

## `run_file`: the main path

```rust
fn run_file(path: &str, externs: HashMap<String, String>) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{} {}: {}", "error:".red().bold(), path, e);
            process::exit(1);
        }
    };

    // Register external modules
    for (name, mod_path) in &externs {
        oxy_core::register_extern(name, mod_path);
    }

    // Run through the full pipeline
    match oxy_core::api::run_compiled(&source) {
        VmResult::Value(_) => {}
        VmResult::Error(msg) => {
            eprintln!("{} {}", "error:".red().bold(), msg);
            process::exit(1);
        }
    }
}
```

Read the file, register any `--extern` modules, call `run_compiled`. The pipeline
(lex → parse → type check → ir_gen → JIT → execute) runs inside `oxy_core::api::run_compiled`.
The CLI just surfaces errors.

## `--dump-tokens`, `--dump-ast`, `--dump-ir`: debug flags

These dump intermediate representations for debugging:

```bash
oxy --dump-tokens examples/hello.ox
# Fn Ident("main") LParen RParen LBrace ...

oxy --dump-ast examples/hello.ox
# Program { items: [Function(FnDef { name: "main", ... })] }

oxy --dump-ir examples/hello.ox
# fn main: block 0: v0 = ConstString("Hello!") ...
```

Internally:
```rust
fn dump_tokens(path: &str) {
    let source = fs::read_to_string(path).unwrap();
    let tokens = oxy_core::lexer::tokenize(&source).unwrap();
    for tok in &tokens { println!("{:?}", tok.kind); }
}

fn dump_ast(path: &str) {
    let source = fs::read_to_string(path).unwrap();
    let program = oxy_core::parser::parse(&source).unwrap();
    println!("{:#?}", program);
}
```

These are the same tools the test suite uses internally. Running them manually gives you
a window into what the pipeline produces at each stage.

## The REPL: interactive mode

```rust
fn run_repl() {
    println!("Oxy REPL (type 'exit' to quit)");
    let stdin = io::stdin();
    let mut accumulated = String::new();

    loop {
        print!(if accumulated.is_empty() { ">>> " } else { "... " });
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;  // EOF
        }

        let line = line.trim_end();
        if line == "exit" || line == "quit" { break; }

        accumulated.push_str(line);
        accumulated.push('\n');

        // Try to parse and run what we have so far
        // If parse fails with "unexpected EOF", accumulate more (multi-line input)
        match try_run_repl_input(&accumulated) {
            ReplResult::Ok(val) => {
                if !matches!(val, Value::Unit) {
                    println!("= {}", val);
                }
                accumulated.clear();
            }
            ReplResult::Incomplete => {}  // wait for more input
            ReplResult::Err(msg) => {
                eprintln!("error: {msg}");
                accumulated.clear();
            }
        }
    }
}
```

The REPL wraps input in `fn main() { ... }` and calls `run_compiled`. Multi-line input
is accumulated until the input forms a parseable program. The "incomplete" detection checks
whether the parse error is "unexpected EOF" (more input needed) vs. a real error.

## The debug build commands (Docker)

```bash
# Run
docker compose run --rm dev bash -c "cargo run --bin oxy -- run examples/hello.ox"

# REPL
docker compose run --rm dev bash -c "cargo run --bin oxy -- repl"

# Dump IR
docker compose run --rm dev bash -c "cargo run --bin oxy -- --dump-ir examples/hello.ox"

# With trace
OXY_VM_TRACE=1 docker compose run --rm dev bash -c "cargo run --bin oxy -- run examples/hello.ox"
```

The `--` separates `cargo run` args from Oxy args. `cargo run` builds and runs the CLI;
everything after `--` is passed to the CLI binary.
