FROM rust:latest AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/main.rs src/main.rs
RUN cargo fetch

RUN cargo build --release

FROM ubuntu:latest AS release

WORKDIR /app

RUN apt-get update
RUN apt-get install -y \
  openssl \
  ca-certificates

COPY --from=builder /app/target/release/worum_top /app/worum_top

CMD ["./worum_top"]
