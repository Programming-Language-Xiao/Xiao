"""FIX.1只读审计，核验归因、精确路由、矩阵和B4-5重开状态。"""
from datetime import datetime
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from record import ROOT, REPOSITORY, inventory as base_inventory, docs_snapshot, fixed_files, git, identity, load, require, sha
from validation import validate

VARIANT_ROOTS = {"b34": "7d8b85e4d8c588b40ec76d641edc865481295032",
                 "b401": "c7a45e10b76b26915499717563761ed20fda6e6c",
                 "introduced": "4be30eafd36959bcdef08ac47987d9c07887cdcc",
                 "fixed": "be95d0d53cec6f07cb1719e426b4173867b2258e"}
FIXED_REQUIRED = {"runtime-build", "exact", "routing", "codegen-tests", "regressions", "native-matrix", "native-library",
                  "b1-vm", "b1-native", "workspace", "clippy", "fmt", "lock", "capture-toolchain-retry",
                  "capture-stages-final2", "capture-guards-final", "capture-pow-receipted", "audit-b4-3", "audit-b4-4", "audit-b4-5", "docs"}
REASONS = {"manifest": "FIX.1证据清单不符", "missing": "FIX.1已登记文件缺失", "readonly": "FIX.1审计改写证据",
           "public": "FIX.1收口/公共状态冒进", "state": "FIX.1运行状态不符"}
REPORTS = {"guards-report.json", "stage-reconciliation.json", "pow-review.json", "pow-review-unreceipted.json"}


def inventory(root):
    result = {}
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(), "FIX.1证据不能有链接")
        require("__pycache__" not in path.parts, "FIX.1证据不能有Python缓存")
        if path.is_file() and path.name != "evidence.json":
            raw = path.read_bytes()
            result[path.relative_to(root).as_posix()] = {"bytes": len(raw), "sha256": sha(raw)}
    return result


def tree(root):
    return {p.relative_to(root).as_posix(): sha(p.read_bytes()) if p.is_file() else "directory" for p in sorted(root.rglob("*"))}


def records_for(root, variant):
    result = {}
    for path in sorted((root / variant).glob("*.json")):
        if path.name in {"source.json", "environment.json"} | REPORTS:
            continue
        row = load(path)
        if "label" in row:
            result[row["label"]] = row
    return result


def run_text(root, row):
    raw = (root / row["log_ref"]).read_bytes()
    require(len(raw) == row["log_bytes"] and sha(raw) == row["log_sha256"]
            and row["log_ref"] == f"logs/sha256/{row['log_sha256'][:2]}/{row['log_sha256']}.log", REASONS["manifest"])
    return raw.decode("utf-8")


def check(root=ROOT, repository=REPOSITORY, final_docs=True, sealed=True):
    before = tree(root)
    require((root / ".gitattributes").read_bytes() == b"* -text -whitespace\n*.log -diff\n", REASONS["manifest"])
    if (root / "evidence.json").exists():
        for name in load(root / "evidence.json")["files"]:
            require((root / name).is_file(), REASONS["missing"])
        require(load(root / "evidence.json") == {"files": inventory(root)}, REASONS["manifest"])
    all_records, all_logs = {}, {}
    for variant, revision in VARIANT_ROOTS.items():
        source = load(root / variant / "source.json")
        require(source["revision"] == revision and source["sha256"] == identity(source["files"])
                and {p: row["canonical_sha256"] for p, row in source["files"].items()} == fixed_files(repository, revision), "FIX.1来源快照不符")
        environment = load(root / variant / "environment.json")
        labels = records_for(root, variant)
        expected = {"runtime-build", "exact"} if variant != "fixed" else FIXED_REQUIRED
        extras = set(labels) - expected
        require(expected <= set(labels) and not extras - {"capture-toolchain", "capture-guards-retry", "capture-pow-final",
            "capture-pow-final2", "capture-pow-retry", "capture-stages-final", "capture-stages-final2",
            "capture-stages-fixed-proof", "capture-stages-retry", "capture-stages-verified", "capture-toolchain-retry",
            "capture-pow-receipted", "audit-b4-3", "audit-b4-4", "audit-b4-5"}, "FIX.1运行集合不符：" + variant)
        for label, row in labels.items():
            require(row["label"] == label and row["cwd"] == environment["source_root"] and row["pid"] > 0
                    and datetime.fromisoformat(row["started_utc"]) < datetime.fromisoformat(row["finished_utc"]), REASONS["state"])
            require(row["environment_sha256"] == sha((root / variant / "environment.json").read_bytes())
                    and row["source_before"] == row["source_after"] == source["sha256"] and row["source_unchanged"]
                    and row["docs_unchanged"], REASONS["state"])
            commit = row["docs_commit"]
            fixed_doc_tree = docs_snapshot(repository, commit)
            require(row["docs_commit_tree"] == fixed_doc_tree and row["docs_before"] == row["docs_after"]
                    and row["docs_before"]["canonical_sha256"] == fixed_doc_tree["canonical_sha256"], "FIX.1文档身份不符")
            tool_commit = row.get("tools_commit")
            if tool_commit:
                require(row["runner_sha256"] == sha(git(repository, "show", tool_commit + ":docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-3-fix1/record.py")), "FIX.1采集器身份不符")
            all_records[variant, label] = row
            all_logs[variant, label] = run_text(root, row)
    result = validate(root, repository, {v: load(root / v / "source.json") for v in VARIANT_ROOTS}, all_records, all_logs)
    result["platforms_executed"] = ["Windows x86_64-pc-windows-msvc"]
    result["not_executed"] = ["Linux", "macOS", "remote CI", "performance", "old 721 F10"]
    result["stage"] = "B4-3-FIX1"
    result["passed"] = True
    result["b4_3_fix1_closed"] = True
    result["b4_5_reopened"] = True
    result["b4_5_closed"] = False
    result["b4_closed"] = False
    result["language_available"] = False
    result["public_cmath"] = "planned"
    result["versions"] = {"abi": "1.18", "codegen": 21, "stdlib_interface": 1}
    result["historical_audits"] = ["B4-3", "B4-4", "B4-5 evidence"]
    result["logs"] = {variant + "/" + label: {"exit_code": row["exit_code"], "pid": row["pid"], "log_sha256": row["log_sha256"]}
                      for (variant, label), row in sorted(all_records.items())}
    if sealed and (root / "closeout.json").exists():
        stored = load(root / "closeout.json")
        require(stored == result, "FIX.1收口报告与审计重算不符")
        review = load(root / "self-negative.json")
        require(review["passed"] is True and len(review["cases"]) == 6 and all(r["read_only"] for r in review["cases"]), "FIX.1反例报告不符")
    require(before == tree(root), REASONS["readonly"])
    return result


