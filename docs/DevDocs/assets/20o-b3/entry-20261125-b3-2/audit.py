"""只读复核 B3-2A 开工基线。"""

from io import BytesIO
import json
from pathlib import Path, PureWindowsPath
import re
import subprocess
import sys
from zipfile import BadZipFile, ZipFile

sys.dont_write_bytecode = True
from baseline import B3, B3_1, B3_1_CLOSEOUT, FREEZE, LOCKS, PRODUCT, ROOT as REPO, RUST, TARGET
from baseline import encoded, git, identity, require

ROOT = Path(__file__).resolve().parent
PREFIX = ROOT.relative_to(REPO).as_posix()
GATES = ("check:docs", "check:usedocs", "check:layout")
NEGATIVE_RECEIPT = "self-negative-result.json"
GENERATED = {"closeout.json", "doc-gates.json", "evidence.json", NEGATIVE_RECEIPT}
GENERATED_PATHS = {f"{PREFIX}/{name}" for name in GENERATED}
NEGATIVE_CASES = {
    "unchanged": None,
    "document-tree-sha256": "定稿文档树或工作树回执不符",
    "guard-script-sha256": "本机 B1 守卫脚本、工作流或回执字节不符",
    "duplicate-guard-case": "本机 B1 模拟守卫的用例集合或退出结果不符",
}


def blob(commit, path):
    return git(REPO, "show", f"{commit}:{path}")


def inventory(root):
    return {path.relative_to(root).as_posix(): identity(path.read_bytes())
            for path in sorted(root.iterdir()) if path.name != "evidence.json"}


def doc_source(commit):
    paths = ("docs/DevDocs", "docs/UseDocs", "core/rust/crates/xiao-runtime-abi/README.md")
    tree = git(REPO, "ls-tree", "-rz", commit, "--", *paths)
    rows = [row for row in tree.split(b"\0") if row
            and row.partition(b"\t")[2].decode("utf-8") not in GENERATED_PATHS]
    return {"document_commit": commit, "docs_tree_sha256": identity(b"\0".join(rows) + b"\0")["sha256"]}


def fixed_inputs():
    upstream = json.loads(blob(B3_1_CLOSEOUT, f"{B3_1}/closeout.json"))
    frozen = json.loads(blob(FREEZE, f"{B3}/entry-20261124-b3-0f/freeze-table.json"))
    require(upstream["b3_1_passed"] and upstream["b3_2_entry_ready"]
            and upstream["time_windows_coff"] == "complete" and upstream["product_commit"] == PRODUCT,
            "B3-1 固定收口不符")
    require(upstream["baseline"]["b1_guards"] == frozen["b1_guards"]
            == {"passed": 28, "actual": 4, "derived": 24}, "继承的 B1 守卫不符")
    locks = {name: identity(blob(PRODUCT, name)) for name in LOCKS}
    require(locks == upstream["baseline"]["locks"], "固定产品锁文件与 B3-1 回执不符")
    intrinsics = json.loads(blob(PRODUCT, "core/rust/crates/xiao-intrinsics/intrinsics.json"))["intrinsics"]
    table = {row["id"]: row for row in intrinsics}
    require(set(table) == {*range(1, 64), 100} and len(table) == len(intrinsics)
            and all(table[index]["status"] == "active" for index in range(1, 53))
            and all(table[index]["status"] == "reserved"
                    and table[index]["vm_binding"] == table[index]["abi_binding"] == "none"
                    for index in range(53, 64))
            and table[100]["status"] == "tombstone", "固定产品编号或绑定不符")
    registry = json.loads(blob(PRODUCT, "docs/module-registry.json"))["modules"]
    require(any(row["id"] == "stdlib.time" and row["status"] == "planned" for row in registry)
            and not any(row["id"] in ("stdlib.args", "stdlib.cli") for row in registry), "开工前登记不符")
    cases = {}
    for module in ("args", "cli"):
        case = frozen["modules"][module]["tutorial_case"]
        for name, item in case["assets"].items():
            path = f"{B3}/entry-20261124-b3-0e/{item['path']}"
            require(identity(blob(frozen["source_commit"], path)) == {key: item[key] for key in ("bytes", "sha256")},
                    f"冻结夹具与 Git 对象不符：{module}/{name}")
        cases[module] = case
    platforms = {}
    for name in ("windows", "linux", "macos"):
        with ZipFile(BytesIO(blob(B3_1_CLOSEOUT, f"{B3_1}/{name}.zip"))) as archive:
            report = json.loads(archive.read("result.json"))
        require(report["passed"] is True and report["source_commit"] == PRODUCT
                and report["runtime"]["sha256"] == upstream["platforms"][name]["runtime_sha256"],
                f"继承的 B3-1 平台身份不符：{name}")
        platforms[name] = {key: report[key] for key in ("target", "runtime", "tools", "ci")}
    return upstream, locks, cases, platforms


