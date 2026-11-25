"""采集 B3-2 的 VM、原生、生成器兼容和启动证据。"""

import argparse
from io import BytesIO
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from zipfile import ZipFile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "entry-20261124-b3-1"))
from run import LOCKS, MANIFEST, ROOT, RUST, TARGETS, Task, git, identity, require

B3_1 = "84660c11ef1377a6ff5744547c8291f01a8879c1"
PLATFORMS = dict(zip(TARGETS, ("windows", "linux", "macos"), strict=True))
FIXTURES = ROOT / "tests/fixtures/docs"
TUTORIALS = {
    f"{module}-{alias}-o{level}"
    for module in ("args", "cli") for alias in ("std", "stdlib") for level in (0, 3)
}
BOUNDARIES = {"cli-types", "cli-limits", "args-denied", "args-children"}
REJECTIONS = {"invalid": "X09-DRIVER-004", **dict.fromkeys(("single", "count", "total"), "X09-DRIVER-005")}


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def previous_artifact(platform):
    path = f"docs/DevDocs/assets/20o-b3/entry-20261124-b3-1/{platform}.zip"
    with ZipFile(BytesIO(git("show", f"{B3_1}:{path}"))) as archive:
        row = json.loads(archive.read("result.json"))["artifacts"][0]
        raw = archive.read(row["path"])
    require(identity(raw) == {key: row[key] for key in ("bytes", "sha256")}, "B3-1 原件摘要不符")
    return raw, {"commit": B3_1, "archive": path, "member": row["path"], **identity(raw)}


