# Oxy

**Rust syntax, scripting freedom.**

Oxy is a bytecode-compiled programming language written in Rust that replicates Rust's syntax — without the borrow checker or ownership rules. Write Rust-like code, run it instantly on a stack-based VM.

## Hello World

```rust
fn main() {
    let name = "World";
    println!("Hello, {}!", name);
}
```

## Quick Start

You only need Docker.

```bash
git clone https://github.com/sabinbajracharya/Oxy.git
cd Oxy

# Setup
docker compose run --rm setup

# Run a program
docker compose run --rm dev bash -c "cargo run -- run examples/hello.ox"

# Start the REPL
docker compose run --rm dev bash -c "cargo run -- repl"

# Run tests
docker compose run --rm dev bash -c "cargo test -p oxy-core"
```

If you have Rust installed, you can also build natively with `cargo build --release`.

## CLI

Oxy ships two binaries: `oxy` (compiler) and `tug` (package manager), following the rustc/cargo model.

### `oxy` — compiler

```
oxy [OPTIONS] <COMMAND>

Commands:
  run <file.ox>            Run an Oxy source file
  test <file.ox>           Run #[test] and #[compile_error] functions
  repl                     Start the interactive REPL

Options:
  --extern <name>=<path>   Register an external module dependency
  --dump-tokens <file>     Show lexer output (debugging)
  --dump-ast <file>        Show parser AST output (debugging)
  --dump-bytecode <file>   Show compiled bytecode (debugging)
```

### `tug` — package manager

```
tug <COMMAND>

Commands:
  new <name>               Scaffold a new project (tug.toml, src/main.ox)
  init                     Initialize a project in the current directory
  build                    Compile the project with dependencies
  run [args...]            Build and run the project
  test                     Build and run tests
  add <spec>               Add a dependency to tug.toml
  remove <name>            Remove a dependency
  install <path|url>       Install a package globally (~/.oxy/packages/)
  uninstall <name>         Remove a globally installed package
  list                     List installed packages
```

## Language Features

### Variables, Functions, Control Flow

```rust
fn main() {
    let x = 42;
    let mut y = 10;
    y += x;

    fn add(a: int, b: int) -> int { a + b }
    println!("{}", add(3, 4));   // 7

    if x > 0 { println!("positive"); }

    for i in 0..5 { println!("{}", i); }

    while y > 0 { y -= 1; }
}
```

### Types

Three numeric types: `int` (signed 64-bit), `byte` (unsigned 8-bit), `float` (64-bit IEEE-754).
Width semantics enforced at function boundaries and typed `let` bindings.

```rust
let a: int = 127;
let b: byte = 0xFF;
let c = 3.14;              // float inferred
let d = a + 1;             // arithmetic widens to int
```

### Structs, Enums, Impl, Traits

```rust
struct Point { x: float, y: float }

impl Point {
    fn new(x: float, y: float) -> Point { Point { x, y } }

    fn distance(self, other: Point) -> float {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

enum Shape { Circle(float), Rectangle(float, float) }

fn area(s: Shape) -> float {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle(w, h) => w * h,
    }
}
```

### Error Handling

```rust
fn divide(a: float, b: float) -> Result<float, String> {
    if b == 0.0 { Err("division by zero".to_string()) }
    else { Ok(a / b) }
}

// ? operator, match, unwrap — all work
let result = divide(10.0, 3.0)?;
```

### Closures and Iterators

```rust
let numbers = vec![1, 2, 3, 4, 5];

let doubled = numbers.iter().map(|x| x * 2);
let evens = numbers.iter().filter(|x| x % 2 == 0);
let sum = numbers.iter().sum();
let found = numbers.iter().find(|x| x > 3);
```

Iterator adapters (map, filter, enumerate, zip, chain, take, skip, flatten) are eager — they produce Vec immediately.

### Pattern Matching

```rust
// Tuple destructuring
let (x, y) = (10, 20);

// Slice destructuring
let [a, b, c] = vec![1, 2, 3];

// Match guards
match n {
    x if x < 0 => println!("negative"),
    x if x % 2 == 0 => println!("even"),
    _ => println!("odd"),
}
```

### Modules and Visibility

```rust
mod math {
    pub fn add(a: int, b: int) -> int { a + b }
    fn helper() -> int { 0 }          // private
}

use math::add;
println!("{}", add(1, 2));            // 3
```

Visibility: `pub`, `pub(crate)`, `pub(super)`, and private (default). Enforced at compile time.

### Testing

Annotate functions with `#[test]` for runtime tests, `#[compile_error]` for tests that must fail compilation:

```rust
fn add(a: int, b: int) -> int { a + b }

#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
    assert_eq!(add(-1, 1), 0);
}

#[test]
fn test_divide_by_zero_panics() {
    let _ = 1 / 0;                   // runtime panic — test passes
}

#[compile_error]
fn test_private_fn_not_accessible() {
    math::helper();                   // must fail to compile — test passes
}
```

Assert macros: `assert!`, `assert_eq!`, `assert_ne!`, `panic!`.

### JSON

```rust
let data = json::parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
println!("{}", data.name);           // Alice

let json_str = json::serialize(data);
```

## Standard Library

