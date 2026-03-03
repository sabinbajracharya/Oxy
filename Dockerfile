# --- Builder stage ---
FROM rust:1.85-slim AS builder

WORKDIR /app

# Cache dependencies by building them first
COPY Cargo.toml ./
COPY Cargo.lock* ./
COPY crates/ferrite-core/Cargo.toml crates/ferrite-core/Cargo.toml
COPY crates/ferrite-cli/Cargo.toml crates/ferrite-cli/Cargo.toml

# Create dummy source files to cache dependency compilation
RUN mkdir -p crates/ferrite-core/src crates/ferrite-cli/src && \
    echo "pub const VERSION: &str = \"0.0.0\";" > crates/ferrite-core/src/lib.rs && \
    echo "fn main() {}" > crates/ferrite-cli/src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf crates/ferrite-core/src crates/ferrite-cli/src

# Copy actual source and build
COPY . .
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ferrite /usr/local/bin/ferrite

ENTRYPOINT ["ferrite"]

# --- Dev stage (for development with full toolchain) ---
FROM rust:1.85-slim AS dev

RUN rustup component add rustfmt clippy

WORKDIR /app

# Install dev dependencies for test crates
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config && \
    rm -rf /var/lib/apt/lists/*

# Mount source at /app — no COPY needed, uses volume
CMD ["bash"]
