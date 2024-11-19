FROM rust:alpine3.20 AS build

WORKDIR /app

COPY . .

# Install necessary tools and libraries
RUN apk add --no-cache \
    curl \
    libgcc \
    libstdc++ \
    gcc \
    libc-dev \
    pkgconf \
    libressl-dev

# Build the application
RUN export PKG_CONFIG_PATH="/usr/lib/pkgconfig" && \
    export LD_LIBRARY_PATH="/usr/lib" && \
    cargo build --release

FROM alpine:latest

WORKDIR /app

COPY --from=build /app/target/release/threads /app/threads

# Copy any necessary configuration files
COPY --from=build /app/config /app/config
COPY --from=build /app/.env /app/.env

# Expose the necessary ports
EXPOSE 19998

# Command to run the application
CMD ["/app/threads"]