def verify_windows(root, upstream, locks):
    with ZipFile(root / "windows-baseline.zip") as archive:
        report = json.loads(archive.read("result.json"))
        require(report["passed"] is True and report["target"] == TARGET
                and report["product_commit"] == PRODUCT and report["product_tree"] == upstream["baseline"]["tree"],
                "Windows 重建的产品身份不符")
        for name, digest in locks.items():
            raw = blob(PRODUCT, name)
            lock = report["lockfiles"][name]
            require(lock["git_utf8_lf"] == digest and lock["checkout"] in (digest, identity(raw.replace(b"\n", b"\r\n"))),
                    f"Windows 锁文件字节不符：{name}")
        task = PureWindowsPath(report["task_root"])
        source_root = PureWindowsPath(report["source_root"])
        env = report["environment"]
        runtime = report["runtime"]
        require(task.is_absolute() and source_root.is_absolute()
                and PureWindowsPath(env["XIAO_STDLIB_ROOT"]) == source_root / "stdlib"
                and all(PureWindowsPath(env[key]) == task / folder
                for key, folder in (("CARGO_TARGET_DIR", "target"), ("XIAO_HOME", "home"),
                                    ("TMP", "temp"), ("TEMP", "temp"), ("TMPDIR", "temp")))
                and env["XIAO_B1_F10"] == env["XIAO_B2_A_PERF"] == "0"
                and PureWindowsPath(runtime["path"]) == task / "target" / TARGET / "release" / "xiao_runtime.lib"
                and runtime["features"] == ["test-failpoints"] and runtime["bytes"] > 0
                and report["runtime_after"] == {key: runtime[key] for key in ("bytes", "sha256")},
                "Windows 任务根或 Runtime 前后身份不符")
        logs = {}
        for label, command in report["commands"].items():
            require(command["exit"] == 0, f"Windows 命令未通过：{label}")
            for stream in ("stdout", "stderr"):
                item = command[stream]
                raw = archive.read(f"logs/{item['sha256']}.log")
                require(identity(raw) == item, f"命令日志字节不符：{label}/{stream}")
                if stream == "stdout":
                    logs[label] = raw.decode("utf-8").replace("\r\n", "\n")
        manifest = json.loads(blob(B3_1_CLOSEOUT, f"{B3_1}/evidence.json"))
        require(json.loads(logs["b3-1-read-only-audit"]) == upstream | {"audit_files": len(manifest["files"])}
                and json.loads(logs["b3-1-self-negative"]) == {"accepted_cases": 3, "baseline_passed": True,
                        "read_only": True, "rejected_cases": upstream["negatives"]}, "上游审计复跑结果不符")
        require(f"release: {RUST}\n" in logs["rustc"] and f"host: {TARGET}\n" in logs["rustc"]
                and all({"--locked", "--release", "--target", TARGET} <= set(report["commands"][label]["argv"])
                        for label in ("build-release-runtime", "runtime-time-boundaries"))
                and "test result: ok. 4 passed; 0 failed; 0 ignored;" in logs["runtime-time-boundaries"],
                "Windows 构建、版本或边界测试不符")
        clang_version = logs["clang"].splitlines()
        clang_targets = [line.removeprefix("Target: ") for line in clang_version if line.startswith("Target: ")]
        require(report["tools"]["clang"]["version"] == clang_version and len(clang_targets) == 1,
                "Windows clang 默认目标回执不符")
        guard_bytes = archive.read("b1-guards-report.json")
        guards = json.loads(guard_bytes)
        guard_report = report["b1_guard_report"]
        workflow_path = ".github/workflows/maintenance-regression.yml"
        workflow_blob = blob(PRODUCT, workflow_path)
        workflow_git = identity(workflow_blob)
        script_path = "tools/gates/b1-probe-guards.ps1"
        script_blob = blob(PRODUCT, script_path)
        script_hashes = {identity(raw)["sha256"] for raw in (script_blob, script_blob.replace(b"\n", b"\r\n"))}
        guard_argv = report["commands"]["b1-count-guards"]["argv"]
        require(guards["workflow"] == workflow_path and guards["scriptSha256"] in script_hashes
                and guard_argv[1:3] == ["-NoProfile", "-File"]
                and PureWindowsPath(guard_argv[3]) == source_root / script_path
                and all(guard_report[key] == value for key, value in identity(guard_bytes).items())
                and guard_report["workflow_git_utf8_lf"] == workflow_git
                and guard_report["workflow_checkout"] in (workflow_git, identity(workflow_blob.replace(b"\n", b"\r\n")))
                and guards["workflowSha256"] == guard_report["workflow_checkout"]["sha256"],
                "本机 B1 守卫脚本、工作流或回执字节不符")
        expected_cases = {(path, case)
                          for path in ("default", "Linux", "macOS", "Windows")
                          for case in ("one", "zero", "two", "ignored", "failed", "failed-exit", "duplicate")}
        cases = guards["cases"]
        require(guards["passed"] == guards["total"] == len(cases) == len(expected_cases)
                and guard_report["passed"] == guard_report["derived_cases"] == len(cases)
                and guard_report["actual_cargo_outputs"] == 0
                and guards["scope"] == "simulated-cargo-output"
                and {(case["Path"], case["Case"]) for case in cases} == expected_cases
                and all(case["Passed"] is True and type(case["ExitCode"]) is int
                        and (case["ExitCode"] == 0) == (case["Case"] == "one") for case in cases),
                "本机 B1 模拟守卫的用例集合或退出结果不符")
    windows = {"target": TARGET, "clang_default_target": clang_targets[0], "runtime": runtime,
               "tools": report["tools"], "task_root": report["task_root"], "runtime_time_tests": 4}
    local_guards = {"archive": "windows-baseline.zip", "report": "b1-guards-report.json",
                    "scope": guards["scope"], "passed": guards["passed"], "simulated_cases": len(cases),
                    "actual_cargo_outputs": guard_report["actual_cargo_outputs"]}
    return windows, local_guards


