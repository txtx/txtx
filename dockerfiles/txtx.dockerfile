FROM rust:bullseye as build

WORKDIR /src

RUN apt update && apt install -y ca-certificates pkg-config libssl-dev libclang-11-dev

RUN rustup update 1.75.0 && rustup default 1.75.0

COPY ./txtx /src/txtx

COPY ./txtx-supervisor-ui /src/txtx-supervisor-ui

WORKDIR /src/txtx

RUN mkdir /out

RUN cargo build --release

RUN cp /src/txtx/target/release/txtx /out

FROM debian:bullseye-slim

RUN apt update && apt install -y ca-certificates libssl-dev

COPY --from=build /out/ /bin/

WORKDIR /workspace

ENTRYPOINT ["txtx"]
