"""B4-5重开只读审计：固定Git对象、运行回执与明确拒绝原因。"""
import argparse
from datetime import datetime
import json
from pathlib import Path, PureWindowsPath
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from record import ROOT, REPOSITORY, docs_snapshot, fixed_files, git, identity, load, require, sha
from suite import COMMANDS, SOURCE_COMMIT, PRODUCT_COMMIT
from validation import validate, product_at

REASONS = {"manifest": "B4-5证据清单不符", "missing": "B4-5已登记文件缺失",
           "source": "B4-5固定产品身份不符", "stage": "B4-5阶段身份不符",
           "public": "B4-5公共状态提前升级", "environment": "B4-5隔离环境不符",
           "workspace": "B4-5门禁未通过：workspace", "readonly": "B4-5审计改写证据"}
CASES = ["baseline", "manifest", "missing", "source", "stage", "environment", "workspace", "public"]


def inventory(root):
    result = {}
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(), "B4-5证据不能含链接")
        require("__pycache__" not in path.parts, "B4-5证据不能含Python缓存")
        if path.is_file() and path.relative_to(root).as_posix() != "evidence.json":
            raw = path.read_bytes()
            result[path.relative_to(root).as_posix()] = {"bytes": len(raw), "sha256": sha(raw)}
    return result


