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

ARG TOOLCHAIN=nightly-2021-01-25

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    export PATH="$PATH:$HOME/.cargo/bin" && \
    rustup toolchain install ${TOOLCHAIN} && \
    rustup target add wasm32-unknown-unknown --toolchain ${TOOLCHAIN} && \
    rustup default ${TOOLCHAIN} && \
    cargo build "--$PROFILE"

FROM bitnami/minideb:buster

ARG PROFILE=release

COPY --from=build /src/target/$PROFILE/btc-parachain /usr/local/bin
COPY --from=build /src/polkabtc-rococo.json /var/lib

# Checks
RUN chmod +x /usr/local/bin/btc-parachain && \
    ldd /usr/local/bin/btc-parachain && \
    /usr/local/bin/btc-parachain --version

RUN /usr/local/bin/btc-parachain export-genesis-state --chain /var/lib/polkabtc-rococo.json > /var/lib/genesis-state
RUN /usr/local/bin/btc-parachain export-genesis-wasm --chain /var/lib/polkabtc-rococo.json > /var/lib/genesis-wasm

EXPOSE 30333 9933 9944
VOLUME ["/data"]

CMD ["/usr/local/bin/btc-parachain"]
