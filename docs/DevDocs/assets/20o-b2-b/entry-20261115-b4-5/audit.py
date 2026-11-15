"""B4-5只读收口审计；固定Git对象验证产品、阶段证据、门禁和pow对账。"""
import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path, PureWindowsPath
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from record import ROOT, REPOSITORY, PRODUCT_ROOTS, canonical, fixed_files, git, identity, load, require, sha, write_json
from validation import validate

FINAL = "final"
SOURCE = ROOT / FINAL / "source.json"
ENVIRONMENT = ROOT / FINAL / "environment.json"
EXPECTED_LABELS = {
    "runtime-build", "workspace", "workspace-failure-exact", "clippy", "fmt", "lock", "toolchain",
    "native-library", "native-matrix", "b1-vm", "b1-native", "toolchain-retry", "stages-retry",
    "guards-retry", "pow-final", "pow-retry", "audit-b0-retry", "audit-b1-retry", "audit-b2-retry",
    "audit-b3-retry", "audit-b3-cabi-retry", "audit-b3-wrapper-retry", "audit-b4-0-1-retry",
    "audit-fix1-retry", "audit-b4-2b-retry", "audit-b4-3-retry", "audit-b4-4-retry", "docs-final", "docs-final-confirmed",
}
PRODUCT_BASELINE = "bb0b8196479e90d28d676f1bdefd04403e4ba6eb"
ALLOWED_STAGE_COMMITS = {
    "B4-1": "c7a45e10b76b26915499717563761ed20fda6e6c",
    "B4-2A/FIX1": "c46102115c63956466361d83243dea8ba3e4e74f",
    "B4-2B": "246bea46105ead280aed91e5e085c8f253a8913d",
    "B4-3": "e0d5993e7c8214f2277d1bb81f9b05357d9305e0",
    "B4-4": PRODUCT_BASELINE,
}
REASONS = {
    "manifest": "B4-5证据清单不符", "missing": "B4-5已登记文件缺失",
    "source": "B4-5固定产品身份不符", "environment": "B4-5隔离环境或路径不符",
    "state": "B4-5阶段或运行结果不符", "docs": "B4-5文档字节绑定不符",
    "pow": "B4-5pow对账不符", "public": "B4-5公共状态提前升级",
    "workspace": "B4-5workspace门禁阻塞未登记", "readonly": "B4-5审计改写证据",
}


def inventory(root):
    files = {}
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(), "证据不能含链接")
        require("__pycache__" not in path.parts, "证据不能含Python缓存")
        if path.is_file() and path.relative_to(root).as_posix() != "evidence.json":
            raw = path.read_bytes()
            files[path.relative_to(root).as_posix()] = {"bytes": len(raw), "sha256": sha(raw)}
    return files


def run_records(root):
    result = {}
    for path in sorted((root / FINAL).glob("*.json")):
        if path.name in {"source.json", "environment.json"}:
            continue
        row = load(path)
        if "label" not in row:
            continue
        result[row["label"]] = row
    extra = set(result) - EXPECTED_LABELS
    require(not extra or all(label.startswith("docs-final-") for label in extra), REASONS["state"])
    require(EXPECTED_LABELS.issubset(result), REASONS["state"])
    return result


def fixed_source(repository, revision):
    return fixed_files(repository, revision)


def check_product(repository, root=ROOT):
    source = load(root / FINAL / "source.json")
    revision = source["revision"]
    require(revision == git(repository, "rev-parse", revision).decode().strip()
            and source["sha256"] == identity(source["files"])
            and {p: row["canonical_sha256"] for p, row in source["files"].items()} == fixed_source(repository, revision), REASONS["source"])
    require(not git(repository, "diff", "--name-only", PRODUCT_BASELINE, revision, "--", "core", "stdlib").strip(), REASONS["source"])
    abi = git(repository, "show", revision + ":core/rust/crates/xiao-runtime-abi/src/lib.rs").decode()
    codegen = git(repository, "show", revision + ":core/rust/crates/xiao-codegen-llvm/src/lib.rs").decode()
    registry = json.loads(git(repository, "show", revision + ":docs/module-registry.json"))
    intrinsics = json.loads(git(repository, "show", revision + ":core/rust/crates/xiao-intrinsics/intrinsics.json"))["intrinsics"]
    require(re.search(r"ABI_MINOR_VERSION[^=]*=\s*18", abi)
            and re.search(r"CODEGEN_VERSION:\s*u32\s*=\s*21", codegen)
            and sorted(row["id"] for row in intrinsics if row["status"] == "active") == list(range(1, 49))
            and next(row for row in intrinsics if row["id"] == 100)["status"] == "tombstone"
            and next(row for row in registry["modules"] if row["id"] == "stdlib.cmath")["status"] == "planned", REASONS["public"])
    return source


