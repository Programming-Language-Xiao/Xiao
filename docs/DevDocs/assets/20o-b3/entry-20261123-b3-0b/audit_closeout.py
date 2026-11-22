"""只读复核 B3-0B 的裁定、候选夹具和交接状态。"""

import argparse
import json
from pathlib import Path
import sys
from zipfile import BadZipFile, ZipFile

sys.dont_write_bytecode = True
from preflight import (ENTRY_REASON, MANIFEST_REASON, PLAN, PRIOR, PRIOR_SEAL, ROOT,
                       archive_tools, base, inventory)

CONTRACT = "4a4f7eb8e421d7ea59c470fa89d74d437a5f92ee"
FIRST_CANDIDATES = "2ec112034365bd481387820d4188a64ba297681e"
WORKSPACE_BASE = "9a8f4285687da5ae2b0401f6c21f984efee03b41"
PREFIX = ROOT.relative_to(base.REPO).as_posix()
MODULES = ("time", "args", "cli")
TUTORIALS = {module: f"tutorials/{module}.md" for module in MODULES}
FIXTURES = {f"tests/fixtures/docs/{module}/b3-0b-candidate.{ext}"
            for module in MODULES for ext in ("xiao", "stdout")} | {
                "tests/fixtures/docs/args/b3-0b-candidate.input.json"}
DOCS = (
    *(f"docs/DevDocs/{name}.md" for name in ("README", "20o-official-library-program",
        "20o-b3-host-libraries-plan", "20o-b3-0-probe-freeze-plan", "20o-b3-0b-time-args-cli-plan")),
    *(f"{PREFIX}/{name}" for name in ("README.md", "contract-draft.md", *TUTORIALS.values())),
)
DOCUMENT_CHECKS = ("check:docs", "check:usedocs", "check:layout")
FROZEN = {".gitattributes", "decisions.json", "contract-draft.md", "preflight.py", "preflight.json", "cli-shape.zip",
          "tutorials.json", "layout-first.json", *TUTORIALS.values(), *(f"candidates/{path}" for path in FIXTURES)}
REQUIRED = FROZEN | {"README.md", "audit_closeout.py", "seal_closeout.py", "self_negative.py", "closeout.json"}
MANIFEST = "evidence.json"
REASONS = {
    "manifest-tampered": "清单字节或身份不符",
    "missing-attributes": "封存缺少文件：.gitattributes",
    "forged-status": "收口回执与实际派生结论不符",
    "forged-observation": "探测回执偏离实测字节",
    "direction-as-freeze": "裁定回执偏离实际答复",
}
GATES = {
    "b3_0b0_decisions_recorded": ("B3-0B-TIME", "B3-0B-ARGS", "B3-0B-CLI"),
    "b3_0b1_time_frozen": ("B3-0B1", "B3-0B1a", "B3-0B1b", "B3-0B4b"),
    "b3_0b2_args_frozen": ("B3-0B2", "B3-0B4a", "B3-0B4b"),
    "b3_0b3_cli_frozen": ("B3-0B3", "B3-0B3a", "B3-0B4a", "B3-0B4b"),
    "b3_0b4_versions_errors_resources_frozen": ("B3-0B4a", "B3-0B4b", "B3-0B4c", "B3-0B4d"),
}
require, git = base.require, base.git
identity, json_bytes = archive_tools.identity, archive_tools.json_bytes


def blob(revision, path):
    return git("show", f"{revision}:{path}")


def verify_boundary():
    require(git("merge-base", WORKSPACE_BASE, CONTRACT).strip().decode() == WORKSPACE_BASE
            and git("rev-parse", f"{WORKSPACE_BASE}~1").strip().decode() == PLAN,
            "冻结提交的父提交身份不符")
    require(not git("diff", "--name-only", "-z", WORKSPACE_BASE, CONTRACT, "--", *base.PRODUCT_ROOTS),
            "候选提交的产品或正式夹具发生变化")


