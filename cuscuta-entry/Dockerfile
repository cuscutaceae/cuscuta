FROM rust:1.94 AS builder
WORKDIR /app/cuscuta-common
COPY cuscuta-common/Cargo.toml cuscuta-common/Cargo.lock ./
COPY cuscuta-common/src ./src
WORKDIR /app/entry
COPY cuscuta-entry/Cargo.toml cuscuta-entry/Cargo.lock ./
COPY cuscuta-entry/src ./src
RUN cargo build --release
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates
WORKDIR /app
COPY --from=builder /app/entry/target/release/cuscuta-entry /usr/local/bin/nofyso-exe
EXPOSE 8080
CMD ["nofyso-exe"]