def check_environment(environment):
    task = PureWindowsPath(environment["task_root"])
    expected = PureWindowsPath(r"D:\project\Xiao\.b4-5\20261115\final")
    require(task == expected and environment["source_root"] == r"D:\project\Xiao\Xiao"
            and environment["removed_environment_prefixes"] == ["XIAO_", "RUST_BIGDECIMAL_"]
            and environment["overrides"]["RUSTUP_TOOLCHAIN"] == "1.96.0"
            and environment["overrides"]["XIAO_B1_F10"] == environment["overrides"]["XIAO_B2_A_PERF"] == "0", REASONS["environment"])
    for key, relative in [("CARGO_TARGET_DIR", "target"), ("XIAO_HOME", "home"), ("TEMP", "temp"),
                          ("TMP", "temp"), ("XIAO_RUNTIME_LIBRARY", "target/release/xiao_runtime.lib")]:
        value = PureWindowsPath(environment["overrides"][key])
        require(value == task / relative, REASONS["environment"])


def check_record(label, row, source, environment, root=ROOT):
    require(row["label"] == label and row["cwd"] == environment["source_root"] and row["pid"] > 0
            and datetime.fromisoformat(row["started_utc"]) < datetime.fromisoformat(row["finished_utc"]), REASONS["state"])
    require(row["environment_sha256"] == sha((root / FINAL / "environment.json").read_bytes())
            and row["source_before"] == row["source_after"] == source["sha256"] and row["source_unchanged"] is True
            and row["docs_unchanged"] is True, REASONS["source"])
    raw = (root / row["log_ref"]).read_bytes()
    require(row["log_bytes"] == len(raw) and row["log_sha256"] == sha(raw)
            and row["log_ref"] == f"logs/sha256/{row['log_sha256'][:2]}/{row['log_sha256']}.log", REASONS["manifest"])
    return raw.decode("utf-8")


def check_pow(root=ROOT):
    require((root / FINAL / "pow-review.json").is_file(), REASONS["missing"])
    report = load(root / FINAL / "pow-review.json")
    require(report["passed"] is True and report["cases"] == 1075 and report["vm_cases"] == 6450
            and report["manifest_sha256"] == "53b751666f42814f94993ed606e3dfe650941b022392d7d04ea57d9f3d16443d"
            and report["summary"] == {"same": 1011, "value_or_error_boundary": 40, "value_bits": 24}
            and all(row["difference"] != "value_bits" or row["old_new_ulp"] == 1 for row in report["rows"]), REASONS["pow"])
    return {"cases": report["cases"], "summary": report["summary"], "vm_cases": report["vm_cases"],
            "report_sha256": sha((root / FINAL / "pow-review.json").read_bytes())}


def check_stages(root=ROOT):
    require((root / FINAL / "stage-reconciliation.json").is_file(), REASONS["missing"])
    report = load(root / FINAL / "stage-reconciliation.json")
    require(report["passed"] is True and {row["stage"] for row in report["stages"]} == set(ALLOWED_STAGE_COMMITS)
            and {row["stage"]: row["product_commit"] for row in report["stages"]} == ALLOWED_STAGE_COMMITS, REASONS["state"])
    return {"stages": len(report["stages"]), "report_sha256": sha((root / FINAL / "stage-reconciliation.json").read_bytes())}


def check_guards(root=ROOT):
    require((root / FINAL / "guards-report.json").is_file(), REASONS["missing"])
    report = load(root / FINAL / "guards-report.json")
    require(report["passed"] == report["total"] == 28 and all(row["Passed"] for row in report["cases"]), REASONS["state"])
    return {"passed": 28, "total": 28, "report_sha256": sha((root / FINAL / "guards-report.json").read_bytes()),
            "scope": report["scope"]}


