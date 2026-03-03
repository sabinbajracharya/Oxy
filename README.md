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
| Traits with default methods | ✅ |
| Generics (basic) | ✅ |
| Error handling (`Result`, `Option`, `?` operator) | ✅ |
| Modules (`mod`, `use`) | ✅ |
| Collections (`Vec`, `HashMap`, tuples, ranges) | ✅ |
| String operations | ✅ |
| JSON serialization/deserialization | ✅ |
| HTTP client (GET, POST, PUT, DELETE, PATCH) | ✅ |
| Async/await, `spawn`, `sleep` | ✅ |
| File I/O (`std::fs`) | ✅ |
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

## VS Code Extension

Ferrite has a VS Code extension with syntax highlighting and a built-in Language Server (LSP) for a full IDE experience.

### Features

- 🎨 **Syntax highlighting** — keywords, types, strings, comments, macros
- ⚠️ **Real-time diagnostics** — parse errors shown as you type
- 💡 **Autocompletion** — keywords, types, functions, code snippets
- 📝 **Hover info** — documentation for keywords and built-in functions
- 🗂️ **Document symbols** — outline view (functions, structs, enums, traits)
- 🔗 **Go-to definition** — jump to definitions in the same file

### Setup

#### 1. Build the LSP server

```bash
docker compose run --rm dev bash -c "cargo build --release -p ferrite-lsp"
```

#### 2. Install the extension

Symlink (or copy) the extension folder into VS Code's extensions directory:

```bash
# macOS / Linux
ln -s "$(pwd)/editors/vscode" ~/.vscode/extensions/ferrite-lang
```

#### 3. Install extension dependencies

If you haven't run setup already:

```bash
docker compose run --rm setup
```

#### 4. Configure the LSP binary path

Open VS Code settings (`Cmd+,` or `Ctrl+,`), search for "ferrite", and set the LSP binary path:

```json
{
    "ferrite.lsp.path": "/absolute/path/to/project-ferrite/target/release/ferrite-lsp"
}
```

> **Tip**: Run `docker compose run --rm dev bash -c "realpath target/release/ferrite-lsp"` to get the absolute path.

#### 5. Reload VS Code

Press `Cmd+Shift+P` (or `Ctrl+Shift+P`) → type "Reload Window" → Enter.

Open any `.fe` file and you should see syntax highlighting and LSP features.

---

## Project Structure

```
project-ferrite/
├── crates/
│   ├── ferrite-core/       # Language engine (lexer, parser, AST, interpreter)
│   ├── ferrite-cli/        # CLI binary (run files, REPL)
│   └── ferrite-lsp/        # Language Server Protocol server
├── editors/
│   └── vscode/             # VS Code extension (syntax + LSP client)
├── examples/               # Example .fe programs
├── tests/                  # Integration tests
├── Dockerfile              # Multi-stage: builder, runtime, dev
├── docker-compose.yml      # Dev, test, setup, build services
├── CLAUDE.md               # AI assistant context
├── CONTRIBUTING.md         # Contribution guidelines
└── LICENSE                 # MIT license
```

## Docker Services

| Service | Command | Purpose |
|---------|---------|---------|
| `setup` | `docker compose run --rm setup` | One-time: install npm deps for VS Code extension |
| `dev` | `docker compose run --rm dev bash` | Interactive dev shell with Rust + Node.js |
| `test` | `docker compose run --rm test` | Run full CI checks (fmt + clippy + tests) |
| `build` | `docker compose build build` | Build release Docker image |

## Development

All commands via Docker (no local Rust needed):

```bash
# Run all tests (395 tests)
docker compose run --rm dev bash -c "cargo test"

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
