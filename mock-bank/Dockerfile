FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p mock-bank

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mock-bank /usr/local/bin/mock-bank
CMD ["mock-bank"]
