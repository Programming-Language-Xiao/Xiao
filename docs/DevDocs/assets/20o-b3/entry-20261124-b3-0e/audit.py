"""只读核对 B3-0E 教程候选、精确入口和固定来源。"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
PARENT = ROOT.parent.relative_to(REPO).as_posix()
PREFIX = ROOT.relative_to(REPO).as_posix()
ENTRY = "93d7fc877e8c2d444f385da1dc68727aed4aea72"
PRODUCT = "e79fab3cdd3ea9d3a7eff31f529dec2498434114"
PRIOR_A = ("b91455fe3a323246c916a19e8c7d0db93c2dd561", "entry-20261122-b3-0")
PRIOR_B = ("b8abebf368ef9180ad05756ae90f5d198580a028", "entry-20261123-b3-0b")
PRIOR_C = ("7f1017b7ff0b7b0a89d11943d5d33e0b432d0b3f", "entry-20261123-b3-0c")
PRIOR_D = ("10c7499f3276ddd3f4ed3524c196e44518ab93c9", "entry-20261124-b3-0d")
MODULES = ("time", "args", "cli", "fs", "env", "log")
INPUTS = {
    "time": {"wall_clock_unix_ms": 946684800000, "monotonic_elapsed_ns": 0},
    "args": {"program_name": "demo.xiao", "argv": ["", "two words", "--", "again", "again"]},
    "env": {"environment": [["XIAO_B3_TUTORIAL_VALUE", "initial"]]},
    "log": {"wall_clock_unix_ms": 946684800000},
}
CASE_CAPABILITIES = {
    "time": ["clock", "stdout"], "args": ["process", "stdout"], "cli": ["stdout"],
    "fs": ["filesystem", "stdout"], "env": ["environment", "stdout"], "log": ["clock", "stdout"],
}
NATIVE_ENV = ("XIAO_TARGET_TRIPLE", "XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLC", "XIAO_RUNTIME_LIBRARY")
DECISIONS = ("B3-0E-PATH", "B3-0E-ALIAS", "B3-0E-ENTRY", "B3-0E-INPUT", "B3-0E-STATE")
GATES = ("check:docs", "check:usedocs", "check:layout")
PROTECTED = (
    "core", "stdlib", "tests", "cli", "tools", "resources", ".cargo", ".github",
    ".gitattributes", "package.json", "bun.lock", "tsconfig.json", "repository.manifest.json",
    "docs/UseDocs", "docs/module-registry.json",
)
SHARED_FILES = {
    ".gitattributes", "README.md", "audit.py", "closeout.json", "decisions.json", "preflight.json",
    "syntax.json", "tutorials.json", "self_negative.py", "negative-cases.json", "doc-gates.json",
    "self-negative-result.json", "seal.py",
}
GENERATED = {"closeout.json", "doc-gates.json", "evidence.json", "self-negative-result.json"}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def git(*arguments):
    launcher = ["cmd", "/c", "git"] if os.name == "nt" else ["git"]
    return subprocess.check_output([*launcher, *arguments], cwd=REPO)


def old_blob(origin, path):
    commit, name = origin
    return git("show", f"{commit}:{PARENT}/{name}/{path}")


def payload(root, path):
    return json.loads((root / path).read_bytes())


def identity(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def inventory(root):
    return {
        path.relative_to(root).as_posix(): identity(path.read_bytes())
        for path in sorted(root.rglob("*"))
        if path.is_file() and path != root / "evidence.json"
    }


def module_paths(module):
    origin = PRIOR_B if module in MODULES[:3] else PRIOR_C
    name = "b3-0b-candidate" if origin == PRIOR_B else "b3-0c-candidate"
    product_path = f"tests/fixtures/docs/{module}/tutorial"
    original_path = f"candidates/tests/fixtures/docs/{module}/{name}"
    return origin, original_path, product_path


def alternate_import(source, module):
    primary = f"from std.{module} import ".encode("ascii")
    canonical = f"from stdlib.{module} import ".encode("ascii")
    require(source.startswith(primary) and source.count(primary) == 1, f"教程导入身份不符：{module}")
    return source.replace(primary, canonical, 1)


def example(root, module, bits):
    origin, original, product_path = module_paths(module)
    directory = f"candidates/{product_path}"
    source = (root / f"{directory}.xiao").read_bytes()
    golden = (root / f"{directory}.stdout").read_bytes()
    require(source == old_blob(origin, f"{original}.xiao"), f"教程源码偏离固定候选：{module}")
    require(golden == old_blob(origin, f"{original}.stdout"), f"独立黄金偏离固定候选：{module}")
    tutorial_path = f"tutorials/{module}.md"
    tutorial = (root / tutorial_path).read_bytes()
    for language, expected in ((b"xiao", source), (b"text", golden)):
        blocks = re.findall(rb"(?ms)^" + b"\x60" * 3 + language + rb"\n(.*?)^" + b"\x60" * 3 + rb"$", tutorial)
        require(blocks == [expected], f"教程代码块与固定夹具不符：{module}")
    require(b"planned" in tutorial, f"教程状态越界：{module}")
    alias = alternate_import(source, module)
    assets = {
        "tutorial": {"path": tutorial_path, **identity(tutorial)},
        "source": {"path": f"{directory}.xiao", **identity(source)},
        "golden": {"path": f"{directory}.stdout", **identity(golden)},
    }
    if module in INPUTS:
        input_path = f"{directory}.input.json"
        require((root / input_path).is_file(), f"缺少教程运行输入：{module}")
        raw = (root / input_path).read_bytes()
        require(json.loads(raw) == INPUTS[module], f"教程运行输入不符：{module}")
        if module != "time":
            require(raw == old_blob(origin, f"{original}.input.json"), f"教程运行输入偏离固定候选：{module}")
        assets["input"] = {"path": input_path, **identity(raw)}
    names = CASE_CAPABILITIES[module]
    return {
        "origin": {"commit": origin[0], "path": f"{PARENT}/{origin[1]}/{original}"},
        "product_fixture_prefix": product_path,
        "assets": assets,
        "canonical_module": f"stdlib.{module}",
        "alias_source": identity(alias),
        "capabilities": {"names": names, "mask": f"0x{sum(1 << bits[name] for name in names):02X}"},
        "target": f"20o_b3_{module}_tutorial",
        "vm": f"{module}_tutorial_matches_vm_golden",
        "native": f"{module}_tutorial_matches_native_golden",
    }


def candidate_matrix(root):
    freeze = json.loads(old_blob(PRIOR_D, "freeze-table.json"))
    versions = json.loads(old_blob(PRIOR_D, "version-matrix.json"))
    handoff = json.loads(old_blob(PRIOR_D, "handoff.json"))
    args_type = "{program_name: str, argv: [str]}"
    require([item["id"] for item in freeze["intrinsics"]] == list(range(49, 64)), "B3-0D 原语输入不符")
    return {
        "status": "planned",
        "product_execution": False,
        "modules": list(MODULES),
        "cases": {module: example(root, module, freeze["capabilities"]["bits"]) for module in MODULES},
        "manifest": "core/rust/Cargo.toml",
        "package": "xiao-driver",
        "rust": "1.96.0",
        "platforms": versions["targets"],
        "product_target": versions["b3_target"],
        "native_required_environment": list(NATIVE_ENV),
        "native_optimizations": [0, 3],
        "aliases": ["std", "stdlib"],
        "alias_rule": "replace_first_module_import_only",
        "expected_exit": 0,
        "expected_cargo_counts": [1, 0, 0],
        "stdout": {"encoding": "UTF-8", "newline": "LF", "compare": "bytes"},
        "inputs": {
            "args": {"run_options": "arguments", "shape": args_type},
            "cli": {"input": "source_literals"},
            "fs": {"initial_directory": "demo", "final_directory": "empty"},
            "env": {"environment": "synthetic_runtime_snapshot"},
            "log": {"wall_clock": "fixed", "stdout": "isolated_capture"},
            "time": {"wall_clock": "fixed", "monotonic": "fixed"},
        },
        "windows_product_coff": handoff["windows_product_coff"],
    }


def baseline():
    previous = json.loads(old_blob(PRIOR_D, "closeout.json"))
    host = json.loads(old_blob(PRIOR_A, "entry.json"))
    product = previous["baseline"]["product"]
    require(product["commit"] == PRODUCT
            and product["tree"] == git("show", "-s", "--format=%T", PRODUCT).decode().strip()
            and previous["b3_0d_passed"] is True and previous["b3_0e_entry_ready"] is True
            and previous["coff_criteria_revision"]["separate_user_approval"] is True
            and previous["windows_product_coff_status"] == "pending", "B3-0D 固定交接不符")
    for path, digest in product["locks"].items():
        require(identity(git("show", f"{PRODUCT}:{path}")) == digest, "固定 Cargo.lock 不符")
    return {
        "entry_commit": ENTRY, "product": product, "guards": host["guards"],
        "prior_commits": {name: ref[0] for name, ref in zip("ABCD", (PRIOR_A, PRIOR_B, PRIOR_C, PRIOR_D))},
        "platform_observations": {
            platform: {key: entry[key] for key in ("target", "runtime", "ci", "b1") if key in entry}
            for platform, entry in host["platforms"].items()
        },
    }


def verify_preflight(root):
    report = payload(root, "preflight.json")
    require(report["baseline"] == baseline(), "B3-0E 前置快照身份不符")
    rows = report["checks"]
    require(list(rows) == ["B3-0B", "B3-0C", "B3-0D", "B3-0D-self-negative"]
            and all(row["exit"] == 0 for row in rows.values()), "前置审计复跑不完整")
    for stage, previous, program, flag in (
        ("B3-0B", PRIOR_B, "audit_closeout.py", "b3_0b_passed"),
        ("B3-0C", PRIOR_C, "audit_closeout.py", "b3_0c_passed"),
        ("B3-0D", PRIOR_D, "audit.py", "b3_0d_passed"),
    ):
        row = rows[stage]
        require(row["argv"][-1] == f"{PARENT}/{previous[1]}/{program}"
                and row["summary"][flag] is True, f"前置审计复跑不符：{stage}")
    require(rows["B3-0D-self-negative"]["summary"] == {
        "cases": 10, "rejected_cases": 9, "read_only": True, "evidence_root_written": False,
    }, "B3-0D 前置负例回执不符")
    return report["baseline"]


def verify_syntax(root, matrix):
    report = payload(root, "syntax.json")
    require(report["kind"] == "syntax_parse" and report["product_execution"] is False,
            "语法回执冒充产品执行")
    names = [f"{module}-{prefix}.xiao" for module in MODULES for prefix in ("std", "stdlib")]
    expected = {}
    for module, case in matrix["cases"].items():
        compiled = case["assets"]["source"]
        expected[f"{module}-std.xiao"] = {key: compiled[key] for key in ("bytes", "sha256")}
        expected[f"{module}-stdlib.xiao"] = case["alias_source"]
    commands = report["commands"]
    parsed = commands["parse"]
    require(report["source_commit"] == PRODUCT and report["examples"] == expected
            and report["probe"] == {"commit": PRIOR_C[0], **identity(old_blob(PRIOR_C, "tutorial_parse.rs"))}
            and set(report["libraries"]) == {"xiao_source", "xiao_syntax"}
            and all(row["exit"] == 0 for row in commands.values())
            and "release: 1.96.0\n" in commands["rustc"]["stdout"]
            and "host: x86_64-pc-windows-msvc\n" in commands["rustc"]["stdout"]
            and parsed["argv"][1:] == names and parsed["stderr"] == ""
            and parsed["stdout"] == "".join(f"{name}: diagnostics=0\n" for name in names),
            "固定候选的语法回执不符")
    return len(expected)


def decision_status(root):
    table = payload(root, "decisions.json")
    rows = table["decisions"]
    require(table["stage"] == "20O-B3-0E" and set(rows) == set(DECISIONS), "裁定台账范围不符")
    pending = []
    for name in DECISIONS:
        row = rows[name]
        if row["status"] == "pending":
            require(row == {"status": "pending"}, "待答项目附有批准字段")
            pending.append(name)
        else:
            require(row["status"] == "approved" and row["answer"] and row["recorded_on"]
                    and row.get("source") and row["scope"], "裁定缺少实际答复")
    return pending


def document_source(commit):
    roots = ("docs/DevDocs", "docs/UseDocs")
    return {"commit": commit, "docs_tree_sha256": identity(git("ls-tree", "-rz", commit, "--", *roots))["sha256"]}


def verify_doc_gates(root):
    receipt = payload(root, "doc-gates.json")
    source = receipt["source"]
    require(source == document_source(source["commit"]) and receipt["worktree_matched_before_after"] is True,
            "文档门禁与受检文档树不符")
    checks = receipt["checks"]
    logs = receipt["logs"]
    require([row["name"] for row in checks] == list(GATES)
            and all(row["exit"] == 0 and row["argv"][-2:] == ["run", row["name"]] for row in checks)
            and set(logs) == {row[stream] for row in checks for stream in ("stdout_sha256", "stderr_sha256")}
            and all(identity(value.encode("utf-8"))["sha256"] == digest for digest, value in logs.items())
            and all(logs[row["stdout_sha256"]] == "通过：未发现 A0 规则问题。\n" for row in checks),
            "文档门禁未通过或日志摘要不符")
    return source["commit"]


def verify_bound_inputs(root, commit):
    tree = git("ls-tree", "-rz", commit, "--", PREFIX).decode("utf-8")
    fixed = {}
    for row in filter(None, tree.split("\0")):
        entry, path = row.split("\t", 1)
        name = path.removeprefix(PREFIX + "/")
        if name not in GENERATED:
            fixed[name] = entry.split()[2]
    inputs = set(inventory(root)) - GENERATED
    require(inputs == fixed.keys(), "受检提交的候选清单不符")
    for name in sorted(inputs):
        raw = (root / name).read_bytes()
        header = f"blob {len(raw)}\0".encode("ascii")
        require(hashlib.sha1(header + raw).hexdigest() == fixed[name], f"候选偏离受检 Git 对象：{name}")


def verify_negative_report(root):
    expected = payload(root, "negative-cases.json")
    rows = payload(root, "self-negative-result.json")["cases"]
    require([row["case"] for row in rows] == list(expected), "自反例用例集合不符")
    for row in rows:
        reason = expected[row["case"]]
        require(row["exit"] == int(bool(reason)) and row["before"] == row["after"]
                and re.fullmatch(r"[a-f0-9]{64}", row["before"])
                and row["stderr"] == (f"B3-0E 拒绝：{reason}\n" if reason else "")
                and bool(row["stdout"]) is (not reason), "自反例拒绝理由或只读性不符")
    return len(rows) - 1


def derive(root, prepare=False):
    fixed = verify_preflight(root)
    pending = decision_status(root)
    submitted = payload(root, "tutorials.json")
    require(submitted["product_execution"] is False, "语法回执冒充产品执行")
    require(submitted["status"] == "planned", "教程状态越界")
    matrix = candidate_matrix(root)
    require({key: value["assets"] for key, value in submitted["cases"].items()}
            == {key: value["assets"] for key, value in matrix["cases"].items()}, "教程或夹具摘要不符")
    require(submitted == matrix, "精确入口矩阵与候选不符")
    parsed = verify_syntax(root, matrix)
    commit = verify_doc_gates(root)
    require(not git("diff", "--name-only", "-z", ENTRY, commit, "--", *PROTECTED),
            "B3-0E 受检快照改动了产品边界")
    verify_bound_inputs(root, commit)
    negatives = None if prepare else verify_negative_report(root)
    passed = not pending and negatives is not None
    return {
        "scope": "20O-B3-0E",
        "entry_commit": ENTRY,
        "product_commit": fixed["product"]["commit"],
        "document_commit": commit,
        "b3_0e_started": True,
        "b3_0e0_decisions_recorded": not pending,
        "b3_0e1_six_tutorials_frozen": not pending,
        "b3_0e2_fixtures_inputs_goldens_frozen": not pending,
        "b3_0e3_vm_entries_frozen": not pending,
        "b3_0e4_native_entries_frozen": not pending,
        "b3_0e5_evidence_audit_passed": negatives is not None,
        "b3_0e_passed": passed,
        "b3_0f_entry_ready": passed,
        "b3_0_passed": False,
        "b3_1_entry_ready": False,
        "b3_product_started": False,
        "product_execution": False,
        "registry_verified": False,
        "old_721_executed": False,
        "decisions_approved": not pending,
        "pending_decisions": pending,
        "no_product_change": True,
        "tutorial_status": "planned",
        "tutorial_count": len(matrix["cases"]),
        "parsed_sources": parsed,
        "doc_gates_passed": True,
        "rejected_negative_cases": negatives,
        "windows_product_coff": matrix["windows_product_coff"],
        "handoff": {
            "next": "20O-B3-0F", "tutorials": "tutorials.json", "modules": list(MODULES),
            "windows_product_coff": matrix["windows_product_coff"],
            "deferred": ["product_execution", "registry_and_tutorial_verification", "old_721", "historical_debts"],
        } if passed else None,
    }


def audit(root, prepare=False):
    files = inventory(root)
    fixture_paths = {f"candidates/{module_paths(module)[2]}.{suffix}"
                     for module in MODULES for suffix in ("xiao", "stdout")}
    input_paths = {f"candidates/{module_paths(module)[2]}.input.json" for module in INPUTS}
    example_paths = fixture_paths | input_paths | {f"tutorials/{name}.md" for name in MODULES}
    expected = SHARED_FILES | example_paths
    if prepare:
        expected -= {"self-negative-result.json"}
    missing = expected - files.keys()
    require(not missing, f"封存缺少文件：{', '.join(sorted(missing))}")
    require(files.keys() == expected, "封存目录包含未登记文件")
    require((root / "evidence.json").read_bytes() == encoded({"schema": 1, "files": files}),
            "证据清单与实际字节不符")
    result = derive(root, prepare)
    require((root / "closeout.json").read_bytes() == encoded(result), "收口状态与实际审计不符")
    return result | {"audit_files": len(files)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, default=ROOT)
    parser.add_argument("--prepare", action="store_true", help="核对尚无自反例回执的准备快照")
    arguments = parser.parse_args()
    try:
        print(encoded(audit(arguments.evidence_root.resolve(), arguments.prepare)).decode(), end="")
    except (KeyError, TypeError, ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"B3-0E 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
