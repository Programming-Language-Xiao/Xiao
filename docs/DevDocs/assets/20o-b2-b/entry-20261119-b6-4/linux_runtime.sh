#!/usr/bin/env bash
set -Eeuo pipefail

sudo apt-get update
sudo apt-get install --yes clang llvm

clang="$(command -v clang)"
llvm_as="$(command -v llvm-as)"
llc="$(command -v llc)"
strip="$(command -v llvm-strip)"
target="$(rustc -vV | sed -n 's/^host: //p')"
test -n "$target"

runtime="$PWD/core/rust/target/release/libxiao_runtime.a"
cargo build --manifest-path core/rust/Cargo.toml --locked --release -p xiao-runtime
test -s "$runtime"

{
  printf 'XIAO_RUNTIME_LIBRARY=%s\n' "$runtime"
  printf 'XIAO_TARGET_TRIPLE=%s\n' "$target"
  printf 'XIAO_CLANG=%s\n' "$clang"
  printf 'XIAO_LLVM_AS=%s\n' "$llvm_as"
  printf 'XIAO_LLC=%s\n' "$llc"
  printf 'XIAO_STRIP=%s\n' "$strip"
} >> "$GITHUB_ENV"
