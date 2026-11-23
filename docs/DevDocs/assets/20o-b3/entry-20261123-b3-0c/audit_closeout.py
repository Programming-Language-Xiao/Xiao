"""只读复核 B3-0C 冻结、门禁、负例和交接。"""

import argparse
import json
from pathlib import Path, PureWindowsPath
import re
import subprocess
import sys

sys.dont_write_bytecode = True
from audit_baseline import ROOT, REPO, START, DIRECTIONS, PRIOR, blob, encoded, git, identity, require, verify_baseline
from check_tutorials import MODULES, data, verify_report


PREFIX = ROOT.relative_to(REPO).as_posix()
FINAL_RECEIPTS = ""
PLANNING_DOCS = (
    "docs/DevDocs/README.md",
    "docs/DevDocs/20o-official-library-program.md",
    "docs/DevDocs/20o-b3-host-libraries-plan.md",
    "docs/DevDocs/20o-b3-0-probe-freeze-plan.md",
    "docs/DevDocs/20o-b3-0c-fs-env-log-plan.md",
)
DOCS = (*PLANNING_DOCS, f"{PREFIX}/README.md", f"{PREFIX}/contract-draft.md",
        *(f"{PREFIX}/tutorials/{name}.md" for name in MODULES))
GATES = ("check:docs", "check:usedocs", "check:layout")
DOC_GATE = "doc-gate.json"
PREFLIGHT_REPORT = "self-negative-preflight.json"
NEGATIVE_REPORT = "self-negative-result.json"
STATUS = "closeout.json"
MANIFEST = "evidence.json"
DETAILS = ("B3-0C1", "B3-0C2", "B3-0C3", "B3-0C4a", "B3-0C4b", "B3-0C4c")
REASONS = {
    "unchanged": "",
    "manifest-tampered": "清单字节或文件身份不符",
    "missing-attributes": "封存缺少文件：.gitattributes",
    "forged-status": "收口状态与实际证据不符",
    "forged-observation": "入口观察偏离固定原件",
    "direction-as-freeze": "裁定台账偏离固定实际答复",
    "replaced-doc-blob": "文档门禁偏离定稿 Git 字节",
    "forged-negative-report": "负例回执偏离固定原件",
}
PREFLIGHT_REASONS = {name: reason for name, reason in REASONS.items() if name != "forged-negative-report"}


def digest(raw):
    return identity(raw)["sha256"]


def hash_format(value):
    return re.fullmatch(r"[a-f0-9]{64}", value) is not None


def paths_changed(left, right):
    return set(git("diff", "--name-only", "-z", left, right).decode().split("\0")) - {""}


def inventory(root):
    return {path.relative_to(root).as_posix(): identity(path.read_bytes())
            for path in sorted(root.rglob("*")) if path.is_file() and path != root / MANIFEST}


def frozen_files(commit):
    names = git("ls-tree", "-r", "-z", "--name-only", commit, "--", PREFIX).decode().split("\0")
    return {name.removeprefix(PREFIX + "/") for name in names if name}


def document_snapshot(commit):
    tree = git("ls-tree", "-rz", commit, "--", "docs/DevDocs", "docs/UseDocs")
    return {"commit": commit, "docs_tree_sha256": digest(tree),
            "markdown": {path: identity(blob(commit, path)) for path in DOCS}}


def inputs(document, receipt):
    if document:
        return document, None, None
    final = None
    if receipt is None:
        final = FINAL_RECEIPTS
        require(re.fullmatch(r"[a-f0-9]{40}", final), "缺少固定封存回执提交")
        receipt = git("rev-parse", f"{final}~1").strip().decode()
        require(paths_changed(receipt, final) == {f"{PREFIX}/{NEGATIVE_REPORT}"},
                "封存负例提交范围不符")
    document = git("rev-parse", f"{receipt}~1").strip().decode()
    require(paths_changed(document, receipt) == {f"{PREFIX}/{name}" for name in (DOC_GATE, PREFLIGHT_REPORT)},
            "文档门禁与预检回执提交范围不符")
    return document, receipt, final


