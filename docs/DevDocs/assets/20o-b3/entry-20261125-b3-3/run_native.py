"""在固定源码和独立任务根执行 B3-3 三平台验证。"""

import argparse
from importlib.util import module_from_spec, spec_from_file_location
from io import BytesIO
import json
from pathlib import Path
import re
import zipfile

HERE = Path(__file__).resolve().parent
spec = spec_from_file_location("b3_time", HERE.parent / "entry-20261124-b3-1" / "run.py")
time = module_from_spec(spec)
spec.loader.exec_module(time)
LOCKS, MANIFEST, ROOT, RUST = time.LOCKS, time.MANIFEST, time.ROOT, time.RUST
git, identity, require = time.git, time.identity, time.require
B3_2 = "f8c2b0edfa6896c8114dcdaaa9a3763f38aa8b49"
BASE_ARCHIVE = "docs/DevDocs/assets/20o-b3/entry-20261125-b3-2-final/runs.zip"
PLATFORMS = dict(zip(time.TARGETS, ("windows", "linux", "macos"), strict=True))
TARGET = "20o_b3_fs_tutorial"
NATIVE_TEST = "native::fs_tutorial_matches_native_golden"
GOLDEN = ROOT / "tests/fixtures/docs/fs/tutorial.stdout"


def old_artifact(platform):
    with zipfile.ZipFile(BytesIO(git("show", f"{B3_2}:{BASE_ARCHIVE}"))) as archive:
        rows = json.loads(archive.read(f"platforms/{platform}.json"))["artifacts"]
        row = next(row for row in rows if row["case"] == "args-std-o0")
        file = row["file"]
        raw = archive.read("objects/" + file["sha256"])
    require(row["generator"] == 24 and identity(raw) == {key: file[key] for key in ("bytes", "sha256")},
            "B3-2 生成器 24 原件身份不符")
    return raw, {"commit": B3_2, "archive": BASE_ARCHIVE, "member": file["path"], **identity(raw)}


def pick_artifacts(task, output):
    nodes = [json.loads(line) for line in re.findall(r"(?m)^B3-3-NATIVE (.+)$", output)]
    expected = {f"fs-{alias}-o{level}" for alias in ("std", "stdlib", "denied") for level in (0, 3)}
    require(len(nodes) == len(expected) and {row["case"] for row in nodes} == expected,
            "教程双路径或权限边界缺项")

    def file(path):
        path = Path(path).resolve(strict=True)
        require(path.is_relative_to(task.output / "temp"), "产物路径越出任务根")
        return {"path": path.relative_to(task.output).as_posix(), **identity(path.read_bytes())}

    for row in nodes:
        path = Path(row.pop("path"))
        row["artifact"] = file(path)
        require(row["artifact"]["sha256"] == row.pop("sha256"), "产物字节与验证回执不符")
        row["stdout"] = file(path.with_name("stdout.bin"))
        row["stderr"] = file(path.with_name("stderr.bin"))
        golden = b"X06-RUNTIME-026\n" * 5 if "denied" in row["case"] else GOLDEN.read_bytes()
        require(row["stdout"]["sha256"] == identity(golden)["sha256"]
                and row["stderr"]["bytes"] == 0, "原生输出不符")
        map_path, digest = row.pop("coff_map"), row.pop("coff_map_sha256")
        if row["target"] == "x86_64-pc-windows-msvc":
            row["map"] = file(map_path)
            require(row["map"]["sha256"] == digest, "COFF map 身份不符")
        else:
            require(map_path is None and digest is None, "非 Windows 产物出现 COFF map")
    return nodes


def verify(task, source):
    before = git("status", "--porcelain=v1", "-z")
    require(not before and task.report["source_commit"] == source, "需要干净的固定源码")
    rust = task.run("rustc", ["rustc", f"+{RUST}", "-vV"], timeout=25)
    require(f"release: {RUST}" in rust, "Rust 版本与冻结表不符")
    target = re.search(r"(?m)^host: (.+)$", rust)[1]
    require(target in PLATFORMS, "Rust 主机不在三平台目标内")
    task.report["target"] = target
    task.env["XIAO_TARGET_TRIPLE"] = target
    for name in ("clang", "llvm-as", "llc"):
        task.tool(name, ["--version"])
    task.report["lockfiles"] = {path: identity(git("show", source + ":" + path)) for path in LOCKS}
    previous, receipt = old_artifact(PLATFORMS[target])
    old = task.output / "temp" / "baseline-generator24"
    old.write_bytes(previous)
    task.env["XIAO_B3_2_NATIVE_ARTIFACT"] = str(old)
    task.report["older_artifact"] = receipt
    flags = ["--locked", "--release", "--manifest-path", MANIFEST, "--target", target]
    cargo = ["cargo", f"+{RUST}"]
    task.run("build-runtime", [*cargo, "build", *flags, "-p", "xiao-runtime"])
    runtime = task.output / "target" / target / "release" / time.TARGETS[target]
    task.env["XIAO_RUNTIME_LIBRARY"] = str(runtime)
    task.report["runtime"] = {"path": str(runtime), **identity(runtime.read_bytes())}
    task.run("runtime-and-abi", [*cargo, "test", *flags, "-p", "xiao-runtime", "--lib",
                                "fs_", "--", "--nocapture"])
    common = [*cargo, "test", *flags, "-p", "xiao-driver", "--test", TARGET, "--features", "fs-native-test"]
    vm = task.run("vm", [*common, "--", "--skip", "native::", "--nocapture"])
    require(all(f"B3-3-VM module=fs alias={alias} exact=true demo_empty=true" in vm
                for alias in ("std", "stdlib")), "VM 冻结教程未通过")
    trace = task.output / "temp" / "clang-commands.log"
    task.env.update(CC_PRINT_OPTIONS="1", CC_PRINT_OPTIONS_FILE=str(trace))
    native = task.run("native", [*common, NATIVE_TEST, "--", "--exact", "--ignored", "--nocapture"])
    require(time.EXACT in native, "原生精确入口未运行")
    task.report["artifacts"] = pick_artifacts(task, native)
    task.report["versions"] = json.loads(re.search(r"(?m)^B3-3-VERSION (.+)$", native)[1])
    if trace.is_file():
        task.report["clang_commands"] = {
            "path": trace.relative_to(task.output).as_posix(), **identity(trace.read_bytes()),
        }
    require(target != "x86_64-pc-windows-msvc" or "clang_commands" in task.report,
            "缺少 Windows MSVC 编译和链接轨迹")
    task.report["runtime_after"] = identity(runtime.read_bytes())
    require(task.report["runtime_after"] == {key: task.report["runtime"][key] for key in ("bytes", "sha256")},
            "运行期间 Runtime 字节变化")
    require(git("status", "--porcelain=v1", "-z") == before and git("rev-parse", "HEAD").decode().strip() == source,
            "验收期间源码变化")
    task.report["passed"] = True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--llvm-bin", type=Path)
    parser.add_argument("--product-commit", required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    require(not output.is_relative_to(ROOT) and not output.exists(), "任务根须位于仓库之外且尚未创建")
    task = time.Task(output, args.llvm_bin)
    try:
        verify(task, args.product_commit)
    finally:
        (output / "result.json").write_text(json.dumps(task.report, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
                                           encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
