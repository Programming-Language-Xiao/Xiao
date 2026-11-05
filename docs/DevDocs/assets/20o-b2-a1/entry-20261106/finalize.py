"""从真实命令、CI和固定代码快照收口A1；不执行产品或改写旧日志。"""

import datetime as dt
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib


ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]


def read(name):
    return json.loads((ROOT / name).read_text(encoding="utf-8"))


def write(name, value):
    (ROOT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def git(*args):
    return subprocess.check_output(["cmd", "/c", "git", *args], cwd=REPOSITORY)


def main():
    report = read("closeout.json")
    code = report["code_commit"]
    metadata = json.loads(next(line for line in (ROOT / "metadata-locked.log").read_text(encoding="utf-8").splitlines() if line.startswith("{")))
    nodes = {row["id"]: row for row in metadata["resolve"]["nodes"]}
    names = {"autocfg", "bigdecimal", "libm", "num-bigint", "num-integer", "num-traits"}
    packages = [{"name": row["name"], "version": row["version"], "source": row["source"],
                 "enabled_features": nodes[row["id"]]["features"], "manifest_path": row["manifest_path"]}
                for row in metadata["packages"] if row["name"] in names]
    write("resolved-features-final.json", {"record": "metadata-locked.json", "packages": packages})
    protected = ["stdlib", "core/rust/crates/xiao-runtime-abi", "core/rust/crates/xiao-intrinsics",
                 "core/rust/crates/xiao-codegen-llvm/src/lib.rs", "tools/gates/b1-probe-guards.ps1",
                 "docs/DevDocs/20o-b2-freeze-draft.md", "docs/DevDocs/assets/20o-b2-a0",
                 "docs/DevDocs/assets/20o-b2-freeze-d", "docs/DevDocs/assets/20o-b2-freeze-close"]
    assert not git("diff", "--name-only", "e06cc88a", "--", *protected)
    source_roots = read(read("build-runtime-locked.json")["source_before"]["manifest"])["scope"]
    assert not git("diff", "--name-only", code, "--", *source_roots)
    changed = git("diff", "--name-only", "e06cc88a").decode().splitlines()
    untracked = git("ls-files", "--others", "--exclude-standard").decode().splitlines()
    code_paths = set(git("diff", "--name-only", "e06cc88a", code).decode().splitlines())
    assert all(path in code_paths or path.startswith("docs/DevDocs/assets/20o-b2-a1/entry-20261106/") for path in changed + untracked)
    # 配置接线只能新增受控Cargo步骤及其触发路径；原目标、时限、并行和命令块逐行保留。
    for path in [".github/workflows/maintenance-regression.yml", ".github/workflows/workspace-gates.yml"]:
        before = git("show", "e06cc88a:" + path).decode().splitlines()
        after = (REPOSITORY / path).read_text(encoding="utf-8").splitlines()
        added = {"      - name: Configure locked Cargo and numeric environment",
                 "        run: python tools/gates/controlled-cargo.py --install",
                 '      - "tools/gates/controlled-cargo.py"'}
        assert [line for line in before if line.strip()] == [line for line in after if line.strip() and line not in added]
    registry_before = json.loads(git("show", "e06cc88a:docs/module-registry.json"))
    registry_after = json.loads((REPOSITORY / "docs/module-registry.json").read_text(encoding="utf-8"))
    for before, after in zip(registry_before["modules"], registry_after["modules"], strict=True):
        if before["id"] == "rust.xiao-driver":
            expected = dict(before, tests=before["tests"] + ["tests/fixtures/docs/numeric"])
            assert after == expected
        else:
            assert before == after
    locks = []
    for path in ["core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock"]:
        before = tomllib.loads((ROOT / (path.replace("/", "-") + ".before.txt")).read_text(encoding="utf-8"))["package"]
        after = tomllib.loads((REPOSITORY / path).read_text(encoding="utf-8"))["package"]
        before = {(row["name"], row["version"]): row for row in before}
        after = {(row["name"], row["version"]): row for row in after}
        assert set(before).issubset(after) and {key[0] for key in set(after) - set(before)} == names
        assert all(before[key] == after[key] for key in before if key[0] != "xiao-runtime")
        raw = (REPOSITORY / path).read_bytes()
        locks.append({"path": path, "added_packages": sorted(set(after) - set(before)), "removed_packages": [],
                      "raw_sha256": hashlib.sha256(raw).hexdigest(), "utf8_lf_sha256": hashlib.sha256(raw.replace(b"\r\n", b"\n")).hexdigest()})
    write("lock-audit.json", {"locks": locks, "existing_packages_unchanged_except_runtime_edges": True})
    artifacts = []
    for line in (ROOT / "a1-native-exact-locked.log").read_text(encoding="utf-8").splitlines():
        match = re.search(r"artifact=(.+) result=success$", line)
        if match:
            path = Path(match.group(1))
            raw = path.read_bytes()
            artifacts.append({"path": str(path), "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()})
    assert len(artifacts) == 6
    for row in read("product-identity.json")["built_tools_before_b1_regression"]:
        assert hashlib.sha256(Path(row["path"]).read_bytes()).hexdigest() == row["sha256"]
    write("native-artifacts-final.json", {"artifacts": artifacts, "internal_only": True, "runtime_tools_unchanged_after_tests": True})
    available = []
    for name in report["effective_checks"]:
        path = ROOT / (name + ".json")
        if path.exists():
            row = read(path.name)
            if row.get("exit_code") == 0:
                assert row["source_unchanged"] and row["source_before"] == row["source_after"]
                available.append(row)
    assert len({row["source_before"]["raw_tree_sha256"] for row in available}) == 1
    ci = ROOT / report["ci_run_response"]
    ci_passed = ci.exists() and read(ci.name).get("conclusion") == "success"
    passed = len(available) == len(report["effective_checks"]) and ci_passed
    report.update(passed=passed, state="complete" if passed else "awaiting-final-records", recorded_utc=dt.datetime.now(dt.timezone.utc).isoformat())
    write("boundary.json", {"passed": True, "entry_commit": "e06cc88ab683a5984caf7d76df95fc34011d4f2f", "code_commit": code,
          "changed_paths": changed, "untracked_paths": untracked, "all_paths_checked": True,
          "product_inputs_unchanged_since_code_commit": True, "protected_paths": protected,
          "workflow_targets_limits_and_commands_unchanged": True, "registry_all_states_unchanged": True,
          "ci_change": "只追加任务级受控Cargo安装步骤及触发路径；数值Rust实现未因CI接线再修改",
          "b1_721_executed": False, "a2_through_a7_started": False, "historical_debts_closed": False})
    write("closeout.json", report)
    print(json.dumps({"passed": passed, "effective_checks": len(available), "required": len(report["effective_checks"]), "ci_passed": ci_passed}, ensure_ascii=False))


if __name__ == "__main__":
    main()