def verify_doc_gate(root, commit, receipt):
    raw = (root / DOC_GATE).read_bytes()
    report = json.loads(raw)
    snapshot = document_snapshot(commit)
    require(report["before"] == report["after"] == snapshot, REASONS["replaced-doc-blob"])
    if receipt:
        require(raw == blob(receipt, f"{PREFIX}/{DOC_GATE}"), "文档门禁回执偏离固定原件")
    tool = report["bun"]
    require(PureWindowsPath(tool["path"]).name.lower() in {"bun", "bun.exe"}
            and tool["bytes"] > 0 and hash_format(tool["sha256"]) and tool["version"], "Bun 工具身份不符")
    logs = report["logs"]
    rows = report["checks"]
    used = {row[key] for row in rows for key in ("stdout_sha256", "stderr_sha256")}
    require(logs.keys() == used and all(digest(value.encode()) == sha for sha, value in logs.items()),
            "文档门禁日志对象不符")
    scripts = json.loads(blob(commit, "package.json"))["scripts"]
    require(len(rows) == len(GATES), "定稿文档门禁缺项")
    for gate, row in zip(GATES, rows):
        require(row["argv"] == [tool["path"], "run", gate]
                and type(row["pid"]) is int and row["pid"] > 0 and row["exit"] == 0
                and row["started_utc"] <= row["finished_utc"]
                and logs[row["stdout_sha256"]] == "通过：未发现 A0 规则问题。\n"
                and logs[row["stderr_sha256"]] in ("", f"$ {scripts[gate]}\n"), f"定稿文档门禁未通过：{gate}")
    return {"commit": commit, "docs_tree_sha256": snapshot["docs_tree_sha256"],
            "markdown": len(snapshot["markdown"]), "gates": len(rows), "receipt_commit": receipt}


def verify_decisions(root):
    record = data(root / "decisions.json")
    directions, details = record["direction_decisions"], record["detail_decisions"]
    require(record["stage"] == "20O-B3-0C" and directions.keys() == DIRECTIONS and details.keys() == set(DETAILS),
            "首轮或精确裁定缺项")
    ids = set()
    for row in (directions | details).values():
        origin = json.loads(row["question_item_id"])
        require(row["status"] == "answered" and row["answer"] and row["scope"] and len(origin) == 3
                and origin[0] == "request_user_input_async" and origin[1].startswith("call_")
                and type(origin[2]) is int, "裁定缺少实际答复或问题卡身份")
        ids.add(tuple(origin))
    require(len(ids) == len(directions) + len(details), "裁定问题卡重复")
    return len(ids)


def verify_negatives(root, name, commit, phase, reason_map, args):
    raw = (root / name).read_bytes()
    require(raw == blob(commit, f"{PREFIX}/{name}"), REASONS["forged-negative-report"])
    payload = json.loads(raw)
    rows = payload["cases"]
    require(payload["mode"] == phase and [row["case"] for row in rows] == list(reason_map), "负例集合或采集阶段不符")
    for row in rows:
        case = row["case"]
        reason = reason_map[case]
        argv = row["argv"]
        require(type(row["exit"]) is int and row["exit"] == int(bool(reason))
                and row["stderr"] == (f"B3-0C 拒绝：{reason}\n" if reason else "")
                and row["read_only"] is True and row["before"] == row["after"] and hash_format(row["before"])
                and argv[1:4] == ["-B", "-X", "utf8"] and PureWindowsPath(argv[4]).name == "audit_closeout.py"
                and argv[5] == "--evidence-root" and PureWindowsPath(argv[6]).name == case and argv[7:] == args,
                f"负例实际结论或只读性不符：{case}")
    return {"commit": commit, "mode": phase, "passed": sum(row["exit"] == 1 for row in rows), "sha256": digest(raw)}