def self_negative(root=ROOT, repository=REPOSITORY):
    cases = []
    with tempfile.TemporaryDirectory(prefix="fix1-audit-negative-") as temp:
        for name in ["baseline", "manifest", "missing", "source", "workspace", "public"]:
            copy = Path(temp) / name
            shutil.copytree(root, copy)
            if name == "manifest":
                (copy / "fixed/pow-review.json").write_bytes(b"tampered")
            elif name == "missing":
                (copy / "fixed/guards-report.json").unlink()
            elif name == "source":
                row = load(copy / "fixed/source.json"); row["revision"] = INTRODUCED_COMMIT
                (copy / "fixed/source.json").write_text(json.dumps(row), encoding="utf-8")
            elif name == "workspace":
                row = load(copy / "fixed/workspace.json"); row["exit_code"] = 101
                (copy / "fixed/workspace.json").write_text(json.dumps(row), encoding="utf-8")
            elif name == "public":
                row = load(copy / "closeout.json"); row["language_available"] = True
                (copy / "closeout.json").write_text(json.dumps(row), encoding="utf-8")
            if name != "manifest" and (copy / "evidence.json").exists():
                (copy / "evidence.json").write_text(json.dumps({"files": inventory(copy)}, ensure_ascii=False), encoding="utf-8")
            before = tree(copy)
            run = subprocess.run([sys.executable, "-B", "-X", "utf8", str(Path(__file__).resolve()), "--root", str(copy),
                                  "--repository-root", str(repository), "--core-only"], capture_output=True, encoding="utf-8")
            after = tree(copy)
            reason = None if name == "baseline" else REASONS["manifest"] if name == "manifest" else REASONS["missing"] if name == "missing" else REASONS["source"] if name == "source" else REASONS["workspace"] if name == "workspace" else REASONS["public"]
            require(before == after and run.returncode == (0 if reason is None else 1)
                    and (not run.stderr if reason is None else run.stderr.strip() == reason), "FIX.1反例实际原因不符：" + name)
            cases.append({"case": name, "exit_code": run.returncode, "reason": reason, "stderr": run.stderr, "read_only": True})
    return {"passed": True, "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--self-negative", action="store_true")
    parser.add_argument("--core-only", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.self_negative:
        print(json.dumps(self_negative(args.root, args.repository_root), ensure_ascii=False)); return
    result = check(args.root, args.repository_root, sealed=not args.core_only)
    print(json.dumps({"passed": True, "fix1_closed": result["b4_3_fix1_closed"], "b4_5_reopened": True,
                      "b4_closed": False, "language_available": False, "product_commit": result["product_commit"]}, ensure_ascii=False))


if __name__ == "__main__":
    try: main()
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error), file=sys.stderr); raise SystemExit(1)
