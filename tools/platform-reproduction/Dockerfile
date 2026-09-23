# Xiao Linux 多架构平台复现镜像。
#
# 这份镜像只用于功能复现和门禁，不替代 Dockerfile.dev，也不采集 09R3 性能数字。
FROM oven/bun:1.4.0 AS bun

FROM rust:1.96.0

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install --no-install-recommends --yes \
        build-essential \
        ca-certificates \
        clang \
        file \
        lld \
        llvm \
        pkg-config \
        xvfb \
        xterm \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add rustfmt clippy \
    && clang --version \
    && llvm-as --version \
    && llc --version

COPY --from=bun /usr/local/bin/bun /usr/local/bin/bun

WORKDIR /workspace

ENV CARGO_TERM_COLOR=never \
    RUST_BACKTRACE=1
