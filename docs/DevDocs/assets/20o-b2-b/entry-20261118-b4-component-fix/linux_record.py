"""WSL FIX.2受控构建、原生测试与内容寻址回执。"""
from datetime import datetime, timezone
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
TASK = Path("/home/xiaoczx/.b4/b4-3-fix2-20261118-01")
PRODUCT_ROOTS = [
    ".cargo", "core/rust/crates/xiao-codegen-llvm",
    "core/rust/crates/xiao-driver", "core/rust/crates/xiao-runtime-abi",
    "stdlib", "tests/fixtures/docs/numeric",
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def identity(path):
    path = Path(path)
    raw = path.read_bytes()
    return {"path": str(path.resolve()), "bytes": len(raw), "sha256": sha(raw)}


def source_snapshot():
    result = {}
    for root_name in PRODUCT_ROOTS:
        base = REPOSITORY / root_name
        paths = [base] if base.is_file() else sorted(base.rglob("*"))
        for path in paths:
            if not path.is_file():
                continue
            data = path.read_bytes()
            result[path.relative_to(REPOSITORY).as_posix()] = {
                "bytes": len(data), "sha256": sha(data),
                "lf_sha256": sha(data.replace(b"\r\n", b"\n")),
            }
    return result


def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == packed, "内容对象损坏或摘要碰撞")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {
        "path": path.relative_to(ROOT).as_posix(), "bytes": len(packed),
        "sha256": digest, "encoding": "gzip" if packed != raw else "identity",
        "raw_bytes": len(raw), "raw_sha256": sha(raw),
    }


def environment(label):
    env = os.environ.copy()
    removed = []
    for name in list(env):
        if (name.startswith("XIAO_") or name.startswith("RUST_BIGDECIMAL_")
                or name in {"RUST_TEST_THREADS", "CARGO_TARGET_DIR", "RUSTC", "CC", "CXX"}):
            removed.append(name)
            env.pop(name)
    env.update({
        "HOME": str(TASK / "home"),
        "CARGO_HOME": "/home/xiaoczx/.cargo",
        "RUSTUP_HOME": "/home/xiaoczx/.rustup",
        "PATH": "/home/xiaoczx/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        "RUSTUP_TOOLCHAIN": "1.96.0",
        "CARGO_TARGET_DIR": str(TASK / "target"),
        "CARGO_BUILD_JOBS": "4", "CARGO_TERM_COLOR": "never",
        "TEMP": str(TASK / "temp"), "TMP": str(TASK / "temp"),
        "TMPDIR": str(TASK / "temp"), "XIAO_HOME": str(TASK / "home"),
        "XIAO_STDLIB_ROOT": str(REPOSITORY / "stdlib"),
        "XIAO_RUNTIME_LIBRARY": str(TASK / "target/release/libxiao_runtime.a"),
        "XIAO_TARGET_TRIPLE": "x86_64-unknown-linux-gnu",
        "XIAO_CLANG": "/usr/bin/clang", "XIAO_LLVM_AS": "/usr/bin/llvm-as",
        "XIAO_LLC": "/usr/bin/llc", "XIAO_B1_F10": "0",
        "XIAO_B2_A_PERF": "0", "PYTHONUTF8": "1",
        "PYTHONIOENCODING": "utf-8",
    })
    TASK.mkdir(parents=True, exist_ok=True)
    for name in ("temp", "home", "logs"):
        (TASK / name).mkdir(parents=True, exist_ok=True)
    command_temp = TASK / "temp" / label
    command_temp.mkdir()
    env.update({"TEMP": str(command_temp), "TMP": str(command_temp),
                "TMPDIR": str(command_temp)})
    return env, removed


def command_for(action):
    cargo = ["cargo", "+1.96.0"]
    if action == "runtime-build":
        return cargo + ["build", "--locked", "--manifest-path", "core/rust/Cargo.toml",
                        "-p", "xiao-runtime", "--release"]
    if action == "codegen-test":
        return cargo + ["test", "--locked", "--manifest-path", "core/rust/Cargo.toml",
                        "-p", "xiao-codegen-llvm", "--release"]
    if action == "codegen-clippy":
        return cargo + ["clippy", "--locked", "--manifest-path", "core/rust/Cargo.toml",
                        "-p", "xiao-codegen-llvm", "--all-targets", "--", "-D", "warnings"]
    if action == "driver-regressions":
        return cargo + [
            "test", "--locked", "--manifest-path", "core/rust/Cargo.toml",
            "-p", "xiao-driver", "--release",
            "--test", "20o_b2_b4_cmath_library",
            "--test", "20o_b2_b4_cmath_vm",
            "--test", "20o_b2_b4_cmath_llvm",
            "--test", "n0_b_dynamic_native",
        ]
    if action == "diagnostic":
        return cargo + [
            "test", "--locked", "--manifest-path", "core/rust/Cargo.toml",
            "-p", "xiao-driver", "--release", "--test", "20o_b2_b6_resources",
            "b6_diagnoses_linux_native_cmath_runtime_component_observation",
            "--", "--exact", "--nocapture",
        ]
    if action == "test-build":
        return cargo + [
            "test", "--locked", "--manifest-path", "core/rust/Cargo.toml",
            "-p", "xiao-driver", "--release",
            "--test", "20o_b2_b4_cmath_library", "--no-run",
        ]
    if action == "test-run":
        matches = sorted((TASK / "target/release/deps").glob(
            "20o_b2_b4_cmath_library-*"
        ))
        matches = [path for path in matches if path.is_file() and "." not in path.name]
        require(len(matches) == 1, "FIX.2 Linux原生测试二进制缺失或不唯一")
        return [str(matches[0]), "--ignored", "--exact",
                "cmath_library_matches_native_o0_o3", "--nocapture"]
    raise ValueError("FIX.2 Linux命令阶段未知：" + action)


