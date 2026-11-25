"""只读复核 B3-2 的固定产品、三平台结果、门禁和封存字节。"""

import argparse
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import subprocess
import sys
from zipfile import BadZipFile, ZipFile

sys.dont_write_bytecode = True
from collect import FIRST_PRODUCT, HISTORY_RUNS, REPOSITORY, WORKFLOW
from run import BOUNDARIES, HERE, LOCKS, PLATFORMS, REJECTIONS, TUTORIALS
from run import encoded, git, identity, previous_artifact, require

PREFIX = "docs/DevDocs/assets/20o-b3/entry-20261125-b3-2-final"
BASELINE = "fd5d38a045736c5695ffcc33bbc114d386826985"
PRODUCT = "879ce625c84965dc4fe4bb11523434635be55826"
MAINTENANCE = "2aae3a74b0bb3ac7e057cc1b1661183337f9e1da"
RUNNER = "56546842c9e95f4a093097d8b9a4f3eee991c0c9"
ENTRY = "docs/DevDocs/assets/20o-b3/entry-20261125-b3-2"
GATES = ("check:docs", "check:usedocs", "check:layout")
GENERATED = {"doc-gates.json", "closeout.json", "self-negative-result.json", "evidence.json"}
NEGATIVES = {
    "unchanged": None,
    "rewritten-manifest": "清单与实际字节不符",
    "missing-attributes": "缺少 .gitattributes",
    "forged-activation": "53～54 未按裁定激活",
    "missing-budget": "VM 边界回执缺项",
    "unicode-admitted": "原生参数拒绝回执不符",
    "wrong-cli-group": "原生教程与固定黄金不符",
    "forged-tutorial": "VM 教程回执缺项",
    "forged-coff": "原生产物验证回执不符",
    "forged-status": "收口状态与实测不符",
}


def blob(commit, path):
    return git("show", f"{commit}:{path}")


def inventory(root):
    return {path.relative_to(root).as_posix(): identity(path.read_bytes())
            for path in sorted(root.rglob("*")) if path.is_file() and path != root / "evidence.json"}


def doc_source(commit):
    require(re.fullmatch("[0-9a-f]{40}", commit), "定稿文档提交身份无效")
    rows = git("ls-tree", "-rz", commit, "--", "docs/DevDocs", "docs/UseDocs").split(b"\0")
    excluded = {f"{PREFIX}/{name}".encode() for name in GENERATED}
    rows = [row for row in rows if row and row.partition(b"\t")[2] not in excluded]
    return {"commit": commit, "docs_tree_sha256": identity(b"\0".join(rows) + b"\0")["sha256"]}


