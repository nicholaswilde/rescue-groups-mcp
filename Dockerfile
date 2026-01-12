FROM rust:latest AS chef
# We only pay the installation cost once,
# it will be cached from the second build onwards
RUN apt-get update && apt-get install -y cmake && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin rescue-groups-mcp

# We do not need the Rust toolchain to run the binary!
FROM gcr.io/distroless/cc-debian12 AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/rescue-groups-mcp /app/rescue-groups-mcp
ENTRYPOINT ["/app/rescue-groups-mcp"]
