"""B4-2B 自审补充的只读验收：修复前后反例、当前代码与解码catch证据。"""
import argparse
import importlib.util
import json
from pathlib import Path, PureWindowsPath
import re
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
ORIGINAL = ROOT.parent / "entry-20261113-b4-2b"
sys.path.insert(0, str(ORIGINAL))
import audit as original

spec = importlib.util.spec_from_file_location("review_record", ROOT / "record.py")
record = importlib.util.module_from_spec(spec)
spec.loader.exec_module(record)
REPOSITORY = record.REPOSITORY
ENTRY_PATH = "docs/DevDocs/assets/20o-b2-b/review-20261113-b4-2b"
TASK = PureWindowsPath(r"D:\project\Xiao\.b4-2b-review\20261113\final")
ALLOWED = {
    "core/rust/crates/xiao-driver/src/module_loader.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_vm.rs",
    "core/rust/crates/xiao-vm/src/semantics/README.md",
}
CARGO = ["cargo", "+1.96.0"]
LOCKED = ["--locked", "--manifest-path", "core/rust/Cargo.toml"]
RUNS = {
    "regressions": CARGO + ["test"] + LOCKED + ["-p", "xiao-driver", "--test", "20o_b2_b4_cmath_vm"] + original.ENTRY_ARGS + ["--", "--nocapture"],
    "clippy": CARGO + ["clippy"] + LOCKED + ["-p", "xiao-runtime", "-p", "xiao-vm", "-p", "xiao-driver", "--all-targets", "--", "-D", "warnings"],
    "fmt": CARGO + ["fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"],
    "docs": ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
}


