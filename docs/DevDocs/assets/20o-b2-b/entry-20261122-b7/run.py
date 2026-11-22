"""在固定 B7 产品的独立任务根采集功能回归与工程门禁。"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
PRODUCT = "e79fab3cdd3ea9d3a7eff31f529dec2498434114"
TREE = "80be41eefc3983001cd3968db0ccceee541f698d"
PRODUCT_ROOTS = (
    ".gitattributes", ".cargo", ".github/workflows", "LICENSE", "core", "stdlib", "tests",
    "tools", "cli", "resources", "package.json", "bun.lock", "tsconfig.json",
    "docs/UseDocs", "docs/module-registry.json",
)
LOCKS = ("core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock")
MANIFEST = ("--manifest-path", "core/rust/Cargo.toml")
LOCKED = (*MANIFEST, "--locked")
GATES = {
    "workspace": ("test", *LOCKED, "--workspace"),
    "clippy": ("clippy", *LOCKED, "--workspace", "--all-targets", "--", "-D", "warnings"),
    "fmt": ("fmt", *MANIFEST, "--all", "--", "--check"),
}
SUITES = {
    "b1-contract": ("20o_b1_imp_contract",),
    "b1-resources": ("20o_b1_imp_resources",),
    "numeric-vm": ("20o_b2_a_contract", "20o_b2_a_resources", "20o_b2_b6_resources", "20o_b2_b2_divide",
                   "20o_b2_b4_cmath_vm", "20o_b2_b4_cmath_library", "20o_b2_b4_cmath_boundary",
                   "20o_b2_b4_numeric_range_routing", "20o_b2_b5_versions"),
}
EXACT = {
    "b1-vm": ("20o_b1_probe", "b1_capabilities_have_vm_golden_outputs", False),
    "b1-native": ("20o_b1_probe", "b1_capabilities_match_native_golden_outputs", True),
    "a1-vm": ("20o_b2_a1_identity", "a1_numeric_values_and_errors_have_vm_golden_outputs", False),
    "a1-native": ("20o_b2_a1_identity", "a1_numeric_values_and_errors_match_native_golden_outputs", True),
    "a5-native": ("20o_b2_a_contract", "a5_language_values_match_native_golden_outputs", True),
    "cmath-native": ("20o_b2_b4_cmath_library", "cmath_library_matches_native_o0_o3", True),
    "divide-native": ("20o_b2_b2_divide", "lfloat_divide_matches_native_golden_outputs", True),
    "cmath-llvm": ("20o_b2_b4_cmath_llvm", "native_cmath_static_dynamic_o0_o3_and_shared_pow", True),
    "float-equality": ("20o_static_float_equality", "native_static_dynamic_comparisons_match_vm_o0_o3", True),
    "windows-22d": ("22d_llvm_concurrency", "native_concurrency_vm_native_differential_matches_from_same_ir", True),
}
HOSTS = {
    "win32": ("windows", "x86_64-pc-windows-msvc", "xiao_runtime.lib", ".exe"),
    "linux": ("linux", "x86_64-unknown-linux-gnu", "libxiao_runtime.a", ""),
    "darwin": ("macos", "aarch64-apple-darwin", "libxiao_runtime.a", ""),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def git(*args, repo=REPO):
    command = ("cmd", "/c", "git") if os.name == "nt" else ("git",)
    return subprocess.check_output([*command, *args], cwd=repo)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def utc():
    return datetime.now(timezone.utc).isoformat()


def write_json(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as output:
        json.dump(value, output, ensure_ascii=False, indent=2, sort_keys=True)
        output.write("\n")


def identity(path):
    with path.open("rb") as handle:
        return {"bytes": path.stat().st_size, "sha256": hashlib.file_digest(handle, "sha256").hexdigest()}


def tracked(manifest):
    return {path.decode("utf-8"): header.split()[-1].decode("ascii")
            for line in manifest.rstrip(b"\0").split(b"\0")
            for header, path in (line.split(b"\t", 1),)}


def source_identity(source, manifest):
    files = tracked(manifest)
    for name, object_id in files.items():
        raw = (source / name).read_bytes()
        observed = hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()
        require(observed == object_id, f"固定产品字节不符：{name}")
    return {"commit": PRODUCT, "tree": TREE, "files": len(files), "tree_list_sha256": sha(manifest),
            "locks": {name: identity(source / name) for name in LOCKS}}


def exported(task, archive, manifest_file):
    require(not task.exists(), f"B7 任务根须全新：{task}")
    task.mkdir(parents=True)
    source = task / "source"
    source.mkdir()
    if archive is None:
        require(git("cat-file", "-p", PRODUCT).splitlines()[0] == f"tree {TREE}".encode(), "固定产品树不符")
        archive = task / "source.tar"
        with archive.open("xb") as output:
            command = ("cmd", "/c", "git") if os.name == "nt" else ("git",)
            subprocess.run([*command, "-c", "core.autocrlf=false", "-c", "core.eol=lf", "archive",
                            "--format=tar", PRODUCT], stdout=output, check=True, cwd=REPO)
        manifest = git("ls-tree", "-rz", PRODUCT, "--", *PRODUCT_ROOTS)
    else:
        manifest = manifest_file.read_bytes()
    require("tests/spec/11x0-protocol/hello-request.json" in tracked(manifest), "产品清单缺少协议夹具")
    with tarfile.open(archive) as bundle:
        bundle.extractall(source, filter="data")
    (source / ".git").mkdir()
    (source / ".git/HEAD").write_text(PRODUCT + "\n", encoding="ascii")
    (task / "product.tree").write_bytes(manifest)
    return source, manifest, identity(archive)


def tool(path, flag, env):
    path = path.resolve(strict=True)
    probe = subprocess.run([path, flag], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    require(probe.stdout.strip(), f"工具版本不可读：{path}")
    return {"path": str(path), "version": probe.stdout.decode("utf-8", "replace").splitlines()[0],
            **identity(path)}


def setup(task, source, settings, llvm, archive_info):
    host, target, library, suffix = HOSTS[sys.platform]
    unset = {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
             "CARGO_BUILD_TARGET", "RUST_TEST_THREADS", "CARGO_BUILD_JOBS"}
    env = {name: value for name, value in os.environ.items()
           if name not in unset and not name.startswith(("XIAO_", "RUST_BIGDECIMAL_", "CARGO_PROFILE_"))}
    rustc = Path(subprocess.check_output(["rustup", "which", "--toolchain", "1.96.0", "rustc"],
                                       env=env).decode().strip())
    rust = rustc.parent
    version = subprocess.check_output([rustc, "-vV"], env=env).decode()
    require(version.startswith("rustc 1.96.0 ") and f"host: {target}" in version.splitlines(), "Rust 或目标不符")
    paths = {name: rust / (name + suffix) for name in ("cargo", "rustc", "rustdoc", "rustfmt", "clippy-driver")}
    paths["bun"] = Path(shutil.which("bun") or "")
    paths["linker"] = Path(shutil.which("link.exe" if host == "windows" else "ld") or "")
    for name in ("clang", "llvm-as", "llc", "llvm-strip"):
        paths[name] = llvm / (name + suffix)
    names = {"clang": "XIAO_CLANG", "llvm-as": "XIAO_LLVM_AS", "llc": "XIAO_LLC", "llvm-strip": "XIAO_STRIP"}
    overrides = {option: str(paths[name].resolve(strict=True)) for name, option in names.items()}
    runtime = task / "target/release" / library
    overrides.update({
        "CARGO_TARGET_DIR": str(task / "target"), "CARGO_TERM_COLOR": "never",
        "RUSTUP_TOOLCHAIN": "1.96.0", "RUSTC": str(rustc), "RUSTDOC": str(paths["rustdoc"]),
        "XIAO_TARGET_TRIPLE": target, "XIAO_RUNTIME_LIBRARY": str(runtime), "XIAO_HOME": str(task / "home"),
        "XIAO_STDLIB_ROOT": str(source / "stdlib"), "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0",
        "BUN_INSTALL_CACHE_DIR": str(task / "cache/bun"),
    })
    env.update(overrides)
    env["PATH"] = os.pathsep.join((str(rust), str(llvm), env["PATH"]))
    tools = {name: tool(path, "/?" if name == "linker" and host == "windows"
                       else "-v" if name == "linker" and host == "macos" else "--version", env)
             for name, path in paths.items()}
    runner = {name: env[name] for name in ("GITHUB_SHA", "GITHUB_REF", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT",
                                          "RUNNER_OS", "RUNNER_ARCH", "ImageVersion") if name in env}
    settings.update({"host": host, "target": target, "task": str(task), "overrides": overrides,
                     "tools": tools, "export": archive_info, "rustc_verbose": version, "runner": runner})
    for name in ("home", "cache", "tmp", "evidence"):
        (task / name).mkdir()
    return runtime, env


def capture(task, label, argv, env, runtime, manifest):
    source = task / "source"
    logs = task / "evidence"
    tmp = task / "tmp" / label
    tmp.mkdir()
    local = env | {name: str(tmp) for name in ("TEMP", "TMP", "TMPDIR")}
    if label == "float-equality":
        local["XIAO_FLOAT_EQUALITY_OUTPUT"] = str(tmp / "artifacts")
    before = source_identity(source, manifest)
    runtime_before = identity(runtime) if runtime.is_file() else None
    argv = list(map(str, argv))
    started = utc()
    log = logs / f"{label}.log"
    with log.open("xb") as output, subprocess.Popen(argv, cwd=source, env=local, stdin=subprocess.DEVNULL,
                                                   stdout=output, stderr=subprocess.STDOUT) as child:
        exit_code = child.wait()
    finished = utc()
    runtime_after = identity(runtime) if runtime.is_file() else None
    after = source_identity(source, manifest)
    text = log.read_text(encoding="utf-8", errors="replace")
    artifacts = {}
    for line in text.splitlines():
        match = re.match(r"^\s*Running .+? \((.+)\)$", line)
        if match:
            binary = source / match[1]
            if binary.is_file():
                artifacts[str(binary)] = identity(binary)
    for path in tmp.rglob("*"):
        if path.is_file() and (path.suffix.lower() in {".exe", ".ll", ".xiaoc", ".xar"}
                               or path.suffix == "" and os.access(path, os.X_OK)):
            artifacts[str(path)] = identity(path)
    write_json(logs / f"{label}.json", {
        "argv": argv, "cwd": str(source), "temp": str(tmp), "pid": child.pid, "exit": exit_code,
        "started_utc": started, "finished_utc": finished, "log": identity(log), "artifacts": artifacts,
        "source_before": before, "source_after": after, "runtime_before": runtime_before, "runtime_after": runtime_after,
    })
    require(before == after and (label == "runtime" or runtime_before == runtime_after),
            f"执行期间源码或 Runtime 发生变化：{label}")
    print(f"{label} pid={child.pid} exit={exit_code}", flush=True)
    return exit_code


def commands(cargo, suite, host):
    base = (cargo, "test", *LOCKED, "-p", "xiao-driver")
    if suite == "workspace":
        for name, args in GATES.items():
            yield name, (cargo, *args)
    elif suite in ("regression", "b1", "numeric"):
        for label, targets in SUITES.items():
            if suite != "regression" and (label.startswith("b1-") != (suite == "b1")):
                continue
            yield label, (*base, *(arg for target in targets for arg in ("--test", target)), "--", "--show-output")
        for label, (target, test, ignored) in EXACT.items():
            if suite != "regression" and (label.startswith("b1-") != (suite == "b1")):
                continue
            if label == "windows-22d" and (suite != "regression" or host != "windows"):
                continue
            yield label, (*base, "--test", target, "--", *(("--ignored",) if ignored else ()),
                          "--exact", test, "--nocapture")
        if suite != "b1":
            yield "f9-runtime", (cargo, "test", *LOCKED, "-p", "xiao-runtime", "--", "--show-output")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("task", type=Path)
    parser.add_argument("--llvm", type=Path, required=True)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--suite", choices=("workspace", "regression", "b1", "numeric"), action="append", required=True)
    args = parser.parse_args()
    require(bool(args.archive) == bool(args.manifest), "源码归档与 Git 字节清单须配套")
    task = args.task.resolve()
    source, manifest, archive_info = exported(task, args.archive, args.manifest)
    settings = {"source": source_identity(source, manifest)}
    runtime, env = setup(task, source, settings, args.llvm, archive_info)
    write_json(task / "evidence/environment.json", settings)
    cargo = settings["tools"]["cargo"]["path"]
    code = capture(task, "runtime", (cargo, "build", *LOCKED, "--release", "-p", "xiao-runtime"),
                   env, runtime, manifest)
    require(code == 0 and runtime.is_file(), "release Runtime 构建失败")
    results = {label: capture(task, label, argv, env, runtime, manifest)
               for suite in args.suite for label, argv in commands(cargo, suite, settings["host"])}
    write_json(task / "evidence/results.json", results)
    return int(any(results.values()))


if __name__ == "__main__":
    raise SystemExit(main())