def tool_identities(env):
    tools = {}
    for name in ("rustc", "cargo", "clang", "llvm-as", "llc"):
        path = shutil.which(name, path=env["PATH"])
        require(path is not None, "Linux工具不存在：" + name)
        tools[name] = identity(path)
    return tools


def artifacts(temp_root):
    output = []
    for path in sorted(Path(temp_root).rglob("*")):
        if path.is_file() and path.suffix.lower() in {".ll", ".o", ".xiao", ".xiaoc", ".xar"}:
            output.append(identity(path))
    return output


def main():
    require(len(sys.argv) == 2, "用法：linux_record.py <stage>")
    label = sys.argv[1]
    require(label and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-_" for c in label),
            "FIX.2标签非法")
    receipt_path = ROOT / ("linux-" + label + ".json")
    require(not receipt_path.exists(), "FIX.2回执禁止覆盖：" + label)
    env, removed = environment(label)
    action = label
    for prefix in ("fix0-linux-", "fix1-linux-", "fix2-linux-", "fix3-linux-", "fix4-linux-"):
        action = action.removeprefix(prefix)
    if "-r" in action and action.rsplit("-r", 1)[1].isdigit():
        action = action.rsplit("-r", 1)[0]
    argv = command_for(action)
    before, runtime_before = source_snapshot(), None
    runtime_path = Path(env["XIAO_RUNTIME_LIBRARY"])
    if runtime_path.is_file():
        runtime_before = identity(runtime_path)
    log_path = TASK / "logs" / (label + ".log")
    require(not log_path.exists(), "FIX.2日志禁止覆盖：" + label)
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    with log_path.open("xb") as output:
        child = subprocess.Popen(argv, cwd=REPOSITORY, env=env,
                                 stdin=subprocess.DEVNULL, stdout=output,
                                 stderr=subprocess.STDOUT)
        code = child.wait()
    elapsed = time.monotonic() - clock
    finished = datetime.now(timezone.utc).isoformat()
    after, runtime_after = source_snapshot(), None
    if runtime_path.is_file():
        runtime_after = identity(runtime_path)
    raw_log = log_path.read_bytes()
    receipt = {
        "label": label, "platform": "linux", "pid": child.pid,
        "started_utc": started, "finished_utc": finished,
        "elapsed_seconds": elapsed, "argv": argv,
        "cwd": str(REPOSITORY), "exit_code": code,
        "host": subprocess.run(["hostname"], capture_output=True, text=True,
                               check=True, env=env).stdout.strip(),
        "kernel": subprocess.run(["uname", "-srmo"], capture_output=True, text=True,
                                 check=True, env=env).stdout.strip(),
        "environment_overrides": {
            key: env[key] for key in env if key.startswith("XIAO_")
        } | {key: env[key] for key in ("RUSTUP_TOOLCHAIN", "CARGO_TARGET_DIR",
             "CARGO_BUILD_JOBS", "HOME", "TEMP", "TMP", "TMPDIR")},
        "removed_environment_names": sorted(removed),
        "tools": tool_identities(env),
        "source_before": before, "source_after": after,
        "source_unchanged": before == after,
        "runtime_before": runtime_before, "runtime_after": runtime_after,
        "artifacts_after": artifacts(env["TMPDIR"]), "log": store(raw_log),
    }
    with receipt_path.open("xb") as output:
        output.write((json.dumps(receipt, ensure_ascii=False, indent=2) + "\n").encode())
    print(json.dumps({"label": label, "exit_code": code, "pid": child.pid,
                      "elapsed_seconds": elapsed, "runtime": runtime_after},
                     ensure_ascii=False))
    if code != 0:
        raise SystemExit(code)


if __name__ == "__main__":
    main()
