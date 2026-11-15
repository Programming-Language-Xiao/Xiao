"""FIX.1业务证据核验：归因、允许路径、两类路由、完整回归与pow差异。"""
from collections import Counter
from functools import lru_cache
import json
import math
from pathlib import Path
import re
from record import git, fixed_files, identity, load, require, sha
from capture import HISTORY, STAGES, FIX_PATHS, FIX_COMMIT, ENTRY_COMMIT, MANIFEST, MANIFEST_SHA, distance, frozen_rule, value

PRODUCT_COMMIT = "cfa79381bbae027cec11136ca380e82f66720c34"
INTRODUCED_COMMIT = "4be30eafd36959bcdef08ac47987d9c07887cdcc"
TARGET = "generic_value_copy_registers_weak_dependency_only_when_called"


@lru_cache(maxsize=None)
def git_blob(repository, commit, path):
    return git(repository, "show", commit + ":" + path)


def test_counts(text):
    result, current = {}, None
    for line in text.splitlines():
        match = re.search(r"Running .*\((?:.*[\\/])([^\\/]+)-[0-9a-f]+\.exe\)", line)
        if match:
            current = match[1]
        doc = re.match(r"\s*Doc-tests (.*)", line)
        if doc:
            current = doc[1].strip() + ":doc"
        summary = re.match(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", line)
        if summary:
            require(current and current not in result, "FIX.1测试汇总目标重复/缺失")
            result[current] = tuple(map(int, summary.groups()))
    return result


def expected_command(variant, label, repository):
    if label == "exact":
        return ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-driver",
                "--test", "n0_b_dynamic_native", "--", "--exact", TARGET, "--nocapture"]
    if label.startswith("capture-"):
        action = next(name for name in ["toolchain", "stages", "guards", "pow"] if label.startswith("capture-" + name + "-"))
        return ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/entry-20261115-b4-3-fix1/capture.py", action]
    if label.startswith("audit-"):
        key = label.removeprefix("audit-")
        key = key.removesuffix("-retry")
        directory = {"b4-3": "entry-20261113-b4-3", "b4-4": "entry-20261115-b4-4", "b4-5": "entry-20261115-b4-5"}[key]
        return ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/" + directory + "/audit.py"]
    from suite import COMMANDS
    return COMMANDS[label]