def verify_decisions(root):
    status = json.loads((root / "decisions.json").read_bytes())
    decisions = status["direction_decisions"] | status["interface_decisions"] | status["detail_decisions"]
    expected = set().union(*map(set, GATES.values()))
    items = [json.loads(row["question_item_id"]) for row in decisions.values()]
    require(status["stage"] == "20O-B3-0B" and expected - {"B3-0B3a"} <= decisions.keys() <= expected
            and all(row["answer"] and row["scope"] for row in decisions.values())
            and all(len(item) == 3 and item[0] == "request_user_input_async"
                    and item[1].startswith("call_") and type(item[2]) is int for item in items)
            and len({tuple(item) for item in items}) == len(decisions),
            "实际裁定的范围或问题身份不完整")
    require(decisions["B3-0B1"]["status"] == "explicit_unit_requested"
            and all(row["status"] == "answered" for key, row in decisions.items() if key != "B3-0B1"),
            "方向或精确答复状态不符")
    return {key: all(item in decisions for item in items) for key, items in GATES.items()}, len(decisions)


def verify_baseline(root):
    prior = PRIOR.relative_to(base.REPO).as_posix()
    recorded = json.loads((root / "preflight.json").read_bytes())
    manifest = blob(PRIOR_SEAL, prior + "/evidence.json")
    original = json.loads(blob(PRIOR_SEAL, prior + "/entry.json"))
    success = recorded["prior_audit"]
    require(recorded["plan_commit"] == PLAN and recorded["prior_seal"] == PRIOR_SEAL
            and recorded["prior_manifest"] == identity(manifest)
            and recorded["prior_archive"] == original["archive"]
            and success["exit"] == 0 and success["read_only"] is True and not success["stderr"]
            and json.loads(success["stdout"]) == original | {"audit_files": len(json.loads(manifest)["files"])}
            and original["b3_0a_baseline_frozen"] is True
            and original["b3_0_passed"] is original["decisions_approved"] is False
            and recorded["product"] == original["product"]
            and recorded["guards"] == original["guards"], "B3-0A 固定基线或审计回执不符")
    reasons = {
        "manifest-tampered": MANIFEST_REASON, "missing-attributes": MANIFEST_REASON,
        "entry-status-tampered": ENTRY_REASON, "false-approval": ENTRY_REASON,
    }
    negatives = recorded["prior_negatives"]
    require(len(negatives) == len(reasons) and {row["case"] for row in negatives} == reasons.keys()
            and all(row["exit"] == 1 and row["read_only"] is True and not row["stdout"]
                    and row["stderr"] == f"B3-0A 拒绝：{reasons[row['case']]}\n" for row in negatives),
            "B3-0A 实际拒绝证据不完整")
    bun = recorded["bun_argv"]
    samples = bun["samples"]
    require(bun["platform"] == "win32" and bun["executed"] is True and bun["tool"]["version"] == "1.4.0"
            and bun["literal_and_unpaired_same_result"] is True
            and samples.keys() == {"literal-replacement", "unpaired-utf16", "empty"}
            and samples["literal-replacement"]["utf16_unit_hex"] == "FFFD"
            and samples["unpaired-utf16"]["utf16_unit_hex"] == "D800"
            and json.loads(samples["literal-replacement"]["stdout"]) == "\ufffd"
            and samples["literal-replacement"]["stdout"] == samples["unpaired-utf16"]["stdout"]
            and json.loads(samples["empty"]["stdout"]) == ""
            and all(row["exit"] == 0 and not row["stderr"] for row in samples.values()),
            "Windows Bun 原始编码观察不符")
    return {"product": original["product"], "inherited": original["inherited"],
            "platforms": sorted(original["platforms"]), "guards": original["guards"],
            "b3_0a_actual_negative_replays": len(negatives), "windows_bun_argv_loss_observed": True}


