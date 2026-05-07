# --- build stage ---
FROM rust:trixie AS builder

WORKDIR /app

COPY . ./

RUN cargo build --release

# --- runtime stage ---
FROM debian:trixie

COPY --from=builder /app/target/release/hls-proxy /usr/local/bin/hls-proxy

ENTRYPOINT ["hls-proxy"]