| Module | What it provides |
|--------|-----------------|
| `math` | `sqrt`, `sin`, `cos`, `abs`, `pow`, `log`, `floor`, `ceil`, `round`, `min`, `max`, `clamp`, `gcd`, `lcm`, `PI`, `E` |
| `rand` | `random`, `rand_int(lo, hi)`, `range`, `bool` |
| `time` | `now`, `millis`, `elapsed` |
| `std::fs` | `read_to_string`, `write`, `append`, `exists`, `create_dir`, `read_dir`, `remove_file`, `rename`, `copy`, `metadata` |
| `std::env` | `args()`, `var`, `vars`, `current_dir`, `home_dir` |
| `std::args` | `parse(spec)` — CLI argument parser |
| `std::path` | `join`, `split`, `extension`, `with_extension`, `parent`, `file_stem`, `is_absolute`, and more |
| `std::process` | `command`, `spawn(program, args, callback)` — line-by-line streaming |
| `std::io` | `stdin` — read from standard input |
| `std::server` | `start(addr, callback)` — closure-driven HTTP server |
| `std::db` | SQLite client (bundled) |
| `std::regex` | `Regex::new(pat).is_match(text)`, `find`, `find_all`, `captures`, `replace`, `replace_all`, `split` |
| `std::net` | `tcp_connect`, `tcp_send`, `tcp_listen`, `udp_bind`, `lookup_host` |
| `json` | `parse`, `serialize`, `deserialize` |

## VS Code Extension

Syntax highlighting, diagnostics, autocompletion, hover, go-to-definition.

```bash
ln -s "$(pwd)/editors/vscode" ~/.vscode/extensions/oxy-lang
# Or package as .vsix:
docker compose run --rm build-ext
code --install-extension editors/vscode/oxy-lang-0.1.0.vsix
```

## Key Differences from Rust

| Rust | Oxy |
|------|-----|
| Ownership / borrow checker | Reference-counted. Collections use `Rc<RefCell<>>` — assignment shares data. Use `.clone()` for independent copies. |
| `&T`, `&mut T`, `&self`, `&str` | Rejected by the parser. Use `T`, `mut T`, `self`, `String` instead. |
| Lifetimes (`'a`, `<'a>`) | Not supported. `'label` used for labeled break/continue. |
| Integer widths (`i8`–`u64`) | Three types: `int` (64-bit), `byte` (8-bit unsigned), `float` (64-bit). |
| Lazy iterators | Iterator adapters are eager — they return `Vec` immediately. |
| Macros | Built-in pseudo-macros only: `println!`, `format!`, `vec!`, `assert!`, `assert_eq!`, `panic!`, `dbg!`. |

## Project Structure

```
crates/
├── oxy-core/src/
│   ├── compiler/        # Bytecode compiler (AST → VM opcodes)
│   │   ├── mod.rs       #   Prescan, compile items, module handling
│   │   ├── expr.rs      #   Expression compilation
│   │   ├── pattern.rs   #   Pattern compilation
│   │   └── visibility.rs#   Visibility enforcement
│   ├── vm/              # Stack-based VM + test runner
│   │   ├── mod.rs       #   Dispatch, builtin_method, run_tests
│   │   ├── builtins/    #   Per-type method implementations
│   │   ├── arith.rs     #   Arithmetic operations
│   │   ├── call.rs      #   Function call dispatch
│   │   └── format.rs    #   String formatting (println!, format!)
│   ├── type_checker/    # Static type validation
│   │   ├── mod.rs       #   TypeChecker struct, check_program
│   │   ├── check_expr.rs#   Expression type inference
│   │   ├── check_item.rs#   Item type checking
│   │   └── resolve.rs   #   Name resolution
│   ├── parser/          # Pratt parser (15 precedence levels)
│   │   ├── mod.rs       #   Parser struct, parse_program
│   │   ├── expr.rs      #   Expression parsing
│   │   ├── item.rs      #   Item parsing (fn, struct, enum, impl)
│   │   └── stmt.rs      #   Statement parsing
│   ├── lexer/           # Tokenizer
│   ├── ast/             # AST node definitions
│   ├── types/           # Value enum, type system
│   ├── stdlib/          # Standard library (fs, env, path, process, etc.)
│   └── symbols.rs       # Canonical symbol definitions
├── oxy-cli/             # CLI binary (run, test, repl)
├── oxy-lsp/             # LSP server (completions, diagnostics, hover)
└── oxy-tug/             # Package manager (tug new, build, add, install)
editors/vscode/          # VS Code extension
examples/
├── features/            # Language feature tests (200+ .ox files)
└── showcase/            # Showcase projects (todo-cli, http-scraper, etc.)
playground/wasm/         # WebAssembly playground
```

## Development

See **[CONTRIBUTING.md](CONTRIBUTING.md)** for the full guide on adding features, built-in methods, tests, and more.

```bash
# Full check (fmt + clippy + tests)
docker compose run --rm dev bash -c "cargo fmt --all && cargo clippy -- -D warnings && cargo test -p oxy-core"

# Run a specific test file
docker compose run --rm dev bash -c "cargo test -p oxy-core -- feature_examples"

# Full CI
docker compose run --rm test
```

A pre-commit hook runs fmt, clippy, and tests automatically. Enable it after cloning:

```bash
git config core.hooksPath .githooks
```

## License

MIT — see [LICENSE](LICENSE).
