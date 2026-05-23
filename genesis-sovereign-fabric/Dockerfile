FROM rust:1.93-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p gsf-api -p gsf-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/gsf-server /usr/local/bin/
COPY --from=builder /app/target/release/gsf /usr/local/bin/
EXPOSE 3000
ENV GSF_BIND_ADDR=0.0.0.0:3000
CMD ["gsf-server"]
