# --- build stage ---
FROM rust:slim-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-musl

# Cache dependencies by building a dummy binary first
COPY Cargo.toml Cargo.lock ./
COPY .cargo .cargo
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release --target x86_64-unknown-linux-musl \
    && rm -rf src

# Build the real binary (only re-compiles our code, deps are cached)
COPY src ./src
RUN touch src/main.rs && cargo build --release --target x86_64-unknown-linux-musl

# --- runtime stage ---
# Static musl binary has no glibc dependency; alpine provides ca-certificates only
FROM alpine:3

RUN apk add --no-cache ca-certificates

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/hls-proxy /usr/local/bin/hls-proxy

ENTRYPOINT ["hls-proxy"]
