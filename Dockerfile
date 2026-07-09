FROM rust:1.91 AS builder

WORKDIR /app
COPY ./src /app/src
COPY ./Cargo.toml /app/Cargo.toml
COPY ./Cargo.lock /app/Cargo.lock
RUN cargo build --release

FROM debian:13-slim AS runtime

WORKDIR /app
COPY --from=builder /app/target/release/e57-to-las /app/e57-to-las

ENTRYPOINT ["/app/e57-to-las"]
