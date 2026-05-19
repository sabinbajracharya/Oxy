# Oxy

**Rust syntax, scripting freedom.**

Oxy is a bytecode-compiled programming language written in Rust that replicates Rust's syntax — **without the borrow checker or ownership rules**. Write Rust-like code, run it instantly on a stack-based VM.

## Why Oxy?

- **Learn Rust syntax** without fighting the borrow checker
- **Rapid prototyping** with Rust ergonomics in a scripting environment
- **Familiar syntax** — if you know Rust, you already know Oxy

## Hello World

```rust
// hello.ox
fn main() {
    let name = "World";
    println!("Hello, {}!", name);
}
```

## What Oxy Supports

| Feature | Status |
|---|---|
| Variables (`let`, `let mut`) | working |
| Functions, closures (with mutable captures), higher-order functions | working |
| Control flow (`if`/`else`, `while`, `loop`, `for..in`, `match`, labeled break/continue) | working |
| Short-circuit `&&` and `\|\|` | working |
| Structs, enums, `impl` blocks (struct and enum) | working |
| Traits with default methods, operator overloading (`+`, `-`, `*`, `/`, `%`, `==`, `<`, etc.) | working |
| Generics on structs, enums, and functions | working |
| Error handling (`Result`, `Option`, `?` operator) | working |
| Type aliases (`type Pos = Point`) | working |
| Modules (`mod`, `use`, `use` groups, `use` globs) | working |
| Integer types: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64` with wrapping arithmetic | working |
| Float types: `f32`, `f64` | working |
| Type suffixes: `42i32`, `0xFFu8`, `3.14f32` | working |
| Collections: `Vec`, `HashMap`, `HashSet`, `BinaryHeap`, `VecDeque`, tuples, ranges | working |
| Iterator adapters: `map`, `filter`, `enumerate`, `zip`, `chain`, `take`, `skip`, `flatten` | working |
| Iterator consumers: `collect`, `sum`, `count`, `nth`, `find`, `position`, `any`, `all`, `fold`, `for_each` | working |
| Vec methods: `sort`, `sort_by`, `reverse`, `dedup`, `chunks`, `windows`, `min`, `max` | working |
| Pattern destructuring (`let (a, b) = …`, `let [x, y] = …`, `Some((i, j))`) | working |
| Match guards (`if` conditions on match arms) | working |
| Static type checking (optional `: Type` annotations) | working |
| String operations, f-string interpolation (`f"Hello {name}"`) | working |
| `#[derive(Debug, Clone, PartialEq, Default)]` | working |
| `#[test]` + built-in test runner (`oxy test`) | working |
| Visibility modifiers (`pub fn`, `pub struct`, `pub` fields) | working |
| JSON serialization/deserialization (`json::parse`, `json::serialize`, `json::deserialize`) | working |
| File I/O (`std::fs` — read, write, dirs, metadata) | working |
| Environment (`std::env` — vars, current_dir, home_dir) | working |
| Processes (`std::process` — run commands, exit codes) | working |
| Regular expressions (`std::regex` — match, find, replace, split) | working |
| Networking (`std::net` — TCP, UDP, DNS lookup) | working |
| Math (`math::sqrt`, `math::sin`, `math::cos`, `math::PI`, `math::E`, etc.) | working |
| Random (`rand::random`, `rand::range`, `rand::bool`) | working |
| Time (`time::now`, `time::millis`, `time::elapsed`) | working |
| CLI args (`std::env::args`) | working |
| REPL (interactive shell with history) | working |
| Colored error messages with suggestions | working |
| VS Code extension (syntax highlighting, LSP diagnostics, completions, hover, go-to-def) | working |
| Package manager (`oxy install`) | working |
| LeetCode solutions (106 benchmark problems passing) | working |

### Deferred (coming soon)

| Feature | Status |
|---|---|
| `async`/`await`, `spawn`, `sleep` | deferred |
| HTTP client request builder (`http::request()`) | deferred |
| HTTP server (`Server::new()`, routing, path params, static files) | deferred |
| DB struct methods (`db.query_row()` returning structs) | deferred |

## What Works Differently from Rust