def verify(root, repository):
    """以固定来源验证本次补充，旧门面与数值内核仍由原始归档证明。"""
    source = record.load(root / "final/source.json")
    environment = record.load(root / "final/environment.json")
    revision = source["revision"]
    record.require(revision == record.git(repository, "rev-parse", revision).decode().strip(), "自审来源不是完整Git身份")
    record.require({p: row["canonical_sha256"] for p, row in source["files"].items()} == record.fixed_files(repository, revision)
                   and source["tree_sha256"] == record.identity(source["files"]), "自审产品来源摘要不符")
    diff = record.git(repository, "diff", "--name-only", original.CODE, revision, "--", *record.PRODUCT_ROOTS).decode().splitlines()
    record.require(set(diff) == ALLOWED, "自审改动超出补测试、消息和说明的范围")
    record.require(PureWindowsPath(environment["task_root"]) == TASK, "自审任务根不符")
    for key in ["CARGO_TARGET_DIR", "XIAO_RUNTIME_LIBRARY", "XIAO_HOME", "TEMP", "TMP"]:
        original.safe_task_child(environment["overrides"][key], TASK)
    record.require(source["source_root"] == environment["source_root"]
                   and PureWindowsPath(environment["overrides"]["XIAO_STDLIB_ROOT"]) == PureWindowsPath(source["source_root"]) / "stdlib",
                   "自审源码或stdlib根不符")
    record.require(environment["overrides"]["XIAO_B1_F10"] == environment["overrides"]["XIAO_B2_A_PERF"] == "0"
                   and environment["removed_environment_prefixes"] == ["XIAO_", "RUST_BIGDECIMAL_"], "自审环境隔离不符")
    objects, referenced, logs, receipts = {}, set(), {}, {}
    for path in (root / "logs/sha256").rglob("*.log"):
        digest = record.sha(path.read_bytes())
        relative = path.relative_to(root).as_posix()
        record.require(relative == f"logs/sha256/{digest[:2]}/{digest}.log" and digest not in objects, "自审日志对象路径或摘要不符")
        objects[digest] = relative
    failed_row = record.load(root / "checked/regressions.json")
    failed_source = record.load(root / "checked/source.json")
    record.require({p: row["canonical_sha256"] for p, row in failed_source["files"].items()} == record.fixed_files(repository, failed_source["revision"]), "初轮提示失败来源不符")
    record.require(failed_row["exit_code"] == 101 and failed_row["source_before"] == failed_row["source_after"] == failed_source["tree_sha256"]
                   and failed_row["source_unchanged"] is True, "初轮提示失败回执不符")
    failed_digest = failed_row["log_sha256"]
    record.require(objects.get(failed_digest) == failed_row["log_ref"], "初轮提示失败日志引用不符")
    failed_log = (root / failed_row["log_ref"]).read_text(encoding="utf-8")
    record.require("test archive_rejects_planned_cmath_with_no_output_or_temporary_residue ... FAILED" in failed_log
                   and "X11-PROTOCOL-007" in failed_log and "2 passed; 1 failed; 0 ignored" in failed_log, "初轮提示失败身份不符")
    referenced.add(failed_digest)
    expected_reasons = {"redistributed_counts": "逐目标测试计数不符：revised/regressions",
                        "historical_state": "历史审计精确结果不符：b34",
                        "missing_negative_report": "缺少已验收的自反例报告"}
    for name in ["before", "after"]:
        result = record.load(root / (name + ".json"))
        record.require([r["case"] for r in result["cases"]] == ["baseline", *expected_reasons], "前后反例集合不符")
        for row in result["cases"]:
            digest = row["log_sha256"]
            record.require(objects.get(digest) == row["log_ref"] and row["read_only"] is True, "前后反例只读或日志身份不符")
            raw = (root / row["log_ref"]).read_text(encoding="utf-8").strip()
            record.require(raw == row["stdout"], "前后反例输出与回执不符")
            referenced.add(digest)
            if name == "before" or row["case"] == "baseline":
                record.require(row["exit_code"] == 0 and json.loads(raw)["passed"] is True, "修复前复现或修复后基线不符")
            else:
                record.require(row["exit_code"] == 1 and raw == expected_reasons[row["case"]], "修复后未按精确原因拒绝")
        if name == "after":
            record.require(result["expected_rejections_checked"] is True, "修复后未核对拒绝原因")
            frozen_auditor = record.git(repository, "show", revision + ":" + original.SELF_EVIDENCE_ROOT + "/audit.py")
            record.require(result["auditor_sha256"] == record.sha(frozen_auditor), "修复后反例不是固定来源审计器")
    docs_cache = {}
    for label, argv in RUNS.items():
        row = record.load(root / "final" / (label + ".json"))
        record.require(row["argv"] == argv and row["label"] == label and row["exit_code"] == 0,
                       "自审命令或退出码不符：" + label)
        record.require(row["pid"] > 0 and row["started_utc"] < row["finished_utc"] and row["cwd"] == source["source_root"], "自审进程或cwd不符")
        record.require(row["source_unchanged"] is True and row["source_before"] == row["source_after"] == source["tree_sha256"], "自审期间产品字节变化")
        record.require(row["environment_sha256"] == record.sha((root / "final/environment.json").read_bytes()), "自审环境摘要变化")
        commit = row["docs_commit"]
        record.require(commit == row["docs_commit_after"] and row["runner_sha256"] == record.sha(record.git(repository, "show", commit + ":" + ENTRY_PATH + "/record.py")), "自审执行器或HEAD变化")
        if commit not in docs_cache:
            docs_cache[commit] = record.docs_tree_from_commit(repository, commit)
        tree = docs_cache[commit]
        record.require(row["docs_tree_unchanged"] is True and row["docs_tree_before"] == row["docs_tree_after"] == row["docs_tree_sha256"]
                       and row["docs_tree_canonical_before"] == row["docs_tree_canonical_after"] == tree["canonical_sha256"]
                       and row["docs_tree_commit_raw_sha256"] == tree["raw_sha256"]
                       and row["docs_tree_commit_canonical_sha256"] == tree["canonical_sha256"], "自审DevDocs未绑定已提交字节")
        digest = row["log_sha256"]
        record.require(objects.get(digest) == row["log_ref"] and row["log_bytes"] == (root / row["log_ref"]).stat().st_size, "自审日志引用不符")
        referenced.add(digest)
        receipts[label] = row
        logs[label] = (root / row["log_ref"]).read_text(encoding="utf-8")
    record.require(referenced == set(objects), "存在没有回执引用的自审日志")
    expected_targets = {"20o_b2_b3_wrapper": (8, 0), "20o_b2_b4_cmath_boundary": (3, 0),
                        "20o_b2_b4_cmath_vm": (7, 0), "20o_b2_b4_frontend": (3, 0), "22c_vm_concurrency": (29, 0)}
    record.require(original.target_counts(logs["regressions"]) == expected_targets, "自审逐目标回归计数不符")
    for width in ["Leb128", "FixedU16"]:
        record.require(f"B4-2B-VM-DECODED-CATCH width={width} error=009 handler=ArithmeticError finally=2 releases=present result=passed" in logs["regressions"], "缺少解码catch或清理证据：" + width)
    record.require(len(re.findall(r"B3-WRAPPER-IMPORT .*executable=false result=planned-rejected", logs["regressions"])) == 30, "自审30条planned拒绝缺失")
    record.require("Compiling xiao-runtime " in logs["regressions"] and str(TASK / "target") in logs["regressions"].replace("/", "\\"), "缺少自审冷构建及任务路径")
    docs = json.loads(logs["docs"])
    record.require(docs["passed"] is True and all(d["code"] == "A0-COVERAGE-002" and d["severity"] == "warning" for d in docs["diagnostics"]), "自审终稿文档门禁失败或新增诊断")
    return {"passed": True, "source_commit": revision, "math_implementation": original.CODE,
            "b4_2_closed": True, "b4_closed": False, "language_available": False,
            "tests": 50, "decoded_catch_widths": ["Leb128", "FixedU16"], "repaired_audit_cases": 3,
            "runs": {k: {"pid": v["pid"], "log_sha256": v["log_sha256"], "docs_commit": v["docs_commit"]} for k, v in receipts.items()}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--collect", action="store_true")
    parser.add_argument("--live-runtime", action="store_true")
    args = parser.parse_args()
    old = original.audit(ORIGINAL, args.repository_root, live=args.live_runtime)
    result = verify(args.root, args.repository_root)
    if args.collect:
        record.require(not (args.root / "review.json").exists() and not (args.root / "evidence.json").exists(), "拒绝覆盖自审收口")
        record.write_json(args.root / "review.json", result)
        record.write_json(args.root / "evidence.json", {"files": original.inventory(args.root)})
    record.require(record.load(args.root / "review.json") == result, "自审结论与实际执行不符")
    evidence = record.load(args.root / "evidence.json")
    record.require(evidence["files"] == original.inventory(args.root), "自审字节清单不符")
    print(json.dumps({"passed": True, "files": len(evidence["files"]), "historical_audit": old["passed"],
                      "self_negative_checked": old["self_negative_checked"], "decoded_catch_widths": result["decoded_catch_widths"],
                      "b4_2_closed": True, "b4_closed": False, "language_available": False}, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        print(str(error))
        raise SystemExit(1)
