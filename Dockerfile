FROM debian:bullseye-slim

# Install OpenSSL for dynamic binary (skip if using musl)
RUN apt-get update && apt-get install -y libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY poDTest /usr/local/bin/poDTest
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]