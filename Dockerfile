# Multi-stage build for Rust version of distbench
FROM rust:1.70 as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace configuration and dependencies first for layer caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY lib ./lib
RUN cargo fetch

COPY src ./src
COPY build.rs ./build.rs

# Build the application in release mode
RUN cargo build --release

# Runtime stage - use a minimal image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 distbench

# Create app directory
WORKDIR /app

# Copy the built binary from builder
COPY --from=builder /app/target/release/runner /app/distbench

# Change ownership
RUN chown -R distbench:distbench /app

USER distbench

# The entry point will be overridden by docker-compose with specific node arguments
ENTRYPOINT ["/app/distbench"]
