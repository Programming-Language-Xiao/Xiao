"""B2-B B0 固定 Git 对象只读审计。"""

import argparse
import hashlib
import json
from datetime import datetime, timedelta, timezone
from pathlib import Path
import re
import subprocess


DEFAULT_ROOT = Path(__file__).resolve().parent
DEFAULT_SOURCE = DEFAULT_ROOT.parents[4]
BASELINE_COMMIT = "d08b8460"
A7_COMMIT = "5749d6e0"
REQUIRED = {
    "b0-build-runtime", "b0-build-adapters", "b0-b1-contract", "b0-b1-resources",
    "b0-b2-resources", "b0-b2-language", "b0-package-tests", "b0-driver-unit",
    "b0-b1-vm-exact", "b0-b1-native-exact", "b0-a1-vm-exact", "b0-a1-native-exact",
    "b0-workspace-check", "b0-benchmarks-check", "b0-workspace-clippy",
    "b0-workspace-fmt", "b0-b1-guards", "b0-docs-final-2",
}
COUNTS = {
    "b0-b1-contract": [(9, 0)], "b0-b1-resources": [(17, 0)],
    "b0-b2-resources": [(3, 0), (8, 0)], "b0-b2-language": [(12, 1)],
    "b0-driver-unit": [(64, 1)],
}
EXACT = {
    "b0-b1-vm-exact": "b1_capabilities_have_vm_golden_outputs",
    "b0-b1-native-exact": "b1_capabilities_match_native_golden_outputs",
    "b0-a1-vm-exact": "a1_numeric_values_and_errors_have_vm_golden_outputs",
    "b0-a1-native-exact": "a1_numeric_values_and_errors_match_native_golden_outputs",
}
PRODUCT_ROOTS = [
    "core/rust", "stdlib", "tests/fixtures", "tests/benchmarks", ".github/workflows",
    ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json",
    "package.json", "bun.lock", ".gitattributes",
]


def fail(reason):
    raise SystemExit(reason)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(source, *args):
    return subprocess.check_output(
        ["cmd", "/c", "git", *args], cwd=source, text=True, encoding="utf-8"
    ).splitlines()


def read(root, name):
    return json.loads((root / name).read_text(encoding="utf-8"))


def inventory(root):
    rows = [
        {"path": path.relative_to(root).as_posix(), "bytes": path.stat().st_size, "sha256": sha(path)}
        for path in sorted(root.rglob("*"))
        if path.is_file() and path != root / "evidence.json" and "__pycache__" not in path.parts
    ]
    manifest = read(root, "evidence.json")
    if manifest.get("file_count") != len(rows) or manifest.get("files") != rows:
        fail("B0证据清单或摘要不符")
    return rows


def require_contained(path_value, parent, label, must_exist=True, file=False):
    path = Path(path_value).resolve()
    if not path.is_relative_to(parent):
        fail(f"B0路径不在任务根内：{label}")
    if must_exist and ((not path.exists()) or (file and not path.is_file()) or (not file and not path.is_dir())):
        fail(f"B0路径不存在或类型不符：{label}")
    return path


