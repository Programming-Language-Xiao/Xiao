"""B4-2B 只读审计：固定 Git 对象、真实回执、日志对象与阶段边界。"""
import argparse
import json
from pathlib import Path, PureWindowsPath
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from record import (ROOT, REPOSITORY, PRODUCT_ROOTS, SELF_EVIDENCE_ROOT, canonical,
                    docs_tree_from_commit, fixed_files, git, identity, load, require, sha, write_json)

FIRST_CODE = "57ca957c"
CODE = "246bea46"
BASE = PureWindowsPath(r"D:\project\Xiao\.b4-2b\20261113-runtime-vm")
SCRIPT = SELF_EVIDENCE_ROOT + "/record.py"
ALLOWED = {
    "core/rust/crates/xiao-runtime/src/lib.rs",
    "core/rust/crates/xiao-runtime/src/cmath_facade_tests.rs",
    "core/rust/crates/xiao-runtime/src/README.md",
    "core/rust/crates/xiao-vm/src/semantics/exec.rs",
    "core/rust/crates/xiao-vm/src/semantics/numeric_intrinsics.rs",
    "core/rust/crates/xiao-vm/src/semantics/README.md",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_vm.rs",
    "core/rust/crates/xiao-driver/tests/README.md",
}
CARGO = ["cargo", "+1.96.0"]
LOCKED = ["--locked", "--manifest-path", "core/rust/Cargo.toml"]
ENTRY_TESTS = ["22c_vm_concurrency", "20o_b2_b4_frontend", "20o_b2_b3_wrapper", "20o_b2_b4_cmath_boundary"]
ENTRY_ARGS = [argument for name in ENTRY_TESTS for argument in ["--test", name]]
HISTORY = {"b1": "entry-20261108-b1", "b2": "entry-20261108-b2", "b3": "entry-20261109-b3-cabi",
           "b34": "entry-20261110-b3-wrapper-final", "b401": "entry-20261110-b4-0-1",
           "fix1": "entry-20261112-b4-2a-fix1"}
RUNS = {
    ("baseline", "entry"): CARGO + ["test"] + LOCKED + ["-p", "xiao-driver"] + ENTRY_ARGS + ["--", "--nocapture"],
    ("fixed", "regressions"): CARGO + ["test"] + LOCKED + ["-p", "xiao-driver", "--test", "20o_b2_b4_cmath_vm"] + ENTRY_ARGS + ["--", "--nocapture"],
    ("fixed", "runtime_vm"): CARGO + ["test"] + LOCKED + ["-p", "xiao-runtime", "-p", "xiao-vm", "--lib", "--", "--nocapture"],
    ("fixed", "bytecode"): CARGO + ["test"] + LOCKED + ["-p", "xiao-bytecode", "--", "--nocapture"],
    ("fixed", "numeric_regressions"): CARGO + ["test"] + LOCKED + ["-p", "xiao-driver", "--test", "20o_b2_a_contract", "--test", "20o_b2_a1_identity", "--test", "20o_b2_b2_divide", "--", "--nocapture"],
    ("fixed", "clippy"): CARGO + ["clippy"] + LOCKED + ["-p", "xiao-runtime", "-p", "xiao-vm", "-p", "xiao-driver", "--all-targets", "--", "-D", "warnings"],
    ("fixed", "fmt"): CARGO + ["fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"],
    ("fixed", "rustc"): ["rustc", "+1.96.0", "-vV"],
    ("fixed", "installed_targets"): ["rustup", "+1.96.0", "target", "list", "--installed"],
    ("fixed", "collector"): ["python", "-B", "-X", "utf8", SCRIPT, "content-store-self-test"],
    ("fixed", "collector_repeat"): ["python", "-B", "-X", "utf8", SCRIPT, "content-store-self-test"],
    ("fixed", "docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
}
for label, directory in HISTORY.items():
    RUNS["fixed", label] = ["python", "-B", "-X", "utf8", f"docs/DevDocs/assets/20o-b2-b/{directory}/audit.py"]
for label in ["regressions", "runtime_vm", "bytecode", "numeric_regressions", "clippy", "fmt", "docs"]:
    RUNS["revised", label] = RUNS["fixed", label]
NEGATIVE_REASONS = {
    "manifest": "证据字节清单不符", "attributes": "证据属性缺失或变化",
    "orphan": "存在没有回执引用的日志对象", "docs": "DevDocs运行字节或提交绑定不符：docs",
    "source": "固定产品源码身份不符", "state": "阶段结论与实际回执不符",
    "environment": "执行路径逃出独立任务根", "negative_report": "自反例报告计数或只读结论不符",
}


def inventory(root):
    files = {}
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(), "证据路径不能经过链接")
        require("__pycache__" not in path.parts, "证据目录禁止Python缓存")
        relative = path.relative_to(root).as_posix()
        if path.is_file() and relative != "evidence.json":
            raw = path.read_bytes()
            files[relative] = {"bytes": len(raw), "sha256": sha(raw)}
    return files


