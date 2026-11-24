"""只读审计 B3-0D 的冻结表、版本矩阵、C ABI、COFF 政策和交接。"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
PREFIX = ROOT.relative_to(REPO).as_posix()
FREEZE_COMMIT = "4b766782f2caf28b3896d7c361b8ddaeb6cbf20e"
START_COMMIT = "95e8bcca3b31c0fa58c3577b43625b5cec33b140"
PRODUCT_COMMIT = "e79fab3cdd3ea9d3a7eff31f529dec2498434114"
PRODUCT_TREE = "80be41eefc3983001cd3968db0ccceee541f698d"
PRIOR_C = "20fc409712fbc9ba905d05a460c3883f6de0e2ec"
STATIC_FILES = {
    ".gitattributes", "README.md", "audit.py", "baseline.json", "c-abi.json", "coff-policy.json",
    "contract-draft.md", "decisions.json", "freeze-table.json", "handoff.json", "negative-cases.json",
    "seal.py", "self_negative.py", "version-matrix.json",
}
GENERATED_FILES = {"closeout.json", "evidence.json", "doc-gates.json"}
OPTIONAL_GENERATED_FILES = {"self-negative-result.json"}
DOCS = {
    "docs/DevDocs/README.md",
    "docs/DevDocs/20o-official-library-program.md",
    "docs/DevDocs/20o-b3-host-libraries-plan.md",
    "docs/DevDocs/20o-b3-0-probe-freeze-plan.md",
    "docs/DevDocs/20o-b3-0d-version-intrinsic-abi-coff-plan.md",
}
PRODUCT_ROOTS = (
    "core", "stdlib", "tests", "cli", "tools", "resources", ".cargo", ".github",
    ".gitattributes", "package.json", "bun.lock", "tsconfig.json", "repository.manifest.json",
    "docs/UseDocs", "docs/module-registry.json",
)
REASONS = {
    "manifest-tampered": "evidence.json 字节或文件身份不符",
    "missing-attributes": "封存缺少文件：.gitattributes",
    "forged-status": "closeout.json 与实际冻结结论不符",
    "forged-decision": "冻结台账偏离固定 Git 对象",
    "forged-version": "版本矩阵偏离固定 Git 对象",
    "forged-coff": "COFF 政策偏离固定 Git 对象",
    "forged-negative-report": "自反例回执偏离固定原件",
}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def git(*args, repo=REPO):
    launcher = ["cmd", "/c", "git"] if os.name == "nt" else ["git"]
    return subprocess.check_output([*launcher, *args], cwd=repo)


def identity(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def blob(commit, path):
    return git("show", f"{commit}:{path}")


def inventory(root):
    return {
        path.relative_to(root).as_posix(): identity(path.read_bytes())
        for path in sorted(root.rglob("*"))
        if path.is_file() and path.name != "evidence.json"
    }


def paths_changed(left, right, *paths):
    raw = git("diff", "--name-only", "-z", left, right, "--", *paths)
    return {item for item in raw.decode("utf-8").split("\0") if item}


def verify_static_files(root):
    for name in sorted(STATIC_FILES):
        if name == "audit.py":
            continue
        expected = blob(FREEZE_COMMIT, f"{PREFIX}/{name}")
        require((root / name).read_bytes() == expected, {
            "decisions.json": REASONS["forged-decision"],
            "version-matrix.json": REASONS["forged-version"],
            "coff-policy.json": REASONS["forged-coff"],
        }.get(name, f"固定文件偏离 Git 对象：{name}"))


def verify_baseline(root):
    baseline = json.loads((root / "baseline.json").read_bytes())
    product = baseline["product"]
    require(product["commit"] == PRODUCT_COMMIT and git("show", "-s", "--format=%T", PRODUCT_COMMIT).strip().decode() == PRODUCT_TREE,
            "固定产品树身份不符")
    for path, expected in product["locks"].items():
        require(identity(blob(PRODUCT_COMMIT, path)) == expected, "固定 Cargo.lock 身份不符")
    require(paths_changed(START_COMMIT, FREEZE_COMMIT, *PRODUCT_ROOTS) == set(), "B3-0D 冻结提交改动了产品树")
    require(baseline["prior"]["b3_0c_closeout"]["commit"] == PRIOR_C and baseline["inherited"] == {
        "b1_guards": {"actual": 4, "derived": 24, "passed": 28},
        "platforms": ["linux", "macos", "windows"],
        "old_721_executed": False,
        "registry_verified": False,
        "b3_product_started": False,
    }, "B3-0C 交接基线不符")
    return baseline


def verify_decisions(root):
    record = json.loads((root / "decisions.json").read_bytes())
    rows = record["decisions"]
    require(record["stage"] == "20O-B3-0D" and set(rows) == {
        "B3-0D-ID", "B3-0D-ENV", "B3-0D-VER", "B3-0D-CABI", "B3-0D-PROTO", "B3-0D-COFF"
    }, "B3-0D 裁定台账不完整")
    ids = [tuple(row["question_item_id"]) for row in rows.values()]
    require(all(row["status"] == "approved" and row["answer"] == "批准候选" and row["scope"] for row in rows.values())
            and all(item[0] == "request_user_input_async" and item[1].startswith("call_") and type(item[2]) is int for item in ids)
            and len(set(ids)) == 6, "B3-0D 裁定来源不完整")
    return len(rows)


def verify_freeze_table(root):
    table = json.loads((root / "freeze-table.json").read_bytes())
    rows = table["intrinsics"]
    require([row["id"] for row in rows] == list(range(49, 64)), "intrinsic 编号不是 49～63 连续区间")
    require(len({row["c_symbol"] for row in rows}) == 15 and all(row["c_symbol"].startswith("xiao_runtime_") for row in rows),
            "intrinsic C 符号不唯一")
    require(table["preserved"]["intrinsic_ids"] == list(range(1, 49)) and table["preserved"]["tombstone"] == 100,
            "旧 intrinsic 或墓碑身份改变")
    capabilities = table["capabilities"]
    require(capabilities["bits"]["environment"] == 7 and capabilities["known_mask"] == "0xFF"
            and capabilities["legacy_default_mask"] == "0x7F" and capabilities["log_required_mask"] == "0x22",
            "能力位或默认掩码不符")
    require(table["product_status"] == {"implemented": False, "registry_verified": False, "tutorials": "planned", "windows_product_coff": "pending"},
            "冻结表越过产品边界")
    return {"intrinsics": len(rows), "ids": [row["id"] for row in rows], "known_mask": capabilities["known_mask"]}


def verify_versions(root):
    matrix = json.loads((root / "version-matrix.json").read_bytes())
    require(matrix["current_product"] == {"runtime_abi": "1.19", "generator": 22, "stdlib_interface": 1}
            and matrix["b3_target"] == {"runtime_abi": "1.20", "generator": 23, "stdlib_interface": 1}, "版本目标不符")
    require(matrix["formats"] == {"xiaoc": "1.0", "xar": "1.0", "index_schema": "1.0", "ir": "1", "native_metadata": "1", "protocol": "1", "core": "1"},
            "格式版本不符")
    require(matrix["compatibility"]["old_1_19_22_artifacts"] == "execute_before_reject"
            and matrix["compatibility"]["regeneration"] == "from_source"
            and matrix["compatibility"]["arguments"] == "execution_input_only_not_artifact_or_cache_identity", "旧产物兼容边界不符")
    return matrix["b3_target"]


def verify_c_abi(root):
    record = json.loads((root / "c-abi.json").read_bytes())
    symbols = record["symbols"]
    require(len(symbols) == 15 and len({row["name"] for row in symbols}) == 15
            and all(row["parameters"][-1] == "u64 capabilities" for row in symbols), "C ABI 符号表不完整")
    require(record["status_codes"] == {"0": "success", "1": "null_or_shape_error", "2": "unknown_tag_or_invalid_argument", "3": "bad_handle", "6": "recoverable_runtime_error", "8": "fatal"},
            "C ABI 状态码不符")
    require(record["output_ownership"].startswith("out XiaoValue slots are initialized"), "C ABI 输出所有权不符")
    return len(symbols)


def verify_coff(root):
    policy = json.loads((root / "coff-policy.json").read_bytes())
    require(policy["policy"] == "complete_required" and policy["status"] == "pending"
            and policy["pass"] == {"verification": "Complete", "declared_equals_observed": True, "unclassified_runtime_symbols": 0, "artifact_hash_bound": True}
            and "UnverifiedCoffExports" in policy["rejected_observation"], "Windows COFF 政策不符")
    return {"policy": policy["policy"], "status": policy["status"]}


def verify_handoff(root):
    handoff = json.loads((root / "handoff.json").read_bytes())
    require(handoff["next"] == "20O-B3-0E" and handoff["b3_0d_passed"] is True
            and handoff["b3_0e_entry_ready"] is True and handoff["b3_product_started"] is False
            and handoff["registry_verified"] is False and handoff["windows_product_coff"]["status"] == "pending"
            and "restricted_fs_mode" in handoff["deferred"], "B3-0E 交接边界不符")
    return handoff


def verify_manifest(root):
    manifest = json.loads((root / "evidence.json").read_bytes())
    expected = inventory(root)
    require(manifest == {"schema": 1, "files": expected}, REASONS["manifest-tampered"])
    return len(expected)


def verify_negative_report(root):
    path = root / "self-negative-result.json"
    if not path.exists():
        return None
    payload = json.loads(path.read_bytes())
    expected = json.loads((root / "negative-cases.json").read_bytes())["cases"]
    require(payload["mode"] == "sealed" and [row["case"] for row in payload["cases"]] == [row["name"] for row in expected], REASONS["forged-negative-report"])
    for row, expected_case in zip(payload["cases"], expected):
        require(row["read_only"] is True and row["before"] == row["after"] and row["exit"] == int(expected_case["expected"] != "pass"), REASONS["forged-negative-report"])
    return len(payload["cases"])


def verify_doc_gates(root):
    report = json.loads((root / "doc-gates.json").read_bytes())
    rows = report["checks"]
    require([row["name"] for row in rows] == ["check:docs", "check:usedocs", "check:layout"]
            and all(row["exit"] == 0 for row in rows)
            and report["passed"] is True, "文档门禁未通过或回执不完整")
    return {"checks": [{"name": row["name"], "exit": row["exit"], "stdout_sha256": row["stdout_sha256"], "stderr_sha256": row["stderr_sha256"]} for row in rows], "passed": True}


def derive(root):
    require(git("cat-file", "-e", f"{FREEZE_COMMIT}^{{commit}}"), "冻结提交未绑定")
    verify_static_files(root)
    baseline = verify_baseline(root)
    decisions = verify_decisions(root)
    freeze = verify_freeze_table(root)
    target = verify_versions(root)
    cabi = verify_c_abi(root)
    coff = verify_coff(root)
    handoff = verify_handoff(root)
    neg = verify_negative_report(root)
    gates = verify_doc_gates(root)
    return {
        "scope": "20O-B3-0D",
        "freeze_commit": FREEZE_COMMIT,
        "product_commit": PRODUCT_COMMIT,
        "prior_b3_0c": PRIOR_C,
        "b3_0d0_decisions_recorded": decisions == 6,
        "b3_0d1_intrinsics_capabilities_frozen": freeze["intrinsics"] == 15,
        "b3_0d2_versions_artifacts_frozen": target == {"runtime_abi": "1.20", "generator": 23, "stdlib_interface": 1},
        "b3_0d3_c_abi_protocol_frozen": cabi == 15,
        "b3_0d4_windows_coff_policy_frozen": coff == {"policy": "complete_required", "status": "pending"},
        "b3_0d5_evidence_audit_passed": True,
        "b3_0d_passed": True,
        "b3_0e_entry_ready": True,
        "b3_0_passed": False,
        "b3_product_started": False,
        "registry_verified": False,
        "old_721_executed": False,
        "windows_product_coff_status": "pending",
        "doc_gates": gates,
        "doc_gates_passed": True,
        "baseline": {"product": baseline["product"], "b1_guards": baseline["inherited"]["b1_guards"], "platforms": baseline["inherited"]["platforms"]},
        "handoff": handoff,
        "negative_cases": neg,
    }


def audit(root):
    files = set(inventory(root))
    expected = STATIC_FILES | GENERATED_FILES
    missing = expected - files
    require(not missing, f"封存缺少文件：{', '.join(sorted(missing))}")
    require(files == expected or files == expected | OPTIONAL_GENERATED_FILES, "封存目录包含未登记文件")
    verify_manifest(root)
    result = derive(root)
    require((root / "closeout.json").read_bytes() == encoded(result), REASONS["forged-status"])
    return result | {"audit_files": len(files)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, default=ROOT)
    args = parser.parse_args()
    try:
        print(encoded(audit(args.evidence_root.resolve())).decode("utf-8"), end="")
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"B3-0D 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