def verify_product(source, root):
    require(re.fullmatch("[0-9a-f]{40}", source), "产品提交身份无效")
    entry = json.loads(blob(BASELINE, f"{ENTRY}/closeout.json"))
    require(entry["b3_2a_entry_and_baseline_verified"] is True
            and entry["b3_2_started"] is False, "B3-2A 固定开工回执不符")
    baseline = "8d1c09f084be172e8b3ce078f131592ac6db94b3"
    intrinsic_path = "core/rust/crates/xiao-intrinsics/intrinsics.json"
    previous = json.loads(blob(baseline, intrinsic_path))
    current = json.loads(blob(source, intrinsic_path))
    ids = {row["id"]: row for row in current["intrinsics"]}
    for number, name in ((53, "program_name"), (54, "argv")):
        row = ids[number]
        require(row["status"] == "active" and row["kind"] == "args" and row["arity"] == "zero"
                and row["name"] == f"os_args_{name}" and row["capability"] == "process"
                and row["abi_binding"] == row["vm_binding"] == f"args_{name}", "53～54 未按裁定激活")
    for table in (current, previous):
        table["intrinsics"] = [row for row in table["intrinsics"] if row["id"] not in (53, 54)]
    require(current == previous, "53～54 以外的 intrinsic 发生变化")
    require(source == PRODUCT, "产品提交与本批固定快照不符")
    decision = json.loads((root / "decision.json").read_bytes())
    originals = json.loads(blob(BASELINE, f"{ENTRY}/decisions.json"))["approved"]
    require(len(originals) == 5 and all(row["status"] == "approved" for row in originals.values())
            and decision == json.loads(blob(source, f"{PREFIX}/decision.json"))
            and decision["answer"] == "批准生成器 24 与原生接线（推荐）"
            and (decision["runtime_abi"], decision["generator_to"], decision["stdlib_interface"]) == ("1.20", 24, 1),
            "开工或生成器批准回执不符")
    versions = {
        "core/rust/crates/xiao-codegen-llvm/src/lib.rs": ("CODEGEN_VERSION: u32 = 24;",),
        "core/rust/crates/xiao-runtime-abi/src/lib.rs": ("ABI_MAJOR_VERSION: u32 = 1;", "ABI_MINOR_VERSION: u32 = 20;"),
        "core/rust/crates/xiao-standard-library/src/lib.rs": ("STDLIB_INTERFACE_VERSION: u32 = 1;",),
    }
    for path, constants in versions.items():
        data = blob(source, path).decode()
        require(all(re.search(rf"(?m)^pub const {re.escape(value)}$", data) for value in constants), "产品版本不符")
    locks = {path: identity(blob(source, path)) for path in LOCKS}
    require(locks == {path: identity(blob(baseline, path)) for path in LOCKS}, "锁文件偏离开工基线")
    registries = [json.loads(blob(commit, "docs/module-registry.json"))["modules"] for commit in (baseline, source)]
    registry = {item["id"]: item for item in registries[1]}
    modules = ("args", "cli")
    require(all(registry[f"stdlib.{module}"]["status"] == "planned" for module in modules)
            and [item for item in registries[1] if item["id"] not in ("stdlib.args", "stdlib.cli")] == registries[0],
            "标准库登记超出 args/cli 范围")
    require(not git("diff", "--name-only", f"{baseline}..{source}", "--", "stdlib/fs.xiao", "stdlib/env.xiao",
                    "stdlib/log.xiao", "docs/UseDocs/library/fs.md", "docs/UseDocs/library/env.md",
                    "docs/UseDocs/library/log.md"), "后续模块源码发生变化")
    fixture_set = entry["tutorial_cases"]
    goldens = {}
    for module, case in fixture_set.items():
        for name, reference in case["assets"].items():
            if name == "tutorial":
                continue
            filename = PurePosixPath(reference["path"]).name
            path = f"tests/fixtures/docs/{module}/{filename}"
            data = blob(source, path)
            require(identity(data) == {key: reference[key] for key in ("bytes", "sha256")}, f"冻结夹具变更：{module}/{name}")
            if filename == "tutorial.stdout":
                goldens[module] = data
    require(set(goldens) == set(modules), "冻结教程缺少黄金输出")
    return locks, goldens, entry["b1_guards"]


def object_bytes(bundle, entry):
    raw = bundle.read(f"objects/{entry['sha256']}")
    require(identity(raw) == {key: entry[key] for key in ("bytes", "sha256")}, "归档对象摘要不符")
    return raw


def trace_matches(trace, executable, level, runtime):
    arguments = [json.loads("[" + ",".join(re.findall(r'"(?:\\.|[^"\\])*"', line)) + "]")
                 for line in trace.splitlines() if line.lstrip().startswith('"')]
    target = PureWindowsPath(executable)
    compiles = [args for args in arguments if "-cc1" in args and "-dumpdir" in args
                and PureWindowsPath(args[args.index("-dumpdir") + 1].removesuffix("-")) == target]
    links = [args for args in arguments if any(
        value.startswith("-out:") and PureWindowsPath(value[5:]) == target for value in args)]
    return len(compiles) == len(links) == 1 and (
        compiles[0][compiles[0].index("-triple") + 1].startswith("x86_64-pc-windows-msvc")
        and f"-O{level}" in compiles[0] and compiles[0][compiles[0].index("-o") + 1] in links[0]
        and any(value.startswith("/MAP:") for value in links[0])
        and PureWindowsPath(runtime) in [PureWindowsPath(value) for value in links[0]]
    )


