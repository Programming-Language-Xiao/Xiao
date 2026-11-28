#!/usr/bin/env bash
set -euo pipefail

logs="$(cd "$(dirname "$0")" && pwd)"
workspace="$(cd "$logs/../../../../../../.." && pwd)"
build_dir="$HOME/.cache/xiao-b3-5c-target"
output="$HOME/.cache/xiao-b3-5c-probe"
mkdir -p "$output"
cd "$workspace/core/rust"

export CARGO_TARGET_DIR="$build_dir"
cargo build --locked --release --target x86_64-unknown-linux-gnu -p xiao-runtime
runtime="$build_dir/x86_64-unknown-linux-gnu/release/libxiao_runtime.a"
sha256sum "$runtime"
clang -target x86_64-unknown-linux-gnu -std=c11 -Wall -Wextra -Werror \
  "$logs/probe.c" "$runtime" -ldl -lpthread -lm -o "$output/probe"
"$output/probe" > "$output/probe.stdout" 2> "$output/probe.stderr"
"$output/probe" closed > "$output/closed.stdout" 2> "$output/closed.stderr"
python3 -c '
import os
import subprocess
import sys

reader, writer = os.pipe()
os.close(reader)
try:
    subprocess.run([sys.argv[1], "broken"], stdout=writer, check=True)
finally:
    os.close(writer)
' "$output/probe" > "$output/broken.stdout" 2> "$output/broken.stderr"
sha256sum "$output/"*
cat "$output/probe.stderr" "$output/closed.stderr" "$output/broken.stderr"
