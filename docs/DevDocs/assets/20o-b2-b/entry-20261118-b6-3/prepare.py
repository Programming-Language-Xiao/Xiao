"""从固定 Git 对象建立 B6-3 的独立 Rust/LLVM 任务。"""

import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import sys


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
sys.dont_write_bytecode = True
sys.path.insert(0, str(REPO / "tools/gates"))

from b2_b6_macos import (
    command_output, file_identity, git, live_source, require, snapshot, source_digest,
    tool_identity, utc_now, write_json,
)
from b2_b6_performance import HOSTS, run_command


PRODUCT = "58176ff5299441f921f3b54fbdc300e2e46b9bb0"
SOURCE_SHA = "7ea95309a41156f506ba3aa4419950a9962b6b8915588a7689d0ac8c00e6db75"
BUILD_OPTIONS = ["--release", "--locked", "--manifest-path", "core/rust/Cargo.toml"]
INPUTS = ("matrix.json", "matrix.py", "prepare.py", "run.py", "abi_probe.c")
SHARED = ("tools/gates/b2_b6_macos.py", "tools/gates/b2_b6_performance.py")


def clean_env():
    return {name: value for name, value in os.environ.items()
            if not name.upper().startswith(("XIAO_", "RUST_BIGDECIMAL_", "CARGO_PROFILE_"))
            and name.upper() not in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                                     "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET", "RUST_TEST_THREADS"}}


def process_env(fixed):
    env = clean_env()
    env.update(fixed["environment"])
    rustc = Path(env["RUSTC"])
    llvm = Path(env["XIAO_CLANG"]).parent
    env["PATH"] = os.pathsep.join((str(rustc.parent), str(llvm), env["PATH"]))
    return env


def input_identities():
    return ({name: file_identity(ROOT / name) for name in INPUTS}
            | {name: file_identity(REPO / name) for name in SHARED})


def prepare(task, llvm_bin):
    host, triple, library, suffix = HOSTS[sys.platform]
    checkout = git("rev-parse", "--verify", "HEAD").decode().strip()
    if "GITHUB_SHA" in os.environ:
        require(host == "macos" and checkout == os.environ["GITHUB_SHA"], "Runner 检出身份不符")
        revision = checkout
    else:
        revision = PRODUCT
    task = task.resolve()
    task.mkdir()
    source, target, evidence = (task / name for name in ("source", "target", "evidence"))
    for folder in (source, evidence, task / "home", task / "bin"):
        folder.mkdir()
    files = snapshot(revision, source)
    require(source_digest(files) == SOURCE_SHA and live_source(source) == files, "B6-3 固定源码不符")
    write_json(evidence / "source.json", files)
    env = clean_env()
    rustc = Path(command_output(["rustup", "which", "--toolchain", "1.96.0", "rustc"], env))
    cargo = rustc.with_name("cargo" + suffix)
    runtime = target / "release" / library
    core = task / "bin" / ("xiao-core" + suffix)
    llvm_bin = llvm_bin.resolve(strict=True)
    overrides = {"RUSTUP_TOOLCHAIN": "1.96.0", "RUSTC": str(rustc), "CARGO_TERM_COLOR": "never",
                 "CARGO_TARGET_DIR": str(target), "XIAO_HOME": str(task / "home"),
                 "XIAO_STDLIB_ROOT": str(source / "stdlib"), "XIAO_RUNTIME_LIBRARY": str(runtime),
                 "XIAO_TARGET_TRIPLE": triple, "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0"}
    tools = {"rustc": tool_identity(rustc, "-Vv", env), "cargo": tool_identity(cargo, "-Vv", env)}
    require(tools["rustc"]["version"].startswith("rustc 1.96.0")
            and f"host: {triple}" in tools["rustc"]["version"], "Rust 版本或目标不符")
    for key, binary in (("XIAO_CLANG", "clang"), ("XIAO_LLVM_AS", "llvm-as"), ("XIAO_LLC", "llc")):
        tools[key] = tool_identity(llvm_bin / (binary + suffix), "--version", env)
        overrides[key] = tools[key]["path"]
    fixed = {
        "platform": host, "target": triple, "os": platform.uname()._asdict(),
        "product": PRODUCT, "checkout": checkout, "source_sha256": SOURCE_SHA,
        "source_root": str(source), "task_root": str(task), "environment": overrides,
        "runtime": str(runtime), "core": str(core), "tools": tools,
        "inputs": input_identities(),
        "github": {key: os.environ.get("GITHUB_" + key) for key in ("RUN_ID", "RUN_ATTEMPT", "JOB")},
    }
    write_json(evidence / "environment.json", fixed)
    env = process_env(fixed)
    for label, extra in (("runtime-build", ["-p", "xiao-runtime"]),
                         ("core-build", ["-p", "xiao-driver", "--bin", "xiao-core"])):
        code = run_command([cargo, "build", *BUILD_OPTIONS, *extra], source, task / f"temp-{label}",
                           env, evidence, label, runtime, files)
        require(code == 0, f"{label} 退出 {code}，见 {evidence / (label + '.log')}")
    # Cargo integration tests may rebuild target/release/xiao-core with dev features.
    # Keep the product binary used by boundary probes outside Cargo's target tree.
    cargo_core = target / "release" / core.name
    shutil.copy2(cargo_core, core)
    require(file_identity(cargo_core) == file_identity(core), "核心副本与本次 release 构建不符")
    write_json(evidence / "build.json", {"core": file_identity(core), "runtime": file_identity(runtime)})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", type=Path, required=True)
    parser.add_argument("--llvm-bin", type=Path, required=True)
    args = parser.parse_args()
    prepare(args.task, args.llvm_bin)


if __name__ == "__main__":
    main()
