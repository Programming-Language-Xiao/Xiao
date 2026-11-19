"""用隔离任务根采集B6-1命令、完整产品身份、实际工具/Runtime和内容寻址日志。"""
from datetime import datetime, timezone
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
TASK = REPOSITORY.parent / ".b6/b6-1-20261117-01"
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]
LLVM = Path("D:/msys64/ucrt64/bin")


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def identity(path):
    path = Path(path)
    raw = path.read_bytes()
    return {"path": str(path.resolve()), "bytes": len(raw), "sha256": sha(raw)}


def git(*args):
    return subprocess.run(["cmd", "/c", "git", "-C", str(REPOSITORY), *args],
                          capture_output=True, check=True).stdout


def source():
    raw = git("ls-files", "-z", "--cached", "--others", "--exclude-standard",
              "--", *PRODUCT_ROOTS)
    names = sorted(set(raw.decode("utf-8").split("\0")) - {""})
    files = {}
    for name in names:
        path = REPOSITORY / name
        require(path.is_file(), "受保护源码丢失：" + name)
        content = path.read_bytes()
        files[name] = {"bytes": len(content), "sha256": sha(content),
                       "lf_sha256": sha(content.replace(b"\r\n", b"\n"))}
    return files


def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == packed, "内容对象摘要碰撞或对象损坏")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": len(packed),
            "sha256": digest, "encoding": "gzip" if len(raw) > 8192 else "identity",
            "raw_bytes": len(raw), "raw_sha256": sha(raw)}


def environment():
    prefixes = ("XIAO_", "RUST_BIGDECIMAL_")
    removed = sorted(name for name in os.environ
                     if name.upper().startswith(prefixes)
                     or name.upper() in ("RUST_TEST_THREADS", "CARGO_TARGET_DIR"))
    env = {key: value for key, value in os.environ.items() if key not in removed}
    rustc = subprocess.run(["rustup", "which", "--toolchain", "1.96.0", "rustc"],
                           capture_output=True, check=True, encoding="utf-8").stdout.strip()
    overrides = {
        "RUSTUP_TOOLCHAIN": "1.96.0", "RUSTC": rustc,
        "CARGO_TARGET_DIR": str(TASK / "target"),
        "CARGO_BUILD_JOBS": "4", "CARGO_TERM_COLOR": "never",
        "TEMP": str(TASK / "temp"), "TMP": str(TASK / "temp"),
        "XIAO_HOME": str(TASK / "home"),
        "XIAO_STDLIB_ROOT": str(REPOSITORY / "stdlib"),
        "XIAO_RUNTIME_LIBRARY": str(TASK / "target/release/xiao_runtime.lib"),
        "XIAO_TARGET_TRIPLE": "x86_64-pc-windows-msvc",
        "XIAO_CLANG": str(LLVM / "clang.exe"),
        "XIAO_LLVM_AS": str(LLVM / "llvm-as.exe"),
        "XIAO_LLC": str(LLVM / "llc.exe"),
        "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0",
        "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8",
    }
    for name in ("CARGO_TARGET_DIR", "TEMP", "TMP", "XIAO_HOME",
                 "XIAO_RUNTIME_LIBRARY"):
        require(Path(overrides[name]).resolve().is_relative_to(TASK.resolve()),
                "隔离路径越出B6任务根：" + name)
    for name in ("RUSTC", "XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLC"):
        require(Path(overrides[name]).is_file(), "固定工具不存在：" + name)
    env.update(overrides)
    return env, overrides, removed


def write_new(path, value):
    with path.open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))


def main():
    require(len(sys.argv) >= 4 and sys.argv[2] == "--",
            "用法：record.py label -- command arguments")
    label, argv = sys.argv[1], sys.argv[3:]
    require(label and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-_" for c in label),
            "命令标签非法")
    receipt_path = ROOT / (label + ".json")
    log_path = TASK / "logs" / (label + ".log")
    require(not receipt_path.exists() and not log_path.exists(), "回执或日志禁止覆盖")
    for path in (TASK / "target", TASK / "temp", TASK / "home", TASK / "logs"):
        path.mkdir(parents=True, exist_ok=True)
    before = source()
    env, overrides, removed = environment()
    tool_records = {}
    for name in ("rustc", "cargo"):
        located = overrides["RUSTC"] if name == "rustc" else subprocess.run(
            ["rustup", "which", "--toolchain", "1.96.0", "cargo"],
            capture_output=True, check=True, encoding="utf-8").stdout.strip()
        version = subprocess.run([located, "-Vv"], capture_output=True,
                                 check=True, encoding="utf-8")
        tool_records[name] = {"binary": identity(located), "stdout": version.stdout}
    for key in ("XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLC"):
        version = subprocess.run([overrides[key], "--version"], capture_output=True,
                                 check=True, encoding="utf-8")
        tool_records[key] = {"binary": identity(overrides[key]), "stdout": version.stdout}
    runtime = Path(overrides["XIAO_RUNTIME_LIBRARY"])
    runtime_before = identity(runtime) if runtime.is_file() else None
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    with log_path.open("xb") as output:
        child = subprocess.Popen(argv, cwd=REPOSITORY, env=env,
                                 stdin=subprocess.DEVNULL, stdout=output,
                                 stderr=subprocess.STDOUT)
        code = child.wait()
    elapsed = time.monotonic() - clock
    finished = datetime.now(timezone.utc).isoformat()
    after = source()
    runtime_after = identity(runtime) if runtime.is_file() else None
    raw_log = log_path.read_bytes()
    receipt = {
        "label": label, "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished,
        "elapsed_seconds": elapsed, "exit_code": code,
        "environment_overrides": overrides, "removed_environment_names": removed,
        "default_libtest_threads": "repository Cargo config RUST_TEST_THREADS=4",
        "cargo_target_cache": "fresh B6-1 task-root target; reused only within this batch",
        "source_before": store(canonical(before)),
        "source_after": store(canonical(after)),
        "source_unchanged": before == after,
        "tools": tool_records, "runtime_before": runtime_before,
        "runtime_after": runtime_after,
        "recorder": identity(Path(__file__)),
        "log": store(raw_log),
    }
    write_new(receipt_path, receipt)
    print(json.dumps({"label": label, "exit_code": code, "pid": child.pid,
                      "elapsed_seconds": elapsed, "source_unchanged": before == after,
                      "runtime_before": runtime_before, "runtime_after": runtime_after,
                      "log": receipt["log"],
                      "tail": raw_log.decode("utf-8", errors="replace")[-1200:]},
                     ensure_ascii=False))
    require(before == after, "B6-1命令运行期间源码发生变化")
    raise SystemExit(code)


if __name__ == "__main__":
    main()
