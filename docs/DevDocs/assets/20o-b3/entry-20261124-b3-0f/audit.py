"""只读核对 B3-0 冻结交接及其固定 Git 对象。"""

import argparse
from functools import cache
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
PREFIX = ROOT.relative_to(REPO).as_posix()
PARENT = ROOT.parent.relative_to(REPO).as_posix()
ENTRY = "277e74c8e94a92db520379f1c32340d26b0c1043"
BOUNDARY = "9a8f4285687da5ae2b0401f6c21f984efee03b41"
STAGES = {
    "A": ("entry-20261122-b3-0", "entry.json", "audit_entry.py", "b3_0a_baseline_frozen"),
    "B": ("entry-20261123-b3-0b", "closeout.json", "audit_closeout.py", "b3_0b_passed"),
    "C": ("entry-20261123-b3-0c", "closeout.json", "audit_closeout.py", "b3_0c_passed"),
    "D": ("entry-20261124-b3-0d", "closeout.json", "audit.py", "b3_0d_passed"),
    "E": ("entry-20261124-b3-0e", "closeout.json", "audit.py", "b3_0e_passed"),
}
MODULES = ("time", "args", "cli", "fs", "env", "log")
CONTRACT_SECTIONS = {
    "time": ("time", "错误与优先级", "性能与三平台"),
    "args": ("args", "错误与优先级", "args 与 cli 接纳预算", "性能与三平台"),
    "cli": ("cli", "错误与优先级", "args 与 cli 接纳预算", "性能与三平台"),
    "fs": ("fs 路径与提交", "错误与取消", "资源与性能"),
    "env": ("env 快照与继承", "错误与取消", "资源与性能"),
    "log": ("log 记录与失败归属", "错误与取消", "资源与性能"),
}
GATES = ("check:docs", "check:usedocs", "check:layout")
FIXED = {".gitattributes", "README.md", "audit.py", "seal.py", "self_negative.py",
         "freeze-table.json", "preflight.json", "negative-cases.json"}
RECEIPTS = {"doc-gates.json", "closeout.json", "evidence.json", "self-negative-result.json"}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def git(*args):
    launcher = ["cmd", "/c", "git"] if os.name == "nt" else ["git"]
    return subprocess.check_output([*launcher, *args], cwd=REPO)


@cache
def blob(commit, path):
    return git("show", f"{commit}:{path}")


def path_at(stage, name):
    return f"{PARENT}/{STAGES[stage][0]}/{name}"


def original(stage, name):
    return blob(ENTRY, path_at(stage, name))


