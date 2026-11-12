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
                    require, require_new_task_root, sha, write_json)

ENTRY = "2f862f51"
CODE = "c4610211"
RUNNER_COMMIT = "cb2444dc0a2afd6d2bf456fe3b1805454b4825c9"
RUNNER_COMMITS = [RUNNER_COMMIT, "13e7670f0ceb5d6ab4ca181effb07da0a8059284",
                  "2e656b123a99bda33c881606fe01898863a2f122"]
RUNNER_PATH = "docs/DevDocs/assets/20o-b2-b/entry-20261112-b4-2a-fix1/record.py"
VERSIONS = {"b34": "7d8b85e4", "b401": "c7a45e10", "fixed": CODE}
TASK_BASE = PureWindowsPath(r"D:\project\Xiao\.b4-2a-fix1\20261112-2f862f51")
TASK_BASE = PureWindowsPath(r"D:\project\Xiao\.b4-2a-fix1\20261112-2f862f51")
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
    ("fixed", "review_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "review_usedocs"): ["bun", "run", "check:usedocs"],
    ("fixed", "review_layout"): ["bun", "run", "check:layout"],
    ("fixed", "audit_final_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "audit_final_usedocs"): ["bun", "run", "check:usedocs"],
    ("fixed", "audit_final_layout"): ["bun", "run", "check:layout"],
    ("fixed", "review_final_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "review_final_usedocs"): ["bun", "run", "check:usedocs"],
    ("fixed", "review_final_layout"): ["bun", "run", "check:layout"],
    ("fixed", "last_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "sealed_review_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "sealed_review_usedocs"): ["bun", "run", "check:usedocs"],
    ("fixed", "sealed_review_layout"): ["bun", "run", "check:layout"],
    ("fixed", "locked_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "cross_status_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "self_review_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "final_self_review_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "sealed_self_review_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "audit_self_review_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "final_audit_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "review_audit_v3_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "audit_review_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "final_self_review_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "last_review_docs"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
    ("fixed", "final_doc_review"): ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"],
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
    paths = list(root.rglob("*"))
    require(not any(path.is_symlink() for path in paths), "证据目录禁止符号链接")
    require(not any("__pycache__" in path.parts for path in paths), "证据目录禁止 Python 缓存")
    return {
        path.relative_to(root).as_posix(): {"bytes": path.stat().st_size, "sha256": sha(path.read_bytes())}
        for path in sorted(paths)
        if path.is_file()
        and path.relative_to(root).as_posix() != "evidence.json"
    }


def all_bytes(root):
    result = {}
    for path in root.rglob("*"):
        relative = path.relative_to(root).as_posix()
        if path.is_dir():
            result[relative + "/"] = None
        elif path.is_file():
            result[relative] = sha(path.read_bytes())
    return result


def safe_child(raw_path, task, allow_missing=False):
    """按 Windows 词法拒绝任务根逃逸和路径组件跳过，再核对现存父级解析路径。"""
    path = PureWindowsPath(raw_path)
    require(path.is_absolute() and ".." not in path.parts and "." not in path.parts,
            "执行路径包含相对跳转")
    require(path != task, "执行路径不能等于任务根")
    require(path.is_relative_to(task), "执行目录逃出任务根")
    current = Path(str(task))
    resolved_task = PureWindowsPath(str(current.resolve()))
    for part in path.relative_to(task).parts:
        current /= part
        require(not current.is_symlink() and not getattr(current, "is_junction", lambda: False)(),
                "执行路径不能经过符号链接或 junction")
        if current.exists():
            require(PureWindowsPath(str(current.resolve())).is_relative_to(resolved_task),
                    "执行目录解析后逃出任务根")
    if not allow_missing:
        require(current.exists(), "登记的执行目录不存在")
    return current


def totals(log):
    rows = re.findall(r"^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", log, re.M)
    require(bool(rows), "缺少 Rust 测试结果")
    return tuple(sum(int(row[index]) for row in rows) for index in range(3))


def verify(root, repository):
    sources, environments, task_roots = {}, {}, {}
    runner_shas = {sha(git(repository, "show", commit + ":" + RUNNER_PATH)) for commit in RUNNER_COMMITS}
    for variant, revision in VERSIONS.items():
        directory = root / variant
        source, env = load(directory / "source.json"), load(directory / "environment.json")
        require(source["revision"] == git(repository, "rev-parse", revision).decode().strip(), "固定来源提交不符：" + variant)
        require({path: row["canonical_sha256"] for path, row in source["files"].items()}
                == fixed_files(repository, revision), "固定 Git 产品树不符：" + variant)
        require(source["tree_sha256"] == identity(source["files"]), "来源内部摘要不符：" + variant)
        task = PureWindowsPath(env["task_root"])
        require(task.name == variant, "任务根变体标记不符：" + variant)
        task_fs = Path(str(task))
        if task_fs.exists():
            require(PureWindowsPath(str(task_fs.resolve())) == task,
                    "任务根不能经符号链接/junction 重定向：" + variant)
        require(source["source_root"] == env["source_root"], "执行源码根不符：" + variant)
        for key in ["CARGO_TARGET_DIR", "XIAO_HOME", "TEMP", "TMP", "XIAO_RUNTIME_LIBRARY"]:
            safe_child(env["overrides"][key], task, allow_missing=True)
        require(env["overrides"]["TEMP"] == env["overrides"]["TMP"], "TEMP/TMP 应绑定同一任务目录")
        require(env["overrides"]["XIAO_STDLIB_ROOT"] == str(PureWindowsPath(source["source_root"]) / "stdlib"), "stdlib 来源不符：" + variant)
        require(env["overrides"]["XIAO_B1_F10"] == env["overrides"]["XIAO_B2_A_PERF"] == "0", "性能开关不符")
        require(env["removed_environment_prefixes"] == ["XIAO_", "RUST_BIGDECIMAL_"]
                and env["runtime_staticlib_built_or_used"] is False, "执行环境边界不符")
        sources[variant], environments[variant], task_roots[variant] = source, env, task
    require(set(task_roots) == {"b34", "b401", "fixed"}
            and len({task.parent for task in task_roots.values()}) == 1
            and all(task == TASK_BASE / variant for variant, task in task_roots.items()),
            "任务根必须位于本批专用目录并相互隔离")
    require(PureWindowsPath(sources["b34"]["source_root"]) == task_roots["b34"] / "source"
            and PureWindowsPath(sources["b401"]["source_root"]) == task_roots["b401"] / "source"
            and PureWindowsPath(sources["fixed"]["source_root"]) == PureWindowsPath(r"D:\project\Xiao\Xiao"),
            "FIX1 源码工作树与固定环境路径不符")
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
                and record["runner_sha256"] in runner_shas, "固定执行器/环境摘要不符：" + label)
        require(record["log"] == label + ".log", "日志路径不符：" + label)
        raw = (directory / record["log"]).read_bytes()
        require(record["log_sha256"] == sha(raw) and record["log_bytes"] == len(raw), "原始日志身份不符：" + label)
        expected_exit = 101 if variant == "b401" else 0
        require(record["exit_code"] == expected_exit, "执行退出码不符：" + label)
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
    review_repo = json.loads(logs["fixed", "review_docs"])
    require(review_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in review_repo["diagnostics"]), "自审最终文档门禁失败或出现新诊断")
    final_self_review_repo = json.loads(logs["fixed", "final_self_review_docs"])
    require(final_self_review_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in final_self_review_repo["diagnostics"]), "FIX1 自审核最终文档门禁失败或出现新诊断")
    require("通过：未发现 A0 规则问题。" in logs["fixed", "review_usedocs"]
            and "通过：未发现 A0 规则问题。" in logs["fixed", "review_layout"], "自审最终 UseDocs/layout 门禁失败")
    audit_repo = json.loads(logs["fixed", "audit_final_docs"])
    require(audit_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in audit_repo["diagnostics"]), "最终审计工具快照的 docs 门禁失败或出现新诊断")
    require("通过：未发现 A0 规则问题。" in logs["fixed", "audit_final_usedocs"]
            and "通过：未发现 A0 规则问题。" in logs["fixed", "audit_final_layout"], "最终审计工具快照的 UseDocs/layout 门禁失败")
    review_repo = json.loads(logs["fixed", "review_final_docs"])
    require(review_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in review_repo["diagnostics"]), "自审最终说明的 docs 门禁失败或出现新诊断")
    require("通过：未发现 A0 规则问题。" in logs["fixed", "review_final_usedocs"]
            and "通过：未发现 A0 规则问题。" in logs["fixed", "review_final_layout"], "自审最终说明的 UseDocs/layout 门禁失败")
    sealed_review = json.loads(logs["fixed", "sealed_review_docs"])
    require(sealed_review["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in sealed_review["diagnostics"]), "最终自审 README 的 docs 门禁失败或出现新诊断")
    require("通过：未发现 A0 规则问题。" in logs["fixed", "sealed_review_usedocs"]
            and "通过：未发现 A0 规则问题。" in logs["fixed", "sealed_review_layout"], "最终自审 README 的 UseDocs/layout 门禁失败")
    locked_repo = json.loads(logs["fixed", "locked_docs"])
    require(locked_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in locked_repo["diagnostics"]), "最终反例计数文档门禁失败或出现新诊断")
    last_repo = json.loads(logs["fixed", "last_docs"])
    require(last_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in last_repo["diagnostics"]), "最终 README/反例说明文档门禁失败或出现新诊断")
    cross_status_repo = json.loads(logs["fixed", "cross_status_docs"])
    require(cross_status_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in cross_status_repo["diagnostics"]), "B4跨阶段状态对账 docs 门禁失败或出现新诊断")
    self_review_repo = json.loads(logs["fixed", "self_review_docs"])
    require(self_review_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in self_review_repo["diagnostics"]), "本轮自审收口 docs 门禁失败或出现新诊断")
    final_self_review_repo = json.loads(logs["fixed", "final_self_review_docs"])
    require(final_self_review_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in final_self_review_repo["diagnostics"]), "自审最终记录文档门禁失败或出现新诊断")
    sealed_self_review_repo = json.loads(logs["fixed", "sealed_self_review_docs"])
    require(sealed_self_review_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in sealed_self_review_repo["diagnostics"]), "自审封存文案 docs 门禁失败或出现新诊断")
    audit_self_review_repo = json.loads(logs["fixed", "audit_self_review_docs"])
    require(audit_self_review_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in audit_self_review_repo["diagnostics"]), "审计自审说明 docs 门禁失败或出现新诊断")
    final_audit_repo = json.loads(logs["fixed", "final_audit_docs"])
    require(final_audit_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in final_audit_repo["diagnostics"]), "最终 FIX1 证据门禁失败或出现新诊断")
    audit_v3_repo = json.loads(logs["fixed", "review_audit_v3_docs"])
    require(audit_v3_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in audit_v3_repo["diagnostics"]), "最终只读审计自反例文案 docs 门禁失败或出现新诊断")
    audit_review_repo = json.loads(logs["fixed", "audit_review_docs"])
    require(audit_review_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in audit_review_repo["diagnostics"]), "最终审计自审记录 docs 门禁失败或出现新诊断")
    last_review_repo = json.loads(logs["fixed", "last_review_docs"])
    require(last_review_repo["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in last_review_repo["diagnostics"]), "最后自审说明的 docs 门禁失败或出现新诊断")
    final_doc_review = json.loads(logs["fixed", "final_doc_review"])
    require(final_doc_review["passed"] is True and all(
        item["code"] == "A0-COVERAGE-002" and item["severity"] == "warning"
        for item in final_doc_review["diagnostics"]), "最终反例计数文案的 docs 门禁失败或出现新诊断")
    require("通过：未发现 A0 规则问题。" in logs["fixed", "usedocs_closeout"]
            and "通过：未发现 A0 规则问题。" in logs["fixed", "layout_closeout"], "最终 UseDocs/layout 门禁失败")
    b1, b2, b3, b34, b401 = (json.loads(logs["fixed", label]) for label in
                              ["b1_history", "b2_history", "b3_history", "b34_history", "b401_history"])
    require(b1 == {"passed": 18, "total": 18, "files": 28, "baseline_commit": "d08b8460",
                   "code_commit": "35872782",
                   "source_manifest": "source-manifests/8a3d7472ccaf8bb0a25d6731c8e852d770ef03d4f0267476956a974f2f0b7cc8.json"},
            "B1 固定历史审计精确结果不符")
    require(b2 == {"passed": True, "files": 99, "checks": 14, "exact_checks": 4,
                   "code_commit": "37b92c5338274121afe98320d4e076620744449f", "abi": "1.18",
                   "codegen": 20, "max_active_intrinsic": 33, "scope": "B2内部单入口"},
            "B2 固定历史审计精确结果不符")
    require(b3 == {"passed": True, "core_only": False, "files": 86, "code_commit": "89d782e1",
                   "checks": 22, "exact": 4, "formal_abi_cases": 17243, "runtime_tests": 269,
                   "active_max": 48, "live_runtime_checked": False,
                   "scope": "B3-2A与B3-3A/B内部；B3-4/B4未开始"},
            "B3-CABI 固定历史审计精确结果不符")
    require(b34 == {"b3_4_closed": True, "cmath": "planned", "code_commit": sources["b34"]["revision"],
                    "identity_source": "fixed Git commit blobs; canonical UTF-8/LF hashes",
                    "language_available": False, "passed": True, "product_files": 1044,
                    "scope": "B3-4源码包装收口；非语言可用性"}, "B3-4 固定历史审计精确结果不符")
    require(b401 == {"b4_0_1_closed": True, "b4_0_closed": True, "b4_1_closed": True,
                     "b4_closed": False, "cmath": "planned",
                     "fixture_sha256": "0e61a2da361448477b15f5508df6d612a3f01e29a06e50699d6ed48a917d1ed3",
                     "language_available": False, "passed": True,
                     "product_commit": sources["b401"]["revision"], "product_files": 1048},
            "B4-0/1 固定历史审计精确结果不符")
    require("diagnostics::tests::real_terminal_session_is_environment_gated ... ignored" in logs["fixed", "driver_lib"],
            "driver lib 的既有 ignored 测试身份不符")
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
    require((root / ".gitattributes").is_file()
            and (root / ".gitattributes").read_bytes() == b"* -text -whitespace\n*.log -diff\n",
            "FIX1 证据目录属性不符")
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
    for label, reason in [("baseline", None), ("missing_attributes", "FIX1 证据目录属性不符"),
                          ("missing_attributes_reindexed", "FIX1 证据目录属性不符"),
                          ("tampered_log", "原始日志身份不符：concurrency"),
                          ("forged_source", "固定 Git 产品树不符：b34"),
                          ("false_closeout", "阶段收口与实际运行不符"),
                          ("false_b4_2a_exit", "B4-2A 退出与恢复门槛不符"),
                          ("refresh_forged_state", "已有收口字段被改写，拒绝刷新"),
                          ("nested_evidence", "证据字节清单不符"),
                          ("parent_escape", "执行路径包含相对跳转")]:
        with tempfile.TemporaryDirectory(prefix="b4-fix1-negative-") as task:
            copied = Path(task) / "evidence"
            shutil.copytree(root, copied)
            if label == "missing_attributes":
                (copied / ".gitattributes").unlink()
            elif label == "missing_attributes_reindexed":
                (copied / ".gitattributes").unlink()
                evidence = load(copied / "evidence.json")
                evidence["files"] = inventory(copied)
                (copied / "evidence.json").write_text(json.dumps(evidence), encoding="utf-8")
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
            elif label == "false_b4_2a_exit":
                result = load(copied / "b4-2a-exit.json")
                result["b4_2b_started"] = True
                (copied / "b4-2a-exit.json").write_text(json.dumps(result), encoding="utf-8")
            elif label == "refresh_forged_state":
                result = load(copied / "closeout.json")
                result["language_available"] = True
                (copied / "closeout.json").write_text(json.dumps(result), encoding="utf-8")
            elif label == "nested_evidence":
                nested = copied / "unlisted/evidence.json"
                nested.parent.mkdir()
                nested.write_text('{"forged":true}', encoding="utf-8")
            elif label == "parent_escape":
                env_path = copied / "fixed/environment.json"
                config = load(env_path)
                config["overrides"]["CARGO_TARGET_DIR"] = config["task_root"] + "\\..\\escaped-target"
                env_path.write_text(json.dumps(config), encoding="utf-8")
                digest = sha(env_path.read_bytes())
                for record_path in (copied / "fixed").glob("*.json"):
                    record = load(record_path)
                    if "environment_sha256" in record:
                        record["environment_sha256"] = digest
                        record_path.write_text(json.dumps(record), encoding="utf-8")
            if label in {"tampered_log", "forged_source", "false_closeout", "false_b4_2a_exit", "parent_escape"}:
                if label == "false_closeout":
                    exit_result = load(copied / "b4-2a-exit.json")
                    exit_result["b4_2b_started"] = True
                    (copied / "b4-2a-exit.json").write_text(json.dumps(exit_result), encoding="utf-8")
                evidence = load(copied / "evidence.json")
                evidence["files"] = inventory(copied)
                (copied / "evidence.json").write_text(json.dumps(evidence), encoding="utf-8")
            if label == "refresh_forged_state":
                evidence = load(copied / "evidence.json")
                evidence["files"] = inventory(copied)
                (copied / "evidence.json").write_text(json.dumps(evidence), encoding="utf-8")
            before = all_bytes(copied)
            options = ["--refresh-results"] if label == "refresh_forged_state" else []
            process = subprocess.run([sys.executable, "-X", "utf8", str(copied / "audit.py"), "--root", str(copied),
                                      "--repository-root", str(repository), *options], capture_output=True, text=True, encoding="utf-8")
            require(all_bytes(copied) == before, "审计改写文件")
            if reason is None:
                require(process.returncode == 0 and json.loads(process.stdout)["passed"], "基线反例失败")
            else:
                require(process.returncode == 1 and process.stdout.strip() == reason and not process.stderr,
                        "反例未按精确原因拒绝：" + label + " stdout=" + process.stdout.strip() + " stderr=" + process.stderr.strip())
            cases.append({"case": label, "exit_code": process.returncode, "reason": reason, "read_only": True})
    with tempfile.TemporaryDirectory(prefix="b4-fix1-task-root-negative-") as task:
        reused = Path(task) / "existing-empty-task-root"
        reused.mkdir()
        before = all_bytes(Path(task))
        try:
            require_new_task_root(reused)
        except ValueError as error:
            require(str(error) == "拒绝复用已有任务根", "任务根复用拒绝原因不符")
        else:
            raise ValueError("新任务根初始化意外接受了已有空目录")
        require(all_bytes(Path(task)) == before, "任务根复用检查改写目录")
        cases.append({"case": "reuse_existing_task_root", "exit_code": 1,
                      "reason": "拒绝复用已有任务根", "read_only": True})
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
        old_state = {key: value for key, value in old_closeout.items() if key != "runs"}
        new_state = {key: value for key, value in result.items() if key != "runs"}
        require(old_state == new_state, "已有收口字段被改写，拒绝刷新")
        require(all(result["runs"].get(key) == value for key, value in old_closeout["runs"].items()),
                "已有运行记录被替换，拒绝刷新")
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
