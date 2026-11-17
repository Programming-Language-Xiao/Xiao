"""由真实运行回执生成最终统计；不改写日志及历史取证。"""
import json
import re
import sys

sys.dont_write_bytecode = True
import record

ROOT, REPO, TASK = record.ROOT, record.REPOSITORY, record.TASK


def raw_object(reference):
    import gzip
    packed = (ROOT / reference["path"]).read_bytes()
    record.require(record.sha(packed) == reference["sha256"], "日志对象摘要错误")
    return gzip.decompress(packed) if reference["encoding"] == "gzip" else packed


def summaries(raw):
    pattern = (r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; "
               r"(\d+) measured; (\d+) filtered out; finished in ([0-9.]+)s")
    return [{"passed": int(row[0]), "failed": int(row[1]), "ignored": int(row[2]),
             "measured": int(row[3]), "filtered": int(row[4]), "seconds": float(row[5])}
            for row in re.findall(pattern, raw.decode("utf-8"))]


def main():
    output = ROOT / "result.json"
    record.require(not output.exists(), "最终结果清单禁止覆盖")
    workspace = json.loads((ROOT / "workspace-final.json").read_text(encoding="utf-8"))
    record.require(workspace["exit_code"] == 0 and workspace["source_unchanged"],
                   "受控完整workspace未通过或源码发生变化")
    raw = raw_object(workspace["log"])
    rows = summaries(raw)
    total = {key: sum(row[key] for row in rows) for key in ("passed", "failed", "ignored")}
    record.require(len(rows) == 191 and total == {"passed": 1754, "failed": 0, "ignored": 55},
                   "完整workspace统计不符")
    source = json.loads(raw_object(workspace["source_before"]))
    record.require(source == record.source(), "受控运行后产品源码身份变化")
    code_commit = record.git("rev-parse", "HEAD").decode().strip()
    changed = record.git("diff", "--name-only", "cba346fc6f3039cbc76889f4907ea8539c38ff48",
                         code_commit, "--", *record.PRODUCT_ROOTS).decode().splitlines()
    record.require(changed == ["core/rust/crates/xiao-driver/tests/20o_b1_imp_resources.rs"],
                   "产品代码提交范围不止获批测试文件")
    single_raw = (TASK / "url-unicode-single-r1.log").read_bytes()
    single_rows = summaries(single_raw)
    resource_raw = (TASK / "resources-final.log").read_bytes()
    resource_rows = summaries(resource_raw)
    record.require(len(single_rows) == 1 and single_rows[0]["passed"] == 1
                   and single_rows[0]["failed"] == 0 and single_rows[0]["seconds"] < 60,
                   "独立Unicode预算用例未在原窗口通过")
    record.require(len(resource_rows) == 1 and resource_rows[0]["passed"] == 18
                   and resource_rows[0]["failed"] == 0 and resource_rows[0]["ignored"] == 0,
                   "独立默认资源目标未18/18通过")
    for test in ("url_decoding_reaches_the_unicode_result_budget",
                 "url_decoding_resets_utf8_state_between_escaped_codepoints"):
        marker = ("test " + test + " ... ok").encode()
        record.require(marker in raw and marker in resource_raw, "完整日志缺少用例：" + test)
    first_raw = (TASK / "workspace-complete-r2.log").read_bytes()
    first_rows = summaries(first_raw)
    record.require(len(first_rows) == 191 and all(row["failed"] == 0 for row in first_rows),
                   "较早完整运行的日志不完整")
    result = {
        "scope": "获批优化URL资源用例；不代表产品URL性能改善或B5阶段签收",
        "authorization": "星崽：优化用例，不改窗口或预算",
        "baseline_commit": "cba346fc6f3039cbc76889f4907ea8539c38ff48",
        "code_commit": code_commit, "allowed_code_changes": changed,
        "runtime_abi": 65555, "codegen_version": 22, "stdlib_interface_version": 1,
        "workspace": {"receipt": "workspace-final.json", "target_count": len(rows),
                      **total, "test_case_count": sum(total.values()),
                      "exit_code": workspace["exit_code"],
                      "elapsed_seconds": workspace["elapsed_seconds"], "pid": workspace["pid"]},
        "resource_target": {"passed": 18, "failed": 0, "ignored": 0,
                            "test_seconds": resource_rows[0]["seconds"],
                            "command_seconds_observed": 140.9214468,
                            "log": record.store(resource_raw),
                            "scope": "较早独立默认并发资源目标；无PID，仅作补充"},
        "unicode_budget_single": {"passed": 1, "failed": 0,
                                  "test_seconds": single_rows[0]["seconds"],
                                  "command_seconds_observed": 6.4054744,
                                  "log": record.store(single_raw),
                                  "scope": "较早单项诊断；不替代完整默认workspace"},
        "earlier_workspace": {"target_count": len(first_rows),
                              "passed": sum(row["passed"] for row in first_rows),
                              "failed": 0, "ignored": sum(row["ignored"] for row in first_rows),
                              "command_seconds_observed": 622.4126253,
                              "log": record.store(first_raw),
                              "scope": "无PID的较早完整运行；权威结论引用workspace-final.json"},
        "source": workspace["source_before"],
        "input_bytes": len(("🚀" * 16383 + "%F0%9F%9A%80").encode()),
        "expected_utf8_bytes": 65536, "expected_codepoints": 16384,
        "separate_utf8_reset_codepoints": 256,
        "driver_timeout_seconds": 60,
        "url_implementation_changed": False, "resource_limits_changed": False,
        "workflow_or_f10_thresholds_changed": False,
        "b5_historical_1_19_21_evidence_rewritten": False,
        "b5_closed": False, "b6_started": False,
        "multi_platform_or_ci_passed": False,
    }
    record.write_new(output, result)
    print(json.dumps({"passed": True, "workspace": result["workspace"],
                      "code_commit": code_commit}, ensure_ascii=False))


if __name__ == "__main__":
    main()