def safe_task_child(raw, task):
    path = PureWindowsPath(raw)
    require(path.is_absolute() and ".." not in path.parts and path != task
            and path.is_relative_to(task), "执行路径逃出独立任务根")
    current = Path(str(task))
    for part in path.relative_to(task).parts:
        current /= part
        require(not current.is_symlink() and not getattr(current, "is_junction", lambda: False)(), "执行路径不能经过链接")


def counts(text):
    rows = re.findall(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text)
    require(rows and all(int(row[1]) == 0 for row in rows), "Rust测试失败或缺少结果")
    return {"passed": sum(int(row[0]) for row in rows), "failed": 0,
            "ignored": sum(int(row[2]) for row in rows)}


def runtime_artifacts(environment):
    target = Path(environment["overrides"]["CARGO_TARGET_DIR"])
    files = sorted([*target.glob("debug/deps/*xiao_runtime-*.rlib"),
                    *target.glob("debug/deps/xiao_runtime-*.lib")])
    require(any(path.suffix == ".rlib" for path in files), "缺少任务Runtime rlib")
    return {"target": str(target), "runtime_library_override_consumed": False,
            "cargo_generated_artifacts": {path.relative_to(target).as_posix(): {
                "bytes": path.stat().st_size, "sha256": sha(path.read_bytes())} for path in files}}


