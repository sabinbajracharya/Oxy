<img width="2816" height="1536" alt="oxide_2d" src="https://github.com/user-attachments/assets/cf3ba2a3-a25f-411e-9ed4-52d87ff5b568" />

Oxide is an interpreted programming language written in Rust that replicates Rust's syntax as closely as possible — but **without the borrow checker or ownership rules**. Write Rust-like code, run it instantly.

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
| Functions, closures (with mutable captures), higher-order functions | ✅ |
| Control flow (`if`/`else`, `while`, `loop`, `for..in`, `match`) | ✅ |
| Structs, enums, `impl` blocks (both structs and enums) | ✅ |
| Traits with default methods, operator overloading | ✅ |
| Generics with trait bounds and `where` clauses | ✅ |
| Error handling (`Result`, `Option`, `?` operator) | ✅ |
| Type aliases (`type Pos = Point`) | ✅ |
| Modules (`mod`, `use`) | ✅ |
| Collections (`Vec`, `HashMap`, tuples, ranges) | ✅ |
| Iterator methods (`map`, `filter`, `zip`, `chain`, `sum`, `flatten`, …) | ✅ |
| Pattern destructuring (`let (a, b) = …`, `let [x, y] = …`) | ✅ |
| String operations, f-string interpolation | ✅ |
| `#[derive(Debug, Clone, PartialEq, Default)]` | ✅ |
| `#[test]` + built-in test runner (`ferrite test`) | ✅ |
| Visibility modifiers (`pub fn`, `pub struct`, `pub` fields) | ✅ |
| JSON serialization/deserialization | ✅ |
| HTTP client (GET, POST, PUT, DELETE, PATCH) | ✅ |
| HTTP server (routing, path params, static files) | ✅ |
| SQLite database (queries, params, in-memory) | ✅ |
| Async/await, `spawn`, `sleep` | ✅ |
| File I/O (`std::fs` — read, write, dirs, metadata) | ✅ |
| Environment (`std::env` — vars, current_dir) | ✅ |
| Process/commands (`std::process` — run programs) | ✅ |
| Regular expressions (`std::regex`) | ✅ |
| Networking (`std::net` — TCP, UDP, DNS) | ✅ |
| Math, random, time stdlib | ✅ |
| CLI args (`std::env::args`) | ✅ |
| REPL (interactive shell with history) | ✅ |
| Colored error messages with suggestions | ✅ |

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
  test <file.fe>         Run #[test] functions in a file
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
docker compose run --rm dev bash -c "cargo run -- test examples/tests.fe"
docker compose run --rm dev bash -c "cargo run -- repl"
docker compose run --rm dev bash -c "cargo run -- --dump-ast examples/hello.fe"

# Via Cargo (if Rust is installed)
cargo run -- run examples/hello.fe
cargo run -- test examples/tests.fe
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

### Iterator Chaining

Ferrite supports Rust-style iterator chaining on `Vec`:

```rust
fn main() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Chain operations together
    let result = data.filter(|x| x % 2 == 0)
                     .map(|x| x * 10)
                     .take(3)
                     .sum();
    println!("{}", result);  // 60

    // Zip, flatten, sort, etc.
    let a = vec![1, 3, 2];
    let b = vec!["one", "three", "two"];
    println!("{:?}", a.zip(b));       // [(1, "one"), (3, "three"), (2, "two")]
    println!("{:?}", a.sort());       // [1, 2, 3]
    println!("{:?}", a.rev());        // [2, 3, 1]
    println!("{:?}", a.min());        // Some(1)

    let nested = vec![vec![1, 2], vec![3, 4]];
    println!("{:?}", nested.flatten()); // [1, 2, 3, 4]
}
```

**Available methods:** `map`, `filter`, `fold`, `any`, `all`, `find`, `position`, `enumerate`, `flat_map`, `collect`, `for_each`, `zip`, `take`, `skip`, `chain`, `flatten`, `sum`, `count`, `rev`, `sort`, `dedup`, `windows`, `chunks`, `min`, `max`

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

    // Or-patterns in match
    let n = 3;
    match n {
        1 | 2 => println!("one or two"),
        3 | 4 => println!("three or four"),
        _ => println!("other"),
    }
}
```

### Testing

Ferrite has a built-in test runner, just like Rust. Mark functions with `#[test]` and use `assert!`, `assert_eq!`, and `assert_ne!` macros:

