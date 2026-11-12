"""FIX1 只读审计：历史归属、修复源码、原始执行与阶段退出逐项核对。"""
import argparse
import json
from pathlib import Path, PureWindowsPath
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True

from record import (PRODUCT_ROOTS, REPOSITORY, ROOT, fixed_files, git, identity, load,
                    require, sha, write_json)

ENTRY = "2f862f51"
CODE = "c4610211"
VERSIONS = {"b34": "7d8b85e4", "b401": "c7a45e10", "fixed": CODE}
ALLOWED = {
    "core/rust/crates/xiao-driver/src/module_loader.rs",
    "core/rust/crates/xiao-driver/src/archive_closure.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_boundary.rs",
}
CARGO = ["cargo", "+1.96.0"]
LOCKED = ["--locked", "--manifest-path", "core/rust/Cargo.toml"]
CONCURRENCY = CARGO + ["test"] + LOCKED + ["-p", "xiao-driver", "--test", "22c_vm_concurrency", "--", "--nocapture"]
RUNS = {
    ("b34", "concurrency"): CONCURRENCY,
    ("b401", "concurrency"): CONCURRENCY,
    ("fixed", "regressions"): CARGO + ["test"] + LOCKED + ["-p", "xiao-driver", "--test", "22c_vm_concurrency", "--test", "20o_b2_b4_cmath_boundary", "--test", "20o_b2_b4_frontend", "--test", "20o_b2_b3_wrapper", "--", "--nocapture"],
    ("fixed", "bytecode"): CARGO + ["test"] + LOCKED + ["-p", "xiao-bytecode", "--", "--nocapture"],
    ("fixed", "driver_lib"): CARGO + ["test"] + LOCKED + ["-p", "xiao-driver", "--lib"],
    ("fixed", "clippy"): CARGO + ["clippy"] + LOCKED + ["-p", "xiao-driver", "-p", "xiao-bytecode", "--all-targets", "--", "-D", "warnings"],
    ("fixed", "fmt"): CARGO + ["fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"],
    ("fixed", "toolchain"): ["rustc", "+1.96.0", "-vV"],
    ("fixed", "targets"): ["rustup", "+1.96.0", "target", "list", "--installed"],
    ("fixed", "docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "final_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "usedocs"): ["bun", "run", "check:usedocs"],
    ("fixed", "layout"): ["bun", "run", "check:layout"],
    ("fixed", "final_docs_closeout"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "final_docs_audit"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "usedocs_closeout"): ["bun", "run", "check:usedocs"],
    ("fixed", "layout_closeout"): ["bun", "run", "check:layout"],
    ("fixed", "sealed_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "evidence_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "final_evidence_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "release_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "sealed_usedocs"): ["bun", "run", "check:usedocs"],
    ("fixed", "sealed_layout"): ["bun", "run", "check:layout"],
}
for label, directory in {
    "b1_history": "entry-20261108-b1", "b2_history": "entry-20261108-b2",
    "b3_history": "entry-20261109-b3-cabi", "b34_history": "entry-20261110-b3-wrapper-final",
    "b401_history": "entry-20261110-b4-0-1",
}.items():
    RUNS["fixed", label] = ["python", "-B", "-X", "utf8", f"docs/DevDocs/assets/20o-b2-b/{directory}/audit.py"]


def inventory(root):
    return {path.relative_to(root).as_posix(): {"bytes": path.stat().st_size, "sha256": sha(path.read_bytes())}
            for path in sorted(root.rglob("*")) if path.is_file() and path.name != "evidence.json"
            and "__pycache__" not in path.parts}


def all_bytes(root):
    return {path.relative_to(root).as_posix(): sha(path.read_bytes()) for path in root.rglob("*") if path.is_file()}


def totals(log):
    rows = re.findall(r"^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", log, re.M)
    require(bool(rows), "缺少 Rust 测试结果")
    return tuple(sum(int(row[index]) for row in rows) for index in range(3))