def verify_tutorials(root):
    planned = json.loads((root / "tutorials.json").read_bytes())
    require(planned["status"] == "planned" and planned["executed"] is False and planned["candidate_root"] == "candidates"
            and planned["cases"].keys() == TUTORIALS.keys() and planned["native_optimizations"] == [0, 3]
            and planned["expected_cargo_counts"] == [1, 0, 0]
            and planned["expected_exit"] == 0 and planned["windows_verification"] == "complete",
            "教程候选误标执行或精确入口不完整")
    files = {}
    for path in sorted(FIXTURES):
        raw = (root / "candidates" / path).read_bytes()
        require(raw == blob(FIRST_CANDIDATES, path), f"迁移后的夹具字节不符：{path}")
        module = path.split("/")[3]
        ext = Path(path).suffix
        if ext != ".json":
            language = "xiao" if ext == ".xiao" else "text"
            doc = (root / TUTORIALS[module]).read_text(encoding="utf-8")
            require(f"```{language}\n{raw.decode()}```" in doc, f"教程与固定夹具字节不符：{path}")
        else:
            require(planned["cases"]["args"]["input"] == path, "教程参数注入夹具未登记")
        files[path] = identity(raw)
    return {"status": "planned", "product_execution": False, "fixtures": files,
            "tutorial_pages": {name: base.identity(root / path) for name, path in TUTORIALS.items()}}


def verify_cli_shape(root, product):
    candidate = archive_tools.load_module("b3_c_wire", archive_tools.ROOT / "candidate.py")
    path = root / "cli-shape.zip"
    with ZipFile(path) as archive:
        book = archive_tools.Evidence(archive)
        report = book.read("probe/result.json")
        require(report["source"] == product, "返回形状探测源码身份不符")
        require(report["receipts"].keys() == {"explicit/vm", "inferred/vm", "inferred/build-o0",
                "inferred/build-o3", "inferred/native-o0", "inferred/native-o3"}, "返回形状探测执行集合不符")
        expected = "row\na\nrow\nb\nrow\nsolo\nrow\n"
        explicit, inferred = (book.raw(f"probe/{case}/entry.xiao") for case in ("explicit", "inferred"))
        require(b" -> [[str]]" in explicit and explicit.replace(b" -> [[str]]", b"", 1) == inferred,
                "返回形状样例差异不符")
        for tag, receipt in report["receipts"].items():
            prefix = f"probe/{tag}"
            require(receipt == book.read(prefix + ".json") and receipt["exit"] == 0 and receipt["pid"] > 0
                    and receipt["stdout"] == identity(book.raw(prefix + ".stdout"))
                    and receipt["stderr"] == identity(book.raw(prefix + ".stderr")), f"实际形状回执不符：{tag}")
        for case, raw in (("explicit", explicit), ("inferred", inferred)):
            prefix = f"probe/{case}/vm"
            _, request = candidate.frames(book.raw(prefix + ".stdin"))
            hello, reply = candidate.frames(book.raw(prefix + ".stdout"))
            require(request["source"]["text"].encode() == raw and hello["accepted"] is True
                    and hello["versions"]["runtime_abi"] == 65555 and hello["versions"]["codegen"] == 22,
                    f"形状样例的输入或固定版本不符：{case}")
            if case == "explicit":
                require(reply["exit_code"] == 1 and any(item["code"] == "X04-PARSE-005"
                        and item["message_id"] == "x04.parse.invalid_array_element_type"
                        for item in reply["diagnostics"]), "显式注解拒绝证据不符")
            else:
                require(reply["exit_code"] == 0 and candidate.output(reply) == expected, "推导形状 VM 输出不符")
        for level in (0, 3):
            _, reply = candidate.frames(book.raw(f"probe/inferred/build-o{level}.stdout"))
            require(reply["exit_code"] == 0 and reply["artifact"]["optimization_level"] == level
                    and reply["artifact"]["artifact_runtime"]["verification"] == "unverified-coff-exports"
                    and book.raw(f"probe/inferred/native-o{level}.stdout").decode().replace("\r\n", "\n") == expected,
                    f"推导形状原生 O{level} 观察不符")
    return {"archive": base.identity(path), "host": "windows", "explicit_annotation": "X04-PARSE-005",
            "inferred_execution": ["vm", "o0", "o3"], "product_coff_verified": False}


