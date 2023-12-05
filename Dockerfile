FROM rust:1.74-bookworm as builder
WORKDIR /app
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl-dev && apt-get install -y --no-install-recommends ca-certificates && update-ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/ttpm /usr/local/bin/ttpm
WORKDIR /ttpm-data
CMD ["ttpm"]
