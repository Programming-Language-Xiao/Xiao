#!/usr/bin/env bash

set -Eeuo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repository_root="$(cd -- "$script_directory/../.." && pwd)"
mode="${1:-native}"

if [[ "$mode" == "--help" || "$mode" == "-h" ]]; then
    cat <<'EOF'
用法：tools/platform-reproduction/reproduce.sh native

在当前 Linux、WSL 或 macOS 主机执行平台功能复现。Docker 多架构入口由
reproduce.ps1 或 README 中的 docker build/run 命令负责。
EOF
    exit 0
fi

if [[ "$mode" != "native" ]]; then
    printf '不支持的复现模式：%s\n' "$mode" >&2
    exit 64
fi

host_system="$(uname -s)"
host_machine="$(uname -m)"
host_triple="$(rustc -vV | sed -n 's/^host: //p')"

if [[ -z "$host_triple" ]]; then
    printf '无法从 rustc 读取主机目标三元组\n' >&2
    exit 1
fi

if [[ "$host_system" == "Darwin" ]]; then
    if [[ "$host_machine" == "arm64" ]]; then
        default_triple="aarch64-apple-darwin"
    else
        default_triple="x86_64-apple-darwin"
    fi
elif [[ "$host_system" == "Linux" ]]; then
    if [[ "$host_machine" == "aarch64" || "$host_machine" == "arm64" ]]; then
        default_triple="aarch64-unknown-linux-gnu"
    else
        default_triple="x86_64-unknown-linux-gnu"
    fi
else
    printf '此脚本只支持 Linux、WSL 和 macOS；当前系统：%s\n' "$host_system" >&2
    exit 64
fi

target_triple="${XIAO_TARGET_TRIPLE:-$host_triple}"
if [[ "$target_triple" != "$host_triple" ]]; then
    printf 'XIAO_TARGET_TRIPLE=%s 与 rustc host=%s 不一致\n' "$target_triple" "$host_triple" >&2
    exit 1
fi
if [[ "$host_triple" != "$default_triple" ]]; then
    printf '无法确认当前主机三元组：rustc=%s，平台推导=%s\n' "$host_triple" "$default_triple" >&2
    exit 1
fi

resolve_tool() {
    local explicit_path="${1:-}"
    local fallback_name="$2"
    if [[ -n "$explicit_path" ]]; then
        printf '%s\n' "$explicit_path"
        return
    fi
    if [[ "$host_system" == "Darwin" && -x "$(brew --prefix llvm 2>/dev/null)/bin/$fallback_name" ]]; then
        printf '%s\n' "$(brew --prefix llvm)/bin/$fallback_name"
        return
    fi
    command -v "$fallback_name"
}

export XIAO_CLANG="$(resolve_tool "${XIAO_CLANG:-}" clang)"
export XIAO_LLVM_AS="$(resolve_tool "${XIAO_LLVM_AS:-}" llvm-as)"
export XIAO_LLC="$(resolve_tool "${XIAO_LLC:-}" llc)"

for required_tool in "$XIAO_CLANG" "$XIAO_LLVM_AS" "$XIAO_LLC"; do
    if [[ ! -x "$required_tool" ]]; then
        printf '工具不可执行：%s\n' "$required_tool" >&2
        exit 1
    fi
done

clang_version="$($XIAO_CLANG --version | sed -n '1p')"
llvm_as_version="$($XIAO_LLVM_AS --version | sed -n '1p')"
llc_version="$($XIAO_LLC --version | sed -n '1p')"
llvm_major="$(printf '%s\n' "$clang_version" | sed -n 's/.*version \([0-9][0-9]*\).*/\1/p')"
if [[ -z "$llvm_major" || "$llvm_major" -lt 18 ]]; then
    printf 'clang/LLVM 主版本必须至少为 18：%s\n' "$clang_version" >&2
    exit 1
fi

printf '平台：%s/%s\n' "$host_system" "$host_machine"
printf 'Rust：%s\n' "$(rustc --version)"
printf 'Bun：%s\n' "$(bun --version)"
printf '目标：%s\n' "$target_triple"
printf 'clang：%s\n' "$clang_version"
printf 'llvm-as：%s\n' "$llvm_as_version"
printf 'llc：%s\n' "$llc_version"

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/xiao-platform-reproduction.XXXXXX")"
cleanup() {
    rm -rf -- "$temporary_root"
}
trap cleanup EXIT

cargo_manifest="$repository_root/core/rust/Cargo.toml"
benchmark_manifest="$repository_root/tests/benchmarks/Cargo.toml"

printf '\n== 准备 Rust 核心和 Runtime ==\n'
(cd "$repository_root" && cargo build --manifest-path "$cargo_manifest" -p xiao-runtime --release)
(cd "$repository_root" && cargo build --manifest-path "$cargo_manifest" -p xiao-driver -p xiao-diagnostics)

runtime_library="$repository_root/core/rust/target/release/libxiao_runtime.a"
if [[ ! -f "$runtime_library" ]]; then
    printf '找不到 Runtime staticlib：%s\n' "$runtime_library" >&2
    exit 1
fi
export XIAO_RUNTIME_LIBRARY="$runtime_library"
export XIAO_TARGET_TRIPLE="$target_triple"

printf '\n== 环境门控测试（显式 --ignored） ==\n'
ignored_command=(cargo test --manifest-path "$cargo_manifest" --workspace -- --ignored)
if [[ "${XIAO_SKIP_REAL_TERMINAL_TEST:-0}" == "1" && "$host_system" == "Darwin" ]]; then
    printf 'macOS CI 无 GUI 会话，显式跳过真实终端测试：real_terminal_session_is_environment_gated\n'
    ignored_command+=(--skip real_terminal_session_is_environment_gated)
