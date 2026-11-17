"""运行本批命令并绑定前后产品树、实际子进程及内容寻址日志。"""
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
TASK = REPOSITORY.parent / ".b5/url-case-opt-20261118-01"
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]


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
    return {"path": str(path), "bytes": len(raw), "sha256": sha(raw)}


def git(*args):
    result = subprocess.run(["cmd", "/c", "git", "-C", str(REPOSITORY), *args],
                            capture_output=True, check=True)
    return result.stdout


def source():
    raw = git("ls-files", "-z", "--cached", "--others", "--exclude-standard",
              "--", *PRODUCT_ROOTS)
    paths = sorted(set(raw.decode("utf-8").split(chr(0))) - {""})
    result = {}
    for name in paths:
        path = REPOSITORY / name
        require(path.is_file(), "产品源码缺失：" + name)
        content = path.read_bytes()
        result[name] = {"bytes": len(content), "sha256": sha(content),
                        "lf_sha256": sha(content.replace(b"\r\n", b"\n"))}
    return result


def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == packed, "内容寻址对象已损坏")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": len(packed),
            "sha256": digest, "encoding": "gzip" if len(raw) > 8192 else "identity",
            "raw_bytes": len(raw), "raw_sha256": sha(raw)}


def write_new(path, value):
    with path.open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2) + chr(10)).encode("utf-8"))


def environment():
    removed = [name for name in os.environ
               if name.upper().startswith(("XIAO_", "RUST_BIGDECIMAL_"))
               or name.upper() in ("RUST_TEST_THREADS", "CARGO_TARGET_DIR")]
    env = {name: value for name, value in os.environ.items() if name not in removed}
    overrides = {
        "RUSTUP_TOOLCHAIN": "1.96.0", "CARGO_TARGET_DIR": str(TASK / "target"),
        "CARGO_BUILD_JOBS": "4", "CARGO_TERM_COLOR": "never",
        "TEMP": str(TASK / "temp"), "TMP": str(TASK / "temp"),
        "XIAO_HOME": str(TASK / "home"), "XIAO_STDLIB_ROOT": str(REPOSITORY / "stdlib"),
        "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0",
        "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8",
    }
    for name in ("CARGO_TARGET_DIR", "TEMP", "TMP", "XIAO_HOME"):
        require(Path(overrides[name]).resolve().is_relative_to(TASK.resolve()),
                "任务目录越界：" + name)
    env.update(overrides)
    return env, overrides, removed


def build_identities():
    target = TASK / "target/debug"
    names = ("deps/xiao_runtime-*.lib", "deps/libxiao_runtime-*.rlib",
             "deps/20o_b1_imp_resources-*.exe")
    return {str(path.relative_to(target)): identity(path)
            for pattern in names for path in sorted(target.glob(pattern))}


def main():
    require(len(sys.argv) >= 4 and sys.argv[2] == "--", "用法：record.py label -- command args")
    label, argv = sys.argv[1], sys.argv[3:]
    require(label and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-" for c in label),
            "命令标签非法")
    receipt = ROOT / (label + ".json")
    log_path = TASK / (label + ".log")
    require(not receipt.exists() and not log_path.exists(), "执行回执或日志禁止覆盖")
    require(TASK.is_dir(), "本轮隔离任务根不存在")
    before = source()
    env, overrides, removed = environment()
    tools = {}
    for name in ("rustc", "cargo"):
        located = subprocess.run(["rustup", "which", "--toolchain", "1.96.0", name],
                                 check=True, capture_output=True, encoding="utf-8").stdout.strip()
        version = subprocess.run([located, "-Vv"], capture_output=True,
                                 encoding="utf-8", check=True)
        tools[name] = {"binary": identity(located), "stdout": version.stdout,
                       "stderr": version.stderr}
    runtime_before = build_identities()
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    with log_path.open("xb") as output:
        child = subprocess.Popen(argv, cwd=REPOSITORY, env=env, stdin=subprocess.DEVNULL,
                                 stdout=output, stderr=subprocess.STDOUT)
        code = child.wait()
    elapsed = time.monotonic() - clock
    finished = datetime.now(timezone.utc).isoformat()
    after = source()
    raw_log = log_path.read_bytes()
    result = {
        "label": label, "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished, "elapsed_seconds": elapsed,
        "exit_code": code, "entry_commit": git("rev-parse", "HEAD").decode().strip(),
        "environment_overrides": overrides, "removed_environment_names": removed,
        "default_libtest_threads": "Cargo config RUST_TEST_THREADS=4; no command override",
        "cargo_target_cache": "isolated target created earlier this task; warm for this replay",
        "source_before": store(canonical(before)), "source_after": store(canonical(after)),
        "source_unchanged": before == after, "tools": tools,
        "runtime_and_test_binaries_before": runtime_before,
        "runtime_and_test_binaries_after": build_identities(),
        "recorder": identity(Path(__file__).resolve()), "log": store(raw_log),
    }
    write_new(receipt, result)
    print(json.dumps({"label": label, "exit_code": code, "pid": child.pid,
                      "elapsed_seconds": elapsed, "source_unchanged": before == after,
                      "log": result["log"], "tail": raw_log.decode("utf-8", errors="replace")[-800:]},
                     ensure_ascii=False))
    require(before == after, "命令运行中产品源码变化")
    raise SystemExit(code)


if __name__ == "__main__":
    main()
