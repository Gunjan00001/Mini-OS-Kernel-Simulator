# syntax=docker/dockerfile:1

# ---- build ----
FROM rust:1-slim AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

# ---- runtime ----
FROM debian:bookworm-slim
COPY --from=build /app/target/release/mini-os-kernel /usr/local/bin/mini-os-kernel
# The simulator is an interactive menu: docker run -it --rm mini-os-kernel
ENTRYPOINT ["mini-os-kernel"]
