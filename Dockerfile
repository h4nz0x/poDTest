FROM alpine:3.18
# Install Docker CLI, dos2unix, and bash
RUN apk add --no-cache docker-cli dos2unix bash
WORKDIR /app
COPY poDTest /usr/local/bin/poDTest
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN dos2unix /usr/local/bin/entrypoint.sh && \
    chmod +x /usr/local/bin/entrypoint.sh && \
    ls -l /usr/local/bin/entrypoint.sh && \
    test -f /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]