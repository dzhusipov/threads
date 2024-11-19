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
    pkgconf 

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
COPY --from=build /app/tmp /app/tmp

# Expose the necessary ports
EXPOSE 9292

# Command to run the application
CMD ["/app/threads"]