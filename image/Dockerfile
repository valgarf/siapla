FROM ubuntu:24.04

# Create directories
RUN mkdir -p /app /data

WORKDIR /app

# Expose port used by server
EXPOSE 80

# Copy the siapla binary (expected to be built externally) into /app
# Build outside the image and `docker build` with --build-arg BINARY=path/to/binary if desired
ARG BINARY=siapla-serve
COPY ${BINARY} /app/siapla-serve

# Ensure executable
RUN chmod +x /app/siapla-serve

CMD ["/app/siapla-serve", "--database-url", "sqlite:/data/default.sqlite"]