def validate_receipt_command(variant, label, row, repository):
    if variant in {"b34", "b401", "introduced"} and label == "runtime-build":
        expected = ["cargo", "+1.96.0", "build", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-runtime", "--release"]
    elif variant in {"b34", "b401", "introduced"}:
        expected = expected_command(variant, label, repository)
    else:
        expected = expected_command(variant, label, repository)
    require(row["argv"] == expected, "FIX.1采集命令不符：" + variant + "/" + label)
    require(row["exit_code"] in {0, 1, 2, 101}, "FIX.1退出码非法")


def validate(root, repository, sources, environments, records, logs):
    require(sources["b34"]["revision"] == "7d8b85e4d8c588b40ec76d641edc865481295032"
            and sources["b401"]["revision"] == "c7a45e10b76b26915499717563761ed20fda6e6c"
            and sources["introduced"]["revision"] == INTRODUCED_COMMIT, "FIX.1归因提交身份不符")
    for label, success in [("b34", True), ("b401", True), ("introduced", False)]:
        text = logs[label, "exact"]
        match = re.search(r"test " + TARGET + r" \.\.\. (ok|FAILED)", text)
        require(match and (match[1] == "ok") == success, "FIX.1三点直接归因结果不符：" + label)
    require("left: false" in logs["introduced", "exact"] and "right: true" in logs["introduced", "exact"], "FIX.1引入提交失败形态不符")

    for variant in ["b34", "b401", "introduced", "fixed"]:
        environment = environments[variant]
        task = Path(environment["task_root"])
        for key, suffix in [("CARGO_TARGET_DIR", "target"), ("XIAO_HOME", "home"), ("TEMP", "temp"),
                            ("TMP", "temp"), ("XIAO_RUNTIME_LIBRARY", "target/release/xiao_runtime.lib")]:
            require(Path(environment["overrides"][key]) == task / suffix, "FIX.1隔离目录不符：" + key)
        runtime = records[variant, "runtime-build"]["runtime_after"]
        require(records[variant, "runtime-build"]["runtime_before"]["exists"] is False and runtime["exists"]
                and runtime["bytes"] > 0 and len(runtime["sha256"]) == 64, "FIX.1 Runtime冷构建身份不符")
        for (run_variant, label), row in records.items():
            if run_variant == variant and label != "runtime-build":
                require(row["runtime_before"] == row["runtime_after"] == runtime
                        and records[variant, "runtime-build"]["finished_utc"] <= row["started_utc"], "FIX.1运行复用了不同Runtime")

    fixed = sources["fixed"]
    require(fixed["revision"] == "be95d0d53cec6f07cb1719e426b4173867b2258e"
            and fixed_files(repository, fixed["revision"]) == fixed_files(repository, PRODUCT_COMMIT), "FIX.1固定产品快照漂移")
    changed = set(git(repository, "diff", "--name-only", ENTRY_COMMIT, PRODUCT_COMMIT, "--",
                      *load(root / "fixed/source.json")["files"].keys()).decode().splitlines())
    require(changed == FIX_PATHS, "FIX.1超出两条产品/测试允许路径")

    route = logs["fixed", "routing"]
    require('FIX1-RANGE class=integer kind=numeric_range' in route and 'expression="depth - 1"' in route
            and "FIX1-ROUTE class=float-pow" in route
            and "lowering=static kernel=cmath_pow llvm_pow_f64=false" in route
            and "class=mixed integer_range=dynamic float_pow=shared_runtime" in route, "FIX.1范围分类/路由回归不符")
    require("sfloat_pow_keeps_existing_static_f32_intrinsic ... ok" in route
            and test_counts(logs["fixed", "routing"]) == {"20o_b2_b4_numeric_range_routing": (3, 0, 0)}, "FIX.1sfloat/static路由回归不符")
    require(test_counts(logs["fixed", "exact"]) == {"n0_b_dynamic_native": (1, 0, 0)}
            and re.search(r"test " + TARGET + r" \.\.\. ok", logs["fixed", "exact"]), "FIX.1原阻塞单项仍失败")
    regression_counts = test_counts(logs["fixed", "regressions"])
    expected_targets = {"n0_b_dynamic_native": (15, 0, 2), "20o_b2_b4_numeric_range_routing": (3, 0, 0),
        "20o_b2_b4_cmath_llvm": (4, 0, 1), "20o_b2_b4_cmath_library": (5, 0, 1),
        "20o_b2_b4_cmath_boundary": (4, 0, 0), "20o_b2_b3_wrapper": (8, 0, 0), "22c_vm_concurrency": (29, 0, 0)}
    require(regression_counts == expected_targets, "FIX.1定向回归目标计数不符")
    require(test_counts(logs["fixed", "codegen-tests"]).get("xiao_codegen_llvm", (0, 1))[1] == 0, "LLVM单测失败")
    require(records["fixed", "workspace"]["exit_code"] == 0
            and all(records["fixed", label]["exit_code"] == 0 for label in ["clippy", "fmt", "lock"]), "FIX.1workspace门禁未全部通过")
    docs_result = json.loads(logs["fixed", "docs"])
    require(records["fixed", "docs"]["exit_code"] == 0 and docs_result["passed"] is True
            and not any(row["severity"] == "error" for row in docs_result["diagnostics"]), "FIX.1最终docs门禁失败")
    require(records["fixed", "capture-toolchain-retry"]["exit_code"] == 0
            and records["fixed", "capture-stages-final2"]["exit_code"] == 0
            and records["fixed", "capture-guards-final"]["exit_code"] == 0
            and records["fixed", "capture-pow-receipted"]["exit_code"] == 0, "FIX.1最终报告生成步骤缺失")
    for label, row in records.items():
        validate_receipt_command(label[0], label[1], row, repository)

    matrix = re.findall(r"B4-3-MATRIX dynamic=(true|false) optimization=([03]).*cases=(\d+) error_cases=(\d+) pow_inputs=(\d+) bit_or_code=exact", logs["fixed", "native-matrix"])
    require(len(matrix) == 4 and {(a, b) for a, b, *_ in matrix} == {("true", "0"), ("true", "3"), ("false", "0"), ("false", "3")}
             , "FIX.1 B4-3矩阵配置不完整")
    require(all(row[2:] == ("18318", "3153", "1075") for row in matrix), "FIX.1 B4-3矩阵计数不符")
    require(len(re.findall(r"B4-4-NATIVE path=(?:std|stdlib)\.cmath optimization=[03].*result=passed", logs["fixed", "native-library"])) == 4,
            "FIX.1 B4-4双路径原生矩阵缺项")
    for label, target in [("b1-vm", "20o_b1_probe"), ("b1-native", "20o_b1_probe")]:
        require(test_counts(logs["fixed", label]) == {target: (1, 0, 0)}, "FIX.1 B1精确入口失败")

    guards = load(root / "fixed/guards-report.json")
    guard_summary = json.loads(logs["fixed", "capture-guards-final"])
    require(guards["passed"] == guards["total"] == 28 and all(row["Passed"] for row in guards["cases"])
            and guards["scope"] == "simulated-cargo-output" and guard_summary["report_sha256"] == sha((root / "fixed/guards-report.json").read_bytes()),
            "FIX.1 28/28计数守卫不符")
    stage_report = load(root / "fixed/stage-reconciliation.json")
    require(stage_report["passed"] is True and stage_report["source_commit"] == fixed["revision"]
            and stage_report["stages"][-1]["product_commit"] == PRODUCT_COMMIT, "FIX.1阶段对账不符")
    pow_report = load(root / "fixed/pow-review.json")
    manifest_raw = git(repository, "show", PRODUCT_COMMIT + ":" + MANIFEST)
    manifest = json.loads(manifest_raw)
    require(sha(manifest_raw) == MANIFEST_SHA and pow_report["passed"] is True
            and len(pow_report["rows"]) == 1075 and pow_report["summary"] == manifest["summary"] ==
            {"same": 1011, "value_or_error_boundary": 40, "value_bits": 24}
            and pow_report["vm_cases"] == 6450, "FIX.1pow矩阵汇总不符")
    require(all(row["old_new_ulp"] == 1 for row in pow_report["rows"] if row["difference"] == "value_bits")
            and all(row["frozen_rule"] == frozen_rule(value(row["a_bits"]), value(row["b_bits"]), row["runtime_power"])
                    and row["new_reference_ulp"] == distance(row["runtime_power"], row["reference"])
                    and (row["new_reference_ulp"] is None or row["new_reference_ulp"] <= 4) for row in pow_report["rows"]),
            "FIX.1pow逐项F-2/F-9/ULP规则不符")
    require(test_counts(logs["fixed", "native-matrix"]) == {"20o_b2_b4_cmath_llvm": (1, 0, 0)}
            and len(re.findall(r"B4-3-MATRIX dynamic=(?:true|false) optimization=[03].*cases=18318 error_cases=3153 pow_inputs=1075 bit_or_code=exact", logs["fixed", "native-matrix"])) == 4,
            "FIX.1原生矩阵门禁未通过")
    history = {}
    for label in ["b4-3", "b4-4", "b4-5"]:
        output = json.loads(logs["fixed", "audit-" + label])
        history[label] = output
    require(history["b4-3"]["passed"] is True and history["b4-4"]["passed"] is True
            and history["b4-5"]["evidence_valid"] is True and history["b4-5"]["stage_passed"] is False,
            "历史B4审计结果不符")
    return {"regression_counts": {k: list(v) for k, v in regression_counts.items()}, "pow_cases": 1075,
            "pow_summary": pow_report["summary"], "native_matrix_configs": 4, "guards": "28/28 simulated",
            "history_audits": history}