def verify(root, repository, live_runtime=False):
    sources, environments, logs, records, referenced, docs_cache = {}, {}, {}, {}, set(), {}
    for variant in ["baseline", "fixed", "revised"]:
        source = load(root / variant / "source.json")
        environment = load(root / variant / "environment.json")
        require(source["revision"] == git(repository, "rev-parse", source["revision"]).decode().strip(), "来源提交必须为完整Git身份")
        require({p: r["canonical_sha256"] for p, r in source["files"].items()} == fixed_files(repository, source["revision"]), "固定产品源码身份不符")
        require(source["tree_sha256"] == identity(source["files"]), "来源内部摘要不符")
        task = PureWindowsPath(environment["task_root"])
        require(task == BASE / variant, "任务根身份不符")
        current = Path(str(task))
        while current != current.parent:
            require(not current.is_symlink() and not getattr(current, "is_junction", lambda: False)(), "任务根不能经过链接")
            current = current.parent
        for key in ["CARGO_TARGET_DIR", "XIAO_RUNTIME_LIBRARY", "XIAO_HOME", "TEMP", "TMP"]:
            safe_task_child(environment["overrides"][key], task)
        require(environment["source_root"] == source["source_root"] == r"D:\project\Xiao\Xiao", "源码执行根不符")
        require(PureWindowsPath(environment["overrides"]["XIAO_STDLIB_ROOT"]) == PureWindowsPath(source["source_root"]) / "stdlib", "stdlib来源不符")
        require(environment["overrides"]["XIAO_B1_F10"] == environment["overrides"]["XIAO_B2_A_PERF"] == "0", "性能开关不符")
        require(environment["removed_environment_prefixes"] == ["XIAO_", "RUST_BIGDECIMAL_"], "环境清理规则不符")
        artifacts = load(root / variant / "runtime-artifacts.json")
        require(artifacts["target"] == environment["overrides"]["CARGO_TARGET_DIR"]
                and artifacts["runtime_library_override_consumed"] is False
                and any(p.endswith(".rlib") for p in artifacts["cargo_generated_artifacts"]), "Runtime归档身份缺失")
        for path, row in artifacts["cargo_generated_artifacts"].items():
            safe_task_child(str(PureWindowsPath(artifacts["target"]) / path), task)
            require(row["bytes"] > 0 and re.fullmatch("[0-9a-f]{64}", row["sha256"]), "Runtime归档摘要格式不符")
        if live_runtime:
            require(runtime_artifacts(environment) == artifacts, "本机Runtime归档字节变化")
        sources[variant], environments[variant] = source, environment
    for variant, code in [("fixed", FIRST_CODE), ("revised", CODE)]:
        changed = set(git(repository, "diff", "--name-only", sources["baseline"]["revision"], sources[variant]["revision"]).decode().splitlines())
        require({p for p in changed if not p.startswith("docs/DevDocs/")} == ALLOWED, "产品改动超出B4-2B允许集合")
        require({p: r["canonical_sha256"] for p, r in sources[variant]["files"].items()} == fixed_files(repository, code), "实施快照偏离固定产品提交")
    objects = {}
    for path in (root / "logs/sha256").rglob("*"):
        if path.is_file():
            digest = sha(path.read_bytes())
            relative = path.relative_to(root).as_posix()
            require(relative == f"logs/sha256/{digest[:2]}/{digest}.log" and digest not in objects, "内容日志对象路径或摘要不符")
            objects[digest] = relative
    for (variant, label), argv in RUNS.items():
        record = load(root / variant / (label + ".json"))
        expected_exit = 1 if (variant, label) == ("fixed", "docs") else 0
        require(record["label"] == label and record["argv"] == argv and record["exit_code"] == expected_exit, "运行命令或退出码不符：" + label)
        require(record["cwd"] == sources[variant]["source_root"] and record["pid"] > 0
                and record["started_utc"] < record["finished_utc"], "运行进程来源不符：" + label)
        require(record["source_unchanged"] is True and record["source_before"] == record["source_after"] == sources[variant]["tree_sha256"], "运行源码发生变化：" + label)
        require(record["environment_sha256"] == sha((root / variant / "environment.json").read_bytes()), "运行环境摘要不符：" + label)
        commit = record["docs_commit"]
        require(commit == git(repository, "rev-parse", commit).decode().strip()
                and record["runner_sha256"] == sha(git(repository, "show", commit + ":" + SCRIPT)), "固定执行器身份不符：" + label)
        if commit not in docs_cache:
            docs_cache[commit] = docs_tree_from_commit(repository, commit)
        tree = docs_cache[commit]
        require(record["docs_commit_after"] == commit and record["docs_tree_unchanged"] is True
                and record["docs_tree_before"] == record["docs_tree_after"] == record["docs_tree_sha256"]
                and record["docs_tree_canonical_before"] == record["docs_tree_canonical_after"] == tree["canonical_sha256"]
                and record["docs_tree_commit_raw_sha256"] == tree["raw_sha256"]
                and record["docs_tree_commit_canonical_sha256"] == tree["canonical_sha256"], "DevDocs运行字节或提交绑定不符：" + label)
        digest = record["log_sha256"]
        require(objects.get(digest) == record["log_ref"] and "log" not in record, "日志对象引用不符：" + label)
        raw = (root / record["log_ref"]).read_bytes()
        require(record["log_bytes"] == len(raw) and sha(raw) == digest, "日志字节不符：" + label)
        referenced.add(digest)
        logs[variant, label], records[variant, label] = raw.decode("utf-8"), record
    require(referenced == set(objects), "存在没有回执引用的日志对象")
    require(counts(logs["baseline", "entry"]) == {"passed": 43, "failed": 0, "ignored": 0}, "入口回归计数不符")
    require(counts(logs["fixed", "regressions"]) == {"passed": 50, "failed": 0, "ignored": 0}, "VM与边界回归计数不符")
    require(counts(logs["revised", "regressions"]) == {"passed": 50, "failed": 0, "ignored": 0}, "修订快照VM与边界回归计数不符")
    require(len(re.findall(r"B3-WRAPPER-IMPORT .*executable=false result=planned-rejected", logs["revised", "regressions"])) == 30, "30条planned导入拒绝不全")
    require("B4-2B-VM-IDENTITY cases=1440" in logs["revised", "regressions"]
            and "B4-2B-VM-OWNERSHIP cases=45" in logs["revised", "regressions"], "VM位模式或所有权专项缺失")
    require("B4-2B-RUNTIME reference_inputs=17243 facades=15 kernel_identity=exact" in logs["revised", "runtime_vm"], "Runtime门面输入证据缺失")
    require(counts(logs["revised", "runtime_vm"]) == {"passed": 307, "failed": 0, "ignored": 0}, "Runtime/VM单测计数不符")
    require(counts(logs["revised", "bytecode"])["passed"] == 65, "TAC回归计数不符")
    numeric = counts(logs["revised", "numeric_regressions"])
    require(numeric == {"passed": 15, "failed": 0, "ignored": 3}, "数值回归通过或既有忽略计数不符")
    for label in HISTORY:
        history = json.loads(logs["fixed", label])
        require(history.get("passed") is True or label == "b1" and history.get("passed") == history.get("total") == 18, "固定历史审计失败：" + label)
    first, second = records["fixed", "collector"], records["fixed", "collector_repeat"]
    require(first["log_ref"] == second["log_ref"] and first["pid"] != second["pid"], "相同采集输出没有共用对象或回执未独立")
    require(json.loads(logs["fixed", "collector"]) == {"passed": True, "distinct_objects": 2,
            "identical_outputs_reused": True, "tampered_object_rejected": True, "docs_tree_static_bound": True}, "采集器自测不符")
    first_docs = json.loads(logs["fixed", "docs"])
    first_errors = [d for d in first_docs["diagnostics"] if d["severity"] == "error"]
    require(first_docs["passed"] is False and len(first_errors) == 1
            and first_errors[0]["code"] == "A0-SIZE-001"
            and first_errors[0]["path"] == "core/rust/crates/xiao-vm/src/semantics/exec.rs", "初次docs失败身份不符")
    docs = json.loads(logs["revised", "docs"])
    require(docs["passed"] is True and all(d["code"] == "A0-COVERAGE-002" and d["severity"] == "warning" for d in docs["diagnostics"]), "终稿文档门禁失败或出现新诊断")
    runtime_log = logs["revised", "regressions"].replace("/", "\\")
    require("Compiling xiao-runtime " in runtime_log and str(BASE / "revised" / "target") in runtime_log, "缺少本任务冷构建Runtime及测试路径")
    return {"passed": True, "stage": "B4-2", "b4_2b_closed": True, "b4_2_closed": True,
            "b4_3_started": False, "b4_closed": False, "language_available": False, "cmath": "planned",
            "product_commit": git(repository, "rev-parse", CODE).decode().strip(),
            "snapshot_commit": sources["revised"]["revision"], "runtime_abi": "1.18", "codegen_version": 20,
            "platforms_executed": ["Windows x86_64-msvc"],
            "not_executed": ["Linux", "macOS", "remote CI", "performance", "old 721 F10"],
            "tests": {"baseline": 43, "driver": 50, "runtime_vm": counts(logs["revised", "runtime_vm"]),
                      "bytecode": 65, "numeric_regressions": numeric, "vm_identity_cases": 1440, "heap_cases": 45},
            "runs": {v + "/" + label: {"pid": row["pid"], "log_sha256": row["log_sha256"],
                      "docs_commit": row["docs_commit"]} for (v, label), row in records.items()}}


