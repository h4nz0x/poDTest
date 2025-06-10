FROM rust:1.80-slim-bookworm

WORKDIR /app
COPY . .
RUN cp ./poDTest /usr/local/bin/poDTest
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]