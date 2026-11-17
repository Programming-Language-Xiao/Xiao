#!/usr/bin/env bash
set -u

label=$1
shift
case "$label" in
    *[!a-zA-Z0-9_-]*|"") exit 2 ;;
esac

task_root=/home/xiaoczx/.b6/b6-1-20261117-01
repository=/mnt/d/project/Xiao/Xiao
mkdir -p "$task_root/target" "$task_root/temp" "$task_root/home" "$task_root/logs"
case "$task_root" in /home/*) ;; *) echo "B6 task root outside Linux home" >&2; exit 2 ;; esac

removed=""
while IFS= read -r name; do
    case "$name" in
        XIAO_*|RUST_BIGDECIMAL_*|RUST_TEST_THREADS|CARGO_TARGET_DIR|RUSTC|CC|CXX)
            removed="$removed$name,"
            unset "$name"
            ;;
    esac
done < <(compgen -e)

export HOME="$task_root/home"
export CARGO_HOME=/home/xiaoczx/.cargo
export RUSTUP_HOME=/home/xiaoczx/.rustup
export PATH=/home/xiaoczx/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export RUSTUP_TOOLCHAIN=1.96.0
export CARGO_TARGET_DIR="$task_root/target"
export CARGO_BUILD_JOBS=4
export CARGO_TERM_COLOR=never
export TEMP="$task_root/temp" TMP="$task_root/temp" TMPDIR="$task_root/temp"
export XIAO_HOME="$task_root/home"
export XIAO_STDLIB_ROOT="$repository/stdlib"
export XIAO_RUNTIME_LIBRARY="$task_root/target/release/libxiao_runtime.a"
export XIAO_TARGET_TRIPLE=x86_64-unknown-linux-gnu
export XIAO_CLANG=/usr/bin/clang XIAO_LLVM_AS=/usr/bin/llvm-as XIAO_LLC=/usr/bin/llc
export XIAO_B1_F10=0 XIAO_B2_A_PERF=0
export PYTHONUTF8=1 PYTHONIOENCODING=utf-8
unset RUST_TEST_THREADS
cd "$repository"

printf '__B6_REMOVED_ENV__=%s\n' "$removed"
printf '__B6_WSL_HOST__=%s\n' "$(hostname)"
printf '__B6_WSL_KERNEL__=%s\n' "$(uname -srmo)"
printf '__B6_RUSTC__=%s\n' "$(rustc -Vv | head -1)"
printf '__B6_CARGO__=%s\n' "$(cargo -V | head -1)"
printf '__B6_RUSTC_SHA__=%s\n' "$(sha256sum "$(rustup which rustc)" | cut -d' ' -f1)"
printf '__B6_CARGO_SHA__=%s\n' "$(sha256sum "$(rustup which cargo)" | cut -d' ' -f1)"
printf '__B6_CLANG__=%s\n' "$(clang --version | head -1)"
printf '__B6_LLVM_AS__=%s\n' "$(llvm-as --version | head -1)"
printf '__B6_LLC__=%s\n' "$(llc --version | head -1)"
printf '__B6_LLVM_TOOLS_SHA__=%s\n' "$(sha256sum /usr/bin/clang /usr/bin/llvm-as /usr/bin/llc | sha256sum | cut -d' ' -f1)"
printf '__B6_CARGO_COMMAND__='
printf '%q ' "$@"
printf '\n'

log="$task_root/logs/$label.log"
if [ -e "$log" ]; then
    echo "B6 log exists; refusing overwrite" >&2
    exit 2
fi
"$@" > "$log" 2>&1 &
child=$!
printf '__B6_CARGO_PID__=%s\n' "$child"
if wait "$child"; then
    code=0
else
    code=$?
fi
cat "$log"
if [ -f "$XIAO_RUNTIME_LIBRARY" ]; then
    printf '__B6_RUNTIME__=%s %s\n' "$(stat -c '%s' "$XIAO_RUNTIME_LIBRARY")" "$XIAO_RUNTIME_LIBRARY"
    printf '__B6_RUNTIME_SHA__=%s\n' "$(sha256sum "$XIAO_RUNTIME_LIBRARY" | cut -d' ' -f1)"
else
    printf '__B6_RUNTIME__=missing\n'
fi
find "$task_root/target/release/deps" -maxdepth 1 -type f \
    \( -name '20o_b2_*' -o -name 'xiao_runtime-*' -o -name 'xiao_driver-*' \
       -o -name 'libxiao_runtime-*.rlib' -o -name 'libxiao_driver-*.rlib' \) \
    ! -name '*.d' ! -name '*.rmeta' -print0 |
    sort -z | xargs -0 -r sha256sum
find "$task_root/temp" -type f \
    \( -name '*.exe' -o -name '*.ll' -o -name '*.o' -o -name '*.xiao' \
       -o -name '*.xiaoc' -o -name '*.xar' \) -print0 |
    sort -z | xargs -0 -r sha256sum
exit "$code"
