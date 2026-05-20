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

```
oxy [OPTIONS] <COMMAND>

Commands:
  run <file.ox>            Run an Oxy source file
  test <file.ox>           Run #[test] and #[compile_error] functions
  repl                     Start the interactive REPL
  install <path|url>       Install a package from a local path or git URL

Options:
  --dump-tokens <file>     Show lexer output (debugging)
  --dump-ast <file>        Show parser AST output (debugging)
  --dump-bytecode <file>   Show compiled bytecode (debugging)
```

## Language Features

### Variables, Functions, Control Flow

```rust
fn main() {
    let x = 42;
    let mut y = 10;
    y += x;

    fn add(a: i64, b: i64) -> i64 { a + b }
    println!("{}", add(3, 4));   // 7

    if x > 0 { println!("positive"); }

    for i in 0..5 { println!("{}", i); }

    while y > 0 { y -= 1; }
}
```

### Types

Integer: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`
Float: `f32`, `f64`
Type suffixes: `42i32`, `0xFFu8`, `3.14f32`

```rust
let a: i8 = 127;
let b = a + 1i8;          // -128 (wrapping)
let c = 3.14f32;
let d = 0xFFu8;           // hex with suffix
```

### Structs, Enums, Impl, Traits

```rust
struct Point { x: f64, y: f64 }

impl Point {
    fn new(x: f64, y: f64) -> Point { Point { x, y } }

    fn distance(self, other: Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

enum Shape { Circle(f64), Rectangle(f64, f64) }

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle(w, h) => w * h,
    }
}
```

### Error Handling

```rust
fn divide(a: f64, b: f64) -> Result<f64, String> {
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
    pub fn add(a: i64, b: i64) -> i64 { a + b }
    fn helper() -> i64 { 0 }          // private
}

use math::add;
println!("{}", add(1, 2));            // 3
```

Visibility: `pub`, `pub(crate)`, `pub(super)`, and private (default). Enforced at compile time.

### Testing

Annotate functions with `#[test]` for runtime tests, `#[compile_error]` for tests that must fail compilation:

```rust
fn add(a: i64, b: i64) -> i64 { a + b }

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
| `math` | `sqrt`, `sin`, `cos`, `abs`, `pow`, `log`, `floor`, `ceil`, `round`, `min`, `max`, `gcd`, `lcm`, `PI`, `E` |
| `rand` | `random`, `range`, `bool` |
| `time` | `now`, `millis`, `elapsed` |
| `std::fs` | `read_to_string`, `write`, `append`, `exists`, `create_dir`, `read_dir`, `remove_file`, `rename`, `copy`, `metadata` |
| `std::env` | `args`, `var`, `vars`, `current_dir`, `home_dir` |
| `std::process` | `command` |
| `std::regex` | `is_match`, `find`, `find_all`, `captures`, `replace`, `replace_all`, `split` |
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
| `&` and `&mut` | Syntax accepted but semantically ignored. |
| Lifetimes (`'a`) | Syntax accepted but ignored. `'label` used for labeled break/continue. |
| Lazy iterators | Iterator adapters are eager — they return `Vec` immediately. |
| Macros | Built-in pseudo-macros only: `println!`, `format!`, `vec!`, `assert!`, `assert_eq!`, `panic!`, `dbg!`. |

## Project Structure

```
crates/
├── oxy-core/src/
│   ├── compiler/        # Bytecode compiler (AST → VM opcodes)
│   ├── vm/              # Stack-based VM + test runner
│   ├── type_checker/    # Static type validation
│   ├── parser/          # Pratt parser (15 precedence levels)
│   ├── lexer/           # Tokenizer
│   ├── ast/             # AST node definitions
│   ├── types/           # Value enum, type system
│   └── stdlib/          # Standard library (fs, env, regex, net, etc.)
├── oxy-cli/             # CLI binary (run, test, repl, install)
└── oxy-lsp/             # LSP server
editors/vscode/          # VS Code extension
examples/                # Example .ox programs + feature tests
playground/wasm/         # WebAssembly playground
```

## Development

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
