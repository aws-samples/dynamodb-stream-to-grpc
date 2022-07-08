FROM rust:1.61.0-alpine3.16 as build

WORKDIR /app

RUN apk update && apk add --no-cache make cmake gcc g++

ADD server/Cargo.toml Cargo.toml
ADD server/Cargo.lock Cargo.lock
ADD server/build.rs build.rs
ADD proto proto

RUN sed -i 's#src/server.rs#dummy.rs#' Cargo.toml && \
    echo 'fn main() {}' > dummy.rs && \
    cargo build --release && \
    rm dummy.rs && \
    sed -i 's#dummy.rs#src/server.rs#' Cargo.toml

ADD server/src src
RUN cargo build --release

FROM alpine:3.16

COPY --from=build /app/target/release/server /usr/local/bin/server

EXPOSE 50051

ENTRYPOINT ["server"]