| Rust Feature | Oxy Behavior |
|---|---|
| Ownership / Move | Values are reference-counted. Collections (Vec, HashMap, HashSet, etc.) use `Rc<RefCell<>>` — assignment shares data, mutations propagate. Use `.clone()` for an independent deep copy. |
| `&` and `&mut` | Syntax accepted but semantically ignored — no borrow checking. |
| Lifetimes (`'a`) | Syntax accepted but ignored. `'label` used for labeled break/continue. |
| `unsafe` | Not supported. |
| Macros (`macro_rules!`) | Not supported. Built-in pseudo-macros like `println!`, `format!`, `vec!`, `assert!`, `assert_eq!`, `assert_ne!`, `panic!` work. |
| Type inference | Dynamic with optional static checking — type annotations on `let`, `fn`, and `const` are validated before execution. |
| `as` casts | Supported for numeric types (`val as i64`, `val as f64`). |
| Iterator laziness | Iterator adapters (`map`, `filter`) are eager — they produce `Vec` immediately rather than lazy chains. Consumers like `collect`, `sum`, `fold` work on Vec values. |

---

## Getting Started

### Prerequisites

You only need **one** of these:

- **Docker** (recommended) — no Rust or Node.js installation required
- **Rust toolchain** — if you prefer a native build

### Option A: Using Docker (Recommended)

```bash
# 1. Clone
git clone https://github.com/sabinbajracharya/Oxy.git
cd Oxy

# 2. First-time setup (builds dev image + VS Code extension deps)
docker compose run --rm setup

# 3. Build
docker compose run --rm dev bash -c "cargo build --release"

# 4. Run a program
docker compose run --rm dev bash -c "cargo run -- run examples/hello.ox"

# 5. Start the REPL
docker compose run --rm dev bash -c "cargo run -- repl"

# 6. Run tests
docker compose run --rm dev bash -c "cargo test"
```

### Option B: Using Cargo (Native)

```bash
git clone https://github.com/sabinbajracharya/Oxy.git
cd Oxy

cargo build --release
cargo run -- run examples/hello.ox
cargo run -- repl
cargo test
```

---

## CLI Usage

```
oxy [OPTIONS] <COMMAND>

Commands:
  run <file.ox>          Run an Oxy source file
  test <file.ox>         Run #[test] functions in a file
  repl                   Start the interactive REPL
  install <path|url>     Install a package from a local path or git URL

Options:
  --version              Show version
  --help                 Show help
  --dump-tokens <file>   Show lexer output (debugging)
  --dump-ast <file>      Show parser AST output (debugging)
```

---

## Language Examples

### Variables and Functions

```rust
fn main() {
    let x = 42;
    let mut y = 10;
    y = y + x;
    println!("y = {}", y);

    fn add(a: i64, b: i64) -> i64 {
        a + b
    }
    println!("{}", add(3, 4));
}
```

### Integer and Float Types

Oxy supports Rust's full set of integer and float types with proper widths and wrapping behavior:

```rust
fn main() {
    let a: i8 = 127;
    let b = a + 1i8;       // -128 (wrapping)
    let c = 255u8;
    let d = c + 1u8;       // 0 (wrapping)
    let e = 42i32;         // type suffix
    let f = 3.14f32;       // float suffix
    let g = 0xFFu8;        // hex with suffix

    // Cross-type arithmetic promotes to wider type
    let h: i64 = 5i8 + 10i32;   // promoted to i64
    let i: f64 = 5i8 + 1.5f32;  // promoted to f64
}
```

### Structs and Impl

```rust
fn main() {
    struct Point {
        x: f64,
        y: f64,
    }

    impl Point {
        fn new(x: f64, y: f64) -> Point {
            Point { x, y }
        }

        fn distance(&self, other: &Point) -> f64 {
            let dx = self.x - other.x;
            let dy = self.y - other.y;
            (dx * dx + dy * dy).sqrt()
        }
    }

    let a = Point::new(0.0, 0.0);
    let b = Point::new(3.0, 4.0);
    println!("Distance: {}", a.distance(&b));
}
```

### Enums and Pattern Matching

