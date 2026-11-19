"""从Windows控制隔离Ubuntu WSL命令；绑定源码、Linux工具、PID和本地Runtime。"""
from datetime import datetime, timezone
import json
from pathlib import Path
import shlex
import subprocess
import sys
import time

sys.dont_write_bytecode = True
import record

ROOT, REPOSITORY = record.ROOT, record.REPOSITORY
DISTRO = "Ubuntu-26.04"
WSL_HOME = "/home/xiaoczx"
WSL_TASK = WSL_HOME + "/.b6/b6-1-20261117-01"
WSL_REPOSITORY = "/mnt/d/project/Xiao/Xiao"


def command_script(argv):
    cargo_command = shlex.join(argv)
    task = shlex.quote(WSL_TASK)
    runtime = shlex.quote(WSL_TASK + "/target/release/libxiao_runtime.a")

    return "\n".join([
        "set -u",
        "removed=''",
        "while IFS= read -r name; do",
        "  case \"$name\" in XIAO_*|RUST_BIGDECIMAL_*|RUST_TEST_THREADS|CARGO_TARGET_DIR) removed=\"$removed$name,\"; unset \"$name\" ;; esac",
        "done < <(compgen -e)",
        "printf '__B6_REMOVED_ENV__=%s\\n' \"$removed\"",
        "TASK=" + task,
        "REPO=" + shlex.quote(WSL_REPOSITORY),
        "mkdir -p \"$TASK/target\" \"$TASK/temp\" \"$TASK/home\" \"$TASK/logs\"",
        "case \"$TASK\" in /home/*) ;; *) echo 'B6_TASK_ROOT_OUTSIDE_LINUX_HOME' >&2; exit 2 ;; esac",
        "export HOME=\"$TASK/home\"",
        "export CARGO_HOME=" + shlex.quote(WSL_HOME + "/.cargo"),
        "export RUSTUP_HOME=" + shlex.quote(WSL_HOME + "/.rustup"),
        "export PATH=" + shlex.quote(WSL_HOME + "/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"),
        "export RUSTUP_TOOLCHAIN=1.96.0",
        "export CARGO_TARGET_DIR=\"$TASK/target\"",
        "export CARGO_BUILD_JOBS=4",
        "export CARGO_TERM_COLOR=never",
        "export TEMP=\"$TASK/temp\" TMP=\"$TASK/temp\" TMPDIR=\"$TASK/temp\"",
        "export XIAO_HOME=\"$TASK/home\"",
        "export XIAO_STDLIB_ROOT=\"$REPO/stdlib\"",
        "export XIAO_RUNTIME_LIBRARY=" + runtime,
        "export XIAO_TARGET_TRIPLE=x86_64-unknown-linux-gnu",
        "export XIAO_CLANG=/usr/bin/clang XIAO_LLVM_AS=/usr/bin/llvm-as XIAO_LLC=/usr/bin/llc",
        "export XIAO_B1_F10=0 XIAO_B2_A_PERF=0",
        "export PYTHONUTF8=1 PYTHONIOENCODING=utf-8",
        "unset RUST_TEST_THREADS",
        "cd \"$REPO\"",
        "printf '__B6_WSL_HOST__=%s\\n' \"$(hostname)\"",
        "printf '__B6_WSL_KERNEL__=%s\\n' \"$(uname -srmo)\"",
        "printf '__B6_RUSTC__=%s\\n' \"$(rustc -Vv | head -1)\"",
        "printf '__B6_CARGO__=%s\\n' \"$(cargo -V | head -1)\"",
        "printf '__B6_RUSTC_SHA__=%s\\n' \"$(sha256sum \"$(rustup which rustc)\" | cut -d' ' -f1)\"",
        "printf '__B6_CARGO_SHA__=%s\\n' \"$(sha256sum \"$(rustup which cargo)\" | cut -d' ' -f1)\"",
        "printf '__B6_CLANG__=%s\\n' \"$(clang --version | head -1)\"",
        "printf '__B6_LLVM_AS__=%s\\n' \"$(llvm-as --version | head -1)\"",
        "printf '__B6_LLC__=%s\\n' \"$(llc --version | head -1)\"",
        "printf '__B6_LLVM_TOOLS_SHA__=%s\\n' \"$(sha256sum /usr/bin/clang /usr/bin/llvm-as /usr/bin/llc | sha256sum | cut -d' ' -f1)\"",
        "printf '__B6_COMMAND__=%s\\n' " + shlex.quote(cargo_command),
        "mkdir -p \"$TASK/logs\"",
        "( " + cargo_command + " ) > \"$TASK/logs/current.log\" 2>&1 &",
        "child=$!",
        "printf '__B6_CARGO_PID__=%s\\n' \"$child\"",
        "if wait \"$child\"; then code=0; else code=$?; fi",
        "cat \"$TASK/logs/current.log\"",
        "if test -f \"$XIAO_RUNTIME_LIBRARY\"; then",
        "  printf '__B6_RUNTIME__=%s\\n' \"$(stat -c '%s %n' \"$XIAO_RUNTIME_LIBRARY\")\"",
        "  printf '__B6_RUNTIME_SHA__=%s\\n' \"$(sha256sum \"$XIAO_RUNTIME_LIBRARY\" | cut -d' ' -f1)\"",
        "else printf '__B6_RUNTIME__=missing\\n'; fi",
        "exit \"$code\"",
    ])


