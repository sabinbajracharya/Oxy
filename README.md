# 🧲 Ferrite

**Rust syntax, scripting freedom.**

Ferrite is an interpreted programming language written in Rust that replicates Rust's syntax as closely as possible — but **without the borrow checker or ownership rules**. Write Rust-like code, run it instantly.

## Why Ferrite?

- **Learn Rust syntax** without fighting the borrow checker
- **Rapid prototyping** with Rust ergonomics in a scripting environment
- **Gradual migration** — write in Ferrite first, port to Rust when ready
- **Familiar syntax** — if you know Rust, you already know Ferrite

## Hello World

```rust
// hello.fe
fn main() {
    let name = "World";
    println!("Hello, {}!", name);
}
```

## What Ferrite Supports

| Feature | Status |
|---|---|
| Variables (`let`, `let mut`) | ✅ |
| Functions, closures, higher-order functions | ✅ |
| Control flow (`if`/`else`, `while`, `loop`, `for..in`, `match`) | ✅ |
| Structs, enums, `impl` blocks | ✅ |
| Traits with default methods, operator overloading | ✅ |
| Generics (basic) | ✅ |
| Error handling (`Result`, `Option`, `?` operator) | ✅ |
| Modules (`mod`, `use`) | ✅ |
| Collections (`Vec`, `HashMap`, tuples, ranges) | ✅ |
| String operations, f-string interpolation | ✅ |
| `#[derive(Debug, Clone, PartialEq, Default)]` | ✅ |
| JSON serialization/deserialization | ✅ |
| HTTP client (GET, POST, PUT, DELETE, PATCH) | ✅ |
| Async/await, `spawn`, `sleep` | ✅ |
| File I/O (`std::fs` — read, write, dirs, metadata) | ✅ |
| Environment (`std::env` — vars, current_dir) | ✅ |
| Process/commands (`std::process` — run programs) | ✅ |
| Regular expressions (`std::regex`) | ✅ |
| Networking (`std::net` — TCP, UDP, DNS) | ✅ |
| Math, random, time stdlib | ✅ |
| CLI args (`std::env::args`) | ✅ |
| REPL (interactive shell) | ✅ |

## What Works Differently from Rust

| Rust Feature | Ferrite Behavior |
|---|---|
| Ownership / Move | Values are reference-counted, freely shared |
| `&` and `&mut` | Syntax accepted, semantically ignored |
| Lifetimes (`'a`) | Syntax accepted, ignored |
| `unsafe` | Not supported |
| Macros (`macro_rules!`) | Not supported (built-in pseudo-macros like `println!` work) |
| Type inference | Dynamic — types checked at runtime |

---

## Getting Started

### Prerequisites

You only need **one** of these:

- **Docker** (recommended) — no Rust or Node.js installation required
- **Rust toolchain** — if you prefer a native build

### Option A: Using Docker (Recommended)

This is the easiest way. You don't need Rust, Node.js, or anything else installed — just Docker.

#### 1. Clone the repository

```bash
git clone https://github.com/your-org/project-ferrite.git
cd project-ferrite
```

#### 2. Run first-time setup

This builds the Docker dev image (with Rust + Node.js) and installs VS Code extension dependencies:

```bash
docker compose run --rm setup
```

#### 3. Build the Ferrite interpreter

```bash
docker compose run --rm dev bash -c "cargo build --release"
```

This creates the binaries inside the Docker volume at `target/release/`.

#### 4. Run a Ferrite program

```bash
# Run the hello world example
docker compose run --rm dev bash -c "cargo run -- run examples/hello.fe"

# Run any .fe file
docker compose run --rm dev bash -c "cargo run -- run examples/fibonacci.fe"
```

#### 5. Start the interactive REPL

```bash
docker compose run --rm dev bash -c "cargo run -- repl"
```

Type Ferrite code interactively. Press `Ctrl+D` to exit.

#### 6. Run the test suite

```bash
# Run all tests
docker compose run --rm dev bash -c "cargo test"

# Or run the full CI checks (format + lint + tests)
docker compose run --rm test
```

### Option B: Using Cargo (Native)

If you have Rust installed locally:

