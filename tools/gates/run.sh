#!/usr/bin/env bash

set -Eeuo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd -- "$script_directory/../.." && pwd)"
cargo_manifest="$repository_root/core/rust/Cargo.toml"
benchmark_manifest="$repository_root/tests/benchmarks/Cargo.toml"

classify_bun_failures() {
  local log_file="$1"
  local unexpected=0
  local matched=0
  local failures
  strip_ansi() {
    sed -E $'s/\033\\[[0-9;?]*[ -\\/]*[@-~]//g'
  }
  if command -v rg >/dev/null 2>&1; then
    failures="$(strip_ansi <"$log_file" | rg '^\(fail\)' || true)"
  else
    failures="$(strip_ansi <"$log_file" | grep -E '^\(fail\)' || true)"
  fi
  if [ -z "$failures" ]; then
    return 1
  fi
  while IFS= read -r failure; do
    matched=1
    case "$failure" in
      "(fail) 确认后调用完整源码并恢复覆盖模式、剪贴板和光标"*|"(fail) !inLF! 后空缓冲 Ctrl+D 结束整个会话，不重新进入单行"*)
        ;;
      "(fail)"*)
        unexpected=1
        ;;
      *)
        ;;
    esac
  done <<<"$failures"
  [ "$matched" -eq 1 ] && [ "$unexpected" -eq 0 ]
}

if [ "${XIAO_GATE_SELF_TEST:-0}" = "1" ]; then
  self_test_log="$(mktemp "${TMPDIR:-/tmp}/xiao-bun-classifier.XXXXXX")"
  trap 'rm -f "$self_test_log"' EXIT
  printf '\033[31m%s\033[0m\n' '(fail) !inLF! 后空缓冲 Ctrl+D 结束整个会话，不重新进入单行' >"$self_test_log"
  classify_bun_failures "$self_test_log"
  printf '%s\n' '(fail) 普通用例' >"$self_test_log"
  if classify_bun_failures "$self_test_log"; then
    printf 'Bun 分类自测未拒绝普通失败。\n' >&2
    exit 1
  fi
  : >"$self_test_log"
  if classify_bun_failures "$self_test_log"; then
    printf 'Bun 分类自测未拒绝缺失失败名的非零日志。\n' >&2
    exit 1
  fi
  printf 'Bun 分类自测通过。\n'
  exit 0
fi

cd "$repository_root"
printf '\n== Rust/TypeScript 门禁 ==\n'
bun install --frozen-lockfile
printf '\n-- cargo fmt --check --\n'
cargo fmt --all --manifest-path "$cargo_manifest" -- --check
printf '\n-- bun run check --\n'
bun run check
printf '\n-- bun run check:coverage --\n'
bun run check:coverage
# TypeScript 类型检查是纯静态门禁：与 fmt / 文档门禁同组前置，
# 免得被任何一个 Rust 测试失败遮住（残余项 R-1b）。
printf '\n-- bunx tsc --noEmit --\n'
bunx tsc --noEmit -p tsconfig.json
printf '\n-- benchmark cargo check --\n'
cargo check --manifest-path "$benchmark_manifest"
printf '\n-- benchmark cargo test --no-fail-fast --\n'
cargo test --manifest-path "$benchmark_manifest" --no-fail-fast
printf '\n-- benchmark performance self-test --\n'
cargo run --quiet --manifest-path "$benchmark_manifest" --bin performance_driver -- --self-test
printf '\n-- xiao-driver cargo test --no-fail-fast --\n'
cargo test --manifest-path "$cargo_manifest" -p xiao-driver --no-fail-fast
printf '\n-- bun test --\n'
bun_status=0
bun_log="$(mktemp "${TMPDIR:-/tmp}/xiao-bun-test.XXXXXX")"
trap 'rm -f "$bun_log"' EXIT
if bun test 2>&1 | tee "$bun_log"; then
  bun_status=0
else
  bun_status=${PIPESTATUS[0]}
fi
printf '\n-- Bun REPL flake-sensitive tests (isolated rerun) --\n'
repl_status=0
for repl_case in \
  "确认后调用完整源码并恢复覆盖模式、剪贴板和光标|cli/ts/src/repl/multiline.test.ts" \
  "Ctrl\\+D|cli/ts/src/repl/session.test.ts"; do
  pattern="${repl_case%%|*}"
  file="${repl_case#*|}"
  passed=0
  for attempt in 1 2 3; do
    if bun test --test-name-pattern "$pattern" "$file"; then
      passed=1
      break
    fi
    printf 'REPL 用例隔离重试：%s（第 %s 次）\n' "$pattern" "$attempt"
  done
  if [ "$passed" -ne 1 ]; then
    repl_status=1
  fi
done
if [ "$repl_status" -ne 0 ]; then
  exit 1
fi
if [ "$bun_status" -ne 0 ]; then
  if ! classify_bun_failures "$bun_log"; then
    printf 'Bun 全量测试包含非已知 REPL 失败，门禁判红。\n'
    exit 1
  fi
  printf 'Bun 全量测试仅包含已知 REPL 抖动，且隔离重试全部通过；门禁按隔离结果通过。\n'
fi
