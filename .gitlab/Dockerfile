FROM rust:latest

RUN apt-get -y update
RUN apt-get install -y build-essential cmake pkg-config libssl-dev clang libclang-dev llvm
RUN rustup toolchain install nightly-2020-03-01
RUN rustup default nightly-2020-03-01
RUN rustup component add rustfmt
RUN rustup toolchain install nightly
RUN rustup target add wasm32-unknown-unknown --toolchain nightly-2020-03-01

RUN cargo install sccache --features "gcs"