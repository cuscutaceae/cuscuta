FROM rust:1.94 AS builder
WORKDIR /app
COPY ./cuscuta-chilo/Cargo.toml ./cuscuta-chilo/Cargo.lock ./
COPY ./cuscuta-chilo/src ./src
RUN apt-get update && apt-get install -y ca-certificates clang cmake pkg-config
RUN cargo test
RUN cargo build --release
FROM debian:trixie-slim
WORKDIR /app
COPY --from=builder /app/target/release/cuscuta-chilo /usr/local/bin/nofyso-exe
EXPOSE 8080
CMD ["nofyso-exe"]