def verify(root, repository):
    sources, environments = {}, {}
    for variant, revision in VERSIONS.items():
        directory = root / variant
        source, env = load(directory / "source.json"), load(directory / "environment.json")
        require(source["revision"] == git(repository, "rev-parse", revision).decode().strip(), "固定来源提交不符：" + variant)
        require({path: row["canonical_sha256"] for path, row in source["files"].items()}
                == fixed_files(repository, revision), "固定 Git 产品树不符：" + variant)
        require(source["tree_sha256"] == identity(source["files"]), "来源内部摘要不符：" + variant)
        task = PureWindowsPath(env["task_root"])
        require(source["source_root"] == env["source_root"], "执行源码根不符：" + variant)
        for key in ["CARGO_TARGET_DIR", "XIAO_HOME", "TEMP", "TMP", "XIAO_RUNTIME_LIBRARY"]:
            path = PureWindowsPath(env["overrides"][key])
            require(path.is_relative_to(task) and path != task, "执行目录逃出任务根：" + variant)
        require(env["overrides"]["XIAO_STDLIB_ROOT"] == str(PureWindowsPath(source["source_root"]) / "stdlib"), "stdlib 来源不符：" + variant)
        require(env["overrides"]["XIAO_B1_F10"] == env["overrides"]["XIAO_B2_A_PERF"] == "0", "性能开关不符")
        require(env["removed_environment_prefixes"] == ["XIAO_", "RUST_BIGDECIMAL_"]
                and env["runtime_staticlib_built_or_used"] is False, "执行环境边界不符")
        sources[variant], environments[variant] = source, env
    for key in ["CARGO_TARGET_DIR", "XIAO_HOME", "TEMP", "XIAO_RUNTIME_LIBRARY"]:
        require(len({env["overrides"][key] for env in environments.values()}) == 3, "历史与修复环境被复用")
    changed = set(git(repository, "diff", "--name-only", ENTRY, CODE, "--", *PRODUCT_ROOTS).decode().splitlines())
    require(changed == ALLOWED, "FIX1 超出已授权产品范围")
    original_22c = "core/rust/crates/xiao-driver/tests/22c_vm_concurrency.rs"
    require(len({sources[variant]["files"][original_22c]["canonical_sha256"] for variant in sources}) == 1,
            "既有 22c 断言被改动")
    logs, records = {}, {}
    for (variant, label), argv in RUNS.items():
        directory = root / variant
        record = load(directory / (label + ".json"))
        require(record["argv"] == argv and record["label"] == label, "命令身份不符：" + label)
        require(record["cwd"] == sources[variant]["source_root"] and record["pid"] > 0
                and record["started_utc"] < record["finished_utc"], "PID/目录/时间不符：" + label)
        require(record["source_unchanged"] is True and record["source_before"] == record["source_after"]
                == sources[variant]["tree_sha256"], "执行期间源码或锁漂移：" + label)
        require(record["environment_sha256"] == sha((directory / "environment.json").read_bytes())
                and record["runner_sha256"] == sha((root / "record.py").read_bytes()), "执行器/环境摘要不符：" + label)
        require(record["log"] == label + ".log", "日志路径不符：" + label)
        raw = (directory / record["log"]).read_bytes()
        require(record["log_sha256"] == sha(raw) and record["log_bytes"] == len(raw), "原始日志身份不符：" + label)
        require(record["exit_code"] == (101 if variant == "b401" else 0), "执行退出码不符：" + label)
        logs[variant, label], records[variant, label] = raw.decode("utf-8"), record
    require(totals(logs["b34", "concurrency"]) == (29, 0, 0), "B3-4 直接复跑未通过 29/29")
    require(totals(logs["b401", "concurrency"]) == (27, 2, 0), "B4-1 直接复跑不符 27/29")
    failed = re.findall(r"^test (\w+) \.\.\. FAILED$", logs["b401", "concurrency"], re.M)
    require(set(failed) == {"isolated_child_preserves_cmath_alias_snapshot_identity", "planned_cmath_xar_is_rejected_before_output"}, "历史失败归属不符")
    regressions = logs["fixed", "regressions"]
    require(totals(regressions) == (43, 0, 0), "修复后回归未通过 43 项")
    for count in ["29 passed; 0 failed; 0 ignored;", "3 passed; 0 failed; 0 ignored;",
                  "8 passed; 0 failed; 0 ignored;"]:
        require(count in regressions, "指定回归目标计数缺失：" + count)
    for text in ["test result: ok. 29 passed; 0 failed; 0 ignored;", "test result: ok. 8 passed; 0 failed; 0 ignored;",
                 "FIX1-CHILD cases=32 code=X06-RUNTIME-012 loaded=false output=false partial_release=false",
                 "FIX1-ARCHIVE cases=48 code=X11-PROTOCOL-007 output=false residue=false",
                 "FIX1-ISOLATION project_cmath=success unimported_cmath=excluded archive=success"]:
        require(text in regressions, "缺少边界运行证据：" + text)
    require(regressions.count("executable=false result=planned-rejected") == 30, "30 条导入后调用拒绝不完整")
    require(totals(logs["fixed", "bytecode"]) == (65, 0, 0), "字节码全包未通过 65 项")
    driver_counts = totals(logs["fixed", "driver_lib"])
    require(driver_counts == (64, 0, 1), "驱动器单元测试结果不符 64/0/1")
    require(json.loads(logs["fixed", "docs"])["passed"] is True, "文档门禁未通过")
    require("host: x86_64-pc-windows-msvc" in logs["fixed", "toolchain"]
            and "rustc 1.96.0" in logs["fixed", "toolchain"], "工具链身份不符")
    require(logs["fixed", "targets"].splitlines() == ["x86_64-pc-windows-msvc"], "跨平台目标证据超出本机 Windows")
    require(json.loads(logs["fixed", "final_docs"])["passed"] is True
            and "通过：未发现 A0 规则问题。" in logs["fixed", "usedocs"]
            and "通过：未发现 A0 规则问题。" in logs["fixed", "layout"], "收口后的文档门禁未全通过")
    repo_result = json.loads(logs["fixed", "final_docs"])
    require(all(item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
                for item in repo_result["diagnostics"]), "收口门禁出现新警告或错误")
    for label in ["final_docs_closeout", "final_docs_audit"]:
        repo_closeout = json.loads(logs["fixed", label])
        require(repo_closeout["passed"] is True and all(
            item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
            for item in repo_closeout["diagnostics"]), "最终状态快照的文档门禁失败或出现新诊断：" + label)
    sealed_repo = json.loads(logs["fixed", "sealed_docs"])
    require(sealed_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in sealed_repo["diagnostics"]), "最终冻结状态文档门禁失败或出现新诊断")
    require("通过：未发现 A0 规则问题。" in logs["fixed", "sealed_usedocs"]
            and "通过：未发现 A0 规则问题。" in logs["fixed", "sealed_layout"], "最终冻结 UseDocs/layout 门禁失败")
    evidence_repo = json.loads(logs["fixed", "evidence_docs"])
    require(evidence_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in evidence_repo["diagnostics"]), "实际证据目录定稿后的文档门禁失败或出现新诊断")
    final_evidence_repo = json.loads(logs["fixed", "final_evidence_docs"])
    require(final_evidence_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in final_evidence_repo["diagnostics"]), "最终证据文档门禁失败或出现新诊断")
    release_repo = json.loads(logs["fixed", "release_docs"])
    require(release_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in release_repo["diagnostics"]), "最终 README 文档门禁失败或出现新诊断")
    require("通过：未发现 A0 规则问题。" in logs["fixed", "usedocs_closeout"]
            and "通过：未发现 A0 规则问题。" in logs["fixed", "layout_closeout"], "最终 UseDocs/layout 门禁失败")
    for label in ["b1_history", "b2_history", "b3_history", "b34_history", "b401_history"]:
        require(json.loads(logs["fixed", label])["passed"] in (True, 18), "固定历史审计未通过：" + label)
    attribution = {"introduced_by": sources["b401"]["revision"], "last_passing": sources["b34"]["revision"],
                   "old_22c": "29/29", "regressed_22c": "27/29", "fixed_22c": "29/29",
                   "reason": "B4-1 接通 cmath 类型检查后，包装编译失败不再构成执行/归档拒绝；FIX1 增加显式 planned 门控。"}
    result = {"passed": True, "fix1_closed": True, "b4_2a_gate_restored": True,
              "b4_2b_started": False, "b4_closed": False, "language_available": False,
              "product_commit": sources["fixed"]["revision"], "entry_commit": git(repository, "rev-parse", ENTRY).decode().strip(),
              "runtime_abi": "1.18", "codegen_version": 20, "cmath": "planned", "old_721_executed": False,
              "tests": {"concurrency": 29, "bytecode": 65, "frontend": 3, "wrapper": 8,
                        "boundary_tests": 3, "child_cases": 32, "archive_cases": 48, "driver_lib": driver_counts[0], "driver_lib_existing_ignored": driver_counts[2]},
              "platforms_executed": ["Windows x86_64-msvc"], "not_executed": ["Linux", "macOS", "remote CI", "performance"],
              "runs": {variant + "/" + label: {"pid": record["pid"], "log_sha256": record["log_sha256"]}
                       for (variant, label), record in records.items()}}
    return attribution, result


