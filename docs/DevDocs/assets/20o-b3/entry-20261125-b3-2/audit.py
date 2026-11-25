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
GENERATED = {"closeout.json", "doc-gates.json", "evidence.json"}


def blob(commit, path):
    return git(REPO, "show", f"{commit}:{path}")


def inventory(root):
    return {path.relative_to(root).as_posix(): identity(path.read_bytes())
            for path in sorted(root.iterdir()) if path.name != "evidence.json"}


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
        env = report["environment"]
        runtime = report["runtime"]
        require(task.is_absolute() and all(PureWindowsPath(env[key]) == task / folder
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
        guard_bytes = archive.read("b1-guards-report.json")
        guards = json.loads(guard_bytes)
        guard_report = report["b1_guard_report"]
        workflow_blob = blob(PRODUCT, ".github/workflows/maintenance-regression.yml")
        workflow_git = identity(workflow_blob)
        require(guards["passed"] == guards["total"] == len(guards["cases"]) == 28
                and guards["scope"] == "simulated-cargo-output"
                and all(case["Passed"] is True for case in guards["cases"])
                and all(guard_report[key] == value for key, value in identity(guard_bytes).items())
                and guard_report["workflow_git_utf8_lf"] == workflow_git
                and guard_report["workflow_checkout"] in (workflow_git, identity(workflow_blob.replace(b"\n", b"\r\n")))
                and guards["workflowSha256"] == guard_report["workflow_checkout"]["sha256"],
                "本机 B1 模拟守卫回执不符")
    return {"target": TARGET, "runtime": runtime, "tools": report["tools"], "task_root": report["task_root"],
            "runtime_time_tests": 4, "simulated_guard_cases": 28}


def derive(root, gates, files):
    document_commit = gates["document_commit"]
    require(re.fullmatch(r"[a-f0-9]{40}", document_commit), "定稿文档提交身份不符")
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
    require({f"{PREFIX}/{name}" for name in files.keys() - GENERATED} == expected, "基线文件集合偏离文档提交")
    for name in files.keys() - GENERATED:
        require(files[name] == identity(blob(document_commit, f"{PREFIX}/{name}")), f"封存文件偏离 Git 对象：{name}")
    upstream, locks, cases, platforms = fixed_inputs()
    windows = verify_windows(root, upstream, locks)
    decisions = json.loads((root / "decisions.json").read_bytes())
    require(set(decisions["pending"]) == {f"B3-2-{name}" for name in ("ID", "PROTO", "CLI", "FIXTURES", "EVIDENCE")},
            "开工裁定台账不符")
    return {"scope": "20O-B3-2A", "b3_1_closeout_commit": B3_1_CLOSEOUT, "product_commit": PRODUCT,
            "document_commit": document_commit, "b3_2a_entry_and_baseline_verified": True,
            "b3_2_started": False, "b3_2_passed": False, "b3_3_entry_ready": False,
            "decisions_approved": False, "pending_decisions": sorted(decisions["pending"]),
            "versions": upstream["versions"], "lockfiles": locks, "tutorial_cases": cases,
            "b3_1_platforms": platforms, "b3_1_ci": upstream["ci"], "b1_guards": upstream["baseline"]["b1_guards"],
            "windows_rebuild": windows}


if __name__ == "__main__":
    try:
        files = inventory(ROOT)
        require((ROOT / "evidence.json").read_bytes() == encoded({"schema": 1, "files": files}), "证据清单与实际字节不符")
        status = derive(ROOT, json.loads((ROOT / "doc-gates.json").read_bytes()), files)
        require((ROOT / "closeout.json").read_bytes() == encoded(status), "开工基线状态与实际结果不符")
        print(encoded(status | {"audit_files": len(files)}).decode(), end="")
    except (OSError, RuntimeError, ValueError, KeyError, BadZipFile, subprocess.SubprocessError) as error:
        print(f"B3-2A 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