```bash
git clone https://github.com/your-org/project-ferrite.git
cd project-ferrite

# Build
cargo build --release

# Run a program
cargo run -- run examples/hello.fe

# Start the REPL
cargo run -- repl

# Run tests
cargo test
```

---

## CLI Usage

```
ferrite [OPTIONS] <COMMAND>

Commands:
  run <file.fe>          Run a Ferrite source file
  repl                   Start the interactive REPL

Options:
  --version              Show version
  --help                 Show help
  --dump-tokens <file>   Show lexer output (debugging)
  --dump-ast <file>      Show parser AST output (debugging)
```

### Examples

```bash
# Via Docker
docker compose run --rm dev bash -c "cargo run -- run examples/hello.fe"
docker compose run --rm dev bash -c "cargo run -- repl"
docker compose run --rm dev bash -c "cargo run -- --dump-ast examples/hello.fe"

# Via Cargo (if Rust is installed)
cargo run -- run examples/hello.fe
cargo run -- repl
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

### Closures and Higher-Order Functions

```rust
fn main() {
    let numbers = vec![1, 2, 3, 4, 5];

    let doubled: Vec<i64> = numbers.iter().map(|x| x * 2).collect();
    println!("{:?}", doubled);

    let evens: Vec<i64> = numbers.iter().filter(|x| x % 2 == 0).collect();
    println!("{:?}", evens);

    let sum = numbers.iter().fold(0, |acc, x| acc + x);
    println!("Sum: {}", sum);
}
```

### JSON

```rust
fn main() {
    struct User {
        name: String,
        age: i64,
    }

    let user = User { name: "Alice".to_string(), age: 30 };
    let json_str = json::serialize(user);
    println!("{}", json_str);

    let parsed = json::deserialize(&json_str);
    println!("Name: {}", parsed.name);
}
```

### HTTP Requests

```rust
fn main() {
    let response = http::get("https://httpbin.org/get");
    println!("Status: {}", response.status);
    println!("Body: {}", response.body);
}
```

> 📁 See the `examples/` directory for more complete examples covering all features.

---

## Standard Library

Ferrite includes a comprehensive standard library accessible via `std::` paths:

| Module | Functions | Description |
|--------|-----------|-------------|
| `math` | `sqrt`, `sin`, `cos`, `tan`, `abs`, `pow`, `floor`, `ceil`, `round`, `log`, `min`, `max`, `PI`, `E` | Math functions and constants |
| `rand` | `random`, `range`, `bool` | Pseudo-random number generation |
| `time` | `now`, `millis`, `elapsed` | Wall-clock time and duration |
| `std::fs` | `read_to_string`, `write`, `append`, `exists`, `is_file`, `is_dir`, `create_dir`, `create_dir_all`, `read_dir`, `remove_file`, `remove_dir`, `rename`, `copy`, `canonicalize`, `metadata` | File system operations |
| `std::env` | `args`, `var`, `vars`, `current_dir`, `set_current_dir`, `home_dir` | Environment variables |
| `std::process` | `exit`, `command`, `command_with_args` | Process control and command execution |
| `std::regex` | `is_match`, `find`, `find_all`, `captures`, `replace`, `replace_all`, `split` | Regular expressions |
| `std::net` | `tcp_connect`, `tcp_send`, `tcp_listen`, `udp_bind`, `udp_send_to`, `lookup_host` | TCP/UDP networking |
| `json` | `serialize`, `deserialize`, `parse`, `to_string_pretty` | JSON serialization |
| `http` | `get`, `post`, `put`, `delete`, `patch`, `get_json`, `post_json` | HTTP client |

```rust
fn main() {
    // File I/O
    let _ = std::fs::write("hello.txt", "Hello from Ferrite!");
    let content = std::fs::read_to_string("hello.txt").unwrap();
    println!("{}", content);

    // Regex
    let has_email = std::regex::is_match(r"\w+@\w+\.\w+", "user@example.com");
    println!("Has email: {}", has_email);

    // Run a command
    let output = std::process::command_with_args("echo", vec!["Hello!"]).unwrap();
    println!("Output: {}", output.stdout);

    // Math
    println!("PI = {}", math::PI);
    println!("sqrt(2) = {}", math::sqrt(2.0));

    // Cleanup
    let _ = std::fs::remove_file("hello.txt");
}
```

---

## VS Code Extension

Ferrite has a VS Code extension with syntax highlighting and a built-in Language Server (LSP) for a full IDE experience.

### Features

- 🎨 **Syntax highlighting** — keywords, types, strings, comments, macros
- ⚠️ **Real-time diagnostics** — parse errors shown as you type
- 💡 **Autocompletion** — keywords, types, functions, code snippets
- 📝 **Hover info** — documentation for keywords and built-in functions
- 🗂️ **Document symbols** — outline view (functions, structs, enums, traits)
- 🔗 **Go-to definition** — jump to definitions in the same file

### Option A: Install from source (symlink)

Best for development — changes to the extension are reflected immediately.

```bash
# 1. Install extension dependencies (one-time)
docker compose run --rm setup

