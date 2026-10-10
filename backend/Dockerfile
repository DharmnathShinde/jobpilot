# Build stage
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Cache dependencies separately from source
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs && cargo build --release && rm -rf src
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs src/lib.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/jobpilot-api /usr/local/bin/jobpilot-api
ENV RUST_LOG=info
EXPOSE 8080
CMD ["jobpilot-api"]