def verify_platform(bundle, platform, commit, locks, goldens, ci):
    report = json.loads(bundle.read(f"platforms/{platform}.json"))
    target = next(triple for triple, name in PLATFORMS.items() if name == platform)
    path_type = PureWindowsPath if platform == "windows" else PurePosixPath
    task = path_type(report["task_root"])
    env = report["environment"]
    runtime = report["runtime"]
    tree = git("show", "-s", "--format=%T", commit).decode().strip()
    require(report["passed"] and report["source_commit"] == commit and report["source_tree"] == tree
            and report["lockfiles"] == locks and report["ci"] == ci and report["target"] == target,
            "平台源码、锁文件、目标或作业身份不符")
    require(all(path_type(env[var]) == task / child for var, child in
                (("CARGO_TARGET_DIR", "target"), ("XIAO_HOME", "home"), ("TEMP", "temp"), ("TMP", "temp"), ("TMPDIR", "temp")))
            and path_type(env["XIAO_STDLIB_ROOT"]) == path_type(report["source_root"]) / "stdlib"
            and env["XIAO_B1_F10"] == env["XIAO_B2_A_PERF"] == "0"
            and path_type(runtime["path"]).parent == task / "target" / target / "release"
            and runtime["bytes"] > 0 and re.fullmatch("[0-9a-f]{64}", runtime["sha256"])
            and report["runtime_after"] == {key: runtime[key] for key in ("bytes", "sha256")},
            "任务根或 Runtime 身份不符")
    commands = report["commands"]
    logs = {}
    for name, row in commands.items():
        require(row["exit"] == 0 and row["argv"], f"平台命令失败或无回执：{platform}/{name}")
        logs[name] = object_bytes(bundle, row["stdout"])
        object_bytes(bundle, row["stderr"])
    require(b"release: 1.96.0" in logs["rustc"] and f"host: {target}".encode() in logs["rustc"],
            "Rust 工具链身份不符")
    for tool in ("clang", "llvm-as", "llc"):
        item = report["tools"][tool]
        require(commands[tool]["argv"] == [item["path"], "--version"]
                and item["bytes"] > 0 and re.fullmatch("[0-9a-f]{64}", item["sha256"])
                and item["version"] == logs[tool].decode().splitlines(), "LLVM 工具版本或摘要不符")
    require("4 passed; 0 failed; 0 ignored;" in logs["runtime-arguments"].decode()
            and b"1 passed; 0 failed; 0 ignored;" in logs["runtime-abi_args"], "Runtime 参数或 C ABI 回执缺项")
    require(report["older_artifact"] == previous_artifact(platform)[1], "旧生成器原件身份不符")
    vm = logs["vm"].decode()
    require(sorted(re.findall(r"(?m)^B3-2-VM module=(\w+) alias=(\w+) exact=true\r?$", vm))
            == sorted((module, alias) for module in ("args", "cli") for alias in ("std", "stdlib")),
            "VM 教程回执缺项")
    vm_checks = ("cli_limits_count_and_utf8_bytes_before_grouping", "args_snapshot_is_shared_with_children_and_scoped_to_one_run",
                 "cli_checks_static_and_dynamic_argument_types", "cli_returns_two_fresh_groups_and_keeps_nul",
                 "protocol_rejects_invalid_and_oversized_arguments_without_echoing_them",
                 "protocol_replays_frozen_args_from_all_three_entries")
    require("test result: ok. 7 passed; 0 failed; 0 ignored;" in vm
            and all(f"test {name} ... ok" in vm for name in vm_checks), "VM 边界回执缺项")
    native = logs["native"].decode().replace("\r\n", "\n")
    require("test result: ok. 1 passed; 0 failed; 0 ignored;" in native, "原生测试回执缺项")
    rejections = re.findall(r"(?m)^B3-2-NATIVE-REJECTION case=(\w+) exit=2 code=(\S+)$", native)
    require(len(rejections) == len(REJECTIONS) and dict(rejections) == REJECTIONS, "原生参数拒绝回执不符")
    version = json.loads(re.search(r"(?m)^B3-2-GENERATOR (.+)$", native)[1])
    require(version == report["version_matrix"] and version["baseline"] == 23 and version["current"] == 24
            and version["older_metadata"] == "accepted"
            and version["baseline_fingerprint"] != version["current_fingerprint"], "版本或缓存指纹回执不符")
    raw_rows = [json.loads(line) for line in re.findall(r"(?m)^B3-2-ARTIFACT (.+)$", native)]
    rows = report["artifacts"]
    require(len(raw_rows) == len(rows) == len(TUTORIALS | BOUNDARIES)
            and {item["case"] for item in rows} == TUTORIALS | BOUNDARIES, "原生产物集合不符")
    trace = object_bytes(bundle, report["clang_commands"]).decode() if platform == "windows" else None
    for raw, row in zip(raw_rows, rows, strict=True):
        actual = object_bytes(bundle, row["file"])
        require(row["case"] == raw["case"] and raw["target"] == row["target"] == target
                and raw["optimization"] == row["optimization"] and raw["generator"] == row["generator"] == 24
                and row["verification"] == raw["verification"] == "complete"
                and row["components"] == row["observed"] == raw["components"] == raw["observed"]
                and raw["unclassified"] == row["unclassified"] == []
                and raw["old_verifier"] == row["old_verifier"] == "X17-XAR-007"
                and path_type(raw["path"]) == task / row["file"]["path"] and Path(row["file"]["path"]).parts[0] == "temp"
                and raw["sha256"] == identity(actual)["sha256"], "原生产物验证回执不符")
        if platform == "windows":
            mapping = object_bytes(bundle, row["map"])
            require(actual.startswith(b"MZ") and b"Publics by Value" in mapping
                    and path_type(raw["coff_map"]) == task / row["map"]["path"]
                    and raw["coff_map_sha256"] == identity(mapping)["sha256"]
                    and trace_matches(trace, raw["path"], row["optimization"], runtime["path"]),
                    "COFF map、真实链接命令或 MSVC 目标不符")
        if row["case"] in TUTORIALS:
            module, _, opt = row["case"].split("-")
            check = f"exact-{row['case']}"
            require(row["optimization"] == int(opt[1:]) and logs[check] == goldens[module]
                    and commands[check]["stderr"] == identity(b"")
                    and path_type(commands[check]["argv"][-1]) == path_type(raw["path"]),
                    "原生教程与固定黄金不符")
    return {"target": target, "vm": 7, "runtime": 5, "native_tutorials": len(TUTORIALS),
            "native_boundaries": len(BOUNDARIES), "native_rejections": len(REJECTIONS),
            "artifact_verification": "complete", "runtime_sha256": runtime["sha256"]}