def audit(root, repository, live=False):
    require((root / ".gitattributes").is_file()
            and (root / ".gitattributes").read_bytes() == b"* -text -whitespace\n*.log -diff\n", "证据属性缺失或变化")
    evidence = load(root / "evidence.json")
    require(evidence["files"] == inventory(root), "证据字节清单不符")
    result = verify(root, repository, live)
    require(load(root / "closeout.json") == result, "阶段结论与实际回执不符")
    report = root / "self-negative-findings.json"
    if report.exists():
        expected = [{"case": "baseline", "exit_code": 0, "read_only": True, "reason": None}]
        expected.extend({"case": name, "exit_code": 1, "read_only": True, "reason": reason}
                        for name, reason in NEGATIVE_REASONS.items())
        require(load(report) == {"passed": True, "cases": expected}, "自反例报告计数或只读结论不符")
    return {"passed": True, "files": len(evidence["files"]), "runs": len(RUNS), "log_objects": len({r["log_sha256"] for r in result["runs"].values()}),
            "b4_2_closed": True, "b4_closed": False, "language_available": False,
            "self_negative_checked": report.exists(), "live_runtime_checked": live}


def all_entries(root):
    """反例前后同时比较文件字节和目录项，包含空目录及清单自身。"""
    return {p.relative_to(root).as_posix() + ("/" if p.is_dir() else ""):
            None if p.is_dir() else sha(p.read_bytes()) for p in root.rglob("*")}


