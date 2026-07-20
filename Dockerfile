FROM rust:1.85-bookworm AS builder

RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

COPY . .

RUN cargo build --release --target wasm32-unknown-unknown

FROM scratch
COPY --from=builder /app/target/wasm32-unknown-unknown/release/*.wasm /