def main(root, source):
    rows = inventory(root)
    bootstrap = read(root, "bootstrap.json")
    baseline = git(source, "rev-parse", "--verify", BASELINE_COMMIT)[0]
    if bootstrap.get("entry_commit") != baseline:
        fail("B0入口记录不符固定Git对象")
    changed = git(source, "diff", "--name-only", A7_COMMIT, BASELINE_COMMIT, "--", *PRODUCT_ROOTS)
    if changed:
        fail("B0产品范围相对A7快照存在差异：" + str(changed))
    a7_lock = subprocess.check_output(["cmd", "/c", "git", "show", f"{A7_COMMIT}:core/rust/Cargo.lock"], cwd=source)
    baseline_lock = subprocess.check_output(["cmd", "/c", "git", "show", f"{BASELINE_COMMIT}:core/rust/Cargo.lock"], cwd=source)
    if a7_lock != baseline_lock:
        fail("B0 Cargo.lock固定Git对象发生变化")
    for name in REQUIRED:
        record = read(root, name + ".json")
        log = root / record["log"]
        raw = log.read_bytes()
        if (
            record.get("exit_code") != 0
            or not record.get("source_unchanged")
            or record.get("source_before") != record.get("source_after")
            or sha(log) != record.get("sha256")
        ):
            fail("B0命令失败、源码变化或日志摘要不符：" + name)
        text = raw.decode("utf-8", errors="replace")
        if name in COUNTS:
            found = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text, re.M)
            if sorted(found) != sorted((str(a), str(b)) for a, b in COUNTS[name]):
                fail("B0计数不符：" + name)
        if name in EXACT and ("--exact" not in record.get("argv", []) or len(re.findall(r"^test result: ok\. 1 passed; 0 failed; 0 ignored;", text, re.M)) != 1):
            fail("B0精确入口不符：" + name)
    guards = read(root, "b1-guards-report.json")
    if guards.get("passed") != guards.get("total") or guards.get("total") != len(guards.get("cases", [])) or guards.get("total") != 28 or not all(row.get("Passed") for row in guards["cases"]):
        fail("B0守卫不是28/28")
    overrides = read(root, "environment.json").get("overrides", {})
    if str(overrides.get("XIAO_B1_F10")) != "0" or str(overrides.get("XIAO_B2_A_PERF")) != "0":
        fail("B0错误启用性能开关")
    bootstrap = read(root, "bootstrap.json")
    environment = read(root, "environment.json")
    repository = Path(source).resolve()
    task_root = require_contained(bootstrap["task_root"], Path(bootstrap["task_root"]).resolve().parent, "task_root")
    if task_root == repository or task_root.is_relative_to(repository):
        fail("B0任务根不能位于仓库内")
    source_root = require_contained(bootstrap["source_root"], task_root, "source_root")
    if environment.get("fresh_target") is not True or environment.get("fresh_xiao_home") is not True:
        fail("B0未声明全新target或Xiao Home")
    for key in ["CARGO_TARGET_DIR", "XIAO_HOME", "TEMP", "TMP"]:
        require_contained(overrides.get(key), task_root, key)
    target_root = Path(overrides["CARGO_TARGET_DIR"]).resolve()
    require_contained(overrides.get("XIAO_STDLIB_ROOT"), source_root, "XIAO_STDLIB_ROOT")
    runtime_path = require_contained(overrides.get("XIAO_RUNTIME_LIBRARY"), task_root, "XIAO_RUNTIME_LIBRARY", file=True)
    if not runtime_path.is_relative_to(target_root):
        fail("B0 Runtime不在独立target内")
    build_record = read(root, "b0-build-runtime.json")
    started = datetime.fromisoformat(build_record["started_utc"])
    finished = datetime.fromisoformat(build_record["finished_utc"])
    modified = datetime.fromtimestamp(runtime_path.stat().st_mtime, timezone.utc)
    if not (started - timedelta(seconds=5) <= modified <= finished + timedelta(seconds=5)):
        fail("B0 Runtime不在本次构建时间窗口内")
    for key in ["XIAO_DIAGNOSTICS_PATH", "XIAO_RUST_DOC_ADAPTER"]:
        require_contained(overrides.get(key), task_root, key, file=True)
    closeout = read(root, "closeout.json")
    if (
        not closeout.get("passed")
        or closeout.get("actual_head") != baseline
        or closeout.get("checks") != sorted(REQUIRED)
        or closeout.get("counts") != {name: [list(pair) for pair in pairs] for name, pairs in COUNTS.items()}
        or closeout.get("exact") != EXACT
        or closeout.get("planned_entry_commit") != "e596cea8de38059962d5f73f8b01286ffa617970"
        or closeout.get("guards") != "28/28"
        or closeout.get("abi") != "1.18"
        or closeout.get("codegen") != 20
        or closeout.get("active_intrinsic_max") != 32
        or closeout.get("b1_721_executed")
        or closeout.get("task_root") != str(task_root)
        or closeout.get("source_root") != str(source_root)
        or closeout.get("fresh_target") is not True
        or closeout.get("fresh_xiao_home") is not True
    ):
        fail("B0 closeout固定状态不符")
    runtime_identity = closeout.get("runtime_identity")
    if (
        not isinstance(runtime_identity, dict)
        or runtime_identity.get("path") != str(runtime_path)
        or runtime_identity.get("bytes") != runtime_path.stat().st_size
        or runtime_identity.get("sha256") != hashlib.sha256(runtime_path.read_bytes()).hexdigest()
        or runtime_identity.get("build_record") != "b0-build-runtime.json"
    ):
        fail("B0 Runtime身份摘要不符")
    print(json.dumps({"passed": len(REQUIRED), "total": len(REQUIRED), "files": len(rows), "baseline_commit": baseline, "planned_entry_commit": "e596cea8"}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE)
    args = parser.parse_args()
    main(args.root.resolve(), args.source_root.resolve())
