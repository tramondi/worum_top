from rust:latest

workdir /app

copy . .
run cargo fetch
run cargo build

cmd ["cargo", "run"]
