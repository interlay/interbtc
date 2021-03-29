FROM rust:latest

RUN apt-get -y update
RUN apt-get install -y build-essential cmake pkg-config libssl-dev clang libclang-dev llvm

ARG TOOLCHAIN=nightly-2021-03-15
RUN rustup toolchain install ${TOOLCHAIN}
RUN rustup default ${TOOLCHAIN}
RUN rustup component add rustfmt
RUN rustup target add wasm32-unknown-unknown --toolchain ${TOOLCHAIN}

RUN cargo install sccache --features "gcs"