def tree_digest(root):
    tree = {p.relative_to(root).as_posix(): sha(p.read_bytes()) if p.is_file() else "directory"
            for p in sorted(root.rglob("*"))}
    return sha(json.dumps(tree, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8"))


def source_check(root, repository):
    source = load(root / "final/source.json")
    require(source["revision"] == SOURCE_COMMIT and source["sha256"] == identity(source["files"])
            and {path: row["canonical_sha256"] for path, row in source["files"].items()} == product_at(repository, SOURCE_COMMIT)
            and product_at(repository, SOURCE_COMMIT) == product_at(repository, PRODUCT_COMMIT), REASONS["source"])
    abi = git(repository, "show", SOURCE_COMMIT + ":core/rust/crates/xiao-runtime-abi/src/lib.rs").decode()
    codegen = git(repository, "show", SOURCE_COMMIT + ":core/rust/crates/xiao-codegen-llvm/src/lib.rs").decode()
    intrinsics = json.loads(git(repository, "show", SOURCE_COMMIT + ":core/rust/crates/xiao-intrinsics/intrinsics.json"))["intrinsics"]
    registry = json.loads(git(repository, "show", SOURCE_COMMIT + ":docs/module-registry.json"))
    require(re.search(r"ABI_MINOR_VERSION[^=]*=\s*18", abi) and re.search(r"CODEGEN_VERSION:\s*u32\s*=\s*21", codegen)
            and sorted(row["id"] for row in intrinsics if row["status"] == "active") == list(range(1, 49))
            and next(row for row in intrinsics if row["id"] == 100)["status"] == "tombstone"
            and next(row for row in registry["modules"] if row["id"] == "stdlib.cmath")["status"] == "planned",
            REASONS["public"])
    return source


def environment_check(environment):
    task = PureWindowsPath(environment["task_root"])
    require(task == PureWindowsPath(r"D:\project\Xiao\.b4-5\20261116\rerun")
            and environment["source_root"] == r"D:\project\Xiao\Xiao"
            and environment["removed_environment_prefixes"] == ["XIAO_", "RUST_BIGDECIMAL_"], REASONS["environment"])
    for key, suffix in [("CARGO_TARGET_DIR", "target"), ("XIAO_HOME", "home"), ("TEMP", "temp"),
                        ("TMP", "temp"), ("XIAO_RUNTIME_LIBRARY", "target/release/xiao_runtime.lib")]:
        require(PureWindowsPath(environment["overrides"][key]) == task / suffix, REASONS["environment"])
    require(environment["overrides"]["XIAO_B1_F10"] == environment["overrides"]["XIAO_B2_A_PERF"] == "0"
            and environment["overrides"]["RUSTUP_TOOLCHAIN"] == "1.96.0"
            and PureWindowsPath(environment["overrides"]["XIAO_STDLIB_ROOT"]) == PureWindowsPath(environment["source_root"]) / "stdlib",
            REASONS["environment"])


def records_check(root, source, environment, final_binding):
    records, logs = {}, {}
    for path in sorted((root / "final").glob("*.json")):
        row = load(path)
        if "label" not in row:
            continue
        label = row["label"]
        require(path.name == label + ".json" and label not in records and label in COMMANDS, "B4-5运行集合不符")
        records[label] = row
    expected = set(COMMANDS) if final_binding else set(COMMANDS) - {"docs-final"}
    require(expected <= set(records), "B4-5运行集合不完整")
    for label, row in records.items():
        require(row["variant"] == "final" and row["cwd"] == environment["source_root"] and row["pid"] > 0
                and datetime.fromisoformat(row["started_utc"]) < datetime.fromisoformat(row["finished_utc"])
                and row["environment_sha256"] == sha((root / "final/environment.json").read_bytes())
                and row["source_before"] == row["source_after"] == source["sha256"]
                and row["source_unchanged"] is True and row["docs_unchanged"] is True, "B4-5运行身份不符")
        require(row["log_ref"] == f"logs/sha256/{row['log_sha256'][:2]}/{row['log_sha256']}.log", REASONS["manifest"])
        raw = (root / row["log_ref"]).read_bytes()
        require(len(raw) == row["log_bytes"] and sha(raw) == row["log_sha256"], REASONS["manifest"])
        logs[label] = raw.decode("utf-8")
    require({row["log_ref"] for row in records.values()} == {p.relative_to(root).as_posix() for p in (root / "logs").rglob("*") if p.is_file()},
            "B4-5存在未引用日志对象")
    return records, logs


def handoff(result):
    return {"passed": True, "stage": "B4-5D", "product_commit": PRODUCT_COMMIT, "source_commit": SOURCE_COMMIT,
            "runtime_sha256": result["runtime"]["sha256"], "runtime_abi": "1.18", "codegen_version": 21,
            "stdlib_interface": 1, "language_available": True, "public_cmath": "planned",
            "b5_started": False,
            "b5_required": ["ABI 1.19 only; keep codegen 21", "old/new exact ABI rejection before execution",
                            "codegen 20 to 21 and ABI 1.18 to 1.19 cache miss/rebuild matrices",
                            "retain existing C symbols/layout/calling conventions", "rerun B1 20 entries on new snapshot"],
            "b6_required": ["resource and performance evidence"],
            "b2_c_required": ["full 721 F10 on new snapshot", "three-platform public artifacts", "tutorial/registry closeout"],
            "retained_boundaries": ["native selected imports only; module-value calls keep B1 limitation",
                                    "FIX.1 integer numeric_range/dynamic weak, Float pow/shared kernel, sfloat llvm.pow.f32",
                                    "22C/22D and 19/11/22 debts remain independent"],
            "not_executed": ["Linux", "macOS", "remote CI", "performance", "old 721 F10"]}


def check(root=ROOT, repository=REPOSITORY, final_binding=True, sealed=True):
    before = tree_digest(root)
    require((root / ".gitattributes").read_bytes() == b"* -text -whitespace\n*.log -diff\n", REASONS["manifest"])
    if (root / "evidence.json").exists():
        manifest = load(root / "evidence.json")
        for name in manifest["files"]:
            require((root / name).is_file(), REASONS["missing"])
        require(manifest == {"files": inventory(root)}, REASONS["manifest"])
    source = source_check(root, repository)
    environment = load(root / "final/environment.json")
    environment_check(environment)
    records, logs = records_check(root, source, environment, final_binding)
    details = validate(root, repository, source, environment, records, logs, final_binding)
    result = {**details, "passed": True, "evidence_valid": True, "blocked": False, "stage": "B4-5",
              "b4_5_closed": True, "b4_closed": True, "language_available": True,
              "runtime_abi": "1.18", "codegen_version": 21, "stdlib_interface": 1, "cmath": "planned",
              "product_commit": PRODUCT_COMMIT, "source_commit": SOURCE_COMMIT,
              "product_files": len(source["files"]), "product_tree_sha256": identity(product_at(repository, PRODUCT_COMMIT)),
              "platforms_executed": ["Windows x86_64-pc-windows-msvc"],
              "not_executed": ["Linux", "macOS", "remote CI", "performance", "old 721 F10"],
              "blockers": [], "next_stage": "B5 implementation; not started",
              "tests": {label: {"exit_code": row["exit_code"], "pid": row["pid"], "log_sha256": row["log_sha256"],
                                "docs_commit": row["docs_commit"]} for label, row in sorted(records.items())}}
    if sealed:
        require(all((root / name).is_file() for name in ["closeout.json", "self-negative.json", "evidence.json", "b5-handoff.json"]),
                REASONS["missing"])
        stored = load(root / "closeout.json")
        require(stored.get("cmath") == "planned" and stored.get("runtime_abi") == "1.18"
                and stored.get("codegen_version") == 21, REASONS["public"])
        require(stored == result and load(root / "b5-handoff.json") == handoff(result), "B4-5收口/交接与重算不符")
        review = load(root / "self-negative.json")
        require(review["passed"] is True and len(review["cases"]) == len(CASES)
                and [row["case"] for row in review["cases"]] == CASES, "B4-5反例报告不符")
        for row in review["cases"]:
            reason = None if row["case"] == "baseline" else REASONS[row["case"]]
            require(row["reason"] == reason and row["exit_code"] == (0 if reason is None else 1)
                    and row["stderr"].strip() == (reason or "") and row["read_only"] is True
                    and re.fullmatch("[0-9a-f]{64}", row["before_sha256"]) and row["before_sha256"] == row["after_sha256"],
                    "B4-5反例原因/只读摘要不符")
    elif (root / "closeout.json").exists():
        require(load(root / "closeout.json").get("cmath") == "planned", REASONS["public"])
    require(before == tree_digest(root), REASONS["readonly"])
    return result


def negative_cases(root=ROOT, repository=REPOSITORY):
    cases = []
    with tempfile.TemporaryDirectory(prefix="b4-5-reopen-negative-") as temporary:
        for name in CASES:
            copy = Path(temporary) / name
            shutil.copytree(root, copy)
            if name == "manifest":
                (copy / "final/pow-review.json").write_bytes(b"tampered")
            elif name == "missing":
                (copy / "final/guards-report.json").unlink()
            elif name in {"source", "stage", "environment", "workspace", "public"}:
                target = {"source": "final/source.json", "stage": "final/stage-reconciliation.json",
                          "environment": "final/environment.json", "workspace": "final/workspace.json", "public": "closeout.json"}[name]
                row = load(copy / target)
                if name == "source":
                    row["revision"] = "bb0b8196479e90d28d676f1bdefd04403e4ba6eb"
                elif name == "stage":
                    row["stages"][0]["product_commit"] = PRODUCT_COMMIT
                elif name == "environment":
                    row["overrides"]["XIAO_RUNTIME_LIBRARY"] = r"D:\old\xiao_runtime.lib"
                elif name == "workspace":
                    row["exit_code"] = 101
                else:
                    row["cmath"] = "verified"
                (copy / target).write_text(json.dumps(row, ensure_ascii=False), encoding="utf-8")
            if name not in {"manifest", "missing"}:
                (copy / "evidence.json").write_text(json.dumps({"files": inventory(copy)}, ensure_ascii=False), encoding="utf-8")
            before = tree_digest(copy)
            run = subprocess.run([sys.executable, "-B", "-X", "utf8", str(Path(__file__).resolve()),
                                  "--root", str(copy), "--repository-root", str(repository), "--core-only"],
                                 capture_output=True, encoding="utf-8")
            after = tree_digest(copy)
            reason = None if name == "baseline" else REASONS[name]
            require(before == after and run.returncode == (0 if reason is None else 1)
                    and run.stderr.strip() == (reason or ""), "B4-5反例原因/只读不符：" + name + "；" + run.stderr)
            cases.append({"case": name, "exit_code": run.returncode, "reason": reason, "stderr": run.stderr,
                          "read_only": before == after, "before_sha256": before, "after_sha256": after})
    return {"passed": True, "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--core-only", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--verify-runs", action="store_true", help="定稿前只读核对全部功能回执")
    parser.add_argument("--self-negative", action="store_true")
    args = parser.parse_args()
    if args.self_negative:
        print(json.dumps(negative_cases(args.root, args.repository_root), ensure_ascii=False))
        return
    result = check(args.root, args.repository_root, final_binding=not (args.core_only or args.verify_runs),
                   sealed=not (args.core_only or args.verify_runs))
    print(json.dumps({key: result[key] for key in ["passed", "b4_5_closed", "b4_closed", "language_available",
                                                 "product_commit", "source_commit"]}, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