class ArgsCliTask(Task):
    def collect_artifacts(self, native, target):
        rows = [json.loads(row) for row in re.findall(r"(?m)^B3-2-ARTIFACT (.+)$", native)]
        require(len(rows) == len(TUTORIALS | BOUNDARIES)
                and {row["case"] for row in rows} == TUTORIALS | BOUNDARIES,
                "原生教程、优化等级或边界不齐")
        for row in rows:
            require(row["target"] == target and row["generator"] == 24
                    and row["verification"] == "complete" and row["components"] == row["observed"]
                    and not row["unclassified"] and row["old_verifier"] == "X17-XAR-007",
                    f"产物验证不符：{row['case']}")
            executable = Path(row.pop("path")).resolve(strict=True)
            require(executable.is_relative_to(self.output / "temp"), "原生产物越出任务根")
            row["file"] = {"path": executable.relative_to(self.output).as_posix(), **identity(executable.read_bytes())}
            map_path = row.pop("coff_map")
            if target == "x86_64-pc-windows-msvc":
                sidecar = Path(map_path).resolve(strict=True)
                require(sidecar.parent == executable.parent, "COFF map 不属于同一构建目录")
                row["map"] = {"path": sidecar.relative_to(self.output).as_posix(), **identity(sidecar.read_bytes())}
            else:
                require(map_path is None, "非 Windows 产物含 COFF map")
            if row["case"] in TUTORIALS:
                module, _, opt = row["case"].split("-")
                require(row["optimization"] == int(opt[1:]), "优化等级与教程标记不符")
                self.run(f"exact-{row['case']}", [sys.executable, "-B", str(__file__),
                         "--execute-tutorial", module, str(executable)], timeout=25)
                command = self.report["commands"][f"exact-{row['case']}"]
                require(command["stdout"] == identity((FIXTURES / module / "tutorial.stdout").read_bytes())
                        and command["stderr"] == identity(b""), "原生教程的输出字节与黄金不符")
        self.report["artifacts"] = rows

    def verify(self):
        before = git("status", "--porcelain=v1", "-z")
        require(not before, "任务需从干净的已提交源码取证")
        rust = self.run("rustc", ["rustc", f"+{RUST}", "-vV"], timeout=25)
        require(f"release: {RUST}" in rust, "Rust 版本与冻结表不符")
        target = re.search(r"(?m)^host: (.+)$", rust)[1]
        require(target in TARGETS, f"目标不在 B3 三平台范围内：{target}")
        self.report["target"] = target
        self.env["XIAO_TARGET_TRIPLE"] = target
        for tool in ("clang", "llvm-as", "llc"):
            self.tool(tool, ["--version"])
        self.report["lockfiles"] = {path: identity(git("show", "HEAD:" + path)) for path in LOCKS}
        old_bytes, old_identity = previous_artifact(PLATFORMS[target])
        old_path = self.output / "temp" / "b3-1-generator23"
        old_path.write_bytes(old_bytes)
        self.env["XIAO_B3_1_NATIVE_ARTIFACT"] = str(old_path)
        self.report["older_artifact"] = old_identity
        cargo = ["cargo", f"+{RUST}"]
        locked = ["--locked", "--release", "--manifest-path", MANIFEST, "--target", target]
        self.run("build-release-runtime", [*cargo, "build", *locked, "-p", "xiao-runtime"])
        runtime = self.output / "target" / target / "release" / TARGETS[target]
        self.env["XIAO_RUNTIME_LIBRARY"] = str(runtime)
        self.report["runtime"] = {"path": str(runtime), **identity(runtime.read_bytes())}

        for filters in ("arguments::tests", "abi::abi_args::tests"):
            name = "runtime-" + filters.split("::")[-2]
            log = self.run(name, [*cargo, "test", *locked, "-p", "xiao-runtime", "--lib", filters])
            require(re.search(r"test result: ok\. [1-9]\d* passed; 0 failed; 0 ignored;", log), f"{name} 未实跑")
        common = [*cargo, "test", *locked, "-p", "xiao-driver", "--test", "20o_b3_args_cli",
                  "--features", "args-cli-native-test"]
        vm = self.run("vm", [*common, "--", "--skip", "native::", "--nocapture"])
        require(set(re.findall(r"(?m)^B3-2-VM module=(\w+) alias=(\w+) exact=true$", vm))
                == {(module, alias) for module in ("args", "cli") for alias in ("std", "stdlib")}
                and "test result: ok. 7 passed; 0 failed; 0 ignored;" in vm, "VM 教程或边界未执行")

        trace = self.output / "temp" / "clang-commands.log"
        self.env.update(CC_PRINT_OPTIONS="1", CC_PRINT_OPTIONS_FILE=str(trace))
        native = self.run("native", [*common, "--", "native::args_cli_matches_native_goldens_and_boundaries",
                                      "--exact", "--ignored", "--nocapture"])
        require("test result: ok. 1 passed; 0 failed; 0 ignored;" in native, "原生精确入口未执行")
        require(dict(re.findall(r"(?m)^B3-2-NATIVE-REJECTION case=(\w+) exit=2 code=(\S+)$", native))
                == REJECTIONS, "原生参数拒绝证据缺项")
        version = json.loads(re.search(r"(?m)^B3-2-GENERATOR (.+)$", native)[1])
        require(version["baseline"] == 23 and version["current"] == 24
                and version["older_metadata"] == "accepted"
                and version["baseline_fingerprint"] != version["current_fingerprint"], "兼容或指纹不符")
        self.report["version_matrix"] = version
        self.collect_artifacts(native, target)
        if trace.exists():
            raw = trace.read_bytes()
            self.report["clang_commands"] = {"path": trace.relative_to(self.output).as_posix(), **identity(raw)}
        require(target != "x86_64-pc-windows-msvc" or "clang_commands" in self.report, "缺少 MSVC 编译及链接命令")
        self.report["runtime_after"] = identity(runtime.read_bytes())
        require(self.report["runtime_after"] == {key: self.report["runtime"][key] for key in ("bytes", "sha256")},
                "验收期间 Runtime 字节变化")
        require(git("status", "--porcelain=v1", "-z") == before
                and git("rev-parse", "HEAD").decode().strip() == self.report["source_commit"], "验收期间源码变化")
        self.report["passed"] = True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--llvm-bin", type=Path)
    parser.add_argument("--execute-tutorial", nargs=2, metavar=("MODULE", "EXECUTABLE"))
    args = parser.parse_args()
    if args.execute_tutorial:
        module, executable = args.execute_tutorial
        require(module in ("args", "cli"), "未知教程")
        argv = [executable]
        if module == "args":
            fixture = json.loads((FIXTURES / module / "tutorial.input.json").read_bytes())
            argv = [fixture["program_name"], *fixture["argv"]]
        raise SystemExit(subprocess.run(argv, executable=executable, timeout=20).returncode)
    require(args.output is not None, "需要任务根")
    output = args.output.resolve()
    require(not output.is_relative_to(ROOT) and not output.exists(), "任务根须位于仓库之外且尚未创建")
    task = ArgsCliTask(output, args.llvm_bin)
    try:
        task.verify()
    finally:
        (output / "result.json").write_bytes(encoded(task.report))


if __name__ == "__main__":
    main()