# 2. Symlink into VS Code extensions
ln -s "$(pwd)/editors/vscode" ~/.vscode/extensions/ferrite-lang

# 3. Reload VS Code: Cmd+Shift+P → "Reload Window"
```

### Option B: Build and install as .vsix

Best for distribution — produces a standalone installable package.

```bash
# 1. Build the .vsix package
docker compose run --rm build-ext

# 2. Install in VS Code
code --install-extension editors/vscode/ferrite-lang-0.1.0.vsix

# 3. Reload VS Code: Cmd+Shift+P → "Reload Window"
```

### How the LSP works

When you open a `.fe` file, the extension automatically starts the Ferrite Language Server via Docker:

```
VS Code ←→ docker compose run --rm -T dev cargo run --release -p ferrite-lsp ←→ stdin/stdout
```

- Docker starts **once** and the LSP stays running for your entire VS Code session
- No local Rust installation needed — everything runs inside the container
- First launch takes ~5-10 seconds (Docker + compile), subsequent opens are instant

### Advanced: Using a native binary

If you have Rust installed locally and want instant LSP startup:

```bash
cargo build --release -p ferrite-lsp
```

Then in VS Code settings (`Cmd+,`):

```json
{
    "ferrite.lsp.mode": "native",
    "ferrite.lsp.path": "/absolute/path/to/project-ferrite/target/release/ferrite-lsp"
}
```

### Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `ferrite.lsp.mode` | `auto` | `auto` = Docker if no custom path, `docker` = always Docker, `native` = local binary |
| `ferrite.lsp.path` | `ferrite-lsp` | Path to local binary (only used in `native` mode) |
| `ferrite.lsp.enabled` | `true` | Enable/disable the language server |

---

## Project Structure

```
project-ferrite/
├── crates/
│   ├── ferrite-core/       # Language engine (lexer, parser, AST, interpreter, stdlib)
│   ├── ferrite-cli/        # CLI binary (run files, REPL)
│   └── ferrite-lsp/        # Language Server Protocol server
├── editors/
│   └── vscode/             # VS Code extension (syntax + LSP client)
├── examples/               # Example .fe programs (15+ examples)
├── tests/                  # Integration tests
├── Dockerfile              # Multi-stage: builder, runtime, dev
├── docker-compose.yml      # Dev, test, setup, build services
├── CLAUDE.md               # AI assistant context
├── CONTRIBUTING.md         # Contribution guidelines + architecture overview
└── LICENSE                 # MIT license
```

## Docker Services

| Service | Command | Purpose |
|---------|---------|---------|
| `setup` | `docker compose run --rm setup` | One-time: install npm deps for VS Code extension |
| `dev` | `docker compose run --rm dev bash` | Interactive dev shell with Rust + Node.js |
| `test` | `docker compose run --rm test` | Run full CI checks (fmt + clippy + tests) |
| `build-ext` | `docker compose run --rm build-ext` | Package VS Code extension as `.vsix` |
| `build` | `docker compose build build` | Build release Docker image |

## Development

All commands via Docker (no local Rust needed):

```bash
# Run all tests (471 tests)
docker compose run --rm dev bash -c "cargo test --workspace"

# Check formatting
docker compose run --rm dev bash -c "cargo fmt --all --check"

# Run linter
docker compose run --rm dev bash -c "cargo clippy -- -D warnings"

# Run a specific test
docker compose run --rm dev bash -c "cargo test -p ferrite-core test_name"
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT — see [LICENSE](LICENSE).
