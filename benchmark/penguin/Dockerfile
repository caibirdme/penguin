FROM ubuntu:24.10
ENV RUST_LOG=error
COPY ./target/release/penguin /usr/local/bin/penguin
COPY ./benchmark/penguin/gateway.yaml /etc/penguin/gateway.yaml
CMD ["penguin", "-c", "/etc/penguin/gateway.yaml", "run"]