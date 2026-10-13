FROM rust:1-bookworm AS build

WORKDIR /app

COPY . .

RUN cargo build --release


FROM debian:bookworm-slim

RUN useradd -r -m appuser

WORKDIR /app

COPY --from=build /app/target/release/product-data-platform /usr/local/bin/app

COPY --chown=appuser:appuser data /app/data
COPY --chown=appuser:appuser prompts /app/prompts

RUN mkdir -p /app/results \
    && chown -R appuser:appuser /app/data /app/results /app/prompts

USER appuser

EXPOSE 3000

CMD ["app"]