# --- build stage ---
FROM rust:trixie AS builder

WORKDIR /app

COPY . ./

RUN cargo build --release

# --- runtime stage ---
FROM debian:trixie

# Install CA certificates for HTTPS support
RUN apt-get update && apt-get install -y ca-certificates

COPY --from=builder /app/target/release/hls-proxy /usr/local/bin/hls-proxy

ENTRYPOINT ["hls-proxy"]