def derive(root, gates, files):
    document_commit = gates["source"]["document_commit"]
    require(re.fullmatch(r"[a-f0-9]{40}", document_commit)
            and gates["source"] == doc_source(document_commit)
            and gates["worktree_matched_before_after"] is True, "定稿文档树或工作树回执不符")
    allowed_docs = "core/rust/crates/xiao-runtime-abi/README.md"
    changes = git(REPO, "diff", "--name-only", "-z", PRODUCT, document_commit).decode().split("\0")
    require(all(not path or path.startswith("docs/DevDocs/") or path == allowed_docs for path in changes),
            "开工基线含产品改动")
    require([row["name"] for row in gates["checks"]] == list(GATES), "文档门禁缺项")
    for row in gates["checks"]:
        require(row["exit"] == 0 and all(identity(gates["logs"][row[stream]].encode())["sha256"] == row[stream]
                for stream in ("stdout", "stderr")), "文档门禁退出码或日志摘要不符")
    expected = set(git(REPO, "ls-tree", "-r", "--name-only", "-z", document_commit, "--", PREFIX)
                   .decode().split("\0")) - {""}
    expected -= GENERATED_PATHS
    require({f"{PREFIX}/{name}" for name in files.keys() - GENERATED} == expected, "基线文件集合偏离文档提交")
    for name in files.keys() - GENERATED:
        require(files[name] == identity(blob(document_commit, f"{PREFIX}/{name}")), f"封存文件偏离 Git 对象：{name}")
    upstream, locks, cases, platforms = fixed_inputs()
    windows, local_guards = verify_windows(root, upstream, locks)
    inherited = upstream["baseline"]["b1_guards"]
    b1_guards = {
        "inherited_b3_1": {"commit": B3_1_CLOSEOUT, "path": f"{B3_1}/closeout.json",
                           "passed": inherited["passed"], "actual_cargo_outputs": inherited["actual"],
                           "derived_negatives": inherited["derived"]},
        "b3_2a_windows_replay": local_guards,
    }
    decisions = json.loads((root / "decisions.json").read_bytes())
    require(set(decisions["pending"]) == {f"B3-2-{name}" for name in ("ID", "PROTO", "CLI", "FIXTURES", "EVIDENCE")},
            "开工裁定台账不符")
    return {"scope": "20O-B3-2A", "b3_1_closeout_commit": B3_1_CLOSEOUT, "product_commit": PRODUCT,
            "document_commit": document_commit, "b3_2a_entry_and_baseline_verified": True,
            "b3_2_started": False, "b3_2_passed": False, "b3_3_entry_ready": False,
            "decisions_approved": False, "pending_decisions": sorted(decisions["pending"]),
            "versions": upstream["versions"], "lockfiles": locks, "tutorial_cases": cases,
            "b3_1_platforms": platforms, "b3_1_ci": upstream["ci"], "b1_guards": b1_guards,
            "windows_rebuild": windows}


