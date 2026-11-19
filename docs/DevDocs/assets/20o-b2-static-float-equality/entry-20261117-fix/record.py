"""本任务的一次性采集器：全新目标目录、内容寻址日志和前后源码身份。"""
import argparse
from datetime import datetime, timezone
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
TASK = REPOSITORY.parent / ".b5/float-equality-fix-20261117-01"
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def identity(path):
    path = Path(path)
    raw = path.read_bytes()
    return {"path": str(path), "bytes": len(raw), "sha256": sha(raw)}


def git(*arguments):
    result = subprocess.run(["cmd", "/c", "git", "-C", str(REPOSITORY), *arguments],
                            capture_output=True, check=True)
    return result.stdout


def source():
    names = git("ls-files", "-z", "--cached", "--others", "--exclude-standard",
                "--", *PRODUCT_ROOTS).decode("utf-8").split(chr(0))
    result = {}
    for name in sorted(set(names) - {""}):
        path = REPOSITORY / name
        if path.is_file():
            raw = path.read_bytes()
            result[name] = {"bytes": len(raw), "sha256": sha(raw),
                            "lf_sha256": sha(raw.replace(b"\r\n", b"\n"))}
    return result


def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == packed, "内容对象摘要冲突")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {
        "path": path.relative_to(ROOT).as_posix(), "bytes": len(packed), "sha256": digest,
        "encoding": "gzip" if len(raw) > 8192 else "identity",
        "raw_bytes": len(raw), "raw_sha256": sha(raw),
    }


def write_new(path, value):
    with path.open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2) + chr(10)).encode("utf-8"))


def environment():
    removed = [name for name in os.environ
               if name.upper().startswith(("XIAO_", "RUST_BIGDECIMAL_"))
               or name.upper() in ("CARGO_TARGET_DIR", "RUST_TEST_THREADS")]
    result = {name: value for name, value in os.environ.items() if name not in removed}
    overrides = {
        "RUSTUP_TOOLCHAIN": "1.96.0", "CARGO_TARGET_DIR": str(TASK / "target"),
        "CARGO_BUILD_JOBS": "4", "CARGO_TERM_COLOR": "never",
        "TEMP": str(TASK / "temp"), "TMP": str(TASK / "temp"),
        "XIAO_HOME": str(TASK / "home"), "XIAO_STDLIB_ROOT": str(REPOSITORY / "stdlib"),
        "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0",
        "XIAO_CLANG": "D:/msys64/ucrt64/bin/clang.exe",
        "XIAO_LLVM_AS": "D:/msys64/ucrt64/bin/llvm-as.exe",
        "XIAO_LLC": "D:/msys64/ucrt64/bin/llc.exe",
        "XIAO_RUNTIME_LIBRARY": str(TASK / "target/debug/xiao_runtime.lib"),
        "XIAO_FLOAT_EQUALITY_OUTPUT": str(TASK / "native"),
        "RUSTC": "C:/Users/xiaoc/.rustup/toolchains/1.96.0-x86_64-pc-windows-msvc/bin/rustc.exe",
        "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8",
    }
    for name in ("CARGO_TARGET_DIR", "TEMP", "TMP", "XIAO_HOME", "XIAO_RUNTIME_LIBRARY",
                 "XIAO_FLOAT_EQUALITY_OUTPUT"):
        require(Path(overrides[name]).resolve().is_relative_to(TASK.resolve()),
                "执行目录越过任务根：" + name)
    result.update(overrides)
    return result, overrides, removed


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("label")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.label == "prepare":
        require(not TASK.exists(), "任务根已经存在，禁止复用")
        for name in ("target", "temp", "home", "logs"):
            (TASK / name).mkdir(parents=True, exist_ok=False)
        _, overrides, removed = environment()
        write_new(ROOT / "environment.json", {
            "task_root": str(TASK), "created_utc": datetime.now(timezone.utc).isoformat(),
            "fresh_target": True, "entry_commit": git("rev-parse", "HEAD").decode().strip(),
            "environment_overrides": overrides, "removed_names": removed,
        })
        print(json.dumps({"task_root": str(TASK), "fresh_target": True}, ensure_ascii=False))
        return
    require(TASK.is_dir(), "须先登记全新任务根")
    require(args.label and all(char in "abcdefghijklmnopqrstuvwxyz0123456789-"
                               for char in args.label), "回执标签非法")
    receipt = ROOT / (args.label + ".json")
    require(not receipt.exists(), "禁止覆盖执行回执")
    argv = args.command[1:] if args.command and args.command[0] == "--" else args.command
    require(bool(argv), "缺少真实执行命令")
    env, overrides, removed = environment()
    before = source()
    runtime = Path(overrides["XIAO_RUNTIME_LIBRARY"])
    runtime_before = identity(runtime) if runtime.is_file() else None
    started = datetime.now(timezone.utc).isoformat()
    log_path = TASK / "logs" / (args.label + ".log")
    with log_path.open("xb") as output:
        child = subprocess.Popen(argv, cwd=REPOSITORY, env=env, stdin=subprocess.DEVNULL,
                                 stdout=output, stderr=subprocess.STDOUT)
        status = child.wait()
    finished = datetime.now(timezone.utc).isoformat()
    after = source()
    raw_log = log_path.read_bytes()
    write_new(receipt, {
        "label": args.label, "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished, "exit_code": status,
        "environment_overrides": overrides, "removed_names": removed,
        "source_before": store(canonical(before)), "source_after": store(canonical(after)),
        "source_unchanged": before == after,
        "runtime_before": runtime_before,
        "runtime_after": identity(runtime) if runtime.is_file() else None,
        "log": store(raw_log),
    })
    print(json.dumps({"label": args.label, "exit_code": status,
                      "source_unchanged": before == after,
                      "tail": raw_log.decode("utf-8", errors="replace")[-5500:]}, ensure_ascii=False))
    require(before == after, "执行中受保护源码变化")
    raise SystemExit(status)


if __name__ == "__main__":
    main()
