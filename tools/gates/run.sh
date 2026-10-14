#!/usr/bin/env bash

set -Eeuo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd -- "$script_directory/../.." && pwd)"
cargo_manifest="$repository_root/core/rust/Cargo.toml"
benchmark_manifest="$repository_root/tests/benchmarks/Cargo.toml"

cd "$repository_root"
printf '\n== Rust/TypeScript 门禁 ==\n'
bun install --frozen-lockfile
printf '\n-- cargo fmt --check --\n'
cargo fmt --all --manifest-path "$cargo_manifest" -- --check
printf '\n-- bun run check --\n'
bun run check
printf '\n-- bun run check:coverage --\n'
bun run check:coverage
printf '\n-- benchmark cargo check --\n'
cargo check --manifest-path "$benchmark_manifest"
printf '\n-- benchmark cargo test --no-fail-fast --\n'
cargo test --manifest-path "$benchmark_manifest" --no-fail-fast
printf '\n-- benchmark performance self-test --\n'
cargo run --quiet --manifest-path "$benchmark_manifest" --bin performance_driver -- --self-test
printf '\n-- xiao-driver cargo test --no-fail-fast --\n'
cargo test --manifest-path "$cargo_manifest" -p xiao-driver --no-fail-fast
printf '\n-- bun test --\n'
bun test
printf '\n-- bunx tsc --noEmit --\n'
bunx tsc --noEmit -p tsconfig.json
