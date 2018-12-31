FROM clux/muslrust:latest as builder

# Cache dependencies
RUN mkdir ogn-web-gateway
WORKDIR /ogn-web-gateway
RUN mkdir src; echo "fn main(){}" > src/main.rs
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
COPY ./src ./src
# dont use the cached main
RUN touch ./src/main.rs

# Build the binary
RUN cargo build --release

FROM alpine:latest
RUN apk --no-cache add ca-certificates
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_DIR=/etc/ssl/certs

WORKDIR /app
COPY --from=builder /ogn-web-gateway/target/x86_64-unknown-linux-musl/release/ogn-web-gateway .

ENTRYPOINT ["./ogn-web-gateway"]
