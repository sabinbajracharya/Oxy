# --- Builder stage ---
FROM rust:1.93-slim AS builder

WORKDIR /app

# Cache dependencies by building them first
COPY Cargo.toml ./
COPY Cargo.lock* ./
COPY crates/oxide-core/Cargo.toml crates/oxide-core/Cargo.toml
COPY crates/oxide-cli/Cargo.toml crates/oxide-cli/Cargo.toml
COPY crates/oxide-lsp/Cargo.toml crates/oxide-lsp/Cargo.toml

# Create dummy source files to cache dependency compilation
RUN mkdir -p crates/oxide-core/src crates/oxide-cli/src crates/oxide-lsp/src && \
    echo "pub const VERSION: &str = \"0.0.0\";" > crates/oxide-core/src/lib.rs && \
    echo "fn main() {}" > crates/oxide-cli/src/main.rs && \
    echo "fn main() {}" > crates/oxide-lsp/src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf crates/oxide-core/src crates/oxide-cli/src crates/oxide-lsp/src

# Copy actual source and build
COPY . .
RUN cargo build --release

# --- Runtime stage ---
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/oxide /usr/local/bin/oxide
COPY --from=builder /app/target/release/oxide-lsp /usr/local/bin/oxide-lsp

ENTRYPOINT ["oxide"]

# --- Dev stage (for development with full toolchain) ---
FROM rust:1.93-slim AS dev

RUN rustup component add rustfmt clippy

# Install Node.js 20 (for VS Code extension tooling)
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Mount source at /app — no COPY needed, uses volume
CMD ["bash"]
