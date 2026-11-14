# Chef image - prepare recipe for dependency caching
FROM rust:1.100.0-slim-trixie AS chef

RUN cargo install cargo-chef --locked
WORKDIR /build

# Plan stage - generate recipe from source
FROM chef AS planner

COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo chef prepare --recipe-path recipe.json

# Builder image - Rust compilation
FROM chef AS builder-image

# Set noninteractive to avoid stuck builds
ARG DEBIAN_FRONTEND=noninteractive

# Disable incremental compilation (useless in Docker, wastes disk)
ENV CARGO_INCREMENTAL=0

# Install mold linker
RUN apt-get update && apt-get install -y --no-install-recommends \
    mold \
    && rm -rf /var/lib/apt/lists/*

# Configure mold as the linker
RUN mkdir -p /build/.cargo && \
    printf '[target.x86_64-unknown-linux-gnu]\nrustflags = ["-C", "link-arg=-fuse-ld=mold"]\n' > /build/.cargo/config.toml

# Cook dependencies from recipe (cached layer)
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy actual source code and build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release

# Runner image
FROM debian:trixie-slim AS runner-image

# CA certificates for HTTPS requests to the scraped websites
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home byrt

# Copy binary
COPY --from=builder-image /build/target/release/eredivisie-standing /home/byrt/app/eredivisie-standing

# Set user and working directory
USER byrt
WORKDIR /home/byrt/app

# Expose port
EXPOSE 8000

CMD ["./eredivisie-standing"]
