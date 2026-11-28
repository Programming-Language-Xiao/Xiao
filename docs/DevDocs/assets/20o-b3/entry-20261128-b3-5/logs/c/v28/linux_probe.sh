#!/usr/bin/env bash
set -euo pipefail

logs="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$logs/../../../../../../../.." && pwd)"
output="$(mktemp -d)"
export CARGO_TARGET_DIR="$output/target"
cd "$repo/core/rust"
rustc -vV
clang --version
cargo build --locked --release --target x86_64-unknown-linux-gnu -p xiao-runtime
runtime="$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/libxiao_runtime.a"
clang -target x86_64-unknown-linux-gnu -std=c11 -Wall -Wextra -Werror \
  "$logs/../probe.c" "$runtime" -ldl -lpthread -lm -o "$output/probe"
"$output/probe" > "$output/valid.stdout" 2> "$output/valid.stderr"
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
sha256sum "$runtime" "$output/probe" "$output/"*.stdout "$output/"*.stderr
cat "$output/valid.stderr" "$output/closed.stderr" "$output/broken.stderr"