```rust
// math_tests.fe

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

#[test]
fn test_negative() {
    assert_ne!(add(1, 1), 3);
    assert!(add(1, 1) > 0, "sum should be positive");
}
```

Run your tests:

```bash
# Via Docker
docker compose run --rm dev bash -c "cargo run -- test math_tests.fe"

# Via Cargo
cargo run -- test math_tests.fe
```

Output:

```
running tests in math_tests.fe

  test_add ... ok
  test_factorial ... ok
  test_negative ... ok

test result: ok. 3 passed
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

### Web Server

Ferrite includes a built-in HTTP server with Express-like routing, path parameters, query strings, and static file serving:

```rust
fn main() {
    let app = Server::new();

    // Simple routes
    app.get("/", |req| {
        Response::html("<h1>Hello, Ferrite!</h1>")
    });

    // Path parameters
    app.get("/users/:id", |req| {
        let id = req.params.get("id").unwrap_or("unknown");
        Response::json(format!("{{\"id\": \"{}\"}}", id))
    });

    // POST with request body
    app.post("/echo", |req| {
        Response::text(req.body)
    });

    // Custom status codes
    app.get("/not-found", |req| {
        Response::status(404, "Not Found")
    });

    // Static files
    app.static_files("./public");

    // Start listening
    app.listen("127.0.0.1:8080");
}
```

**Request object fields:** `method`, `path`, `body`, `params` (HashMap), `query` (HashMap), `headers` (HashMap)

**Response helpers:**
| Function | Description |
|----------|-------------|
| `Response::text(body)` | 200 plain text response |
| `Response::json(body)` | 200 JSON response with `application/json` content type |
| `Response::html(body)` | 200 HTML response with `text/html` content type |
| `Response::status(code, body)` | Response with custom status code |

**Supported HTTP methods:** `app.get()`, `app.post()`, `app.put()`, `app.delete()`, `app.patch()`

### Database (SQLite)

Ferrite includes a built-in SQLite database with parameterized queries:

```rust
fn main() {
    let db = Db::memory();  // or Db::open("app.db") for a file

    db.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)");
    db.execute("INSERT INTO users (name, age) VALUES (?1, ?2)", vec!["Alice", 30]);
    db.execute("INSERT INTO users (name, age) VALUES (?1, ?2)", vec!["Bob", 25]);

    // Query returns Vec of HashMaps
    let rows = db.query("SELECT name, age FROM users WHERE age > ?1", vec![20]);
    for row in rows {
        println!("{} is {}", row.get("name").unwrap(), row.get("age").unwrap());
    }

    // Single row lookup (returns Option)
    let user = db.query_row("SELECT * FROM users WHERE id = ?1", vec![1]);

    println!("Last ID: {}", db.last_insert_id());
    db.close();
}
```

**Methods:**
| Method | Description |
|--------|-------------|
| `Db::open(path)` | Open or create a SQLite database file |
| `Db::memory()` | Open an in-memory database |
| `db.execute(sql)` / `db.execute(sql, params)` | Execute a statement, returns rows affected |
| `db.query(sql)` / `db.query(sql, params)` | Query rows, returns `Vec<HashMap>` |
| `db.query_row(sql, params)` | Query a single row, returns `Option<HashMap>` |
| `db.last_insert_id()` | Get the last auto-increment ID |
| `db.close()` | Close the database connection |

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
| `Server` | `new`, `get`, `post`, `put`, `delete`, `patch`, `static_files`, `listen` | HTTP server with routing |
| `Response` | `text`, `json`, `html`, `status` | HTTP response builders |
| `Db` | `open`, `memory`, `execute`, `query`, `query_row`, `last_insert_id`, `close` | SQLite database |

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
# Run all tests (500+ tests)
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