def verify_complete(root):
    report = json.loads((root / "doc-gate.json").read_bytes())
    first = json.loads((root / "layout-first.json").read_bytes())
    require([row["exit"] for row in first["checks"]] == [0, 0, 1]
            and all(f"A0-LAYOUT-002 tests/fixtures/docs/{module}" in first["checks"][-1]["stdout"]
                    for module in MODULES), "首轮布局失败回执不符")
    docs = report["markdown_before"]
    require(report["contract"] == CONTRACT and docs.keys() == set(DOCS)
            and docs == report["markdown_after"], "文档门禁前后身份不符")
    for path, row in docs.items():
        raw = git("cat-file", "blob", row["blob"])
        require(identity(raw) == row["text"], f"门禁定稿 Git 字节不符：{path}")
        if path.startswith(PREFIX + "/"):
            require(raw == (root / path.removeprefix(PREFIX + "/")).read_bytes(),
                    f"封存 Markdown 偏离门禁定稿：{path}")
    require([row["argv"][1:] for row in report["checks"]] == [["run", name] for name in DOCUMENT_CHECKS]
            and all(row["exit"] == 0 and row["pid"] > 0 and row["started_utc"] <= row["finished_utc"]
                    and "通过：未发现 A0 规则问题。" in row["stdout"] for row in report["checks"]),
            "定稿文档门禁未通过或缺少实际执行")
    results = json.loads((root / "self-negative-result.json").read_bytes())
    cases = results["cases"]
    require(results["mode"] == "preflight" and len(cases) == len(REASONS) + 1
            and {row["case"] for row in cases} == REASONS.keys() | {"unchanged"}, "自反例集合不符")
    for row in cases:
        reason = REASONS.get(row["case"])
        require(row["exit"] == int(reason is not None)
                and row["stderr"] == (f"B3-0B 拒绝：{reason}\n" if reason else "")
                and row["read_only"] is True and row["before"] == row["after"]
                and row["argv"][-1] == "--preflight", f"自反例实际理由或只读性不符：{row['case']}")
    return {"docs": len(docs), "gates": len(report["checks"]), "negatives": len(REASONS)}


def derive(root, preflight):
    verify_boundary()
    for name in sorted(FROZEN):
        reason = {"decisions.json": REASONS["direction-as-freeze"],
                  "preflight.json": REASONS["forged-observation"]}.get(name, f"固定契约文件偏离：{name}")
        require((root / name).read_bytes() == blob(CONTRACT, f"{PREFIX}/{name}"), reason)
    flags, decision_count = verify_decisions(root)
    baseline = verify_baseline(root)
    cli_shape = verify_cli_shape(root, baseline["product"])
    tutorials = verify_tutorials(root)
    complete = None if preflight else verify_complete(root)
    flags.update(b3_0b5_tutorials_frozen=True, b3_0b6_evidence_audit_passed=complete is not None)
    passed = all(flags.values())
    return flags | {
        "scope": "20O-B3-0B", "contract_commit": CONTRACT, "workspace_base": WORKSPACE_BASE,
        "baseline": baseline, "decision_count": decision_count,
        "decisions_approved": all(flags[stage] for stage in GATES), "no_product_change": True,
        "b3_0b_passed": passed, "b3_0c_entry_ready": passed, "b3_0c_started": False,
        "b3_0_passed": False, "b3_1_entry_ready": False, "b3_product_started": False,
        "registry_verified": False, "old_721_executed": False,
        "tutorials": tutorials, "cli_shape": cli_shape, "complete_checks": complete,
        "handoff": {
            "next": "20O-B3-0C", "modules": ["fs", "env", "log"], "windows_product_coff": "complete",
            "version_candidate": {"abi": "1.20", "codegen": 23, "stdlib_interface": 1},
            "version_final_stage": "20O-B3-0D",
        },
    }


def audit(root, preflight):
    files = inventory(root)
    required = REQUIRED if preflight else REQUIRED | {"doc-gate.json", "self-negative-result.json"}
    missing = required - files.keys()
    require(not missing, f"封存缺少文件：{', '.join(sorted(missing))}")
    require((root / MANIFEST).read_bytes() == json_bytes({"schema": 1, "files": files}), REASONS["manifest-tampered"])
    status = derive(root, preflight)
    require((root / "closeout.json").read_bytes() == json_bytes(status), REASONS["forged-status"])
    return status | {"audit_files": len(files)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, default=ROOT)
    parser.add_argument("--preflight", action="store_true")
    options = parser.parse_args()
    try:
        print(json_bytes(audit(options.evidence_root.resolve(), options.preflight)).decode(), end="")
    except (KeyError, ValueError, OSError, BadZipFile) as error:
        print(f"B3-0B 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
