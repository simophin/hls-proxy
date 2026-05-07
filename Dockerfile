# --- build stage ---
# Pin to bookworm so the compiled binary targets the same glibc as the runtime image
FROM rust:slim-bookworm AS builder

WORKDIR /app

# Cache dependencies by building a dummy binary first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Build the real binary (only re-compiles our code, deps are cached)
COPY src ./src
RUN touch src/main.rs && cargo build --release

# --- runtime stage ---
FROM debian:bookworm-slim

# ca-certificates: TLS cert validation for upstream requests
# libssl3: runtime OpenSSL library required by native-tls
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/hls-proxy /usr/local/bin/hls-proxy

ENTRYPOINT ["hls-proxy"]
