"""只读审计 B3-0D 的冻结表、版本矩阵、C ABI、COFF 政策和交接。"""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import zipfile

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
PREFIX = ROOT.relative_to(REPO).as_posix()
FREEZE_COMMIT = "5096853d3d0cdf2edfa213db7c129c452f1abe42"
START_COMMIT = "95e8bcca3b31c0fa58c3577b43625b5cec33b140"
PRODUCT_COMMIT = "e79fab3cdd3ea9d3a7eff31f529dec2498434114"
PRODUCT_TREE = "80be41eefc3983001cd3968db0ccceee541f698d"
PRIOR_C = "20fc409712fbc9ba905d05a460c3883f6de0e2ec"
COFF_ARCHIVE_COMMIT = "8c8c5fb19441346c607add19bf1b381160104612"
COFF_ARCHIVE_PATH = "docs/DevDocs/assets/20o-b3/entry-20261123-b3-0b/cli-import.zip"
COFF_ARCHIVE_SHA256 = "43524587e36d4d1c5d1b83fb8143b46c261cbd8c2f39f36ea55871cc4ebb19e6"
COFF_CRITERIA_REVISION = "coff-criteria-revision.json"
PLAN_PATH = "docs/DevDocs/20o-b3-0d-version-intrinsic-abi-coff-plan.md"
STATIC_FILES = {
    ".gitattributes", "README.md", "audit.py", "c-abi.json", "coff-policy.json", "coff-probe.json",
    "baseline.json", "coff_probe.py", COFF_CRITERIA_REVISION,
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
    PLAN_PATH, f"{PREFIX}/README.md",
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
    "forged-probe-observation": "COFF 探测回执偏离固定 Git 对象",
    "forged-coff-criteria": "COFF 收口条件修订偏离固定 Git 对象",
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


def normalized_audit_sha256(raw):
    normalized = re.sub(rb'FREEZE_COMMIT = "[a-f0-9_]+"', b'FREEZE_COMMIT = "<fixed-object>"', raw, count=1)
    return hashlib.sha256(normalized).hexdigest()


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
            "coff-probe.json": REASONS["forged-probe-observation"],
            COFF_CRITERIA_REVISION: REASONS["forged-coff-criteria"],
        }.get(name, f"固定文件偏离 Git 对象：{name}"))


def verify_baseline(root):
    baseline = json.loads((root / "baseline.json").read_bytes())
    require(baseline["entry_commit"] == START_COMMIT, "B3-0D 固定入口提交不符")
    product = baseline["product"]
    require(product["commit"] == PRODUCT_COMMIT and git("show", "-s", "--format=%T", PRODUCT_COMMIT).strip().decode() == PRODUCT_TREE,
            "固定产品树身份不符")
    for path, expected in product["locks"].items():
        require(identity(blob(PRODUCT_COMMIT, path)) == expected, "固定 Cargo.lock 身份不符")
    no_product_change = paths_changed(START_COMMIT, FREEZE_COMMIT, *PRODUCT_ROOTS) == set()
    require(no_product_change, "B3-0D 冻结提交改动了产品树")
    require(baseline["prior"]["b3_0c_closeout"]["commit"] == PRIOR_C and baseline["inherited"] == {
        "b1_guards": {"actual": 4, "derived": 24, "passed": 28},
        "platforms": ["linux", "macos", "windows"],
        "old_721_executed": False,
        "registry_verified": False,
        "b3_product_started": False,
    }, "B3-0C 交接基线不符")
    require(normalized_audit_sha256((root / "audit.py").read_bytes()) == baseline["audit_sha256"],
            "审计脚本偏离固定源码摘要")
    require(identity((root / "self_negative.py").read_bytes())["sha256"] == baseline["self_negative_sha256"],
            "自反例脚本偏离固定源码摘要")
    return baseline | {"no_product_change": no_product_change}