def data(root, name):
    return json.loads((root / name).read_bytes())


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def identity(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def inventory(root):
    return {file.relative_to(root).as_posix(): identity(file.read_bytes())
            for file in sorted(root.rglob("*")) if file.is_file() and file != root / "evidence.json"}


def documents_only(paths):
    return all(path.startswith(b"docs/DevDocs/") for path in paths.split(b"\0") if path)


def source_file(stage, name):
    return {"path": path_at(stage, name), **identity(original(stage, name))}


def expected_upstream():
    statuses = {}
    receipts = {}
    for stage, (_, filename, script, status) in STAGES.items():
        closeout = json.loads(original(stage, filename))
        manifest = json.loads(original(stage, "evidence.json"))["files"]
        require(closeout[status] is True and all(closeout[key] is False for key in
                ("b3_product_started", "registry_verified", "old_721_executed")),
                f"B3-0{stage} 的冻结边界不符")
        require(manifest[filename] == identity(original(stage, filename)), f"B3-0{stage} 的收口身份不符")
        statuses[stage] = closeout
        receipts[stage] = {
            "script": path_at(stage, script), "exit": 0, "stderr": "",
            "stdout": identity(encoded(closeout | {"audit_files": len(manifest)})),
            "files": len(manifest),
        }
    return statuses, receipts


def freeze_table(statuses, audits):
    baseline = statuses["A"]
    product = baseline["product"]
    require(product["tree"] == git("show", "-s", "--format=%T", product["commit"]).decode().strip(),
            "受检产品树身份不符")
    require(set(product["locks"]) == {"core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock"},
            "固定 Cargo.lock 登记不符")
    for path, digest in product["locks"].items():
        require(identity(blob(product["commit"], path)) == digest and identity(blob(ENTRY, path)) == digest,
                f"固定 Cargo.lock 不符：{path}")
    require(baseline["guards"] == {"actual": 4, "derived": 24, "passed": 28}
            and set(baseline["platforms"]) == {"windows", "linux", "macos"}, "三平台或 B1 守卫前置不符")
    decisions = {stage: json.loads(original(stage, "decisions.json")) for stage in "BCDE"}
    fields = {
        "B": ("direction_decisions", "interface_decisions", "detail_decisions"),
        "C": ("direction_decisions", "detail_decisions"),
        "D": ("decisions",), "E": ("decisions",),
    }
    counts = {stage: sum(len(decisions[stage][key]) for key in keys) for stage, keys in fields.items()}
    require(all(statuses[stage]["decisions_approved"] is True for stage in "BCDE")
            and counts == {"B": 12, "C": 9, "D": 6, "E": 5}
            and all(statuses[stage]["decision_count"] == counts[stage] for stage in "BC")
            and statuses["D"]["coff_criteria_revision"]["separate_user_approval"] is True
            and statuses["E"]["pending_decisions"] == [] and statuses["E"]["b3_0f_entry_ready"] is True,
            "方向答复不能代替精确冻结")
    versions = json.loads(original("D", "version-matrix.json"))
    declarations = json.loads(original("D", "freeze-table.json"))
    tutorials = json.loads(original("E", "tutorials.json"))
    coff = tutorials["windows_product_coff"]
    require(versions["b3_target"] == {"runtime_abi": "1.20", "generator": 23, "stdlib_interface": 1}
            and declarations["capabilities"]["bits"]["environment"] == 7
            and [row["id"] for row in declarations["intrinsics"]] == list(range(49, 64)), "联合冻结版本或编号不符")
    require(coff == statuses["D"]["handoff"]["windows_product_coff"] == statuses["E"]["windows_product_coff"]
            and coff["policy"] == "complete_required" and coff["status"] == "pending"
            and coff["accepted_path_verified"] is False and coff["positive_verifier_check"] == "B3-1",
            "COFF 观察或验收边界不符")
    require(tutorials["modules"] == list(MODULES) and tutorials["status"] == "planned"
            and tutorials["product_execution"] is False and statuses["E"]["parsed_sources"] == 12,
            "教程证据状态不符")
    modules = {}
    for module in MODULES:
        stage = "B" if module in MODULES[:3] else "C"
        case = tutorials["cases"][module]
        contract_lines = original(stage, "contract-draft.md").decode("utf-8").splitlines()
        headings = {line.lstrip("# ") for line in contract_lines if line.startswith("##")}
        api_row = next((line for line in contract_lines if line.startswith(f"| {module} |")), "")
        require(set(CONTRACT_SECTIONS[module]) <= headings and api_row,
                f"冻结契约缺少签名或章节：{module}")
        modules[module] = {
            "canonical_module": case["canonical_module"],
            "contract": path_at(stage, "contract-draft.md"),
            "contract_sections": list(CONTRACT_SECTIONS[module]),
            "public_signatures": api_row.split("|")[2].strip(),
            "intrinsics": [row for row in declarations["intrinsics"] if row["module"] == module],
            "tutorial_status": "planned",
            "tutorial_case": case,
        }
    references = {
        "A": ("observations.zip", "decisions.md"),
        "B": ("decisions.json", "contract-draft.md", "cli-import.zip", "cli-shape.zip"),
        "C": ("decisions.json", "contract-draft.md", "tutorial-check.json"),
        "D": ("decisions.json", "contract-draft.md", "freeze-table.json", "version-matrix.json",
              "c-abi.json", "coff-policy.json", "coff-probe.json", "coff-criteria-revision.json"),
        "E": ("decisions.json", "tutorials.json", "syntax.json"),
    }
    return {
        "source_commit": ENTRY,
        "product_boundary_base": BOUNDARY,
        "product_baseline": product,
        "b1_guards": baseline["guards"],
        "host_observations": baseline["platforms"],
        "stages": {
            stage: {"closeout": source_file(stage, STAGES[stage][1]), "files": audits[stage]["files"],
                    "manifest": source_file(stage, "evidence.json"),
                    "inputs": [source_file(stage, name) for name in names]}
            for stage, names in references.items()
        },
        "decision_counts": counts | {"D_coff_scope_revision": 1},
        "direction_sufficient_for_freeze": False,
        "module_order": list(MODULES),
        "modules": modules,
        "versions": versions,
        "capabilities": declarations["capabilities"],
        "windows_product_coff": coff,
        "deferred": ["fs_restricted_mode_deferred_unscheduled", "cli_full_option_parsing",
                     "product_execution", "registry_and_tutorial_verification", "old_721", "historical_debts"],
    }


def doc_source(commit):
    return {"commit": commit, "tree_sha256": identity(git("ls-tree", "-rz", commit, "--",
                                                         "docs/DevDocs", "docs/UseDocs"))["sha256"]}


def verify_doc_gates(root):
    report = data(root, "doc-gates.json")
    source = report["source"]
    checks = report["checks"]
    logs = report["logs"]
    require(source == doc_source(source["commit"]) and report["worktree_matched_before_after"] is True,
            "文档门禁未绑定受检快照")
    require([row["name"] for row in checks] == list(GATES)
            and all(row["exit"] == 0 and row["argv"][-2:] == ["run", row["name"]] for row in checks)
            and set(logs) == {row[key] for row in checks for key in ("stdout_sha256", "stderr_sha256")}
            and all(identity(log.encode())["sha256"] == sha for sha, log in logs.items())
            and all(logs[row["stdout_sha256"]] == "通过：未发现 A0 规则问题。\n" for row in checks),
            "文档门禁回执不符")
    return source["commit"]


def negative_count(root):
    expected = data(root, "negative-cases.json")
    rows = data(root, "self-negative-result.json")
    require([row["case"] for row in rows] == list(expected), "自反例回执不完整")
    for row in rows:
        reason = expected[row["case"]]
        require(row["exit"] == int(bool(reason)) and row["before"] == row["after"]
                and re.fullmatch(r"[0-9a-f]{64}", row["before"])
                and row["stderr"] == (f"B3-0F 拒绝：{reason}\n" if reason else "")
                and bool(row["stdout"]) is (not reason), "自反例回执不符")
    return len(rows) - 1


def derive(root, prepare):
    statuses, expected = expected_upstream()
    preflight = data(root, "preflight.json")
    require(preflight["source_commit"] == ENTRY and preflight["stages"] == expected,
            "前置审计回执偏离固定来源")
    require(preflight["b3_0e_self_negative"] == {
        "script": path_at("E", "self_negative.py"), "exit": 0, "stderr": "",
        "stdout": {"baseline_passed": True, "read_only": True,
                   "rejected_cases": statuses["E"]["rejected_negative_cases"]},
    }, "B3-0E 自反例复跑回执不符")
    table = data(root, "freeze-table.json")
    require(table["direction_sufficient_for_freeze"] is False, "方向答复不能代替精确冻结")
    require(table["windows_product_coff"] == statuses["D"]["handoff"]["windows_product_coff"],
            "COFF 观察或验收边界不符")
    require(table == freeze_table(statuses, expected), "冻结表与固定来源不符")
    commit = verify_doc_gates(root)
    no_product_change = documents_only(git("diff", "--name-only", "-z", BOUNDARY, commit))
    require(no_product_change, "受检提交越过 B3-0 产品边界")
    for name in sorted(FIXED):
        require((root / name).read_bytes() == blob(commit, f"{PREFIX}/{name}"),
                f"冻结输入偏离受检提交：{name}")
    rejections = None if prepare else negative_count(root)
    flags = {
        "b3_0a_baseline_frozen": statuses["A"]["b3_0a_baseline_frozen"],
        "b3_0b_time_args_cli_frozen": statuses["B"]["b3_0b_passed"],
        "b3_0c_fs_env_log_frozen": statuses["C"]["b3_0c_passed"],
        "b3_0d_version_coff_frozen": statuses["D"]["b3_0d_passed"],
        "b3_0e_tutorials_frozen": statuses["E"]["b3_0e_passed"],
        "b3_0f_evidence_audit_passed": rejections is not None,
    }
    decisions_approved = all(statuses[stage]["decisions_approved"] for stage in "BCDE")
    passed = all(flags.values())
    return flags | {
        "scope": "20O-B3-0F",
        "source_commit": ENTRY,
        "document_commit": commit,
        "b3_0f_started": True,
        "b3_0_passed": passed,
        "b3_1_entry_ready": passed and decisions_approved and no_product_change,
        "b3_1_started": False,
        "b3_product_started": False,
        "registry_verified": False,
        "old_721_executed": False,
        "no_product_change": no_product_change,
        "decisions_approved": decisions_approved,
        "prior_evidence_files": sum(row["files"] for row in expected.values()),
        "negative_rejections": rejections,
        "next": "20O-B3-1",
        "windows_product_coff": table["windows_product_coff"],
        "freeze_table": "freeze-table.json",
    }


def audit(root, prepare):
    files = inventory(root)
    expected = FIXED | (RECEIPTS - {"evidence.json"})
    if prepare:
        expected.remove("self-negative-result.json")
    missing = expected - files.keys()
    require(not missing, f"封存缺少文件：{', '.join(sorted(missing))}")
    require(files.keys() == expected, "封存目录包含未登记文件")
    require((root / "evidence.json").read_bytes() == encoded({"schema": 1, "files": files}), "证据清单与实际字节不符")
    result = derive(root, prepare)
    require((root / "closeout.json").read_bytes() == encoded(result), "收口状态与实际审计不符")
    return result | {"audit_files": len(files)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, default=ROOT)
    parser.add_argument("--prepare", action="store_true", help="核对自反例回执生成前的准备快照")
    args = parser.parse_args()
    try:
        print(encoded(audit(args.evidence_root.resolve(), args.prepare)).decode(), end="")
    except (KeyError, TypeError, ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"B3-0F 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