```rust
fn main() {
    enum Shape {
        Circle(f64),
        Rectangle(f64, f64),
    }

    fn area(shape: Shape) -> f64 {
        match shape {
            Shape::Circle(r) => 3.14159 * r * r,
            Shape::Rectangle(w, h) => w * h,
        }
    }

    println!("{}", area(Shape::Circle(5.0)));
    println!("{}", area(Shape::Rectangle(4.0, 6.0)));
}
```

### Error Handling

```rust
fn main() {
    fn divide(a: f64, b: f64) -> Result<f64, String> {
        if b == 0.0 {
            Err("division by zero".to_string())
        } else {
            Ok(a / b)
        }
    }

    match divide(10.0, 3.0) {
        Ok(result) => println!("Result: {}", result),
        Err(e) => println!("Error: {}", e),
    }
}
```

### Closures and Iterators

```rust
fn main() {
    let numbers = vec![1, 2, 3, 4, 5];

    // Iterator adapters are eager (return Vec)
    let doubled = numbers.iter().map(|x| x * 2);
    println!("{:?}", doubled);

    let evens = numbers.iter().filter(|x| x % 2 == 0);
    println!("{:?}", evens);

    // Consumers work on Vec
    let sum = numbers.iter().sum();
    println!("Sum: {}", sum);

    let found = numbers.iter().find(|x| x > 3);
    println!("First > 3: {:?}", found);

    // Chain, enumerate, zip
    let pairs = numbers.iter().enumerate().collect();
    println!("{:?}", pairs);
}
```

### Pattern Destructuring

```rust
fn main() {
    // Tuple destructuring
    let point = (10, 20);
    let (x, y) = point;
    println!("x={}, y={}", x, y);

    // Slice/Vec destructuring
    let coords = vec![1, 2, 3];
    let [a, b, c] = coords;
    println!("{} {} {}", a, b, c);

    // Nested destructuring in match
    let val = Some((1, 2));
    match val {
        Some((x, y)) => println!("({}, {})", x, y),
        None => println!("nothing"),
    }
}
```

### Match Guards

```rust
fn main() {
    let n = 5;
    match n {
        x if x < 0 => println!("negative"),
        x if x % 2 == 0 => println!("even"),
        _ => println!("odd"),
    }
}
```

### Testing

```rust
// math_tests.ox

fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn factorial(n: i64) -> i64 {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
    assert_eq!(add(-1, 1), 0);
}

#[test]
fn test_factorial() {
    assert_eq!(factorial(0), 1);
    assert_eq!(factorial(5), 120);
}
```

```
$ oxy test math_tests.ox
running tests in math_tests.ox

  test_add ... ok
  test_factorial ... ok

test result: ok. 2 passed
```

#### Assert Macros

| Macro | Description |
|-------|-------------|
| `assert!(expr)` | Fails if `expr` is falsy |
| `assert!(expr, "message")` | Fails with custom message |
| `assert_eq!(left, right)` | Fails if `left != right`, shows both values |
| `assert_ne!(left, right)` | Fails if `left == right`, shows both values |

### JSON

```rust
fn main() {
    let data = json::parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
    println!("{}", data.name);
    println!("{}", data.age);

    let output = json::serialize(data);
    println!("{}", output);
}
```

---

## Standard Library

Oxy includes a standard library accessible via `std::` paths (or directly for `math`, `json`):

| Module | Functions | Description |
|--------|-----------|-------------|
| `math` | `sqrt`, `sin`, `cos`, `tan`, `abs`, `pow`, `floor`, `ceil`, `round`, `log`, `log2`, `log10`, `min`, `max`, `gcd`, `lcm`, `PI`, `E` | Math functions and constants |
| `rand` | `random`, `range`, `bool` | Pseudo-random number generation |
| `time` | `now`, `millis`, `elapsed` | Wall-clock time and duration |
| `std::fs` | `read_to_string`, `write`, `append`, `exists`, `is_file`, `is_dir`, `create_dir`, `create_dir_all`, `read_dir`, `remove_file`, `remove_dir`, `rename`, `copy`, `canonicalize`, `metadata` | File system operations |
| `std::env` | `args`, `var`, `vars`, `current_dir`, `home_dir` | Environment variables |
| `std::process` | `command`, `command_with_args` | Process/command execution |
| `std::regex` | `is_match`, `find`, `find_all`, `captures`, `replace`, `replace_all`, `split` | Regular expressions |
| `std::net` | `tcp_connect`, `tcp_send`, `tcp_listen`, `udp_bind`, `udp_send_to`, `lookup_host` | TCP/UDP networking |
| `json` | `parse`, `serialize`, `deserialize`, `to_string_pretty` | JSON serialization |