def verify_negative_receipt(root, source, files):
    require(NEGATIVE_RECEIPT in files, "缺少 B3-2A 自反例回执")
    report = json.loads((root / NEGATIVE_RECEIPT).read_bytes())
    require(report["schema"] == 1 and report["source"] == source, "自反例回执的文档身份不符")
    rows = report["cases"]
    require([row["case"] for row in rows] == list(NEGATIVE_CASES)
            and all(row["accepted"] is (NEGATIVE_CASES[row["case"]] is None)
                    and row["reason"] == NEGATIVE_CASES[row["case"]]
                    and re.fullmatch(r"[a-f0-9]{64}", row["before_sha256"])
                    and row["before_sha256"] == row["after_sha256"] for row in rows),
            "自反例结果或只读摘要不符")
    # 原样基线的回执在独立报告写入清单前采集。
    prepared = {name: item for name, item in files.items() if name != NEGATIVE_RECEIPT}
    prepared["evidence.json"] = identity(encoded({"schema": 1, "files": prepared}))
    require(rows[0]["before_sha256"] == identity(encoded(prepared))["sha256"], "自反例未绑定封存输入")


def audit(root, *, prepare=False):
    files = inventory(root)
    require((root / "evidence.json").read_bytes() == encoded({"schema": 1, "files": files}),
            "证据清单与实际字节不符")
    gates = json.loads((root / "doc-gates.json").read_bytes())
    status = derive(root, gates, files)
    require((root / "closeout.json").read_bytes() == encoded(status), "开工基线状态与实际结果不符")
    if not prepare:
        verify_negative_receipt(root, gates["source"], files)
    return status | {"audit_files": len(files)}


if __name__ == "__main__":
    try:
        print(encoded(audit(ROOT)).decode(), end="")
    except (OSError, RuntimeError, ValueError, KeyError, TypeError, BadZipFile, subprocess.SubprocessError) as error:
        print(f"B3-2A 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
