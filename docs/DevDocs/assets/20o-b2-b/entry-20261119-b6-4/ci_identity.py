"""打印固定 Runner 上的源码、锁和原生 Runtime 身份。"""

import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys


LOCKS = ("core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock")
CONTEXT = (
    "GITHUB_SHA", "GITHUB_REF", "GITHUB_EVENT_NAME", "GITHUB_RUN_ID",
    "GITHUB_RUN_ATTEMPT", "GITHUB_JOB", "RUNNER_OS", "RUNNER_ARCH",
    "ImageOS", "ImageVersion", "XIAO_B1_F10", "XIAO_B2_A_PERF",
)
LLVM = ("XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLC", "XIAO_STRIP")


def git(*args):
    command = ["cmd", "/c", "git"] if os.name == "nt" else ["git"]
    return subprocess.check_output([*command, *args], text=True, encoding="utf-8").strip()


def file_identity(path):
    path = Path(path).resolve(strict=True)
    with path.open("rb") as stream:
        sha256 = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": sha256}


def version(command):
    return subprocess.check_output(command, text=True, encoding="utf-8").strip()


def snapshot(phase):
    target = Path(os.environ.get("CARGO_TARGET_DIR") or "core/rust/target").resolve()
    runtime = target / "release" / ("xiao_runtime.lib" if os.name == "nt" else "libxiao_runtime.a")
    head, devdocs = git("rev-parse", "HEAD", "HEAD:docs/DevDocs").splitlines()
    tree = git("cat-file", "-p", head).splitlines()[0].removeprefix("tree ")
    result = {
        "head": head,
        "tree": tree,
        "devdocs_tree": devdocs,
        "tracked_changes": git("status", "--porcelain=v1", "--untracked-files=no").splitlines(),
        "locks": {path: file_identity(path) for path in LOCKS},
        "cargo_target_dir": str(target),
        "runtime": file_identity(runtime) if runtime.is_file() else None,
    }
    if phase == "before":
        result["host"] = platform.platform()
        result["rustc"] = version(["rustc", "-vV"])
        result["cargo_delegate"] = file_identity(version(["rustup", "which", "cargo"]))
        result["controlled_cargo"] = shutil.which("cargo")
        if os.environ.get("GITHUB_JOB") == "linux-workspace-gates":
            result["bun"] = version(["bun", "--version"])
            result["rustfmt"] = version(["rustfmt", "--version"])
            result["clippy"] = version(["clippy-driver", "--version"])
    return result


def native():
    return {
        "target": os.environ["XIAO_TARGET_TRIPLE"],
        "runtime": file_identity(os.environ["XIAO_RUNTIME_LIBRARY"]),
        "diagnostics": file_identity(os.environ["XIAO_DIAGNOSTICS_PATH"]),
        "llvm": {
            name: file_identity(os.environ[name])
            | {"version": version([os.environ[name], "--version"])}
            for name in LLVM
        },
    }


def main():
    phase, = sys.argv[1:]
    if phase not in {"before", "after", "native"}:
        raise SystemExit("应指定 before、after 或 native")
    result = native() if phase == "native" else snapshot(phase)
    result["context"] = {key: os.environ[key] for key in CONTEXT if key in os.environ}
    print(f"B6-4-IDENTITY {phase} {json.dumps(result, sort_keys=True)}", flush=True)


if __name__ == "__main__":
    main()