```rust
fn main() {
    // File I/O
    std::fs::write("hello.txt", "Hello from Oxy!");
    let content = std::fs::read_to_string("hello.txt").unwrap();
    println!("{}", content);

    // Regex
    let has_email = std::regex::is_match(r"\w+@\w+\.\w+", "user@example.com");
    println!("Has email: {}", has_email);

    // Math
    println!("PI = {}", math::PI);
    println!("sqrt(2) = {}", math::sqrt(2.0));

    // Cleanup
    std::fs::remove_file("hello.txt");
}
```

---

## VS Code Extension

Oxy has a VS Code extension with syntax highlighting and a Language Server (LSP):

- Syntax highlighting — keywords, types, strings, comments, macros
- Real-time diagnostics — parse and type errors shown as you type
- Autocompletion — keywords, types, functions, snippets, dot-completions
- Hover info — signatures with types for functions, structs, enums
- Document symbols — outline view
- Go-to definition — jump to definitions in the same file

```bash
# Install from source (symlink)
ln -s "$(pwd)/editors/vscode" ~/.vscode/extensions/oxy-lang

# Or build as .vsix
docker compose run --rm build-ext
code --install-extension editors/vscode/oxy-lang-0.1.0.vsix
```

### Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `oxy.lsp.mode` | `auto` | `auto` = Docker if no custom path, `docker` = always Docker, `native` = local binary |
| `oxy.lsp.path` | `oxy-lsp` | Path to local binary (only used in `native` mode) |
| `oxy.lsp.enabled` | `true` | Enable/disable the language server |

---

## Package Manager

```bash
# Install from a local path
oxy install ./my-package

# Install from a git URL
oxy install https://github.com/user/my-package
```

Package manifest (`package.ox`):
```rust
name = "my-package"
version = "0.1.0"
entry = "lib.ox"
```

---

## Project Structure

```
oxy/
├── crates/
│   ├── oxy-core/       # Language engine (lexer, parser, compiler, VM, type checker, stdlib)
│   │   └── src/
│   │       ├── compiler/     # Bytecode compiler (AST → stack-based VM opcodes)
│   │       ├── vm/           # Stack-based bytecode VM with 40+ opcodes
│   │       ├── type_checker/ # Static type validation before execution
│   │       ├── package/      # Package manager (install, manifest parsing)
│   │       ├── parser/       # Pratt parser (15 precedence levels)
│   │       ├── lexer/        # Tokenizer (~60 token kinds)
│   │       └── ast/          # AST node definitions
│   ├── oxy-cli/        # CLI binary (run, repl, test, install)
│   └── oxy-lsp/        # LSP server (diagnostics, completions, hover)
├── editors/
│   └── vscode/             # VS Code extension (syntax + LSP client)
├── examples/               # Example .ox programs including 106 LeetCode solutions
├── playground/wasm/        # WebAssembly playground
├── Dockerfile              # Multi-stage: builder, runtime, dev
├── docker-compose.yml      # Dev, test, setup, build-ext services
└── LICENSE                 # MIT license
```

---

## Development

All commands via Docker (no local Rust needed):

```bash
# Run all tests
docker compose run --rm dev bash -c "cargo test --workspace"

# Check formatting
docker compose run --rm dev bash -c "cargo fmt --all --check"

# Run linter
docker compose run --rm dev bash -c "cargo clippy --all-targets --all-features -- -D warnings"

# Run a specific test
docker compose run --rm dev bash -c "cargo test -p oxy-core test_name"

# Full CI check
docker compose run --rm test
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT — see [LICENSE](LICENSE).
