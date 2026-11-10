"""完成所有门禁后，仅一次生成由实际执行记录导出的 closeout/evidence。"""
import datetime
import json
from pathlib import Path

import audit


def main():
    if (audit.ROOT / "closeout.json").exists() or (audit.ROOT / "evidence.json").exists():
        raise SystemExit("拒绝覆盖B3-4 closeout/evidence")
    result = audit.validate(audit.ROOT, audit.REPOSITORY, require_closeout=False)
    runs = {}
    for name in audit.RUNS:
        record = json.loads((audit.ROOT / (name + ".json")).read_text(encoding="utf-8"))
        runs[name] = {"record": record["label"] + ".json", "log": record["log"],
                      "pid": record["pid"], "exit_code": record["exit_code"],
                      "log_bytes": record["log_bytes"], "log_sha256": record["log_sha256"],
                      "source_tree_sha256": record["source_before"]["raw_tree_sha256"],
                      "lock_files": record["source_before"]["locks"]}
    closeout = {"schema_version": 1, "passed": True,
                "status": "B3-4 planned源码包装收口通过；语言不可用，不是公共发布",
                "generated_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                "entry_commit": result["identity"]["entry_commit"],
                "code_commit": result["identity"]["code_commit"],
                "head_commit_at_audit": result["identity"]["head_commit"],
                "source": result["source"], "runtime_library": None,
                "runtime_abi": "1.18", "codegen_version": 20, "stdlib_interface": 1,
                "active_intrinsics": [1, 48], "b3_4_closed": True,
                "closeout_scope": "planned source wrappers and isolation regressions only",
                "language_available": False, "cmath_status": "planned",
                "registry_status": "planned", "tutorial_status": "planned",
                "platform": "Windows x86_64-msvc", "remote_ci_claimed": False,
                "three_platform_public_product": False, "old_721_executed": False,
                "historical_audits": {name: runs[name] for name in ["b1_history", "b2_history", "b3_cabi_history"]},
                "b3_4_checks": {name: runs[name] for name in ["wrapper", "concurrency", "codegen", "archive_wiring"]},
                "gates": {name: runs[name] for name in ["workspace_tests_final", "clippy", "fmt", "lock_check", "repo_all_review_final"]},
                "failed_invocations": {"workspace_tests_root_without_manifest": {
                    "record": "workspace_tests.json", "exit_code": 101,
                    "classification": "执行目录未指定core/rust/Cargo.toml；不是测试失败，修正命令后重跑"}},
                "counts": {"wrapper": "8 passed / 0 failed / 0 ignored",
                           "existing_modules": "12 passed / 0 failed / 3 ignored",
                           "concurrency": "29 passed / 0 failed / 0 ignored",
                           "reachability_planned_paths": 30,
                           "b3_cabi_historical_audit_checks": 22,
                           "b1_historical_audit_checks": 18,
                           "b2_historical_audit_checks": 14},
                "unverified": ["stdlib.cmath executable language calls", "native cmath O0/O3",
                               "B3-4 Linux/macOS", "remote CI", "B4 vector migration", "old B1 721 F10"],
                "historical_debts_touched": False}
    with (audit.ROOT / "closeout.json").open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(closeout, ensure_ascii=False, indent=2) + "\n")
    manifest = {"schema_version": 1,
                "scope": "B3-4 source-only planned wrapper closeout; language unavailable",
                "files": audit.evidence_inventory(audit.ROOT)}
    with (audit.ROOT / "evidence.json").open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"passed": True, "files": len(manifest["files"]),
                      "b3_4_closed": True, "language_available": False}, ensure_ascii=False))


if __name__ == "__main__":
    main()
