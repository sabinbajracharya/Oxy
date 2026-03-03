# 🧲 Ferrite

**Rust syntax, scripting freedom.**

Ferrite is an interpreted programming language written in Rust that replicates Rust's syntax as closely as possible — but **without the borrow checker or ownership rules**.

## Why Ferrite?

- **Learn Rust syntax** without fighting the borrow checker
- **Rapid prototyping** with Rust ergonomics in a scripting environment
- **Gradual migration** — write in Ferrite first, port to Rust when ready

## Quick Start

### With Cargo

```bash
cargo install ferrite-cli

# Run a file
ferrite run hello.fe

# Start the REPL
ferrite repl
```

### With Docker

```bash
docker run --rm -v $(pwd):/code ferrite:latest run /code/hello.fe
```

### Hello World

```rust
// hello.fe — this is valid Ferrite AND valid Rust!
fn main() {
    let name = "World";
    println!("Hello, {}!", name);
}
```

## What Works Differently

| Rust Feature | Ferrite Behavior |
|---|---|
| Ownership / Move | Values are reference-counted, freely shared |
| `&` and `&mut` | Syntax accepted, semantically ignored |
| Lifetimes (`'a`) | Syntax accepted, ignored |
| `unsafe` | Not supported |
| Macros (`macro_rules!`) | Not supported (built-in pseudo-macros like `println!` work) |
| Type inference | Dynamic — types checked at runtime |

## Building from Source

```bash
git clone https://github.com/your-org/project-ferrite.git
cd project-ferrite
cargo build --release
```

### Using Docker for Development

```bash
# Drop into a dev container (no Rust install needed)
docker compose run dev

# Run the full test suite
docker compose run test
```

## Project Structure

```
project-ferrite/
├── crates/
│   ├── ferrite-core/    # Language engine (lexer, parser, interpreter)
│   └── ferrite-cli/     # CLI binary (REPL + file execution)
├── tests/e2e/           # End-to-end test programs
├── examples/            # Example .fe programs
├── Dockerfile
└── docker-compose.yml
```

## Development

```bash
cargo test                   # Run all tests
cargo fmt --all --check      # Check formatting
cargo clippy -- -D warnings  # Lint
```

## License

MIT — see [LICENSE](LICENSE).