fi
if [[ "${XIAO_USE_XVFB:-0}" == "1" && "$host_system" == "Linux" && -x "$(command -v xvfb-run 2>/dev/null || true)" ]]; then
    (cd "$repository_root" && xvfb-run -a "${ignored_command[@]}")
else
    (cd "$repository_root" && "${ignored_command[@]}")
fi

printf '\n== Rust/TypeScript 门禁 ==\n'
(cd "$repository_root" && cargo test --manifest-path "$cargo_manifest" -p xiao-driver)
(cd "$repository_root" && bun install --frozen-lockfile)
(cd "$repository_root" && bun test)
(cd "$repository_root" && bunx tsc --noEmit -p tsconfig.json)
(cd "$repository_root" && cargo check --manifest-path "$benchmark_manifest")
(cd "$repository_root" && bun run check)
(cd "$repository_root" && bun run check:coverage)
(cd "$repository_root" && cargo fmt --all --manifest-path "$cargo_manifest" -- --check)

printf '\n== 打包、发现、构建和协议回环 ==\n'
package_directory="$temporary_root/package"
(
    cd "$repository_root/cli/ts"
    bun run src/platform/packaging.ts --outdir "$package_directory"
)

cli_path="$package_directory/xiao"
if [[ ! -x "$cli_path" ]]; then
    printf '找不到打包后的 xiao：%s\n' "$cli_path" >&2
    exit 1
fi

source_directory="$temporary_root/source"
mkdir -p "$source_directory/tests" "$temporary_root/outside"
printf 'value = 1 + 2\n' > "$source_directory/main.xiao"
printf 'value = 1 + 2\n' > "$source_directory/tests/smoke.xiao"

native_output="$temporary_root/native"
llvm_output="$temporary_root/native.ll"
build_json="$($cli_path --json build -o "$native_output" "$source_directory/main.xiao" --emit-llvm "$llvm_output" -O0)"
printf '%s\n' "$build_json"
[[ "$build_json" == *'"type":"result"'* ]]
[[ "$build_json" == *'"operation":"build"'* ]]
[[ "$build_json" == *'"exit_code":0'* ]]
[[ -x "$native_output" && -s "$llvm_output" ]]

debug_output="$temporary_root/debug-native"
debug_json="$($cli_path --json build -debug -o "$debug_output" "$source_directory/main.xiao" -O0)"
printf '%s\n' "$debug_json"
[[ "$debug_json" == *'"type":"result"'* ]]
[[ "$debug_json" == *'"exit_code":0'* ]]
[[ -x "$debug_output" ]]

cp "$source_directory/main.xiao" "$temporary_root/outside/main.xiao"
printf '\n== 同目录核心发现 ==\n'
run_json="$(cd "$temporary_root/outside" && env -u XIAO_CORE_PATH PATH=/usr/bin:/bin "$cli_path" --json run "$temporary_root/outside/main.xiao")"
printf '%s\n' "$run_json"
[[ "$run_json" == *'"type":"result"'* ]]
[[ "$run_json" == *'"request_id"'* ]]
[[ "$run_json" == *'"exit_name":"success"'* ]]
[[ "$run_json" == *'"exit_code":0'* ]]

test_json="$(cd "$temporary_root/outside" && env -u XIAO_CORE_PATH PATH=/usr/bin:/bin "$cli_path" --json test "$source_directory")"
printf '%s\n' "$test_json"
[[ "$test_json" == *'"type":"test_result"'* ]]
[[ "$test_json" == *'"total":1'* ]]
[[ "$test_json" == *'"passed":1'* ]]
[[ "$test_json" == *'"exit_code":0'* ]]

if command -v file >/dev/null 2>&1; then
    file "$native_output" "$cli_path"
fi

outside_cli_directory="$temporary_root/path-cli"
path_core_directory="$temporary_root/path-bin"
mkdir -p "$outside_cli_directory" "$path_core_directory"
cp "$cli_path" "$outside_cli_directory/xiao"
cp "$package_directory/xiao-core" "$path_core_directory/xiao-core"
chmod +x "$outside_cli_directory/xiao" "$path_core_directory/xiao-core"

printf '\n== PATH 核心发现 ==\n'
path_run_json="$(cd "$temporary_root/outside" && env -u XIAO_CORE_PATH PATH="$path_core_directory:/usr/bin:/bin" "$outside_cli_directory/xiao" --json run "$temporary_root/outside/main.xiao")"
printf '%s\n' "$path_run_json"
[[ "$path_run_json" == *'"type":"result"'* ]]
[[ "$path_run_json" == *'"exit_name":"success"'* ]]
[[ "$path_run_json" == *'"exit_code":0'* ]]

printf '\n== 开发回环核心发现 ==\n'
development_run_json="$(cd "$repository_root" && env -u XIAO_CORE_PATH PATH=/usr/bin:/bin "$(command -v bun)" "$repository_root/cli/ts/src/main.ts" --json run "$source_directory/main.xiao")"
printf '%s\n' "$development_run_json"
[[ "$development_run_json" == *'"type":"result"'* ]]
[[ "$development_run_json" == *'"exit_name":"success"'* ]]
[[ "$development_run_json" == *'"exit_code":0'* ]]

printf '\n== 版本协商失配 ==\n'
mismatch_json="$("$(command -v bun)" "$script_directory/check-protocol.ts" "$repository_root/core/rust/target/debug/xiao-core")"
printf '%s\n' "$mismatch_json"
[[ "$mismatch_json" == *'"accepted":false'* ]]
[[ "$mismatch_json" == *'"error_code":"X11-PROTOCOL-004"'* ]]

printf '\n平台复现脚本完成：%s\n' "$host_triple"
