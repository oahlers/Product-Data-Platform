FROM rust:1-bookworm AS build
WORKDIR /app
COPY . .
RUN cargo build --release
FROM debian:bookworm-slim
RUN useradd -r appuser
COPY --from=build /app/target/release/product-data-platform /usr/local/bin/app
COPY data /app/data
COPY prompts /app/prompts
WORKDIR /app
USER appuser
EXPOSE 3000
CMD ["app"]
