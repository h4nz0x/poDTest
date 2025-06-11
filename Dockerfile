FROM alpine:3.18

WORKDIR /app
COPY poDTest /usr/local/bin/poDTest
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]