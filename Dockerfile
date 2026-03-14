# Stage 1: Build
FROM rust:1.94.0-slim-trixie AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src/ src/

# Touch main.rs to force rebuild of our code (not deps)
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM debian:trixie-slim

# Install CA certificates for HTTPS requests
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN groupadd -r appuser && useradd -r -g appuser appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/eredivisie-standing ./

# Set environment variables
ENV API_KEY=""

# Switch to non-root user
USER appuser

# Expose the port the app runs on
EXPOSE 8000

CMD ["./eredivisie-standing"]
