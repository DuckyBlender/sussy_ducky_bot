FROM rust:1-slim-trixie AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir /data && chown nobody:nogroup /data
COPY --from=builder /app/target/release/sussy_ducky_bot /usr/local/bin/sussy_ducky_bot
# prompts.json and logs/ live here
ENV DATA_DIR=/data
USER nobody
CMD ["sussy_ducky_bot"]
