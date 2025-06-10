FROM rust:1.80-slim-bookworm

WORKDIR /app
COPY . .
RUN cargo build --release
RUN cp target/release/poDTest /usr/local/bin/poDTest
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]