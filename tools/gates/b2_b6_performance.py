"""Run the fixed B6-2 performance catalog in an isolated release task."""

import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

from b2_b6_macos import (
    command_output, file_identity, git, live_source, require, snapshot, source_digest,
    tool_identity, utc_now, write_json,
)


BASELINE = Path(__file__).with_name("b2-b6-performance-baseline.json")
CATALOG = Path("tests/fixtures/20o-b2-b6/catalog.json")
BACKENDS = ("vm", "o0", "o3")
HOSTS = {
    "win32": ("windows", "x86_64-pc-windows-msvc", "xiao_runtime.lib", ".exe"),
    "linux": ("linux", "x86_64-unknown-linux-gnu", "libxiao_runtime.a", ""),
    "darwin": ("macos", "aarch64-apple-darwin", "libxiao_runtime.a", ""),
}


def run_command(argv, source, temp, env, logs, label, runtime, files):
    temp.mkdir()
    child_env = dict(env, TMPDIR=str(temp), TEMP=str(temp), TMP=str(temp))
    prior = file_identity(runtime) if runtime.is_file() else None
    started = utc_now()
    log = logs / f"{label}.log"
    with log.open("xb") as stream:
        child = subprocess.Popen(argv, cwd=source, env=child_env,
                                 stdout=stream, stderr=subprocess.STDOUT)
        code = child.wait()
    after = file_identity(runtime) if runtime.is_file() else None
    after_source = source_digest(live_source(source))
    write_json(log.with_suffix(".json"), {
        "argv": [str(item) for item in argv], "pid": child.pid, "cwd": str(source),
        "temp": str(temp), "started_utc": started, "finished_utc": utc_now(),
        "exit_code": code, "log": {"name": log.name, **file_identity(log)},
        "runtime_before": prior, "runtime_after": after, "source_after_sha256": after_source,
    })
    require(after_source == source_digest(files), f"Source changed during {label}")
    require(after is not None and (prior is None if label == "runtime-build" else prior == after),
            f"Runtime changed during {label}")
    print(f"B6-2 {label} pid={child.pid} exit={code}", flush=True)
    return code


def copy_receipts(measurement, receipts):
    if measurement.is_dir():
        # The generated source, LLVM, object and executable are bound by their
        # recorded digests. Upload the receipts and the unmodified process logs.
        shutil.copytree(measurement, receipts,
                        ignore=shutil.ignore_patterns("program.xiao", "program.xiaoc",
                                                      "numeric-o*", "clock-o*.o"))


