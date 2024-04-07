# Using latest as of 2024-04-07
FROM lukemathwalker/cargo-chef:0.1.66-rust-1.77 AS chef
# Switch working directory to `app`
# Docker will mkdir if it doesn't exist
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef AS planner
COPY . .
# Compute a lock-like file...
RUN cargo chef prepare --recipe-path recipe.json


# Builder Stage
# FROM rust:1.77 AS builder
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build project dependencies, not application!
RUN cargo chef cook --release --recipe-path recipe.json
# If Dependencies stay the same, previous layers remain cached
COPY . .
# Setting SQLX to offline mode
ENV SQLX_OFFLINE true
# Building Binary in release
RUN cargo build --release --bin zero2prod

# Runtime Stage
FROM debian:bookworm-slim AS runtime
# FROM rust:1.77 AS runtime
WORKDIR /app
# Install OpenSSL - Dynamically linked to some dependencies
# Install ca-certificates - Needed for HTTPS connection
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates \
  # Clean up (every body do your share)
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*
# Copy compiled binary from builder environment
COPY --from=builder /app/target/release/zero2prod zero2prod
# Need configuration file at runtime
COPY configuration configuration
ENV APP_ENVIRONMENT production
# When `docker run`
ENTRYPOINT ["./zero2prod"]

