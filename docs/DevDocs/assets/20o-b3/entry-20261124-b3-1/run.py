"""在独立任务根执行 B3-1 的 time 功能与链接验收。"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[5]
MANIFEST = "core/rust/Cargo.toml"
RUST = "1.96.0"
TARGETS = {
    "x86_64-pc-windows-msvc": "xiao_runtime.lib",
    "x86_64-unknown-linux-gnu": "libxiao_runtime.a",
    "aarch64-apple-darwin": "libxiao_runtime.a",
}
EXACT = "test result: ok. 1 passed; 0 failed; 0 ignored;"
TARGET = "20o_b3_time_tutorial"


def identity(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def git(*args):
    launcher = ["cmd", "/c", "git"] if os.name == "nt" else ["git"]
    return subprocess.check_output([*launcher, *args], cwd=ROOT)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


class Task:
    def __init__(self, output, llvm_bin):
        self.output = output
        self.logs = output / "logs"
        self.env = os.environ.copy()
        for key in tuple(self.env):
            if key.startswith(("RUST_BIGDECIMAL_", "XIAO_TEST_TIME_")):
                del self.env[key]
        for path in ("logs", "temp", "home", "target"):
            (output / path).mkdir(parents=True)
        self.env.update(CARGO_TARGET_DIR=str(output / "target"), XIAO_HOME=str(output / "home"),
                        TEMP=str(output / "temp"), TMP=str(output / "temp"), TMPDIR=str(output / "temp"),
                        XIAO_STDLIB_ROOT=str(ROOT / "stdlib"), CARGO_TERM_COLOR="never")
        self.llvm_bin = llvm_bin
        self.report = {"passed": False, "source_commit": git("rev-parse", "HEAD").decode().strip(),
                       "source_tree": git("show", "-s", "--format=%T", "HEAD").decode().strip(),
                       "task_root": str(output), "commands": {}, "tools": {},
                       "ci": {key: os.environ.get(key) for key in
                              ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "GITHUB_JOB", "RUNNER_OS")}}

    def run(self, name, argv, timeout=600):
        process = subprocess.run(argv, cwd=ROOT, env=self.env, capture_output=True, timeout=timeout)
        row = {"argv": argv, "exit": process.returncode}
        for stream in ("stdout", "stderr"):
            raw = getattr(process, stream)
            log = identity(raw)
            path = self.logs / (log["sha256"] + ".log")
            if not path.exists():
                path.write_bytes(raw)
            row[stream] = log
        self.report["commands"][name] = row
        for raw in (process.stdout, process.stderr):
            print(raw.decode("utf-8", errors="replace"), end="", flush=True)
        require(process.returncode == 0, f"{name} 退出 {process.returncode}")
        return process.stdout.decode("utf-8").replace("\r\n", "\n")

    def tool(self, name, flags):
        filename = name + (".exe" if os.name == "nt" else "")
        path = (self.llvm_bin / filename).resolve(strict=True) if self.llvm_bin else Path(shutil.which(name)).resolve(strict=True)
        version = self.run(name, [str(path), *flags], timeout=25)
        self.report["tools"][name] = {"path": str(path), **identity(path.read_bytes()), "version": version.splitlines()[0]}
        self.env["XIAO_" + name.upper().replace("-", "_")] = str(path)

    def verify(self):
        before = git("status", "--porcelain=v1", "-z")
        require(not before, "任务需从干净的已提交源码取证")
        version = self.run("rustc", ["rustc", f"+{RUST}", "-vV"], timeout=25)
        require(f"release: {RUST}" in version, "Rust 版本与冻结表不符")
        target = re.search(r"(?m)^host: (.+)$", version)[1]
        require(target in TARGETS, f"目标不在 time 三平台范围内：{target}")
        self.report["target"] = target
        self.env["XIAO_TARGET_TRIPLE"] = target
        for tool in ("clang", "llvm-as", "llc"):
            self.tool(tool, ["--version"])
        self.report["lockfiles"] = {path: identity(git("show", "HEAD:" + path)) for path in
                                    ("core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock")}
        cargo = ["cargo", f"+{RUST}"]
        locked = ["--locked", "--manifest-path", MANIFEST, "--target", target]
        self.run("build-release-runtime", [*cargo, "build", *locked, "--release", "-p", "xiao-runtime",
                                            "--features", "test-failpoints"])
        runtime = self.output / "target" / target / "release" / TARGETS[target]
        self.env["XIAO_RUNTIME_LIBRARY"] = str(runtime)
        self.report["runtime"] = {"path": str(runtime), **identity(runtime.read_bytes()), "test_failpoints": True}
        self.run("runtime-boundaries", [*cargo, "test", *locked, "-p", "xiao-runtime", "--lib", "time::", "--", "--nocapture"])
        common = [*cargo, "test", *locked, "-p", "xiao-driver", "--test", TARGET, "--features", "time-native-test"]
        vm = self.run("vm-exact", [*common, "time_tutorial_matches_vm_golden", "--", "--exact", "--nocapture"])
        require(vm.count(EXACT) == 1 and all(f"backend=vm alias={alias} exact=true" in vm for alias in ("std", "stdlib")),
                "VM 精确入口未覆盖双路径")
        self.run("vm-boundaries", [*common, "--", "--skip", "time_tutorial_matches_vm_golden"])
        native = self.run("native-exact", [*common, "time_tutorial_matches_native_golden", "--", "--exact",
                                          "--ignored", "--nocapture"])
        require(native.count(EXACT) == 1, "原生精确入口计数不符")
        rows = re.findall(r"(?m)^B3-1-TUTORIAL backend=native alias=(\w+) opt=(\d+) verification=complete exact=true artifact=(.+)$", native)
        require([(name, opt) for name, opt, _ in rows] == [(name, opt) for name in ("std", "stdlib") for opt in ("0", "3")],
                "原生精确入口优化等级或别名缺项")
        bindings = {
            path: (sha, map_path, map_sha)
            for path, sha, map_path, map_sha in re.findall(
                r"(?m)^B3-1-COFF artifact=(.+?) sha256=([a-f0-9]{64}) map=(.+?) map_sha256=([a-f0-9]{64})$", native)
        }
        self.report["artifacts"] = []
        for alias, level, path in rows:
            path = Path(path).resolve()
            require(path.is_relative_to(self.output / "temp"), "原生产物越出任务根")
            node = {"alias": alias, "optimization": int(level), "path": path.relative_to(self.output).as_posix(),
                    **identity(path.read_bytes()), "verification": "complete"}
            if target == "x86_64-pc-windows-msvc":
                sha, map_path, map_sha = bindings[str(path)]
                sidecar = Path(map_path).resolve()
                require(sha == node["sha256"] and sidecar.parent == path.parent, "COFF 产物或清单路径不符")
                node["linker_map"] = {"path": sidecar.relative_to(self.output).as_posix(), **identity(sidecar.read_bytes())}
                require(node["linker_map"]["sha256"] == map_sha, "COFF 链接清单与产品验证回执不符")
            self.report["artifacts"].append(node)
        require(git("status", "--porcelain=v1", "-z") == before, "验收运行改写了源码")
        self.report["passed"] = True


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--llvm-bin", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    require(not output.is_relative_to(ROOT) and not output.exists(), "任务根须位于仓库之外且尚未创建")
    task = Task(output, args.llvm_bin)
    try:
        task.verify()
    finally:
        (output / "result.json").write_bytes((json.dumps(task.report, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode())
