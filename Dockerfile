# Using latest as of 2024-04-07
FROM rust:1.77

# Switch working directory to `app`
# Docker will mkdir if it doesn't exist
WORKDIR /app
RUN apt update && apt install lld clang -y
COPY . .

# Building Binary in release
RUN cargo build --release

# When `docker run`
ENTRYPOINT ["./target/release/zero2prod"]
