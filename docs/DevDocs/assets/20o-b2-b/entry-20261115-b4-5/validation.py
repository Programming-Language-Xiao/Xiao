"""B4-5运行回执与报告明细核对；只读固定Git对象和归档字节。"""
from functools import lru_cache
import json
from pathlib import Path
import re
from record import PREFIX, canonical, docs_snapshot, fixed_files, git, identity, require, sha
from suite import COMMANDS, HISTORY
from capture import MANIFEST, MANIFEST_SHA, STAGES, distance, frozen_rule, value

CODE = "4b27b7f5bf8fdd0818aad040fbf11efdcc02fb67"
ENTRY = "d41b89b75da608e6bbda213601d6e68128893cbe"
FAILED_TARGET = "generic_value_copy_registers_weak_dependency_only_when_called"


@lru_cache(maxsize=None)
def docs_at(repository, commit):
    return docs_snapshot(Path(repository), commit)


def blob(repository, commit, relative):
    return git(repository, "show", commit + ":" + relative)


def counts(text):
    current, results = None, {}
    for line in text.splitlines():
        running = re.search(r"Running .*\((?:.*[\\/])([^\\/]+)-[0-9a-f]+\.exe\)", line)
        if running:
            current = running[1]
        if line.lstrip().startswith("Doc-tests "):
            current = line.split("Doc-tests ", 1)[1].strip() + ":doc"
        match = re.match(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", line)
        if match:
            require(current and current not in results, "B4-5测试目标重复或缺失")
            results[current] = [int(v) for v in match.groups()]
    return results


def validate(root, repository, source, records, logs, final_binding=True):
    require(source["revision"] == CODE and fixed_files(repository, CODE) == fixed_files(repository, ENTRY), "B4-5固定产品身份不符")
    runtime = records["runtime-build"]["runtime_after"]
    require(records["runtime-build"]["runtime_before"]["exists"] is False and runtime["exists"] is True
            and runtime["bytes"] > 0 and re.fullmatch(r"[0-9a-f]{64}", runtime["sha256"]), "B4-5 Runtime身份不符")
    for label, row in records.items():
        if label != "runtime-build":
            require(row["runtime_before"] == row["runtime_after"] == runtime
                    and records["runtime-build"]["finished_utc"] <= row["started_utc"], "B4-5 Runtime身份不符")
        if label == "workspace-failure-exact":
            argv = ["cargo", "+1.96.0", "test", "--manifest-path", "core/rust/Cargo.toml", "--locked", "-p", "xiao-driver", "--test",
                    "n0_b_dynamic_native", "--", "--exact", "generic_value_copy_registers_weak_dependency_only_when_called", "--nocapture"]
        else:
            key = "final-docs" if label.startswith("docs-final") else "pow" if label in {"pow-final", "pow-retry"} else label.removesuffix("-retry")
            argv = COMMANDS[key]
        require(row["argv"] == argv, "B4-5运行命令不符")
        commit = row["docs_commit"]
        docs = docs_at(str(repository), commit)
        require(row["docs_commit_tree"] == docs and row["docs_before"] == row["docs_after"]
                and row["docs_before"]["paths"] == docs["paths"] and row["docs_before"]["canonical_sha256"] == docs["canonical_sha256"],
                "B4-5文档字节绑定不符")
        require(row["runner_sha256"] == sha(blob(repository, commit, PREFIX + "/record.py"))
                and row["base_runner_sha256"] == sha(blob(repository, commit, "docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/record.py")), "B4-5采集器身份不符")
    require({r["log_ref"] for r in records.values()} == {p.relative_to(root).as_posix() for p in (root / "logs").rglob("*") if p.is_file()},
            "B4-5存在未引用日志对象")
    if final_binding:
        latest = max((r for label, r in records.items() if label.startswith("docs-final")), key=lambda r: r["started_utc"])
        for path in root.iterdir():
            if path.suffix in {".md", ".py", ".rs"}:
                require(sha(canonical(path.read_bytes())) == sha(canonical(blob(repository, latest["docs_commit"], PREFIX + "/" + path.name))),
                        "B4-5最终文档工具未定稿")
        require(fixed_files(repository, latest["docs_commit"]) == fixed_files(repository, CODE), "B4-5最终文档快照产品漂移")
    for name in HISTORY:
        result = json.loads(logs["audit-" + name + "-retry"])
        require(result.get("passed") == result.get("total") == 18 if name in {"b0", "b1"} else result.get("passed") is True,
                "B4-5历史审计失败：" + name)
    require(FAILED_TARGET in logs["workspace"] and FAILED_TARGET in logs["workspace-failure-exact"]
            and "left: false" in logs["workspace"] and "right: true" in logs["workspace"], "B4-5workspace失败证据不符")
    for label, target in [("native-library", "20o_b2_b4_cmath_library"), ("native-matrix", "20o_b2_b4_cmath_llvm"), ("b1-vm", "20o_b1_probe"), ("b1-native", "20o_b1_probe")]:
        require(counts(logs[label]) == {target: [1, 0, 0]}, "B4-5精确执行计数不符")
    require(len(re.findall(r"B4-3-MATRIX dynamic=(?:true|false) optimization=[03].*cases=18318 error_cases=3153 pow_inputs=1075 bit_or_code=exact", logs["native-matrix"])) == 4
            and len(re.findall(r"B4-4-NATIVE path=(?:std|stdlib)\.cmath optimization=[03].*result=passed", logs["native-library"])) == 4, "B4-5原生矩阵不完整")
    read = lambda name: json.loads((root / "final" / name).read_text(encoding="utf-8"))
    stages = read("stage-reconciliation.json")
    require(stages["source_commit"] == CODE and len(stages["stages"]) == 5, "B4-5阶段身份不符")
    for row, (stage, (name, commit)) in zip(stages["stages"], STAGES.items()):
        relative = "docs/DevDocs/assets/20o-b2-b/" + HISTORY[name]
        require(row["stage"] == stage and row["product_commit"] == commit
                and row["closeout_sha256"] == sha(blob(repository, CODE, relative + "/closeout.json"))
                and row["evidence_sha256"] == sha(blob(repository, CODE, relative + "/evidence.json"))
                and row["product_tree_sha256"] == identity(fixed_files(repository, commit)), "B4-5阶段身份不符")
    require(json.loads(logs["stages-retry"])["report_sha256"] == sha((root / "final/stage-reconciliation.json").read_bytes()), "B4-5阶段身份不符")
    guards = read("guards-report.json")
    guard_log = json.loads(logs["guards-retry"])
    require(guards["scope"] == guard_log["scope"] == "simulated-cargo-output" and guard_log["source_commit"] == CODE
            and len(guards["cases"]) == 28 and guard_log["report_sha256"] == sha((root / "final/guards-report.json").read_bytes())
            and guards["workflowSha256"] == guard_log["workflow_sha256"] == source["files"][".github/workflows/maintenance-regression.yml"]["raw_sha256"]
            and guards["scriptSha256"] == source["files"]["tools/gates/b1-probe-guards.ps1"]["raw_sha256"], "B4-5守卫来源不符")
    report = read("pow-review.json")
    raw = blob(repository, CODE, MANIFEST)
    frozen = json.loads(raw)
    require(sha(raw) == MANIFEST_SHA and report["source_commit"] == CODE and len(report["rows"]) == 1075, "B4-5pow对账不符")
    rows = [line.split("\t")[1:] for line in logs["pow-final"].splitlines() if line.startswith("B4-5-POW\t")]
    require(len(rows) == 1075 and len({r[0] for r in rows}) == 1075, "B4-5pow明细不完整")
    by_id = {r[0]: r[1:] for r in rows}
    for row, old in zip(report["rows"], frozen["rows"]):
        expected = old["new_runtime_float_power"]
        require(row["id"] == old["id"] and by_id[row["id"]] == [old["a_bits"], old["b_bits"], *([expected] * 8)]
                and row == {"id": old["id"], "a_bits": old["a_bits"], "b_bits": old["b_bits"], "legacy": old["legacy_float_power"],
                            "runtime_power": expected, "cmath_pow": expected, "vm_operator_and_primitive_3_carriers": [expected] * 6,
                            "reference": old["reference"], "old_new_ulp": distance(old["legacy_float_power"], expected),
                            "new_reference_ulp": distance(expected, old["reference"]), "difference": old["difference"],
                            "frozen_rule": frozen_rule(value(old["a_bits"]), value(old["b_bits"]), expected)}, "B4-5pow对账不符")
    require(json.loads(logs["pow-final"].splitlines()[-1])["report_sha256"] == sha((root / "final/pow-review.json").read_bytes()), "B4-5pow报告摘要不符")
