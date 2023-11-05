FROM rust:1.73-bookworm AS build
WORKDIR /app/
# Dependencies
RUN apt-get update && apt-get upgrade -y && apt-get install libclang-dev -y
# Copy project files
COPY . .
# Build
RUN --mount=type=cache,target=/usr/local/cargo/registry/ \
    --mount=type=cache,target=/usr/local/cargo/git/db/ \
    --mount=type=cache,target=/app/target \
    cargo build --release
# Export binaries from cache
RUN --mount=type=cache,target=/app/target \
    mv /app/target/release/gartic_on_demand /app



FROM debian:12.2-slim AS final
RUN apt-get update && apt-get upgrade -y && apt-get install libssl3 ca-certificates -y
WORKDIR /app/
COPY --from=build /app/gartic_on_demand /app
COPY ./config.json /app
COPY ./migrations /app/migrations
COPY ./assets /app/assets
ENTRYPOINT ["/app/gartic_on_demand"]
