# Standalone build
# https://github.com/paritytech/substrate/blob/master/.maintain/Dockerfile
FROM phusion/baseimage:0.10.2 as build

ENV DEBIAN_FRONTEND=noninteractive
ARG PROFILE=release

WORKDIR /src
COPY . /src

RUN apt-get update && \
    apt-get dist-upgrade -y -o Dpkg::Options::="--force-confold" && \
    apt-get install -y cmake pkg-config libssl-dev git clang

ARG TOOLCHAIN=nightly-2021-03-15

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    export PATH="$PATH:$HOME/.cargo/bin" && \
    rustup toolchain install ${TOOLCHAIN} && \
    rustup target add wasm32-unknown-unknown --toolchain ${TOOLCHAIN} && \
    rustup default ${TOOLCHAIN} && \
    cargo build "--$PROFILE"

FROM bitnami/minideb:buster

ARG PROFILE=release

COPY --from=build /src/target/$PROFILE/interbtc-parachain /usr/local/bin

# Checks
RUN chmod +x /usr/local/bin/interbtc-parachain && \
    ldd /usr/local/bin/interbtc-parachain && \
    /usr/local/bin/interbtc-parachain --version

RUN /usr/local/bin/interbtc-parachain export-genesis-state --chain staging --parachain-id 21 > /var/lib/genesis-state
RUN /usr/local/bin/interbtc-parachain export-genesis-wasm --chain staging > /var/lib/genesis-wasm

EXPOSE 30333 9933 9944
VOLUME ["/data"]

CMD ["/usr/local/bin/interbtc-parachain"]