def negative(root, repository):
    cases = []
    mutations = [None, *NEGATIVE_REASONS]
    reasons = NEGATIVE_REASONS
    for mutation in mutations:
        with tempfile.TemporaryDirectory(prefix="xiao-b4-2b-negative-") as temporary:
            copied = Path(temporary) / "evidence"
            shutil.copytree(root, copied)
            if mutation == "manifest":
                (copied / "evidence.json").write_text('{"files": {}}', encoding="utf-8")
            elif mutation == "attributes":
                (copied / ".gitattributes").unlink()
            elif mutation == "negative_report":
                (copied / "self-negative-findings.json").write_text(
                    json.dumps({"passed": True, "cases": [{"case": "baseline", "read_only": False}]}), encoding="utf-8")
            elif mutation == "orphan":
                raw = b"unreferenced B4-2B object\n"
                digest = sha(raw)
                path = copied / f"logs/sha256/{digest[:2]}/{digest}.log"
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)
            elif mutation in {"docs", "source", "state", "environment"}:
                relative = {"docs": "revised/docs.json", "source": "revised/source.json",
                            "state": "closeout.json", "environment": "fixed/environment.json"}[mutation]
                value = load(copied / relative)
                if mutation == "docs": value["docs_tree_sha256"] = "0" * 64
                if mutation == "source": value["files"]["core/rust/Cargo.lock"]["canonical_sha256"] = "0" * 64
                if mutation == "state": value["language_available"] = True
                if mutation == "environment": value["overrides"]["CARGO_TARGET_DIR"] = str(BASE / "fixed" / ".." / "escaped")
                (copied / relative).write_text(json.dumps(value), encoding="utf-8")
            if mutation and mutation != "manifest":
                (copied / "evidence.json").write_text(json.dumps({"files": inventory(copied)}), encoding="utf-8")
            before = all_entries(copied)
            process = subprocess.run([sys.executable, "-B", "-X", "utf8", str(copied / "audit.py"),
                "--root", str(copied), "--repository-root", str(repository)], capture_output=True, text=True, encoding="utf-8")
            require(before == all_entries(copied), "反例审计写入证据")
            if mutation is None:
                require(process.returncode == 0 and json.loads(process.stdout)["passed"] is True, "反例基线失败")
            else:
                require(process.returncode == 1 and process.stdout.strip() == reasons[mutation] and not process.stderr, "未按精确原因拒绝：" + mutation + ":" + process.stdout + process.stderr)
            cases.append({"case": mutation or "baseline", "exit_code": process.returncode, "read_only": True, "reason": reasons.get(mutation)})
    return {"passed": True, "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--collect", action="store_true")
    parser.add_argument("--refresh-inventory", action="store_true")
    parser.add_argument("--live-runtime", action="store_true")
    parser.add_argument("--self-negative", action="store_true")
    parser.add_argument("--write-negative", type=Path)
    args = parser.parse_args()
    require(sum([args.collect, args.refresh_inventory, args.self_negative]) <= 1, "生成与只读反例模式不可混用")
    require(not args.write_negative or args.self_negative, "反例输出只允许在反例模式使用")
    if args.collect:
        require(not (args.root / "closeout.json").exists() and not (args.root / "evidence.json").exists(), "拒绝覆盖已收口证据")
        for variant in ["baseline", "fixed", "revised"]:
            write_json(args.root / variant / "runtime-artifacts.json", runtime_artifacts(load(args.root / variant / "environment.json")))
        write_json(args.root / "closeout.json", verify(args.root, args.repository_root, True))
        write_json(args.root / "evidence.json", {"files": inventory(args.root)})
    elif args.refresh_inventory:
        require(load(args.root / "closeout.json") == verify(args.root, args.repository_root), "清单刷新不能改写阶段结论")
        temporary = args.root / "evidence.json.tmp"
        write_json(temporary, {"files": inventory(args.root)})
        temporary.replace(args.root / "evidence.json")
    result = negative(args.root, args.repository_root) if args.self_negative else audit(args.root, args.repository_root, args.live_runtime)
    if args.write_negative: write_json(args.write_negative, result)
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        print(str(error))
        raise SystemExit(1)
