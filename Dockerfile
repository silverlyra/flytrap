FROM rust:latest as builder

WORKDIR /usr/src/app
COPY . .
# Will build and cache the binary and dependent crates in release mode
RUN --mount=type=cache,target=/usr/local/cargo,from=rust:latest,source=/usr/local/cargo \
  --mount=type=cache,target=target \
  cargo build --release --package flytrap-demo && \
  mv ./target/release/flytrap-demo /flytrap

FROM debian:bookworm-slim

RUN useradd -s /bin/bash -d /app -m app

USER app
WORKDIR /app

COPY --from=builder /flytrap /app/flytrap

CMD /app/flytrap
