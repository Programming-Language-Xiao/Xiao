#!/usr/bin/env bash

set -Eeuo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd -- "$script_directory/../.." && pwd)"
cargo_manifest="$repository_root/core/rust/Cargo.toml"
benchmark_manifest="$repository_root/tests/benchmarks/Cargo.toml"

cd "$repository_root"
printf '\n== Rust/TypeScript 门禁 ==\n'
cargo test --manifest-path "$cargo_manifest" -p xiao-driver
bun install --frozen-lockfile
bun test
bunx tsc --noEmit -p tsconfig.json
cargo check --manifest-path "$benchmark_manifest"
cargo test --manifest-path "$benchmark_manifest"
cargo run --quiet --manifest-path "$benchmark_manifest" --bin performance_driver -- --self-test
bun run check
bun run check:coverage
cargo fmt --all --manifest-path "$cargo_manifest" -- --check
