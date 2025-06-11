FROM alpine:3.18

# Install Docker CLI
RUN apk add --no-cache docker-cli

WORKDIR /app
COPY poDTest /usr/local/bin/poDTest
COPY entrypoint.sh .
RUN chmod +x .

ENTRYPOINT ["./entrypoint.sh"]