def check(root=ROOT, repository=REPOSITORY, final_binding=True, sealed=True):
    before = {p.relative_to(root).as_posix(): sha(p.read_bytes()) if p.is_file() else "directory" for p in root.rglob("*")}
    require((root / "final/guards-report.json").is_file(), REASONS["missing"])
    if (root / "evidence.json").exists():
        for name in load(root / "evidence.json")["files"]:
            require((root / name).is_file(), REASONS["missing"])
        require(load(root / "evidence.json") == {"files": inventory(root)}, REASONS["manifest"])
    source = check_product(repository, root)
    environment = load(root / FINAL / "environment.json")
    check_environment(environment)
    if (root / "evidence.json").exists():
        for name in load(root / "evidence.json")["files"]:
            require((root / name).is_file(), REASONS["missing"])
        require(load(root / "evidence.json") == {"files": inventory(root)}, REASONS["manifest"])
    records = run_records(root)
    logs = {label: check_record(label, row, source, environment, root) for label, row in records.items()}
    expected_ok = {"runtime-build", "clippy", "fmt", "lock", "native-library", "native-matrix", "b1-vm", "b1-native",
                   "toolchain-retry", "stages-retry", "guards-retry", "pow-final", "audit-b0-retry",
                   "audit-b1-retry", "audit-b2-retry", "audit-b3-retry", "audit-b3-cabi-retry", "audit-b3-wrapper-retry",
                   "audit-b4-0-1-retry", "audit-fix1-retry", "audit-b4-2b-retry", "audit-b4-3-retry", "audit-b4-4-retry"}
    require(all(records[label]["exit_code"] == 0 for label in expected_ok if label in records), REASONS["state"])
    require(records["toolchain"]["exit_code"] == 2 and "capture.py" in logs["toolchain"]
            and records["pow-retry"]["exit_code"] == 1 and "唯一release依赖" in logs["pow-retry"], REASONS["state"])
    for docs_label in sorted(label for label in records if label.startswith("docs-final")):
        docs_result = json.loads(logs[docs_label])
        require(docs_result["passed"] is True and not any(row["severity"] == "error" for row in docs_result["diagnostics"]), REASONS["docs"])
    require(records["workspace"]["exit_code"] == 101 and records["workspace-failure-exact"]["exit_code"] == 101
            and "generic_value_copy_registers_weak_dependency_only_when_called" in logs["workspace"]
            and "left: false" in logs["workspace"] and "right: true" in logs["workspace"], REASONS["workspace"])
    require(logs["toolchain-retry"].startswith("{") and logs["stages-retry"].startswith("{")
            and logs["guards-retry"].startswith("{") and logs["pow-final"].startswith("B4-5-POW"), REASONS["state"])
    validate(root, repository, source, records, logs, final_binding)
    result = {
        "passed": False, "evidence_valid": True, "blocked": True, "stage": "B4-5", "b4_5_closed": False, "b4_closed": False,
        "language_available": False, "workspace_gates": "blocked_existing_n0_b_dynamic_assertion",
        "runtime_abi": "1.18", "codegen_version": 21, "stdlib_interface": 1, "cmath": "planned",
        "product_commit": source["revision"], "product_files": len(source["files"]),
        "pow": check_pow(root), "stages": check_stages(root), "guards": check_guards(root),
        "tests": {label: {"exit_code": row["exit_code"], "log_sha256": row["log_sha256"]} for label, row in sorted(records.items())},
        "platforms_executed": ["Windows x86_64-pc-windows-msvc"],
        "not_executed": ["Linux", "macOS", "remote CI", "performance", "old 721 F10", "workspace targets after first failure"],
        "blockers": [{"target": "n0_b_dynamic_native::generic_value_copy_registers_weak_dependency_only_when_called",
                      "observed": "false", "expected": "true", "source": "workspace.json", "ownership": "pre-existing product/test stage; B4-5 did not modify core or tests"}],
        "next_stage": "B4-5 blocked; return issue to owning native lowering stage before closeout",
    }
    if sealed and (root / "closeout.json").exists():
        stored = load(root / "closeout.json")
        require(stored.get("passed") is False and stored.get("language_available") is False and stored.get("b4_5_closed") is False, REASONS["public"])
        require(stored == result, REASONS["state"])
    after = {p.relative_to(root).as_posix(): sha(p.read_bytes()) if p.is_file() else "directory" for p in root.rglob("*")}
    require(before == after, REASONS["readonly"])
    return result


def negative_cases(root=ROOT, repository=REPOSITORY):
    cases = []
    with tempfile.TemporaryDirectory(prefix="b4-5-negative-") as temporary:
        for name in ["manifest", "missing", "source", "public"]:
            copy = Path(temporary) / name
            shutil.copytree(root, copy)
            reason = REASONS[name]
            if name == "manifest":
                (copy / "final" / "pow-review.json").write_bytes(b"tampered")
            elif name == "missing":
                (copy / "final" / "guards-report.json").unlink()
            elif name == "source":
                data = load(copy / "final/source.json"); data["revision"] = PRODUCT_BASELINE
                (copy / "final/source.json").write_text(json.dumps(data), encoding="utf-8")
            else:
                data = load(copy / "closeout.json"); data["language_available"] = True
                (copy / "closeout.json").write_text(json.dumps(data), encoding="utf-8")
            if name != "manifest":
                (copy / "evidence.json").write_text(json.dumps({"files": inventory(copy)}, ensure_ascii=False), encoding="utf-8")
            before = {p.relative_to(copy).as_posix(): sha(p.read_bytes()) for p in copy.rglob("*") if p.is_file()}
            run = subprocess.run([sys.executable, "-B", "-X", "utf8", str(Path(__file__).resolve()),
                                  "--root", str(copy), "--repository-root", str(repository)], capture_output=True, encoding="utf-8")
            after = {p.relative_to(copy).as_posix(): sha(p.read_bytes()) for p in copy.rglob("*") if p.is_file()}
            require(before == after and run.returncode == 1 and run.stderr.strip() == reason, reason)
            cases.append({"case": name, "exit_code": run.returncode, "reason": reason, "stderr": run.stderr, "read_only": True})
    return {"passed": True, "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--self-negative", action="store_true")
    args = parser.parse_args()
    if args.self_negative:
        print(json.dumps(negative_cases(), ensure_ascii=False))
        return
    result = check(args.root, args.repository_root)
    print(json.dumps({"passed": True, "evidence_valid": True, "stage_passed": result["passed"], "b4_5_closed": result["b4_5_closed"],
                      "workspace_gates": result["workspace_gates"], "product_commit": result["product_commit"]}, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