def verify_decisions(root):
    record = json.loads((root / "decisions.json").read_bytes())
    rows = record["decisions"]
    require(record["stage"] == "20O-B3-0D" and record["execution_scope_revision"] == COFF_CRITERIA_REVISION
            and record["execution_scope_revision_approved"] is True
            and record["execution_scope_revision_source"] == "星崽会话直批（2026-11-24）"
            and set(rows) == {
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
    require(matrix["compatibility"]["old_1_19_22_artifacts"] == "reject_before_execution"
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
            and policy["accepted_observation"] == [
                "retained_coff_symbol_table", "trusted_linker_map_bound_to_artifact_hash",
                "trusted_pdb_component_manifest_bound_to_artifact_hash",
            ]
            and "UnverifiedCoffExports" in policy["rejected_observation"], "Windows COFF 政策不符")
    return {"policy": policy["policy"], "status": policy["status"]}


def verify_coff_criteria_revision(root, coff):
    revision = json.loads((root / COFF_CRITERIA_REVISION).read_bytes())
    effect = {
        "historical_probe_products_complete": False,
        "accepted_path_verified": False,
        "positive_verifier_check": "B3-1",
        "product_acceptance": ["B3-1", "B3-7"],
        "windows_product_coff_status": coff["status"],
    }
    require(revision["stage"] == "20O-B3-0D4"
            and revision["kind"] == "execution_scope_revision"
            and revision["related_approved_decision"] == "B3-0D-COFF"
            and revision["separate_user_approval"] is True
            and revision["approval"] == {
                "source": "星崽会话直批",
                "recorded_on": "2026-11-24",
                "message": "批准修订，推送吧，然后你规划下一阶段",
            }
            and revision["original"]["commit"] == START_COMMIT
            and revision["current_scope"] == "policy_freeze_audit"
            and revision["effect"] == effect, REASONS["forged-coff-criteria"])
    for version in (revision["original"], revision["first_revision"]):
        plan = blob(version["commit"], PLAN_PATH).decode("utf-8")
        require(f"7. {version['clause']}" in plan.splitlines(), "D4 修订引文与固定规划对象不符")
    return revision


def coff_log_paths(root):
    record = json.loads((root / "coff-probe.json").read_bytes())
    return {
        command[stream]["path"]
        for observation in record["observations"]
        for command in observation["commands"]
        for stream in ("stdout", "stderr")
    }


def verify_coff_probe(root):
    record = json.loads((root / "coff-probe.json").read_bytes())
    expected_artifacts = [("std", 0), ("std", 3), ("stdlib", 0), ("stdlib", 3)]
    rows = record["observations"]
    archive_bytes = blob(COFF_ARCHIVE_COMMIT, COFF_ARCHIVE_PATH)
    archive_identity = identity(archive_bytes)
    archive_metadata = {
        "path": "../entry-20261123-b3-0b/cli-import.zip",
        "commit": COFF_ARCHIVE_COMMIT,
        **archive_identity,
    }
    require(record["kind"] == "feasibility_probe_only" and record["product_acceptance"] is False
            and record["source_commit"] == PRODUCT_COMMIT and record["target"] == "x86_64-pc-windows-msvc"
            and archive_identity["sha256"] == COFF_ARCHIVE_SHA256
            and record["archive"] == archive_metadata
            and record["archive_contains_pdb_or_map_sidecars"] == [] and len(rows) == 4,
            "Windows COFF 探测来源或范围不符")
    tool = record["tools"]["llvm_readobj"]
    require(tool["bytes"] == 2140197 and tool["sha256"] == "768a76a82fedb79b5305ca6bfffa63c00b5738789c7f47225b247345ab99328e"
            and "LLVM version 22.1.2" in tool["version"], "COFF 探测工具身份不符")
    try:
        with zipfile.ZipFile(io.BytesIO(archive_bytes)) as archive:
            archived_files = json.loads(archive.read("evidence.json"))
            result_identity = archived_files["probe/result.json"]
            result_raw = archive.read("objects/" + result_identity["sha256"])
            require(identity(result_raw) == result_identity, "COFF 来源归档中的探测结果摘要不符")
            source_result = json.loads(result_raw)
            require(source_result["source"]["commit"] == PRODUCT_COMMIT,
                    "COFF 来源产物身份不符")
            source_artifacts = {}
            for case, level in expected_artifacts:
                relative = f"probe/case-{case}/native-o{level}.exe"
                artifact_identity = archived_files[relative]
                artifact_raw = archive.read("objects/" + artifact_identity["sha256"])
                require(identity(artifact_raw) == artifact_identity, "COFF 来源产物摘要不符")
                native = next(item for item in source_result["cases"][case]["native"] if item["level"] == level)
                require(native["verification"] == "unverified-coff-exports"
                        and native["exe"] == artifact_identity, "COFF 来源产物观察身份不符")
                source_artifacts[relative] = artifact_identity
            sidecars = [path for path in archived_files if path.lower().endswith((".pdb", ".map"))]
            require(sidecars == [], "COFF 来源归档出现 PDB 或 map sidecar")
    except (KeyError, OSError, zipfile.BadZipFile) as error:
        raise ValueError("COFF 来源 Git 归档不完整") from error
    symbols = []
    for row, (case, level) in zip(rows, expected_artifacts):
        require(row["archive_path"] == f"probe/case-{case}/native-o{level}.exe"
                and row["target"] == "x86_64-pc-windows-msvc"
                and row["prior_verification"] == "unverified-coff-exports"
                and row["artifact"] == source_artifacts[row["archive_path"]]
                and len(row["commands"]) == 2, "COFF 探测产物矩阵不符")
        outputs = []
        expected_arguments = (
            ["--file-headers", "--coff-exports", "<artifact>"],
            ["--coff-debug-directory", "<artifact>"],
        )
        for command, arguments in zip(row["commands"], expected_arguments):
            require(command["exit"] == 0 and command["argv"] == [tool["path"], *arguments], "COFF 探测命令未成功")
            for stream in ("stdout", "stderr"):
                output = command[stream]
                path = Path(output["path"])
                require(not path.is_absolute() and path.as_posix().startswith("logs/sha256/"), "COFF 探测日志路径不安全")
                raw = (root / path).read_bytes()
                require(identity(raw) == {"bytes": output["bytes"], "sha256": output["sha256"]}
                        and path.stem == output["sha256"], "COFF 探测日志摘要不符")
                if stream == "stdout":
                    outputs.append(raw.decode("utf-8"))
                else:
                    require(raw == b"", "COFF 探测标准错误非空")
        headers, debug = outputs
        pointer = re.search(r"PointerToSymbolTable: 0x([0-9A-Fa-f]+)", headers)
        count = re.search(r"SymbolCount: (\d+)", headers)
        export_rva = re.search(r"ExportTableRVA: 0x([0-9A-Fa-f]+)", headers)
        export_size = re.search(r"ExportTableSize: 0x([0-9A-Fa-f]+)", headers)
        require(all((pointer, count, export_rva, export_size)), "COFF 探测日志缺少 PE/COFF 字段")
        observed_symbols = re.findall(r"^\s+Name: (xiao_runtime_[A-Za-z0-9_]+)$", headers, re.MULTILINE)
        debug_types = re.findall(r"^\s+Type: ([A-Za-z0-9_]+) \(0x[0-9A-Fa-f]+\)$", debug, re.MULTILINE)
        facts = row["observed"]
        require(int(pointer.group(1), 16) == facts["symbol_table_pointer"] == 0
                and int(count.group(1)) == facts["symbol_count"] == 0
                and int(export_rva.group(1), 16) == facts["export_table_rva"] > 0
                and int(export_size.group(1), 16) == facts["export_table_size"] > 0
                and facts["object_format"] == "COFF-x86-64"
                and facts["retained_coff_symbol_table"] is False
                and observed_symbols == facts["exported_runtime_symbols"]
                and debug_types == facts["debug_directory_types"]
                and facts["codeview_present"] is ("CodeView" in debug_types),
                "COFF 探测观察字段与原始日志不符")
        symbols.append(observed_symbols)
    require(record["conclusion"] == {
        "all_four_products_have_no_coff_symbol_table": True,
        "exports_are_readable": True,
        "product_verification_remains": "UnverifiedCoffExports",
        "windows_product_coff_status": "pending",
    }, "COFF 探测结论越界")
    require(all(item == symbols[0] for item in symbols) and len(symbols[0]) == 4,
            "COFF 导出观察在四份产物间不一致")
    return {"artifacts": len(rows), "archive_commit": COFF_ARCHIVE_COMMIT,
            "symbol_table_readable": False, "exports_readable": True,
            "exported_symbols": symbols[0], "tool_sha256": tool["sha256"],
            "tool_version": "LLVM 22.1.2"}


def verify_handoff(root, coff, revision):
    handoff = json.loads((root / "handoff.json").read_bytes())
    coff_handoff = {
        **coff,
        "product_evidence": False,
        "criteria_revision": COFF_CRITERIA_REVISION,
        "accepted_path_verified": revision["effect"]["accepted_path_verified"],
        "positive_verifier_check": revision["effect"]["positive_verifier_check"],
    }
    require(handoff["next"] == "20O-B3-0E" and handoff["b3_0d_passed"] is True
            and handoff["b3_0e_entry_ready"] is True and handoff["b3_product_started"] is False
            and handoff["registry_verified"] is False and handoff["windows_product_coff"] == coff_handoff
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
        reason = "" if expected_case["expected"] == "pass" else expected_case["expected"]
        expected_stderr = f"B3-0D 拒绝：{reason}\n" if reason else ""
        require(isinstance(row, dict), REASONS["forged-negative-report"])
        argv = row.get("argv")
        require(row["read_only"] is True and row["before"] == row["after"]
                and re.fullmatch(r"[a-f0-9]{64}", row["before"])
                and row["exit"] == int(bool(reason)) and row["stderr"] == expected_stderr
                and (not row["stdout"] if reason else bool(row["stdout"]))
                and isinstance(argv, list) and len(argv) == 7
                and argv[1:4] == ["-B", "-X", "utf8"] and Path(argv[4]).name == "audit.py"
                and argv[5] == "--evidence-root" and Path(argv[6]).name == row["case"],
                REASONS["forged-negative-report"])
    return len(payload["cases"])


def doc_snapshot():
    return {
        "commit": FREEZE_COMMIT,
        "tree_sha256": identity(git("ls-tree", "-rz", FREEZE_COMMIT, "--", "docs/DevDocs", "docs/UseDocs"))["sha256"],
        "markdown": {path: identity(blob(FREEZE_COMMIT, path)) for path in sorted(DOCS)},
    }


def verify_doc_gates(root):
    report = json.loads((root / "doc-gates.json").read_bytes())
    rows = report["checks"]
    require([row["name"] for row in rows] == ["check:docs", "check:usedocs", "check:layout"]
            and all(row["exit"] == 0 for row in rows)
            and report["passed"] is True, "文档门禁未通过或回执不完整")
    require(report["source"] == doc_snapshot() and report["worktree_verified_before_after"] is True,
            "文档门禁未绑定冻结提交中的 DevDocs/UseDocs 字节")
    used = {row[key] for row in rows for key in ("stdout_sha256", "stderr_sha256")}
    require(report["logs"].keys() == used
            and all(identity(text.encode("utf-8"))["sha256"] == digest for digest, text in report["logs"].items()),
            "文档门禁日志摘要不符")
    for row in rows:
        require(row["stdout"] == "通过：未发现 A0 规则问题。\n" and row["argv"][-2:] == ["run", row["name"]],
                "文档门禁命令或输出不符")
    return {"checks": [{"name": row["name"], "exit": row["exit"], "stdout_sha256": row["stdout_sha256"], "stderr_sha256": row["stderr_sha256"]} for row in rows], "passed": True}


def derive(root):
    verify_static_files(root)
    baseline = verify_baseline(root)
    decisions = verify_decisions(root)
    freeze = verify_freeze_table(root)
    target = verify_versions(root)
    cabi = verify_c_abi(root)
    coff = verify_coff(root)
    coff_criteria_revision = verify_coff_criteria_revision(root, coff)
    coff_probe = verify_coff_probe(root)
    handoff = verify_handoff(root, coff, coff_criteria_revision)
    neg = verify_negative_report(root)
    gates = verify_doc_gates(root)
    decisions_approved = decisions == 6
    no_product_change = baseline["no_product_change"]
    checks = {
        "b3_0d0_decisions_recorded": decisions_approved,
        "b3_0d1_intrinsics_capabilities_frozen": freeze["intrinsics"] == 15,
        "b3_0d2_versions_artifacts_frozen": target == {"runtime_abi": "1.20", "generator": 23, "stdlib_interface": 1},
        "b3_0d3_c_abi_protocol_frozen": cabi == 15,
        "b3_0d4_windows_coff_policy_frozen": coff["policy"] == "complete_required",
        "b3_0d5_evidence_audit_passed": neg is not None and gates["passed"],
    }
    b3_0d_passed = all(checks.values())
    b3_0e_entry_ready = b3_0d_passed and decisions_approved and no_product_change
    b3_0_passed = False  # B3-0E 尚未收口。
    return {
        "scope": "20O-B3-0D",
        "freeze_commit": FREEZE_COMMIT,
        "product_commit": PRODUCT_COMMIT,
        "prior_b3_0c": PRIOR_C,
        "b3_0d_started": baseline["entry_commit"] == START_COMMIT,
        **checks,
        "b3_0d_passed": b3_0d_passed,
        "b3_0e_entry_ready": b3_0e_entry_ready,
        "b3_1_entry_ready": b3_0_passed and decisions_approved and no_product_change,
        "decisions_approved": decisions_approved,
        "no_product_change": no_product_change,
        "b3_0_passed": b3_0_passed,
        "b3_product_started": False,
        "registry_verified": False,
        "old_721_executed": False,
        "windows_product_coff_status": coff["status"],
        "coff_criteria_revision": coff_criteria_revision,
        "windows_coff_probe": coff_probe,
        "doc_gates": gates,
        "doc_gates_passed": gates["passed"],
        "baseline": {"product": baseline["product"], "b1_guards": baseline["inherited"]["b1_guards"], "platforms": baseline["inherited"]["platforms"]},
        "handoff": handoff if b3_0e_entry_ready else None,
        "negative_cases": neg,
    }


def audit(root):
    files = set(inventory(root))
    expected = STATIC_FILES | GENERATED_FILES | coff_log_paths(root)
    require((root / "evidence.json").is_file(), "封存缺少文件：evidence.json")
    expected_inventory = expected - {"evidence.json"}
    missing = expected_inventory - files
    require(not missing, f"封存缺少文件：{', '.join(sorted(missing))}")
    require(files == expected_inventory or files == expected_inventory | OPTIONAL_GENERATED_FILES, "封存目录包含未登记文件")
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
