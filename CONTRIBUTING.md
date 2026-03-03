# Contributing to Ferrite

Thank you for your interest in contributing to Ferrite! 🧲

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/project-ferrite.git`
3. Create a feature branch: `git checkout -b feat/my-feature`
4. Make your changes
5. Run checks: `cargo fmt --all && cargo clippy -- -D warnings && cargo test`
6. Commit with conventional commit messages
7. Push and open a Pull Request

## Development Setup

### Option A: Local Rust Toolchain

```bash
# Install Rust via rustup (https://rustup.rs)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add required components
rustup component add rustfmt clippy

# Build and test
cargo build && cargo test
```

### Option B: Docker (no Rust install needed)

```bash
docker compose run dev       # Dev shell
docker compose run test      # Full CI checks
```

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — new feature
- `fix:` — bug fix
- `refactor:` — code restructuring (no behavior change)
- `test:` — adding or updating tests
- `docs:` — documentation changes
- `chore:` — maintenance tasks

## Code Standards

- Run `cargo fmt --all` before committing
- All code must pass `cargo clippy -- -D warnings`
- All tests must pass: `cargo test`
- Public items must have doc comments (`///`)
- New features must include tests

## Testing

- **Unit tests:** In `#[cfg(test)]` modules alongside source code
- **Integration tests:** In `tests/` directories within crates
- **E2E tests:** `.fe` files with `.expected` output in `tests/e2e/programs/`

## Questions?

Open an issue or start a discussion — we're happy to help!
