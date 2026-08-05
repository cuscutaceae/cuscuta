FROM rust:1.94 AS builder
WORKDIR /app
# Copy workspace manifest and all member manifests
COPY Cargo.toml ./
COPY cuscuta-common/Cargo.toml cuscuta-common/
COPY cuscuta-entry/Cargo.toml cuscuta-entry/
COPY cuscuta-worker/Cargo.toml cuscuta-worker/
COPY cuscuta-chilo/Cargo.toml cuscuta-chilo/
COPY cuscuta-mock/Cargo.toml cuscuta-mock/
COPY cuscuta-test/Cargo.toml cuscuta-test/
COPY cuscutactl/Cargo.toml cuscutactl/
# Dummy sources for unused members (allows dep caching)
RUN mkdir -p cuscuta-common/src && echo "//! Dummy" > cuscuta-common/src/lib.rs
RUN mkdir -p cuscuta-entry/src && echo "//! Dummy \nfn main() {}" > cuscuta-entry/src/main.rs
RUN mkdir -p cuscuta-worker/src && echo "//! Dummy \nfn main() {}" > cuscuta-worker/src/main.rs
RUN mkdir -p cuscuta-chilo/src && echo "//! Dummy \nfn main() {}" > cuscuta-chilo/src/main.rs
RUN mkdir -p cuscuta-mock/src && echo "//! Dummy \nfn main() {}" > cuscuta-mock/src/main.rs
RUN mkdir -p cuscuta-test/src && echo "//! Dummy \nfn main() {}" > cuscuta-test/src/lib.rs
RUN mkdir -p cuscutactl/src && echo "//! Dummy \nfn main() {}" > cuscutactl/src/main.rs
# Pre-build to cache dependencies
RUN cargo build --release -p cuscuta-entry
# Copy real source and rebuild
COPY cuscuta-common/src cuscuta-common/src
COPY cuscuta-entry/src cuscuta-entry/src
RUN touch cuscuta-common/src/lib.rs cuscuta-entry/src/main.rs
RUN cargo build --release -p cuscuta-entry
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates
WORKDIR /app
COPY --from=builder /app/target/release/cuscuta-entry /usr/local/bin/nofyso-exe
EXPOSE 8080
CMD ["nofyso-exe"]
