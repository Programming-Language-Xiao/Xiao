"""B2-B B0 只读基线审计。"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
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


def fail(reason):
    raise SystemExit(reason)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(name):
    return json.loads((ROOT / name).read_text(encoding="utf-8"))


def inventory(write=False):
    files = [p for p in sorted(ROOT.rglob("*")) if p.is_file() and p.name != "evidence.json" and "__pycache__" not in p.parts]
    rows = [{"path": p.relative_to(ROOT).as_posix(), "bytes": p.stat().st_size, "sha256": sha(p)} for p in files]
    if write:
        (ROOT / "evidence.json").write_text(json.dumps({"file_count": len(rows), "files": rows}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    else:
        manifest = read("evidence.json")
        if manifest["file_count"] != len(rows) or manifest["files"] != rows:
            fail("B0证据清单或摘要不符")
    return len(rows)


def main():
    files = inventory()
    bootstrap = read("bootstrap.json")
    current = subprocess.check_output(["cmd", "/c", "git", "rev-parse", "HEAD"], cwd=REPO, text=True).strip()
    if bootstrap["entry_commit"] != current or current != "d08b8460f1941fb82f4ce36b38359fc2ba448f33":
        fail("B0实际HEAD或入口记录不符")
    protected = ["core/rust", "stdlib", "tests/benchmarks", ".github/workflows", ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json", "package.json", "bun.lock", ".gitattributes"]
    if subprocess.check_output(["cmd", "/c", "git", "diff", "--name-only", "5749d6e0", current, "--", *protected], cwd=REPO, text=True).splitlines():
        fail("B0产品范围相对A7快照存在差异")
    if subprocess.check_output(["cmd", "/c", "git", "diff", "--name-only", "5749d6e0", current, "--", "core/rust/Cargo.lock"], cwd=REPO, text=True).splitlines():
        fail("B0 Cargo.lock相对A7快照发生变化")
    for name in REQUIRED:
        record = read(name + ".json")
        raw = (ROOT / record["log"]).read_bytes()
        if record["exit_code"] != 0 or not record["source_unchanged"] or record["source_before"] != record["source_after"] or sha(ROOT / record["log"]) != record["sha256"]:
            fail("B0命令失败、源码变化或日志摘要不符：" + name)
        text = raw.decode("utf-8", errors="replace")
        if name in COUNTS:
            found = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text, re.M)
            if sorted(found) != sorted((str(a), str(b)) for a, b in COUNTS[name]):
                fail("B0计数不符：" + name)
        if name in EXACT:
            if "--exact" not in record["argv"] or len(re.findall(r"^test result: ok\. 1 passed; 0 failed; 0 ignored;", text, re.M)) != 1:
                fail("B0精确入口不符：" + name)
    guards = read("b1-guards-report.json")
    if guards["passed"] != guards["total"] or guards["total"] != len(guards["cases"]) or guards["total"] != 28 or not all(row["Passed"] for row in guards["cases"]):
        fail("B0守卫不是28/28")
    environment = read("environment.json")
    overrides = environment.get("overrides", {})
    if str(overrides.get("XIAO_B1_F10")) != "0" or str(overrides.get("XIAO_B2_A_PERF")) != "0":
        fail("B0错误启用性能开关")
    closeout = {"passed": True, "scope": "B2-B B0只读基线", "actual_head": current, "planned_entry_commit": "e596cea8de38059962d5f73f8b01286ffa617970", "planned_entry_stale": True, "checks": sorted(REQUIRED), "counts": COUNTS, "exact": EXACT, "guards": "28/28", "abi": "1.18", "codegen": 20, "active_intrinsic_max": 32, "cmath": "planned", "b1_f10": False, "b2_a_perf": False, "b1_721_executed": False, "product_changed": False, "registry_changed": False, "b2_b_product_started": False, "b2_c_started": False}
    (ROOT / "closeout.json").write_text(json.dumps(closeout, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"passed": True, "files": files, "checks": len(REQUIRED), "actual_head": current, "planned_entry_stale": True}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", action="store_true")
    args = parser.parse_args()
    print(json.dumps({"files": inventory(True)})) if args.manifest else main()
