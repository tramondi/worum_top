FROM rust:latest

WORKDIR /app

COPY . .
RUN cargo fetch
RUN cargo build --release

ENTRYPOINT ["./target/release/worum_top"]