def run(options):
    host, triple, library, suffix = HOSTS[sys.platform]
    checkout = git("rev-parse", "--verify", "HEAD").decode().strip()
    fixed = json.loads(BASELINE.read_bytes())
    if "GITHUB_SHA" in os.environ:
        require(checkout == os.environ["GITHUB_SHA"] and host == "macos",
                "Runner checkout or platform differs from the workflow")
    else:
        require(git("rev-parse", "--verify", fixed["source_commit"]).decode().strip()
                == fixed["source_commit"], "Missing fixed source commit")
        checkout = fixed["source_commit"]
    backends = options.backend or BACKENDS
    task = options.task.resolve()
    task.mkdir()
    source, target, logs = (task / name for name in ("source", "target", "evidence"))
    source.mkdir()
    logs.mkdir()
    files = snapshot(checkout, source)
    require(len(files) == fixed["files"] and source_digest(files) == fixed["sha256"]
            and files == live_source(source) and files.get(CATALOG.as_posix()) == fixed["catalog_sha256"],
            "Fixed source or performance catalog differs from the exported task")
    catalog = json.loads((source / CATALOG).read_bytes())
    loads = [row for row in catalog["loads"]
             if options.suite == "all" or options.suite == row["suite"]]
    require(loads and len({(row["case"], row["profile"]) for row in loads}) == len(loads),
            "Missing or duplicate measurement loads")
    write_json(logs / "source.json", files)
    env = {name: value for name, value in os.environ.items()
           if not name.startswith(("XIAO_", "RUST_BIGDECIMAL_", "CARGO_PROFILE_"))
           and name not in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                            "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET", "RUST_TEST_THREADS"}}
    rustc = Path(command_output(["rustup", "which", "--toolchain", "1.96.0", "rustc"], env))
    cargo = rustc.with_name("cargo" + suffix)
    runtime = target / "release" / library
    home = task / "home"
    home.mkdir()
    llvm_bin = options.llvm_bin.resolve(strict=True)
    env.update(RUSTUP_TOOLCHAIN="1.96.0", RUSTC=str(rustc), CARGO_TERM_COLOR="never",
               PATH=os.pathsep.join((str(rustc.parent), str(llvm_bin), env["PATH"])),
               CARGO_TARGET_DIR=str(target), XIAO_HOME=str(home),
               XIAO_STDLIB_ROOT=str(source / "stdlib"), XIAO_RUNTIME_LIBRARY=str(runtime),
               XIAO_TARGET_TRIPLE=triple, XIAO_B1_F10="0", XIAO_B2_A_PERF="0")
    tools = {"rustc": tool_identity(rustc, "-Vv", env), "cargo": tool_identity(cargo, "-Vv", env)}
    require(tools["rustc"]["version"].startswith("rustc 1.96.0")
            and f"host: {triple}" in tools["rustc"]["version"], "Rust version or host differs")
    for key, tool in (("XIAO_CLANG", "clang"), ("XIAO_LLVM_AS", "llvm-as"), ("XIAO_LLC", "llc")):
        tools[key] = tool_identity(llvm_bin / (tool + suffix), "--version", env)
        env[key] = tools[key]["path"]
    write_json(logs / "environment.json", {
        "platform": host, "system": platform.uname()._asdict(), "cpus": os.cpu_count(),
        "runner_image": os.environ.get("ImageVersion"), "runner_checkout": checkout,
        "github": {name: os.environ.get(f"GITHUB_{name}") for name in ("RUN_ID", "RUN_ATTEMPT", "JOB")},
        "source": fixed, "source_root": str(source), "task_root": str(task), "tools": tools,
        "target": triple, "cargo_target_dir": str(target), "home": str(home),
        "runtime": str(runtime), "stdlib": env["XIAO_STDLIB_ROOT"],
        "backends": backends, "suite": options.suite, "loads_per_backend": len(loads),
    })
    build = [cargo, "build", "--release", "--locked", "--manifest-path", "core/rust/Cargo.toml"]
    for label, extra in (("runtime-build", ["-p", "xiao-runtime"]),
                         ("measure-build", ["-p", "xiao-driver", "--example", "b2_b6_performance"])):
        require(run_command([*build, *extra], source, task / f"temp-{label}",
                            env, logs, label, runtime, files) == 0, f"{label} failed")
    worker = target / "release" / "examples" / ("b2_b6_performance" + suffix)
    worker_identity = file_identity(worker)
    results = {}
    for backend in backends:
        measurement = task / backend
        argv = [worker, "--backend", backend, "--suite", options.suite, "--output", measurement]
        code = run_command(argv, source, task / f"temp-{backend}", env, logs, backend, runtime, files)
        copy_receipts(measurement, logs / backend)
        receipt = measurement / "result.json"
        result = json.loads(receipt.read_bytes()) if receipt.is_file() else {}
        rows = result.get("records", [])
        complete = bool(result.get("complete") and [row["load"] for row in rows] == loads)
        passed = complete and all(row["status"] == "passed" for row in rows)
        require(not result or result["passed"] == passed, f"{backend} result disagrees with its rows")
        require((code == 0) == passed, f"{backend} exit code disagrees with its rows")
        require(file_identity(worker) == worker_identity, f"Release controller changed during {backend}")
        results[backend] = {"complete": complete, "passed": passed, "exit_code": code,
                            "counts": {status: sum(row["status"] == status for row in rows)
                                       for status in ("passed", "failed", "unverifiable")}}
    passed = all(result["passed"] for result in results.values())
    write_json(logs / "result.json", {
        "passed": passed, "complete": all(result["complete"] for result in results.values()),
        "platform": host, "suite": options.suite, "source": fixed, "worker": worker_identity,
        "runtime": file_identity(runtime), "backends": results,
    })
    return 0 if passed else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    baseline = sub.add_parser("baseline")
    baseline.add_argument("revision")
    measure = sub.add_parser("run")
    measure.add_argument("--task", type=Path, required=True)
    measure.add_argument("--llvm-bin", type=Path, required=True)
    measure.add_argument("--backend", choices=BACKENDS, action="append")
    measure.add_argument("--suite", choices=("all", "numeric", "cmath", "compat"), default="all")
    options = parser.parse_args()
    if options.command == "baseline":
        revision = git("rev-parse", "--verify", options.revision).decode().strip()
        require(git("cat-file", "-t", revision).strip() == b"commit", "Baseline must be a commit")
        files = snapshot(revision)
        print(json.dumps({"source_commit": revision, "files": len(files), "sha256": source_digest(files),
                          "catalog_sha256": files[CATALOG.as_posix()]}, indent=2))
        return 0
    require(not options.backend or len(set(options.backend)) == len(options.backend),
            "Select each backend at most once")
    return run(options)


if __name__ == "__main__":
    sys.exit(main())
