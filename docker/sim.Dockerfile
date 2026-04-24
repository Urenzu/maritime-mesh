FROM rust:1.87-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p sim

FROM debian:bookworm-slim
COPY --from=builder /build/target/release/sim /usr/local/bin/sim
EXPOSE 10110
CMD ["sim"]