def ci_run(bundle, label, filename):
    run = json.loads(bundle.read(f"ci/{label}/run.json"))
    jobs = json.loads(bundle.read(f"ci/{label}/jobs.json"))["jobs"]
    require(run["status"] == "completed" and run["run_attempt"] >= 1
            and run["repository"]["full_name"] == REPOSITORY and run["path"] == filename
            and jobs and all(job["head_sha"] == run["head_sha"] and job["run_id"] == run["id"] for job in jobs),
            "CI 作业身份不符")
    status = {"run": run["id"], "attempt": run["run_attempt"], "commit": run["head_sha"],
              "conclusion": run["conclusion"], "jobs": {job["name"]: job["id"] for job in jobs}}
    return run, jobs, status


def verify_ci(bundle, source):
    run, jobs, runner = ci_run(bundle, "runner", WORKFLOW)
    workflow = blob(source, f"{PREFIX}/runner.yml").replace(b"B3_ARGS_CLI_SOURCE_COMMIT", source.encode())
    require(run["head_sha"] == RUNNER and run["head_branch"] == "codex/b3-2-args-cli-runner"
            and run["conclusion"] == "success"
            and bundle.read("ci/runner/workflow.yml") == workflow, "Runner 未通过或未检出固定产品")
    null_ci = dict.fromkeys(("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "GITHUB_JOB", "RUNNER_OS"))
    platform_ci = {"windows": null_ci}
    for name, os_name in (("linux", "Linux"), ("macos", "macOS")):
        job, = (job for job in jobs if job["name"] == f"B3-2 args and cli ({name})")
        require(job["conclusion"] == "success" and any(step["name"] == "Upload receipts and products"
                and step["conclusion"] == "success" for step in job["steps"]), f"{name} 专项未通过")
        platform_ci[name] = {"GITHUB_RUN_ID": str(run["id"]), "GITHUB_RUN_ATTEMPT": str(run["run_attempt"]),
                             "GITHUB_JOB": "args-cli", "RUNNER_OS": os_name}
    _, _, workspace = ci_run(bundle, "workspace", ".github/workflows/workspace-gates.yml")
    _, maintenance_jobs, maintenance = ci_run(bundle, "maintenance", ".github/workflows/maintenance-regression.yml")
    require(workspace["commit"] == source and workspace["conclusion"] == "success",
            "固定工作区门禁未通过")
    ir = "core/rust/crates/xiao-ir/src/lower.rs"
    test_start = b"#[cfg(test)]\nmod tests {"
    gate = maintenance["commit"]
    require(gate == MAINTENANCE and git("diff", "--name-only", f"{source}..{gate}").decode().splitlines() == [ir]
            and all(test_start in blob(revision, ir) for revision in (source, gate))
            and blob(source, ir).split(test_start, 1)[0] == blob(gate, ir).split(test_start, 1)[0],
            "维护门禁改动了专项验收的生产代码")
    require(maintenance["conclusion"] == "success" and all(job["conclusion"] == "success" for job in maintenance_jobs),
            "维护门禁未通过")
    history = {}
    for label, (run_id, snapshot, name) in HISTORY_RUNS.items():
        filename = "workspace-gates.yml" if name == "workspace" else "maintenance-regression.yml"
        _, jobs, old = ci_run(bundle, label, f".github/workflows/{filename}")
        require((old["run"], old["commit"]) == (run_id, snapshot) and old["conclusion"] in ("failure", "cancelled")
                and any(job["conclusion"] == "failure" for job in jobs), "历史 CI 失败回执不符")
        history[label] = old
    return platform_ci, {"runner": runner, "workspace": workspace, "maintenance": maintenance}, history


def derive(root, prepare):
    with ZipFile(root / "runs.zip") as bundle:
        require(len(bundle.namelist()) == len(set(bundle.namelist())), "归档对象重复")
        source = json.loads(bundle.read("platforms/windows.json"))["source_commit"]
        locks, goldens, guards = verify_product(source, root)
        platform_ci, runs, history = verify_ci(bundle, source)
        results = {name: verify_platform(bundle, name, source, locks, goldens, platform_ci[name])
                   for name in PLATFORMS.values()}
        first = json.loads(bundle.read("platforms/first-windows.json"))
        failed = first["commands"]["runtime-arguments"]
        for row in first["commands"].values():
            for stream in ("stdout", "stderr"):
                object_bytes(bundle, row[stream])
        require(first["passed"] is False and first["source_commit"] == FIRST_PRODUCT
                and failed["exit"] == 101 and b"error[E0061]" in object_bytes(bundle, failed["stderr"]),
                "首轮 Windows 失败回执缺项")
    gates = json.loads((root / "doc-gates.json").read_bytes())
    commit = gates["source"]["commit"]
    require(gates["source"] == doc_source(commit) and gates["matched_before_after"] is True
            and [row["name"] for row in gates["checks"]] == list(GATES)
            and all(row["exit"] == 0 for row in gates["checks"]), "定稿文档门禁回执不符")
    for row in gates["checks"]:
        require(all(identity(gates["logs"][row[key]].encode())["sha256"] == row[key] for key in ("stdout", "stderr")),
                "文档门禁日志摘要不符")
    require(not [path for path in git("diff", "--name-only", f"{runs['maintenance']['commit']}..{commit}").decode().splitlines()
                 if not path.startswith("docs/DevDocs/")], "定稿文档修改了已验产品")
    files = inventory(root)
    fixed_names = git("ls-tree", "-r", "--name-only", "-z", commit, "--", PREFIX).decode().split("\0")
    fixed_names = {name.removeprefix(f"{PREFIX}/") for name in fixed_names if name} - GENERATED
    require(files.keys() - GENERATED == fixed_names, "封存输入文件集合与定稿 Git 对象不符")
    for name in fixed_names:
        require(files[name] == identity(blob(commit, f"{PREFIX}/{name}")), f"封存输入偏离受检 Git 对象：{name}")
    negatives = None
    if not prepare:
        receipt = json.loads((root / "self-negative-result.json").read_bytes())
        cases = receipt["cases"]
        require(receipt["document_commit"] == commit and [row["case"] for row in cases] == list(NEGATIVES)
                and all(row["before"] == row["after"] and re.fullmatch("[a-f0-9]{64}", row["before"])
                        and row["exit"] == int(NEGATIVES[row["case"]] is not None)
                        and row["reason"] == NEGATIVES[row["case"]] for row in cases), "自反例回执不符")
        negatives = sum(row["exit"] == 1 for row in cases)
    passed = negatives is not None
    status = {
        "stage": "20O-B3-2", "product_commit": source, "document_commit": commit,
        "b3_2_started": True, "b3_2a_entry_and_baseline_verified": True,
        "b3_2b_args_intrinsics_implemented": True, "b3_2c_cli_pure_xiao_implemented": True,
        "b3_2d_protocol_vm_native_consistent": True, "b3_2e_args_cli_tutorials_passed": True,
        "b3_2f_evidence_audit_passed": passed, "b3_2_passed": passed, "b3_3_entry_ready": passed,
        "b3_3_started": False, "decisions_approved": True, "no_product_scope_creep": True,
        "versions": {"abi": "1.20", "generator": 24, "stdlib_interface": 1, "protocol": 1, "core": 1},
        "platforms": results, "windows_args_cli_coff": results["windows"]["artifact_verification"],
        "ci": runs, "history": history, "b1_guards": guards, "negatives": negatives,
        "registry_status": "planned", "tutorial_status": "planned", "public_product_acceptance": False,
        "old_721_executed": False, "next": "20O-B3-3" if passed else None,
        "deferred": ["fs_env_log", "B3-7_public_matrix_performance_and_registry", "old_721_and_historical_debts"],
    }
    if passed:
        prepared = status | {"b3_2f_evidence_audit_passed": False, "b3_2_passed": False,
                             "b3_3_entry_ready": False, "negatives": None, "next": None}
        files.pop("self-negative-result.json")
        files["closeout.json"] = identity(encoded(prepared))
        files["evidence.json"] = identity(encoded({"schema": 1, "files": files}))
        require(cases[0]["before"] == identity(encoded(files))["sha256"], "原样自反例未绑定封存输入")
    return status


def audit(root, prepare=False):
    files = inventory(root)
    require(".gitattributes" in files, "缺少 .gitattributes")
    require((root / "evidence.json").read_bytes() == encoded({"schema": 1, "files": files}), "清单与实际字节不符")
    status = derive(root, prepare)
    require((root / "closeout.json").read_bytes() == encoded(status), "收口状态与实测不符")
    return status | {"audit_files": len(files)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, default=HERE)
    parser.add_argument("--prepare", action="store_true")
    arguments = parser.parse_args()
    try:
        print(encoded(audit(arguments.evidence_root.resolve(), arguments.prepare)).decode(), end="")
    except (OSError, RuntimeError, ValueError, KeyError, TypeError, BadZipFile, subprocess.SubprocessError) as error:
        print(f"B3-2 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
