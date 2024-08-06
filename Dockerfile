FROM rust:latest

WORKDIR /app

COPY . .
RUN cargo fetch
RUN cargo build --release

CMD ["cargo", "run"]
