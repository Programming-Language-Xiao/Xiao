"""汇总A0有效结果并生成逐文件清单；保留初轮和重试的原始结果。"""

import hashlib
import json
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent

    def read(name):
        return json.loads((root / name).read_text(encoding="utf-8"))

    initial = read("result.json")
    commands = read("commands.json")
    retry = read("repo-all-retry.json")
    fixed = read("repo-all-local-copy.json")
    boundary = read("boundary.json")
    assert [item["name"] for item in commands if not item["passed"]] == ["repo-all"]
    assert not initial["passed"] and retry["exit_code"] == 1 and not retry["passed"]
    assert fixed["passed"] and fixed["exit_code"] == 0
    assert initial["source_status_after"] == "" and initial["bound_files_unchanged"]
    tools_unchanged = all(hashlib.sha256(Path(item["path"]).read_bytes()).hexdigest() == item["sha256"] for item in read("built-tools.json"))
    assert tools_unchanged
    workspace = read("workspace-doc-gate.json") if (root / "workspace-doc-gate.json").exists() else {}
    workspace_passed = workspace.get("exit_code") == 0 and workspace.get("passed") is True
    passed = boundary["passed"] and workspace_passed
    report = {
        "source_commit": initial["source_commit"], "passed": passed,
        "a0_windows_source_baseline_passed": True, "workspace_doc_gate_passed": workspace_passed,
        "effective_repo_all": "repo-all-local-copy.json", "original_result_preserved": "result.json",
        "original_commands": len(commands), "original_failures": ["repo-all: exit 2, TypeScript resolution"],
        "first_retry": "exit 1, A0-LAYOUT-002; repo-all-retry.json",
        "resolution": "工作树内实体复制相同TypeScript5.9.3，132文件逐字一致；未改产品或门禁",
        "functional_tests": {"contract": 9, "resources": 17, "vm_exact": 1, "windows_native_exact": 1},
        "guards": {"passed": 28, "total": 28, "scope": "simulated-cargo-output"},
        "runtime_diagnostics_adapter_unchanged_after_tests": tools_unchanged,
        "windows": "passed" if passed else "source_passed_final_document_gate_pending",
        "linux": "未验证", "macos": "未验证", "a1_entry_open": passed,
        "a1_started": False, "b2_a_product_started": False, "b1_721_executed": False,
        "abi": "1.18", "codegen": 20, "active_intrinsic_max": 32, "cmath": "planned",
        "ci_used_as_local_evidence": False, "historical_19_11_22_22c_22d_debts_closed": False,
    }
    (root / "closeout.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    files = []
    for path in sorted(root.rglob("*")):
        if path.is_file() and path.name != "evidence.json":
            raw = path.read_bytes()
            files.append({"path": path.relative_to(root).as_posix(), "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()})
    manifest = {"schema_version": 1, "source_commit": initial["source_commit"],
                "scope": "A0证据目录全部文件的实际字节；清单自身不纳入自身摘要；不是全库文件数量",
                "file_count": len(files), "files": files}
    (root / "evidence.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"files": len(files), "passed": passed, "workspace_doc_gate_passed": workspace_passed}, ensure_ascii=False))


if __name__ == "__main__":
    main()
