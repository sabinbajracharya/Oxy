# --- Builder stage ---
FROM rust:1.93-slim AS builder

WORKDIR /app

# Cache dependencies by building them first
COPY Cargo.toml ./
COPY Cargo.lock* ./
COPY crates/oxy-core/Cargo.toml crates/oxy-core/Cargo.toml
COPY crates/oxy-cli/Cargo.toml crates/oxy-cli/Cargo.toml
COPY crates/oxy-lsp/Cargo.toml crates/oxy-lsp/Cargo.toml

# Create dummy source files to cache dependency compilation
RUN mkdir -p crates/oxy-core/src crates/oxy-cli/src crates/oxy-lsp/src && \
    echo "pub const VERSION: &str = \"0.0.0\";" > crates/oxy-core/src/lib.rs && \
    echo "fn main() {}" > crates/oxy-cli/src/main.rs && \
    echo "fn main() {}" > crates/oxy-lsp/src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf crates/oxy-core/src crates/oxy-cli/src crates/oxy-lsp/src

# Copy actual source and build
COPY . .
RUN cargo build --release

# --- Runtime stage ---
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/oxy /usr/local/bin/oxy
COPY --from=builder /app/target/release/oxy-lsp /usr/local/bin/oxy-lsp

ENTRYPOINT ["oxy"]

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
