# backend-rust のイメージ（compose から使う）。
FROM rust:1.98.1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p shop

FROM debian:bookworm-slim
COPY --from=build /src/target/release/shop /usr/local/bin/shop
USER nobody
EXPOSE 8000
CMD ["shop"]
