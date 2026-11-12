"""只读验证固定 Git 来源、受控执行与 blocked 结论；不要求当前 HEAD。"""
import argparse
import json
from pathlib import Path, PureWindowsPath
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True

from record import (ENTRY, REPOSITORY, ROOT, SOURCE, PRODUCT_ROOTS, fixed_files,
                    git, load, require, sha, tree_sha, write_json)

CARGO = ["cargo", "+1.96.0"]
MANIFEST = ["--locked", "--manifest-path", "core/rust/Cargo.toml"]
RUNS = {
    "bytecode": CARGO + ["test"] + MANIFEST + ["-p", "xiao-bytecode", "--", "--nocapture"],
    "frontend_wrapper": CARGO + ["test"] + MANIFEST + ["-p", "xiao-driver", "--test", "20o_b2_b4_frontend", "--test", "20o_b2_b3_wrapper", "--", "--nocapture"],
    "concurrency": CARGO + ["test"] + MANIFEST + ["-p", "xiao-driver", "--test", "22c_vm_concurrency", "--", "--nocapture"],
    "clippy": CARGO + ["clippy"] + MANIFEST + ["-p", "xiao-bytecode", "--all-targets", "--", "-D", "warnings"],
    "fmt": CARGO + ["fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"],
    "toolchain": ["rustc", "+1.96.0", "-vV"],
    "docs": ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
}
for name, directory in {
    "b1_history": "entry-20261108-b1", "b2_history": "entry-20261108-b2",
    "b3_history": "entry-20261109-b3-cabi", "b34_history": "entry-20261110-b3-wrapper-final",
    "b40_history": "entry-20261110-b4-0-1",
}.items():
    RUNS[name] = ["python", "-B", "-X", "utf8", f"docs/DevDocs/assets/20o-b2-b/{directory}/audit.py"]
RUNS["b40_fixed_history"] = RUNS["b40_history"]


def inventory(root):
    return {path.relative_to(root).as_posix(): {"bytes": path.stat().st_size, "sha256": sha(path.read_bytes())}
            for path in sorted(root.rglob("*")) if path.is_file() and path.name != "evidence.json"
            and "__pycache__" not in path.parts}


def byte_snapshot(root):
    return {path.relative_to(root).as_posix(): sha(path.read_bytes())
            for path in root.rglob("*") if path.is_file()}