def derive(root, commit, receipt, final, fixed):
    changes = paths_changed(START, commit)
    require(set(PLANNING_DOCS) <= changes
            and all(path in PLANNING_DOCS or path.startswith(PREFIX + "/") for path in changes),
            "冻结提交越出规划或证据范围")
    for name in sorted(fixed):
        reason = {"decisions.json": REASONS["direction-as-freeze"],
                  "baseline.json": REASONS["forged-observation"]}.get(name, f"冻结文件偏离固定原件：{name}")
        expected = blob(commit, f"{PREFIX}/{name}")
        if final and name == "audit_closeout.py":
            expected = expected.replace(b'\nFINAL_RECEIPTS = ""\n', f'\nFINAL_RECEIPTS = "{final}"\n'.encode(), 1)
        require((root / name).read_bytes() == expected, reason)
    baseline = verify_baseline(data(root / "baseline.json"))
    product = baseline["product"]
    require(git("show", "-s", "--format=%T", product["commit"]).strip().decode() == product["tree"]
            and all(identity(blob(product["commit"], path)) == record for path, record in product["locks"].items()),
            "固定产品树或锁身份不符")
    count = verify_decisions(root)
    tutorials = verify_report(root, data(root / "tutorial-check.json"))
    gates = verify_doc_gate(root, commit, receipt)
    preflight = None
    if receipt:
        preflight = verify_negatives(root, PREFLIGHT_REPORT, receipt, "preflight", PREFLIGHT_REASONS,
                                     ["--document-commit", commit])
    negatives = None
    if final:
        negatives = verify_negatives(root, NEGATIVE_REPORT, final, "receipt", REASONS, ["--receipt-commit", receipt])
    complete = negatives is not None and negatives["passed"] == len(REASONS) - 1
    flags = dict(b3_0c0_decisions_recorded=True, b3_0c1_fs_frozen=True, b3_0c2_env_frozen=True,
                 b3_0c3_log_frozen=True, b3_0c4_versions_errors_resources_frozen=True,
                 b3_0c5_tutorials_frozen=True, b3_0c6_evidence_audit_passed=complete)
    passed = all(flags.values())
    prior_commit, prior_path, _ = PRIOR["0b"]
    contracts = {
        f"{prior_path}/contract-draft.md": prior_commit,
        f"{PREFIX}/contract-draft.md": commit,
    }
    return flags | {
        "scope": "20O-B3-0C", "workspace_base": START, "contract_commit": commit, "decision_count": count,
        "decisions_approved": True, "no_product_change": True, "b3_0c_started": True, "b3_0c_passed": passed,
        "b3_0d_entry_ready": passed, "b3_0d_started": False, "b3_0_passed": False, "b3_1_entry_ready": False,
        "b3_product_started": False, "registry_verified": False, "old_721_executed": False,
        "baseline": {k: baseline[k] for k in ("product", "platforms", "b1_guards", "b3_0b_cases")},
        "tutorials": tutorials, "doc_gates": gates, "preflight_negative_checks": preflight, "negative_checks": negatives,
        "handoff": {
            "next": "20O-B3-0D", "modules": ["time", "args", "cli", *MODULES],
            "contracts": {path: {"commit": revision, **identity(blob(revision, path))}
                          for path, revision in contracts.items()},
            "version_candidate": {"abi": "1.20", "codegen": 23, "stdlib_interface": 1, "intrinsics": 15},
            "pending": ["intrinsic_ids", "environment_capability_bit_and_legacy_default_mask",
                        "request_protocol_and_native_context", "c_abi_registration_and_artifact_matrix",
                        "windows_product_coff_verifier"],
            "windows_product_coff_policy": baseline["windows_product_coff_policy"],
            "windows_product_coff_status": baseline["windows_product_coff_status"],
        },
    }


def audit(root, commit, receipt, final):
    fixed = frozen_files(commit)
    expected = fixed | {DOC_GATE, STATUS}
    if receipt:
        expected.add(PREFLIGHT_REPORT)
    if final:
        expected.add(NEGATIVE_REPORT)
    files = inventory(root)
    missing = expected - files.keys()
    require(not missing, f"封存缺少文件：{', '.join(sorted(missing))}")
    require(expected == files.keys(), "封存目录包含未登记文件")
    require((root / MANIFEST).read_bytes() == encoded({"schema": 1, "files": files}), REASONS["manifest-tampered"])
    result = derive(root, commit, receipt, final, fixed)
    require((root / STATUS).read_bytes() == encoded(result), REASONS["forged-status"])
    return result | {"audit_files": len(files)}


def add_anchor_arguments(parser):
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--document-commit", help="只预检已受检的文档提交")
    group.add_argument("--receipt-commit", help="复核固定的第一轮回执")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, default=ROOT)
    add_anchor_arguments(parser)
    args = parser.parse_args()
    try:
        report = audit(args.evidence_root.resolve(), *inputs(args.document_commit, args.receipt_commit))
        print(encoded(report).decode(), end="")
    except (OSError, ValueError, KeyError, TypeError, IndexError, subprocess.SubprocessError) as error:
        print(f"B3-0C 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
