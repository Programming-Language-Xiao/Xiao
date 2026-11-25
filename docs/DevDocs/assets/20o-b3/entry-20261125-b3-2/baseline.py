"""在独立任务根验证 B3-2 固定输入及 Windows release Runtime。"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from time import monotonic
from zipfile import ZIP_DEFLATED, ZipFile

ROOT = Path(__file__).resolve().parents[5]
PRODUCT = "8d1c09f084be172e8b3ce078f131592ac6db94b3"
FREEZE = "7f6cc6a26cf0b1350da7a6eea09203761e1de366"
B3 = "docs/DevDocs/assets/20o-b3"
B3_1 = f"{B3}/entry-20261124-b3-1"
B3_1_CLOSEOUT = "84660c11ef1377a6ff5744547c8291f01a8879c1"
TARGET = "x86_64-pc-windows-msvc"
RUST = "1.96.0"
LOCKS = ("core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock")


def identity(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def git(root, *args):
    launcher = ["cmd", "/c", "git"] if os.name == "nt" else ["git"]
    return subprocess.check_output([*launcher, *args], cwd=root)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


class Baseline:
    def __init__(self, source_root, output, llvm_bin, pwsh, bash):
        self.source_root = source_root
        self.output = output
        self.llvm_bin = llvm_bin
        self.pwsh = pwsh
        self.bash = bash
        self.logs = output / "logs"
        self.env = os.environ.copy()
        for key in tuple(self.env):
            if key.startswith(("XIAO_", "RUST_BIGDECIMAL_", "CARGO_PROFILE_")) or key in {
                "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
            }:
                self.env.pop(key)
        for name in ("home", "target", "temp", "logs"):
            (output / name).mkdir(parents=True)
        self.env.update({
            "CARGO_TARGET_DIR": str(output / "target"),
            "XIAO_HOME": str(output / "home"),
            "TEMP": str(output / "temp"),
            "TMP": str(output / "temp"),
            "TMPDIR": str(output / "temp"),
            "XIAO_STDLIB_ROOT": str(source_root / "stdlib"),
            "XIAO_B1_F10": "0",
            "XIAO_B2_A_PERF": "0",
            "CARGO_TERM_COLOR": "never",
        })
        self.report = {
            "passed": False,
            "product_commit": git(source_root, "rev-parse", "HEAD").decode().strip(),
            "product_tree": git(source_root, "show", "-s", "--format=%T", "HEAD").decode().strip(),
            "source_root": str(source_root),
            "task_root": str(output),
            "target": TARGET,
            "rust": {},
            "cargo": {},
            "tools": {},
            "commands": {},
            "environment": {key: self.env[key] for key in (
                "CARGO_TARGET_DIR", "XIAO_HOME", "TEMP", "TMP", "TMPDIR",
                "XIAO_STDLIB_ROOT", "XIAO_B1_F10", "XIAO_B2_A_PERF")},
        }

    def run(self, name, argv, cwd=None, timeout=900):
        started = monotonic()
        try:
            process = subprocess.run(argv, cwd=cwd or self.source_root, env=self.env,
                                     capture_output=True, timeout=timeout)
            exit_code, stdout, stderr = process.returncode, process.stdout, process.stderr
        except subprocess.TimeoutExpired as error:
            exit_code, stdout, stderr = None, error.stdout or b"", error.stderr or b""
        row = {"argv": argv, "exit": exit_code, "timeout_seconds": timeout,
               "elapsed_seconds": round(monotonic() - started, 3)}
        for stream, raw in (("stdout", stdout), ("stderr", stderr)):
            digest = identity(raw)
            path = self.logs / f"{digest['sha256']}.log"
            if not path.exists():
                path.write_bytes(raw)
            row[stream] = digest
        self.report["commands"][name] = row
        for raw in (stdout, stderr):
            print(raw.decode("utf-8", errors="replace"), end="", flush=True)
        require(exit_code is not None, f"{name} 超时")
        require(exit_code == 0, f"{name} 退出 {exit_code}")
        return stdout.decode("utf-8").replace("\r\n", "\n")

    def tool(self, name):
        path = (self.llvm_bin / f"{name}.exe").resolve(strict=True)
        version = self.run(name, [str(path), "--version"], timeout=30)
        self.report["tools"][name] = {
            "path": str(path), **identity(path.read_bytes()), "version": version.splitlines()
        }
        self.env[f"XIAO_{name.upper().replace('-', '_')}"] = str(path)

    def verify(self):
        require(self.report["product_commit"] == PRODUCT, "源码 worktree 未固定到 B3-1 产品提交")
        require(git(self.source_root, "status", "--porcelain=v1", "-z") == b"", "产品 worktree 不干净")
        require(not git(ROOT, "diff", "--name-only", B3_1_CLOSEOUT, "--", B3_1)
                and not git(ROOT, "ls-files", "--others", "--exclude-standard", "--", B3_1),
                "B3-1 当前证据偏离固定封存提交")
        for label, script in (("b3-1-read-only-audit", "audit.py"), ("b3-1-self-negative", "self_negative.py")):
            self.run(label, [sys.executable, "-B", "-X", "utf8", str(ROOT / B3_1 / script)],
                     cwd=ROOT, timeout=240)
        rustc = self.run("rustc", ["rustc", f"+{RUST}", "-vV"], timeout=30)
        require(f"release: {RUST}\n" in rustc and f"host: {TARGET}\n" in rustc, "Rust 工具链或目标不符")
        self.report["rust"] = {"version": rustc.splitlines()}
        cargo = self.run("cargo", ["cargo", f"+{RUST}", "-Vv"], timeout=30)
        require(f"cargo {RUST} " in cargo, "Cargo 版本不符")
        self.report["cargo"] = {"path": shutil.which("cargo"), "version": cargo.splitlines()}
        for name in ("clang", "llvm-as", "llc", "llvm-nm", "llvm-readobj"):
            self.tool(name)
        self.env["XIAO_TARGET_TRIPLE"] = TARGET
        self.report["lockfiles"] = {}
        for path in LOCKS:
            raw = (self.source_root / path).read_bytes()
            canonical = git(self.source_root, "show", f"{PRODUCT}:{path}")
            require(raw.replace(b"\r\n", b"\n") == canonical, f"检出锁文件与固定对象不符：{path}")
            self.report["lockfiles"][path] = {"checkout": identity(raw), "git_utf8_lf": identity(canonical)}
        cargo = ["cargo", f"+{RUST}"]
        locked = ["--locked", "--offline", "--manifest-path", "core/rust/Cargo.toml",
                  "--target", TARGET]
        self.run("build-release-runtime", [
            *cargo, "build", *locked, "--release", "-p", "xiao-runtime",
            "--features", "test-failpoints",
        ])
        runtime = self.output / "target" / TARGET / "release" / "xiao_runtime.lib"
        raw = runtime.read_bytes()
        self.report["runtime"] = {"path": str(runtime), **identity(raw),
                                  "features": ["test-failpoints"]}
        self.env["XIAO_RUNTIME_LIBRARY"] = str(runtime)
        test = self.run("runtime-time-boundaries", [
            *cargo, "test", *locked, "--release", "-p", "xiao-runtime",
            "--features", "test-failpoints", "--lib", "time::", "--", "--nocapture",
        ])
        require("test result: ok. 4 passed; 0 failed; 0 ignored;" in test,
                "继承 time Runtime 边界不符")
        self.report["runtime_after"] = identity(runtime.read_bytes())
        require(self.report["runtime_after"] == identity(raw), "Runtime 前后字节摘要不一致")
        guard_report = self.output / "b1-guards-report.json"
        self.run("b1-count-guards", [
            str(self.pwsh), "-NoProfile", "-File",
            str(self.source_root / "tools/gates/b1-probe-guards.ps1"),
            "-BashPath", str(self.bash), "-ReportPath", str(guard_report),
        ], timeout=120)
        guards = json.loads(guard_report.read_bytes())
        require(guards["passed"] == guards["total"] == 28
                and guards["scope"] == "simulated-cargo-output", "B1 计数守卫与运行范围不符")
        workflow = (self.source_root / guards["workflow"]).read_bytes()
        workflow_blob = git(self.source_root, "show", f"HEAD:{guards['workflow']}")
        require(guards["workflowSha256"] == identity(workflow)["sha256"]
                and workflow.replace(b"\r\n", b"\n") == workflow_blob,
                "B1 守卫使用的维护工作流偏离固定产品对象")
        self.report["b1_guard_report"] = {"path": "b1-guards-report.json",
                                          **identity(guard_report.read_bytes()),
                                          "workflow_checkout": identity(workflow),
                                          "workflow_git_utf8_lf": identity(workflow_blob),
                                          "passed": 28, "actual_cargo_outputs": 0, "derived_cases": 28}
        require(git(self.source_root, "rev-parse", "HEAD").decode().strip() == PRODUCT
                and not git(self.source_root, "status", "--porcelain=v1", "-z"), "基线执行后产品快照发生变化")
        self.report["passed"] = True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--llvm-bin", type=Path, required=True)
    parser.add_argument("--pwsh", type=Path, required=True)
    parser.add_argument("--bash", type=Path, required=True)
    args = parser.parse_args()
    source_root = args.source_root.resolve(strict=True)
    output = args.output.resolve()
    require(not output.exists() and not output.is_relative_to(ROOT)
            and not output.is_relative_to(source_root), "任务根须位于仓库之外且尚不存在")
    baseline = Baseline(source_root, output, args.llvm_bin.resolve(strict=True),
                        args.pwsh.resolve(strict=True), args.bash.resolve(strict=True))
    try:
        baseline.verify()
    finally:
        (output / "result.json").write_bytes(encoded(baseline.report))
    with ZipFile(output / "windows-baseline.zip", "x", ZIP_DEFLATED) as archive:
        archive.write(output / "result.json", "result.json")
        archive.write(output / "b1-guards-report.json", "b1-guards-report.json")
        for path in sorted(baseline.logs.glob("*.log")):
            archive.write(path, f"logs/{path.name}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"B3-2A Windows 基线失败：{error}", file=sys.stderr)
        raise SystemExit(1) from None