def verify_runs(root, repository):
    source = load(root / "source.json")
    require(source["source_commit"] == git(repository, "rev-parse", SOURCE).decode().strip(), "固定测试提交不符")
    require(source["entry_commit"] == git(repository, "rev-parse", ENTRY).decode().strip(), "入口提交不符")
    require(source["files"] == fixed_files(repository, SOURCE), "固定 Git 产品树与来源清单不符")
    require(tree_sha(source["files"]) == source["canonical_tree_sha256"], "来源清单摘要不符")
    changes = git(repository, "diff", "--name-only", ENTRY, SOURCE, "--", *PRODUCT_ROOTS).decode().splitlines()
    require(set(changes) == {"core/rust/crates/xiao-bytecode/src/encode/tests.rs",
                             "core/rust/crates/xiao-bytecode/tests/b4_cmath_tac.rs"}, "测试提交越过允许范围")
    # 证明原 22c 运行采用的 B4-1 生产代码没有被本轮 TAC 测试修改。
    old_changes = git(repository, "diff", "--name-only", "c7a45e10", ENTRY,
                      "--", *PRODUCT_ROOTS).decode().splitlines()
    require(old_changes == ["core/rust/crates/xiao-bytecode/tests/b4_cmath_tac.rs"], "B4-1 至入口的产品差异不符")
    config = load(root / "environment.json")
    task = PureWindowsPath(config["task_root"])
    env = config["overrides"]
    require(config["removed_environment_prefixes"] == ["XIAO_", "RUST_BIGDECIMAL_"], "环境清理策略不符")
    for key in ["CARGO_TARGET_DIR", "XIAO_HOME", "TEMP", "TMP", "XIAO_RUNTIME_LIBRARY"]:
        require(PureWindowsPath(env[key]).is_relative_to(task) and PureWindowsPath(env[key]) != task, "执行目录逃出任务根")
    require(env["XIAO_STDLIB_ROOT"] == str(PureWindowsPath(config["repository"]) / "stdlib"), "stdlib 来源不符")
    require(env["XIAO_B1_F10"] == env["XIAO_B2_A_PERF"] == "0", "性能开关不符")
    require(config["runtime_staticlib_built_or_used"] is False, "本批未构建原生 Runtime")
    logs, records = {}, {}
    commands = dict(RUNS)
    bare = load(root / "b40_bare.json")
    require(PureWindowsPath(bare["argv"][-1]) == task / "objects.git", "裸仓库对象来源不符")
    commands["b40_bare"] = RUNS["b40_fixed_history"] + ["--repository-root", bare["argv"][-1]]
    for label, argv in commands.items():
        record = load(root / (label + ".json"))
        require(record["label"] == label and record["argv"] == argv, "执行命令不符：" + label)
        require(record["cwd"] == config["repository"] and record["pid"] > 0, "执行身份不符：" + label)
        require(record["started_utc"] < record["finished_utc"], "执行时间不符：" + label)
        require(record["source_before"] == record["source_after"] == source["canonical_tree_sha256"]
                and record["source_unchanged"] is True, "执行源码漂移：" + label)
        require(record["environment_sha256"] == sha((root / "environment.json").read_bytes())
                and record["runner_sha256"] == sha((root / "record.py").read_bytes()), "执行器或环境身份不符：" + label)
        require(record["log"] == label + ".log", "日志路径不符：" + label)
        data = (root / record["log"]).read_bytes()
        require(record["log_sha256"] == sha(data) and record["log_bytes"] == len(data), "原始日志身份不符：" + label)
        expected_exit = 101 if label == "concurrency" else 1 if label == "b40_history" else 0
        require(record["exit_code"] == expected_exit, "退出码不符：" + label)
        logs[label], records[label] = data.decode("utf-8"), record
    for label, expected in {"bytecode": (65, 0, 0), "frontend_wrapper": (11, 0, 0), "concurrency": (27, 2, 0)}.items():
        results = re.findall(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", logs[label])
        require(results and tuple(sum(int(row[i]) for row in results) for i in range(3)) == expected,
                "测试结果不符：" + label)
    require(len(re.findall(r"^B4-2A-CASE ", logs["bytecode"], re.M)) == 66, "TAC 往返样本不完整")
    require("test encode::tests::decoded_intrinsic_ids_are_checked_in_both_operand_widths ... ok" in logs["bytecode"], "缺少解码拒绝证据")
    failed = re.findall(r"^test (\w+) \.\.\. FAILED$", logs["concurrency"], re.M)
    require(set(failed) == {"isolated_child_preserves_cmath_alias_snapshot_identity", "planned_cmath_xar_is_rejected_before_output"}, "22c 失败身份不符")
    require(json.loads(logs["docs"])["passed"] is True, "文档门禁未通过")
    require("host: x86_64-pc-windows-msvc" in logs["toolchain"] and "rustc 1.96.0" in logs["toolchain"], "工具链身份不符")
    require(logs["b40_history"].strip() == "固定B4代码提交后的受保护产品树发生变化", "原历史复跑失败原因不符")
    for label in ["b1_history", "b2_history", "b3_history", "b34_history", "b40_fixed_history", "b40_bare"]:
        report = json.loads(logs[label])
        require(report["passed"] in (True, 18), "历史审计未通过：" + label)
    return {"source_commit": source["source_commit"], "canonical_tree_sha256": source["canonical_tree_sha256"],
            "source_files": len(source["files"]), "status": "blocked", "b4_2a_closed": False,
            "b4_2b_started": False, "language_available": False, "old_721_executed": False,
            "tests": {"bytecode": 65, "frontend_wrapper": 11, "concurrency": "27/29"},
            "runs": {label: {"pid": record["pid"], "log_sha256": record["log_sha256"]}
                     for label, record in records.items()}}


def audit(root, repository):
    evidence = load(root / "evidence.json")
    actual = inventory(root)
    require(set(evidence["files"]) == set(actual), "证据目录文件集合不符")
    require(evidence["files"] == actual, "证据文件清单不符")
    require(evidence["result"] == verify_runs(root, repository), "阶段结果与实际运行不符")
    return {"evidence_valid": True, "stage_passed": False, "b4_2a_closed": False,
            "b4_2b_started": False, "files": len(actual), "source_commit": SOURCE}


def self_negative(root, repository):
    cases = []
    for label, reason in [("baseline", None), ("missing_attributes", "证据目录文件集合不符"),
                          ("tamper_record", "证据文件清单不符"),
                          ("rebuild_inventory_after_log_tamper", "原始日志身份不符：bytecode"),
                          ("forged_source", "固定 Git 产品树与来源清单不符"),
                          ("false_closeout", "阶段结果与实际运行不符")]:
        with tempfile.TemporaryDirectory(prefix="b4-2a-negative-") as task:
            copied = Path(task) / "evidence"
            shutil.copytree(root, copied)
            if label == "missing_attributes":
                (copied / ".gitattributes").unlink()
            elif label == "tamper_record":
                with (copied / "bytecode.json").open("ab") as stream:
                    stream.write(b"\n")
            elif label in {"rebuild_inventory_after_log_tamper", "forged_source"}:
                if label == "rebuild_inventory_after_log_tamper":
                    with (copied / "bytecode.log").open("ab") as stream:
                        stream.write(b"\nforged output\n")
                else:
                    source = load(copied / "source.json")
                    source["files"]["core/rust/Cargo.lock"] = "0" * 64
                    (copied / "source.json").write_text(json.dumps(source), encoding="utf-8")
                evidence = load(copied / "evidence.json")
                evidence["files"] = inventory(copied)
                (copied / "evidence.json").write_text(json.dumps(evidence), encoding="utf-8")
            elif label == "false_closeout":
                evidence = load(copied / "evidence.json")
                evidence["result"]["b4_2a_closed"] = True
                (copied / "evidence.json").write_text(json.dumps(evidence), encoding="utf-8")
            before = byte_snapshot(copied)
            # 基线故意不带 -B，验证默认 Python 调用也不会生成 __pycache__。
            flags = [] if label == "baseline" else ["-B"]
            child = subprocess.run([sys.executable, *flags, "-X", "utf8", str(copied / "audit.py"),
                                    "--root", str(copied), "--repository-root", str(repository)],
                                   capture_output=True, text=True, encoding="utf-8")
            require(before == byte_snapshot(copied), "审计改写证据")
            if reason is None:
                require(child.returncode == 0 and json.loads(child.stdout)["evidence_valid"], "基线审计失败")
            else:
                require(child.returncode == 1 and child.stdout.strip() == reason and not child.stderr,
                        "反例没有按预期原因拒绝：" + label)
            cases.append({"case": label, "exit_code": child.returncode, "expected_reason": reason,
                          "actual_output": child.stdout.strip(), "read_only": True})
    # 历史审计同样必须识别伪造的产品摘要，不能靠刷新历史清单掩盖。
    legacy = ROOT.parent.parent / "entry-20261110-b4-0-1"
    with tempfile.TemporaryDirectory(prefix="b4-1-source-negative-") as task:
        copied = Path(task) / "legacy"
        shutil.copytree(legacy, copied)
        source = load(copied / "source-manifest.json")
        source["product_files"][0]["utf8_lf_or_binary_sha256"] = "0" * 64
        (copied / "source-manifest.json").write_text(json.dumps(source), encoding="utf-8")
        # 固定来源检查先于证据清单；此反例只改来源，保留原始证据以防误过。
        before = byte_snapshot(copied)
        child = subprocess.run([sys.executable, "-B", "-X", "utf8", str(legacy / "audit.py"),
                                "--root", str(copied), "--repository-root", str(repository)],
                               capture_output=True, text=True, encoding="utf-8")
        reason = "B4源清单逐路径规范摘要与固定Git对象不符"
        require(child.returncode == 1 and child.stderr.strip() == reason and not child.stdout,
                "历史来源反例没有按预期原因拒绝")
        require(before == byte_snapshot(copied), "历史审计改写证据")
        cases.append({"case": "legacy_forged_source", "exit_code": child.returncode,
                      "expected_reason": reason, "actual_output": child.stderr.strip(), "read_only": True})
    return {"passed": True, "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--collect", action="store_true", help="显式生成实际字节清单；不覆盖已有 evidence.json")
    parser.add_argument("--refresh-inventory", action="store_true", help="显式刷新清单；保留且重新验证原阶段结果")
    parser.add_argument("--self-negative", action="store_true", help="只在临时证据副本验证反例及只读性")
    args = parser.parse_args()
    if args.refresh_inventory:
        require(not args.collect and not args.self_negative, "刷新清单不能合并其它操作")
        evidence = load(args.root / "evidence.json")
        require(evidence["result"] == verify_runs(args.root, args.repository_root), "阶段结果与实际运行不符")
        evidence["files"] = inventory(args.root)
        temporary = args.root / "evidence.json.tmp"
        write_json(temporary, evidence)
        temporary.replace(args.root / "evidence.json")
    if args.collect:
        require(not args.self_negative, "不能合并采集与反例")
        result = verify_runs(args.root, args.repository_root)
        write_json(args.root / "evidence.json", {"result": result, "files": inventory(args.root)})
    report = self_negative(args.root, args.repository_root) if args.self_negative else audit(args.root, args.repository_root)
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        print(str(error))
        raise SystemExit(1)