def audit(root, repository):
    evidence = load(root / "evidence.json")
    require(evidence["files"] == inventory(root), "证据字节清单不符")
    attribution, result = verify(root, repository)
    require(load(root / "attribution.json") == attribution, "归属结论与直接复跑不符")
    require(load(root / "closeout.json") == result, "阶段收口与实际运行不符")
    require(load(root / "b4-2a-exit.json") == a_exit(result), "B4-2A 退出与恢复门槛不符")
    return {"passed": True, "fix1_closed": True, "b4_2a_gate_restored": True,
            "b4_2a_closed": True, "b4_2b_started": False, "files": len(evidence["files"]), "code_commit": CODE}


def a_exit(result):
    return {"stage": "B4-2A", "passed": True, "b4_2a_closed": True, "b4_2b_started": False,
            "b4_closed": False, "language_available": False, "replay_commit": result["product_commit"],
            "tac_test_commit": "09fa912b45122d0c038a3cb8ad303fdf5078c0da",
            "tac_bytecode_tests": 65, "concurrency_tests": 29, "frontend_tests": 3, "wrapper_tests": 8,
            "reason": "FIX1 恢复既有 planned 门控；TAC 往返与全部退出回归已在修复快照重跑。"}


def negative(root, repository):
    cases = []
    for label, reason in [("baseline", None), ("missing_attributes", "证据字节清单不符"),
                          ("tampered_log", "原始日志身份不符：concurrency"),
                          ("forged_source", "固定 Git 产品树不符：b34"),
                          ("false_closeout", "阶段收口与实际运行不符")]:
        with tempfile.TemporaryDirectory(prefix="b4-fix1-negative-") as task:
            copied = Path(task) / "evidence"
            shutil.copytree(root, copied)
            if label == "missing_attributes":
                (copied / ".gitattributes").unlink()
            elif label == "tampered_log":
                with (copied / "b34/concurrency.log").open("ab") as output:
                    output.write(b"\nforged success\n")
            elif label == "forged_source":
                source = load(copied / "b34/source.json")
                source["files"]["core/rust/Cargo.lock"]["canonical_sha256"] = "0" * 64
                (copied / "b34/source.json").write_text(json.dumps(source), encoding="utf-8")
            elif label == "false_closeout":
                result = load(copied / "closeout.json")
                result["b4_2b_started"] = True
                (copied / "closeout.json").write_text(json.dumps(result), encoding="utf-8")
            if label in {"tampered_log", "forged_source", "false_closeout"}:
                if label == "false_closeout":
                    exit_result = load(copied / "b4-2a-exit.json")
                    exit_result["b4_2b_started"] = True
                    (copied / "b4-2a-exit.json").write_text(json.dumps(exit_result), encoding="utf-8")
                evidence = load(copied / "evidence.json")
                evidence["files"] = inventory(copied)
                (copied / "evidence.json").write_text(json.dumps(evidence), encoding="utf-8")
            before = all_bytes(copied)
            process = subprocess.run([sys.executable, "-X", "utf8", str(copied / "audit.py"), "--root", str(copied),
                                      "--repository-root", str(repository)], capture_output=True, text=True, encoding="utf-8")
            require(all_bytes(copied) == before, "审计改写文件")
            if reason is None:
                require(process.returncode == 0 and json.loads(process.stdout)["passed"], "基线反例失败")
            else:
                require(process.returncode == 1 and process.stdout.strip() == reason and not process.stderr,
                        "反例未按精确原因拒绝：" + label)
            cases.append({"case": label, "exit_code": process.returncode, "reason": reason, "read_only": True})
    return {"passed": True, "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--collect", action="store_true", help="显式生成归属、收口和实际字节清单；拒绝覆盖")
    parser.add_argument("--refresh-results", action="store_true",
                        help="用新增的真实运行回执刷新收口结果，且验证不改写既有历史运行")
    parser.add_argument("--refresh-inventory", action="store_true", help="先验证原结论，仅刷新实际文件清单")
    parser.add_argument("--self-negative", action="store_true")
    parser.add_argument("--write-negative", type=Path,
                        help="运行反例后以独占创建保存 JSON 结果")
    args = parser.parse_args()
    require(sum([args.collect, args.refresh_results, args.refresh_inventory, args.self_negative]) <= 1,
            "采集/刷新/反例不能混用")
    require(not args.write_negative or args.self_negative, "--write-negative 只配合 --self-negative")
    if args.collect:
        require(not any((args.root / name).exists() for name in ["attribution.json", "closeout.json", "b4-2a-exit.json", "evidence.json"]), "拒绝覆盖既有收口")
        attribution, result = verify(args.root, args.repository_root)
        write_json(args.root / "attribution.json", attribution)
        write_json(args.root / "closeout.json", result)
        write_json(args.root / "b4-2a-exit.json", a_exit(result))
        write_json(args.root / "evidence.json", {"files": inventory(args.root)})
    elif args.refresh_results:
        attribution, result = verify(args.root, args.repository_root)
        old_attribution = load(args.root / "attribution.json")
        old_closeout = load(args.root / "closeout.json")
        require(old_attribution == attribution, "固定归属结论变化，拒绝刷新")
        require(old_closeout["product_commit"] == result["product_commit"]
                and old_closeout["entry_commit"] == result["entry_commit"]
                and old_closeout["fix1_closed"] is True
                and old_closeout["b4_2a_gate_restored"] is True
                and old_closeout["b4_2b_started"] is False,
                "已有有效收口身份变化，拒绝刷新")
        require(all(old_closeout["runs"].get(key) == value for key, value in result["runs"].items()
                    if key in old_closeout["runs"]), "已有运行记录被替换，拒绝刷新")
        (args.root / "closeout.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        exit_result = a_exit(result)
        (args.root / "b4-2a-exit.json").write_text(json.dumps(exit_result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        evidence = {"files": inventory(args.root)}
        temporary = args.root / "evidence.json.tmp"
        write_json(temporary, evidence)
        temporary.replace(args.root / "evidence.json")
    elif args.refresh_inventory:
        attribution, result = verify(args.root, args.repository_root)
        require(load(args.root / "attribution.json") == attribution and load(args.root / "closeout.json") == result
                and load(args.root / "b4-2a-exit.json") == a_exit(result),
                "历史结论不能随清单刷新改变")
        temporary = args.root / "evidence.json.tmp"
        write_json(temporary, {"files": inventory(args.root)})
        temporary.replace(args.root / "evidence.json")
    report = negative(args.root, args.repository_root) if args.self_negative else audit(args.root, args.repository_root)
    if args.write_negative:
        write_json(args.write_negative, report)
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        print(str(error))
        raise SystemExit(1)
