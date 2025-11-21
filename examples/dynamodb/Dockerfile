# Build stage - use musl for static linking
FROM rust:1.83-alpine AS builder

# Install musl dev tools
RUN apk add --no-cache musl-dev

# Set working directory
WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code and tests
COPY src ./src
COPY tests ./tests

# Build statically linked binary and run tests
RUN cargo test --release --target x86_64-unknown-linux-musl && \
    cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage - scratch (empty image)
FROM scratch

# Copy the static binary from builder
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/ddb_convert /ddb_convert

# Set the entrypoint
ENTRYPOINT ["/ddb_convert"]

# Default command (show help)
CMD ["--help"]
