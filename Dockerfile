# --- Builder stage ---
FROM rust:1.85-slim AS builder

WORKDIR /app

# Cache dependencies by building them first
COPY Cargo.toml ./
COPY Cargo.lock* ./
COPY crates/ferrite-core/Cargo.toml crates/ferrite-core/Cargo.toml
COPY crates/ferrite-cli/Cargo.toml crates/ferrite-cli/Cargo.toml
COPY crates/ferrite-lsp/Cargo.toml crates/ferrite-lsp/Cargo.toml

# Create dummy source files to cache dependency compilation
RUN mkdir -p crates/ferrite-core/src crates/ferrite-cli/src crates/ferrite-lsp/src && \
    echo "pub const VERSION: &str = \"0.0.0\";" > crates/ferrite-core/src/lib.rs && \
    echo "fn main() {}" > crates/ferrite-cli/src/main.rs && \
    echo "fn main() {}" > crates/ferrite-lsp/src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf crates/ferrite-core/src crates/ferrite-cli/src crates/ferrite-lsp/src

# Copy actual source and build
COPY . .
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ferrite /usr/local/bin/ferrite
COPY --from=builder /app/target/release/ferrite-lsp /usr/local/bin/ferrite-lsp

ENTRYPOINT ["ferrite"]

# --- Dev stage (for development with full toolchain) ---
FROM rust:1.85-slim AS dev

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