def main():
    record.require(len(sys.argv) >= 4 and sys.argv[2] == "--",
                   "用法：linux_record.py label -- cargo arguments")
    label, argv = sys.argv[1], sys.argv[3:]
    record.require(label and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-_" for c in label),
                   "Linux回执标签非法")
    receipt_path = ROOT / ("linux-" + label + ".json")
    record.require(not receipt_path.exists(), "Linux回执禁止覆盖")
    before = record.source()
    runner = ROOT / "linux_run.sh"
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    child = subprocess.Popen(
        ["wsl.exe", "-d", DISTRO, "--", "/bin/bash",
         WSL_REPOSITORY + "/" + runner.relative_to(REPOSITORY).as_posix(),
         label, *argv],
        cwd=REPOSITORY, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    raw, _ = child.communicate()
    elapsed = time.monotonic() - clock
    finished = datetime.now(timezone.utc).isoformat()
    after = record.source()
    text = raw.decode("utf-8", errors="replace")
    def marker(name):
        for line in text.splitlines():
            if line.startswith(name + "="):
                return line.split("=", 1)[1]
        return None
    receipt = {
        "label": label, "argv": argv, "wsl_distro": DISTRO,
        "repository": WSL_REPOSITORY, "task_root": WSL_TASK,
        "pid": marker("__B6_CARGO_PID__"),
        "started_utc": started, "finished_utc": finished,
        "elapsed_seconds": elapsed, "wsl_exit_code": child.returncode,
        "source_before": record.store(record.canonical(before)),
        "source_after": record.store(record.canonical(after)),
        "source_unchanged": before == after,
        "host": marker("__B6_WSL_HOST__"), "kernel": marker("__B6_WSL_KERNEL__"),
        "rustc": marker("__B6_RUSTC__"), "rustc_sha256": marker("__B6_RUSTC_SHA__"),
        "cargo": marker("__B6_CARGO__"), "cargo_sha256": marker("__B6_CARGO_SHA__"),
        "clang": marker("__B6_CLANG__"), "llvm_as": marker("__B6_LLVM_AS__"),
        "llc": marker("__B6_LLC__"), "llvm_tools_sha256": marker("__B6_LLVM_TOOLS_SHA__"),
        "runtime_line": marker("__B6_RUNTIME__"),
        "runtime_sha256": marker("__B6_RUNTIME_SHA__"),
        "removed_environment_names": marker("__B6_REMOVED_ENV__"),
        "linux_runner": record.identity(Path(__file__).with_name("linux_run.sh")),
        "runner": record.identity(Path(__file__).with_name("linux_run.sh")),
        "default_libtest_threads": "repository Cargo config supplies RUST_TEST_THREADS=4; no command override",
        "environment": {
            "CARGO_TARGET_DIR": WSL_TASK + "/target",
            "TEMP": WSL_TASK + "/temp", "TMP": WSL_TASK + "/temp",
            "TMPDIR": WSL_TASK + "/temp", "XIAO_HOME": WSL_TASK + "/home",
            "XIAO_STDLIB_ROOT": WSL_REPOSITORY + "/stdlib",
            "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0",
        },
        "recorder": record.identity(Path(__file__)),
        "log": record.store(raw),
    }
    record.write_new(receipt_path, receipt)
    print(json.dumps({key: receipt[key] for key in (
        "label", "wsl_exit_code", "pid", "elapsed_seconds", "source_unchanged",
        "rustc", "clang", "llvm_as", "llc", "runtime_line", "runtime_sha256", "log"
    )}, ensure_ascii=False))
    record.require(before == after, "Linux命令运行期间仓库源码变化")
    raise SystemExit(child.returncode)


if __name__ == "__main__":
    main()
