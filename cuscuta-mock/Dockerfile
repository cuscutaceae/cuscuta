FROM rust:1.94 AS builder
WORKDIR /app
COPY cuscuta-mock/Cargo.toml cuscuta-mock/Cargo.lock ./
COPY cuscuta-mock/src ./src
RUN cargo build --release
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/cuscuta-mock /usr/local/bin/nofyso-runnable
EXPOSE 8080
CMD ["nofyso-runnable"]