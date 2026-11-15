"""核对B4-5重开运行、历史链、pow细项和固定文档身份。"""
from functools import lru_cache
import json
from pathlib import Path
import re

from record import PREFIX, canonical, docs_snapshot, fixed_files, git, identity, load, require, sha
from suite import COMMANDS, HISTORY, TARGETS, SOURCE_COMMIT, PRODUCT_COMMIT
from capture import MANIFEST, MANIFEST_SHA, STAGES, distance, frozen_rule, value


@lru_cache(maxsize=None)
def product_at(repository, commit):
    return fixed_files(repository, commit)


@lru_cache(maxsize=None)
def docs_at(repository, commit):
    return docs_snapshot(repository, commit)


def blob(repository, commit, relative):
    return git(repository, "show", commit + ":" + relative)


def counts(text):
    current, results = None, {}
    for line in text.splitlines():
        match = re.search(r"Running .*\((?:.*[\\/])([^\\/]+)-[0-9a-f]+\.exe\)", line)
        if match:
            current = match[1]
        if line.lstrip().startswith("Doc-tests "):
            current = line.split("Doc-tests ", 1)[1].strip() + ":doc"
        match = re.match(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", line)
        if match:
            require(current and current not in results, "B4-5测试目标重复/缺失")
            results[current] = [int(v) for v in match.groups()]
    return results


def validate(root, repository, source, environment, records, logs, final_binding=True):
    for label, row in records.items():
        require(row["argv"] == COMMANDS[label], "B4-5运行命令不符：" + label)
        require(row["exit_code"] == 0, "B4-5门禁未通过：" + label)
        commit = row["docs_commit"]
        docs = docs_at(repository, commit)
        require(row["docs_commit_tree"] == docs and row["docs_before"] == row["docs_after"]
                and row["docs_before"]["paths"] == docs["paths"]
                and row["docs_before"]["canonical_sha256"] == docs["canonical_sha256"],
                "B4-5文档字节绑定不符")
        require(row["tools_commit"] == commit
                and row["runner_sha256"] == sha(blob(repository, commit, PREFIX + "/record.py"))
                and row["base_runner_sha256"] == sha(blob(repository, commit, "docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/record.py")),
                "B4-5采集器身份不符")
    if final_binding:
        commit = records["docs-final"]["docs_commit"]
        require(product_at(repository, commit) == product_at(repository, PRODUCT_COMMIT), "B4-5最终文档产品漂移")
        for path in root.iterdir():
            if path.suffix in {".py", ".rs", ".md"}:
                require(sha(canonical(path.read_bytes())) == sha(canonical(blob(repository, commit, PREFIX + "/" + path.name))),
                        "B4-5最终文档工具未定稿")
        docs_result = json.loads(logs["docs-final"])
        require(docs_result["passed"] is True
                and not any(row["severity"] == "error" for row in docs_result["diagnostics"]), "B4-5最终文档门禁失败")
        require(all(row["finished_utc"] <= records["docs-final"]["started_utc"]
                    for label, row in records.items() if label != "docs-final"), "B4-5最终文档门禁顺序不符")

    runtime = records["runtime-build"]["runtime_after"]
    require(records["runtime-build"]["runtime_before"]["exists"] is False and runtime["exists"] is True
            and runtime["path"] == environment["overrides"]["XIAO_RUNTIME_LIBRARY"]
            and runtime["bytes"] > 0 and re.fullmatch("[0-9a-f]{64}", runtime["sha256"]), "B4-5 Runtime冷构建身份不符")
    for label, row in records.items():
        if label != "runtime-build":
            require(row["runtime_before"] == row["runtime_after"] == runtime
                    and records["runtime-build"]["finished_utc"] <= row["started_utc"], "B4-5 Runtime身份漂移")

    history = {}
    for name in HISTORY:
        result = json.loads(logs["audit-" + name])
        if name == "b4-5-blocked":
            require(result.get("evidence_valid") is True and result.get("stage_passed") is False,
                    "B4-5旧blocked历史漂移")
        else:
            require(result.get("passed") == result.get("total") == 18 if name in {"b0", "b1"} else result.get("passed") is True,
                    "B4-5历史审计失败：" + name)
        history[name] = result

    regressions = counts(logs["regressions"])
    expected = {"20o_b2_b3_wrapper": [8, 0, 0], "20o_b2_b4_cmath_boundary": [4, 0, 0],
                "20o_b2_b4_cmath_library": [5, 0, 1], "20o_b2_b4_cmath_llvm": [4, 0, 1],
                "20o_b2_b4_numeric_range_routing": [3, 0, 0], "22c_vm_concurrency": [29, 0, 0],
                "n0_b_dynamic_native": [13, 0, 2], "20o_b2_b4_frontend": [3, 0, 0],
                "20o_b2_b4_cmath_vm": [7, 0, 0]}
    require(regressions == expected and set(regressions) == set(TARGETS), "B4-5当前回归目标/计数不符")
    workspace = counts(logs["workspace"])
    require(workspace and all(values[1] == 0 for values in workspace.values())
            and all(workspace.get(name) == values for name, values in expected.items())
            and "FAILED" not in logs["workspace"], "B4-5完整workspace门禁未通过")
    require(counts(logs["tac"]) == {"b4_cmath_tac": [2, 0, 0]}
            and "B4-2A-SUMMARY ids=34..48" in logs["tac"], "B4-5 TAC/字节码往返不符")
    text = logs["regressions"]
    require("FIX1-RANGE class=integer kind=numeric_range" in text
            and "expression=\"depth - 1\"" in text
            and "lowering=static kernel=cmath_pow llvm_pow_f64=false" in text
            and "class=mixed integer_range=dynamic float_pow=shared_runtime" in text
            and "sfloat_pow_keeps_existing_static_f32_intrinsic ... ok" in text, "B4-5 FIX.1路由证据缺失")
    imports = re.findall(r"B4-4-IMPORT path=((?:std|stdlib)\.cmath) function=(\w+) identity=stdlib\.cmath executable=true result=passed", text)
    require(len(imports) == len(set(imports)) == 30
            and {path for path, _ in imports} == {"std.cmath", "stdlib.cmath"}, "B4-5三十条导入路径不完整")
    require(len(re.findall(r"B4-4-REPLAY path=(?:std|stdlib)\.cmath source,xiaoc,xar=passed", text)) >= 2
            and "B4-4-SIGNATURES functions=15 paths=2 invalid_types=8" in text
            and "B4-4-LLVM-REMOVAL functions=15 paths=2" in text
            and "B4-4-CHILD cases=32" in text and "B4-4-ARCHIVE cases=48" in text
            and "B4-2B-VM-ERROR error=009 decoded_span=exact" in text
            and "B4-2B-VM-OWNERSHIP cases=45 errors=002" in text, "B4-5归档/错误/清理证据不完整")
    for label, target in [("native-matrix", "20o_b2_b4_cmath_llvm"), ("native-library", "20o_b2_b4_cmath_library"),
                          ("b1-vm", "20o_b1_probe"), ("b1-native", "20o_b1_probe")]:
        require(counts(logs[label]) == {target: [1, 0, 0]}, "B4-5精确入口计数不符：" + label)
    configs = re.findall(r"B4-3-MATRIX dynamic=(true|false) optimization=([03]).*cases=18318 error_cases=3153 pow_inputs=1075 bit_or_code=exact", logs["native-matrix"])
    require(len(configs) == 4 and set(configs) == {("true", "0"), ("true", "3"), ("false", "0"), ("false", "3")},
            "B4-5动态/静态O0/O3矩阵不完整")
    native_paths = re.findall(r"B4-4-NATIVE path=((?:std|stdlib)\.cmath) optimization=([03]).*result=passed", logs["native-library"])
    require(len(native_paths) == 4 and set(native_paths) == {("std.cmath", "0"), ("std.cmath", "3"), ("stdlib.cmath", "0"), ("stdlib.cmath", "3")},
            "B4-5双路径原生矩阵不完整")

    stages = load(root / "final/stage-reconciliation.json")
    require(stages["passed"] is True and stages["source_commit"] == SOURCE_COMMIT
            and stages["product_commit"] == PRODUCT_COMMIT and len(stages["stages"]) == len(STAGES), "B4-5阶段身份不符")
    for row, (stage, (name, commit)) in zip(stages["stages"], STAGES.items()):
        relative = "docs/DevDocs/assets/20o-b2-b/" + HISTORY[name]
        require(row["stage"] == stage and row["product_commit"] == commit and row["directory"] == relative
                and row["closeout_sha256"] == sha(blob(repository, SOURCE_COMMIT, relative + "/closeout.json"))
                and row["evidence_sha256"] == sha(blob(repository, SOURCE_COMMIT, relative + "/evidence.json"))
                and row["product_tree_sha256"] == identity(product_at(repository, commit)), "B4-5阶段身份不符")
    require(json.loads(logs["stages"])["report_sha256"] == sha((root / "final/stage-reconciliation.json").read_bytes()),
            "B4-5阶段报告回执不符")
    guards = load(root / "final/guards-report.json")
    guard_log = json.loads(logs["guards"])
    require(guards["passed"] == guards["total"] == len(guards["cases"]) == 28 and all(row["Passed"] for row in guards["cases"])
            and guards["scope"] == guard_log["scope"] == "simulated-cargo-output"
            and guard_log["source_commit"] == SOURCE_COMMIT
            and guard_log["report_sha256"] == sha((root / "final/guards-report.json").read_bytes())
            and guards["workflowSha256"] == guard_log["workflow_sha256"] == source["files"][".github/workflows/maintenance-regression.yml"]["raw_sha256"]
            and guards["scriptSha256"] == source["files"]["tools/gates/b1-probe-guards.ps1"]["raw_sha256"], "B4-5守卫来源不符")

    report_path = root / "final/pow-review.json"
    report = load(report_path)
    raw = blob(repository, SOURCE_COMMIT, MANIFEST)
    frozen = json.loads(raw)
    require(sha(raw) == MANIFEST_SHA and report["source_commit"] == SOURCE_COMMIT and report["passed"] is True
            and report["manifest_sha256"] == MANIFEST_SHA and len(report["rows"]) == report["cases"] == 1075
            and report["vm_cases"] == 6450 and report["summary"] == frozen["summary"]
            and report["rules"] == frozen["rules"], "B4-5pow对账不符")
    rows = [line.split("\t")[1:] for line in logs["pow"].splitlines() if line.startswith("B4-5-POW\t")]
    require(len(rows) == 1075 and len({r[0] for r in rows}) == 1075 and all(len(r) == 11 for r in rows),
            "B4-5pow明细不完整")
    actual = {r[0]: r[1:] for r in rows}
    for row, old in zip(report["rows"], frozen["rows"]):
        expected_value = old["new_runtime_float_power"]
        require(row == {"id": old["id"], "a_bits": old["a_bits"], "b_bits": old["b_bits"], "legacy": old["legacy_float_power"],
                        "runtime_power": expected_value, "cmath_pow": expected_value, "vm_operator_and_primitive_3_carriers": [expected_value] * 6,
                        "reference": old["reference"], "old_new_ulp": distance(old["legacy_float_power"], expected_value),
                        "new_reference_ulp": distance(expected_value, old["reference"]), "difference": old["difference"],
                        "frozen_rule": frozen_rule(value(old["a_bits"]), value(old["b_bits"]), expected_value)}
                and actual[row["id"]] == [old["a_bits"], old["b_bits"], *([expected_value] * 8)]
                and (row["new_reference_ulp"] is None or row["new_reference_ulp"] <= 4)
                and (row["difference"] != "value_bits" or row["old_new_ulp"] == 1), "B4-5pow对账不符")
    summary = json.loads(logs["pow"].splitlines()[-1])
    require(summary["report_sha256"] == sha(report_path.read_bytes()) and summary["report_path"] == "final/pow-review.json"
            and report["fixture_sha256"] == sha(blob(repository, SOURCE_COMMIT, "tests/fixtures/docs/cmath/pow-legacy-diff.tsv"))
            and report["probe_sha256"] == sha(blob(repository, records["pow"]["docs_commit"], PREFIX + "/pow_probe.rs")),
            "B4-5pow报告回执不符")
    tools = json.loads(logs["toolchain"])
    require(set(tools) == {"rustc", "cargo", "targets", "clang", "llvm-as", "llc"}
            and "1.96.0" in tools["rustc"]["stdout"] and all(re.fullmatch("[0-9a-f]{64}", row["sha256"]) for row in tools.values()),
            "B4-5工具链记录不符")
    return {"regressions": regressions, "workspace": workspace, "imports": 30, "native_matrix_configs": 4,
            "pow": {"cases": 1075, "vm_cases": 6450, "summary": report["summary"], "report_sha256": sha(report_path.read_bytes())},
            "guards": {"passed": 28, "total": 28, "scope": "simulated-cargo-output"},
            "history_audits": history, "runtime": runtime}
