"""只读复核 B3-1 time 的产品、三平台回执和收口边界。"""

import argparse
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import subprocess
import sys
from zipfile import BadZipFile, ZipFile

sys.dont_write_bytecode = True
from run import EXACT, LOCKS, ROOT as REPO, git, identity, require

ROOT = Path(__file__).resolve().parent
PREFIX = ROOT.relative_to(REPO).as_posix()
B3 = ROOT.parent.relative_to(REPO).as_posix()
ENTRY = "c3ebdaab8169d17409eeb24ee41acd22e133f0d7"
FREEZE = "7f6cc6a26cf0b1350da7a6eea09203761e1de366"
FIRST_PRODUCT = "dc1abd4b556c4396bcd817b3dc25d5d940aa7f9a"
REPOSITORY = "Programming-Language-Xiao/Xiao"
TARGETS = {"windows": "x86_64-pc-windows-msvc", "linux": "x86_64-unknown-linux-gnu",
           "macos": "aarch64-apple-darwin"}
GATES = ("check:docs", "check:usedocs", "check:layout")
CASES = [(alias, opt) for alias in ("std", "stdlib") for opt in (0, 3)]
GENERATED = {"ci.zip", "closeout.json", "doc-gates.json", "evidence.json", "matrix.json",
             "self-negative-result.json"}
CI_NULL = dict.fromkeys(("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "GITHUB_JOB", "RUNNER_OS"))


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()


def read(root, name):
    return json.loads((root / name).read_bytes())


def blob(commit, path):
    return git("show", f"{commit}:{path}")


def inventory(root):
    return {file.relative_to(root).as_posix(): identity(file.read_bytes()) for file in sorted(root.rglob("*"))
            if file.is_file() and file != root / "evidence.json"}


def changed_paths(left, right):
    return set(filter(None, git("diff", "--name-only", "-z", left, right).decode().split("\0")))


def product(source, root):
    entry = json.loads(blob(FREEZE, f"{B3}/entry-20261124-b3-0f/closeout.json"))
    require(entry["b3_1_entry_ready"] and entry["b3_0_passed"] and not entry["b3_1_started"],
            "B3-0F 固定交接不符")
    require(git("merge-base", FIRST_PRODUCT, source).decode().strip() == FIRST_PRODUCT,
            "产品提交脱离实施入口")
    require((root / "decisions.json").read_bytes() == blob(ENTRY, f"{PREFIX}/decisions.json"),
            "time 实施裁定与原件不符")
    require(git("merge-base", ENTRY, source).decode().strip() == ENTRY, "产品提交脱离已裁定入口")
    changes = changed_paths(ENTRY, source)
    allowed = (
        "core/rust/crates/xiao-artifacts/", "core/rust/crates/xiao-bytecode/",
        "core/rust/crates/xiao-codegen-llvm/", "core/rust/crates/xiao-driver/",
        "core/rust/crates/xiao-intrinsics/", "core/rust/crates/xiao-runtime-abi/",
        "core/rust/crates/xiao-runtime/", "core/rust/crates/xiao-types/", "core/rust/crates/xiao-vm/",
        "core/rust/crates/xiao-xar/", "docs/DevDocs/", "tests/fixtures/docs/time/",
    )
    files = {"core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock", "docs/module-registry.json", "stdlib/time.xiao"}
    require(changes and all(path.startswith(allowed) or path in files for path in changes), "产品改动越出 time 实施范围")
    intrinsic_path = "core/rust/crates/xiao-intrinsics/intrinsics.json"
    table = {row["id"]: row for row in json.loads(blob(source, intrinsic_path))["intrinsics"]}
    previous = {row["id"]: row for row in json.loads(blob(ENTRY, intrinsic_path))["intrinsics"]}
    require(all(table.get(key) == value for key, value in previous.items()), "既有 intrinsic 身份变化")
    frozen = json.loads(blob(FREEZE, f"{B3}/entry-20261124-b3-0f/freeze-table.json"))
    baseline = frozen["product_baseline"]
    windows = read(root, "baseline-windows.json")
    require(windows["source_commit"] == baseline["commit"] == git("rev-parse", "e79fab3c").decode().strip()
            and windows["build"]["exit"] == 0 and windows["target"] == TARGETS["windows"]
            and all(windows["locks"][path]["git_utf8_lf"] == baseline["locks"][path] == identity(blob(ENTRY, path))
                    for path in LOCKS) and frozen["b1_guards"] == {"passed": 28, "actual": 4, "derived": 24},
            "开工 Runtime、锁或 B1 守卫回执不符")
    declared = [row for case in frozen["modules"].values() for row in case["intrinsics"]]
    abi_source = blob(source, "core/rust/crates/xiao-runtime/src/abi_time.rs").decode()
    rust_intrinsics = blob(source, "core/rust/crates/xiao-intrinsics/src/lib.rs").decode()
    effects = {("clock_read",): "reads_clock", ("allocate",): "allocates", ("sleep", "cancellable"): "sleeps"}
    for row in declared:
        actual = table[row["id"]]
        status = "active" if row["id"] <= 52 else "reserved"
        require(actual["name"] == row["name"] and actual["status"] == status, "time 或预登记原语身份不符")
        if status == "reserved":
            require(actual["vm_binding"] == actual["abi_binding"] == "none", "预登记原语带可调用绑定")
        else:
            need = row["capabilities"][0] if row["capabilities"] else "none"
            require(actual["capability"] == need, "time 能力登记与冻结表不符")
            symbol = row["c_symbol"]
            binding = symbol.removeprefix("xiao_runtime_")
            parameters = row["parameters"]
            expected = [parameter.split()[1] for parameter in parameters]
            signature = [name + ": *const XiaoValue" for name in expected]
            if row["returns"] != "none":
                signature.append("out: *mut XiaoValue")
            signature.append("capabilities: u64")
            match = re.search(rf'pub extern "C" fn {symbol}\(([^)]+)\) -> i32', abi_source)
            require(match and [part.strip() for part in match[1].split(",") if part.strip()] == signature
                    and actual["abi_binding"] == actual["vm_binding"] == binding and f'"{symbol}"' in rust_intrinsics
                    and actual["return_type"] == row["returns"] and actual["effect"] == effects[tuple(row["effects"])]
                    and actual["arity"] == ("one" if len(expected) == 1 else "two"), "time 效果、签名或 C ABI 符号不符")
    require(set(table) == set(previous) | set(range(49, 64)), "活动与预登记范围不符")
    runtime_time = blob(source, "core/rust/crates/xiao-runtime/src/time.rs").decode()
    vm_time = blob(source, "core/rust/crates/xiao-vm/src/semantics/time_intrinsics.rs").decode()
    native_test = blob(source, "core/rust/crates/xiao-driver/tests/20o_b3_time_tutorial.rs").decode()
    native_build = blob(source, "core/rust/crates/xiao-codegen-llvm/src/build.rs").decode()
    coff_verifier = blob(source, "core/rust/crates/xiao-codegen-llvm/src/artifact/coff_link.rs").decode()
    require("capabilities & CAPABILITY_CLOCK == 0" in runtime_time
            and "time::require_clock(self.capabilities)" in vm_time and "time::require_clock(capabilities)" in abi_source,
            "产品源码缺少 time 能力检查")
    require('"TIME_SLEEP_LIMIT_EXCEEDED"' in runtime_time and "RuntimeError::invalid_value" in runtime_time
            and "RuntimeError::numeric_overflow" in runtime_time, "time 错误身份与冻结契约不符")
    require("ArtifactVerification::Complete" in native_test and "composition.declared_components" in native_test
            and "composition.observed_components" in native_test and "unclassified_runtime_symbols.is_empty()" in native_test
            and "compile_time_coff" in native_build and "coff_link_identity" in native_test
            and "artifact_sha256" in coff_verifier and "map_sha256" in coff_verifier
            and "pe_timestamp" in coff_verifier and "image_base" in coff_verifier,
            "Windows COFF 完整性验证没有绑定真实链接映射和映像")
    require(b"pub const ABI_MINOR_VERSION: u32 = 20;" in blob(source, "core/rust/crates/xiao-runtime-abi/src/lib.rs")
            and b"pub const CODEGEN_VERSION: u32 = 23;" in blob(source, "core/rust/crates/xiao-codegen-llvm/src/lib.rs")
            and b"pub const STDLIB_INTERFACE_VERSION: u32 = 1;" in blob(source, "core/rust/crates/xiao-standard-library/src/lib.rs"),
            "产品版本与冻结目标不符")
    fixture = "tests/fixtures/docs/time"
    for name in ("tutorial.xiao", "tutorial.stdout", "tutorial.input.json"):
        original = f"{B3}/entry-20261124-b3-0e/candidates/{fixture}/{name}"
        require(blob(source, f"{fixture}/{name}") == blob(ENTRY, original), "正式 time 夹具偏离冻结字节")
    registry = "docs/module-registry.json"
    original = json.loads(blob(ENTRY, registry))["modules"]
    current = json.loads(blob(source, registry))["modules"]
    added = [row for row in current if row["id"] == "stdlib.time"]
    require(len(added) == 1 and added[0]["status"] == "planned"
            and [row for row in current if row["id"] != "stdlib.time"] == original,
            "time 的 planned 登记越界")
    return {"tree": git("show", "-s", "--format=%T", source).decode().strip(),
            "locks": {path: identity(blob(source, path)) for path in LOCKS},
            "baseline": baseline["commit"], "b1_guards": frozen["b1_guards"]}


def log(bundle, info):
    raw = bundle.read("logs/" + info["sha256"] + ".log")
    require(identity(raw) == info, "平台日志摘要不符")
    return raw.decode("utf-8").replace("\r\n", "\n")


def artifact_bytes(bundle, row):
    raw = bundle.read(row["path"])
    require(identity(raw) == {key: row[key] for key in ("bytes", "sha256")}, "原生产物或链接映射字节不符")
    return raw


def platform(root, name, source, fixed, ci):
    with ZipFile(root / f"{name}.zip") as bundle:
        receipt = json.loads(bundle.read("result.json"))
        require(receipt["passed"] is True and receipt["source_commit"] == source
                and receipt["source_tree"] == fixed["tree"] and receipt["lockfiles"] == fixed["locks"]
                and receipt["target"] == TARGETS[name] and receipt["runtime"]["test_failpoints"] is True,
                f"{name} 功能回执或源码身份不符")
        require(receipt["ci"] == ci, f"{name} CI 身份不符")
        path = PureWindowsPath if name == "windows" else PurePosixPath
        task = path(receipt["task_root"])
        environment = receipt["environment"]
        runtime = receipt["runtime"]
        require(path(environment["CARGO_TARGET_DIR"]) == task / "target"
                and path(environment["XIAO_HOME"]) == task / "home"
                and all(path(environment[key]) == task / "temp" for key in ("TEMP", "TMP", "TMPDIR"))
                and path(environment["XIAO_STDLIB_ROOT"]) == path(receipt["source_root"]) / "stdlib"
                and environment["XIAO_B1_F10"] == environment["XIAO_B2_A_PERF"] == "0"
                and path(runtime["path"]).is_relative_to(task / "target" / TARGETS[name] / "release")
                and receipt["runtime_after"] == {key: runtime[key] for key in ("bytes", "sha256")}
                and re.fullmatch(r"[a-f0-9]{64}", runtime["sha256"]) and runtime["bytes"] > 0,
                "平台任务根或 Runtime 身份不符")
        commands = receipt["commands"]
        require(set(commands) == {"rustc", "clang", "llvm-as", "llc", "build-release-runtime",
                                  "runtime-boundaries", "vm-exact", "vm-boundaries", "native-exact"},
                "平台验收命令缺项")
        outputs = {}
        for label, command in commands.items():
            require(command["exit"] == 0, f"{name} 命令 {label} 未通过")
            outputs[label] = log(bundle, command["stdout"])
            log(bundle, command["stderr"])
        require(f"host: {TARGETS[name]}\n" in outputs["rustc"] and "release: 1.96.0\n" in outputs["rustc"],
                "平台 Rust 工具身份不符")
        for tool in ("clang", "llvm-as", "llc"):
            tool_info = receipt["tools"][tool]
            require(tool_info["path"] == commands[tool]["argv"][0] and tool_info["version"] == outputs[tool].splitlines()
                    and re.fullmatch(r"[a-f0-9]{64}", tool_info["sha256"]) and tool_info["bytes"] > 0,
                    "平台 LLVM 工具身份与原始输出不符")
        for label in ("build-release-runtime", "runtime-boundaries", "vm-exact", "vm-boundaries", "native-exact"):
            argv = commands[label]["argv"]
            require(argv[:2] == ["cargo", "+1.96.0"] and "--locked" in argv and TARGETS[name] in argv,
                    "平台命令的 Rust、锁或目标不符")
        require(all("--release" in commands[label]["argv"] for label in ("build-release-runtime", "runtime-boundaries")),
                "Runtime 边界使用了非 release 构建")
        for label, test in (("vm-exact", "time_tutorial_matches_vm_golden"),
                            ("native-exact", "time_tutorial_matches_native_golden")):
            require(outputs[label].count(EXACT) == 1
                    and f"test {test} ... ok" in outputs[label]
                    and test in commands[label]["argv"] and "--exact" in commands[label]["argv"],
                    "教程执行回执缺少精确通过")
        require(all(f"backend=vm alias={alias} exact=true" in outputs["vm-exact"]
                    for alias in ("std", "stdlib")), "VM 导入覆盖不符")
        require("test result: ok. 4 passed; 0 failed; 0 ignored;" in outputs["runtime-boundaries"]
                and "validates_borrowed_inputs_and_preserves_owned_output_on_failure ... ok" in outputs["runtime-boundaries"]
                and all(f"test {case} ... ok" in outputs["vm-boundaries"] for case in (
                    "time_vm_errors_and_child_clock_follow_the_frozen_contract",
                    "time_driver_control_interrupts_a_long_sleep", "reserved_intrinsic_ids_fail_before_execution")),
                "time 错误、能力或 ABI 边界回执缺项")
        samples = {}
        for milliseconds in (1, 10, 100):
            match = re.search(rf"requested_ms={milliseconds} samples_ns=(\[[0-9, ]+\])", outputs["runtime-boundaries"])
            require(match, "缺少真实 sleep 样本")
            values = json.loads(match[1])
            require(len(values) == 5 and min(values) >= milliseconds * 1_000_000, "真实 sleep 提前结束")
            samples[str(milliseconds)] = values
        match = re.search(r"cancellation_ns=(\[[0-9, ]+\])", outputs["runtime-boundaries"])
        require(match, "缺少真实取消样本")
        cancel = sorted(json.loads(match[1]))
        require(len(cancel) == 5 and cancel[2] <= 100_000_000 and cancel[4] <= 1_000_000_000, "取消延迟超出冻结阈值")
        artifacts = receipt["artifacts"]
        require([(row["alias"], row["optimization"]) for row in artifacts] == CASES,
                "原生教程导入和等级不符")
        boundaries = receipt["native_boundaries"]
        require([row["case"] for row in boundaries] == ["clock", "denied"], "原生授权或时钟探测缺项")
        markers = [f"B3-1-TUTORIAL backend=native alias={alias} opt={opt}" for alias, opt in CASES]
        markers += [f"B3-1-BOUNDARY case={row['case']}" for row in boundaries]
        native = set(outputs["native-exact"].splitlines())
        if name == "windows":
            # 比较时统一 Windows 原生标记里的路径分隔符，日志摘要已按原始字节校验。
            native = {line.replace("\\", "/") for line in native}
        for row, marker in zip(artifacts + boundaries, markers, strict=True):
            archive_path = PurePosixPath(row["path"])
            require(archive_path.parts[:1] == ("temp",) and ".." not in archive_path.parts
                    and (name != "windows" or "\\" not in row["path"]),
                    "原生产物越出任务根")
            output_path = (task / row["path"]).as_posix()
            require(row["verification"] == "complete", "原生产物验证等级不符")
            require(f"{marker} verification=complete exact=true artifact={output_path}" in native,
                    "原生教程或边界执行回执缺项")
            magic = {"windows": b"MZ", "linux": b"\x7fELF", "macos": b"\xcf\xfa\xed\xfe"}[name]
            require(artifact_bytes(bundle, row).startswith(magic), "原生产物格式与目标不符")
            if name == "windows":
                link = row["linker_map"]
                map_path = (task / link["path"]).as_posix()
                require(link["path"] == row["path"] + ".map"
                        and f"B3-1-COFF artifact={output_path} sha256={row['sha256']} "
                            f"map={map_path} map_sha256={link['sha256']}" in native,
                        "COFF 映像或 map 未绑定验证回执")
                require(b"Publics by Value" in artifact_bytes(bundle, link), "COFF 映射缺少链接器符号表")
        return {"native": len(artifacts), "native_boundaries": len(boundaries), "vm": 2, "runtime_sha256": runtime["sha256"],
                "sleep_samples_ns": samples, "cancel_median_ns": cancel[2], "cancel_max_ns": cancel[4]}


def doc_source(commit):
    paths = ("docs/DevDocs", "docs/UseDocs", "core/rust/crates/xiao-runtime-abi/README.md")
    tree = git("ls-tree", "-rz", commit, "--", *paths)
    rows = []
    for record in tree.split(b"\0"):
        if not record:
            continue
        path = record.partition(b"\t")[2].decode("utf-8")
        if path.startswith(PREFIX + "/") and path[len(PREFIX) + 1:] in GENERATED:
            continue
        rows.append(record)
    return {"commit": commit, "tree_sha256": identity(b"\0".join(rows) + b"\0")["sha256"]}


def ci_run(bundle, label, commit, filename):
    run = json.loads(bundle.read(f"{label}/run.json"))
    jobs = json.loads(bundle.read(f"{label}/jobs.json"))["jobs"]
    require(run["head_sha"] == commit and run["status"] == "completed" and run["run_attempt"] >= 1
            and run["repository"]["full_name"] == REPOSITORY and run["path"] == f".github/workflows/{filename}"
            and all(job["head_sha"] == commit and job["run_id"] == run["id"] for job in jobs),
            "CI 运行身份或快照不符")
    return run, jobs


def verify_ci(root, source, matrix):
    with ZipFile(root / "ci.zip") as bundle:
        run, jobs = ci_run(bundle, "time", matrix["runner_commit"], "b3-1-time.yml")
        require(run["head_branch"] == "codex/b3-1-time-runner" and run["conclusion"] == "success", "Unix time Runner 未通过")
        require(bundle.read("time/workflow.yml") == blob(source, f"{PREFIX}/time-runner.yml").replace(
            b"B3_TIME_SOURCE_COMMIT", source.encode()), "CI 工作流未固定同一个产品对象")
        runs = {"time": {"run": run["id"], "attempt": run["run_attempt"], "jobs": {}}}
        platform_ci = {"windows": CI_NULL}
        for platform, runner_os in (("linux", "Linux"), ("macos", "macOS")):
            job, = [job for job in jobs if job["name"] == f"B3-1 time ({runner_os})"]
            require(job["conclusion"] == "success" and any(step["name"] == "Preserve original receipts and logs"
                    and step["conclusion"] == "success" for step in job["steps"]), "Unix time 作业或证据上传未通过")
            runs["time"]["jobs"][platform] = job["id"]
            platform_ci[platform] = {"GITHUB_RUN_ID": str(run["id"]), "GITHUB_RUN_ATTEMPT": str(run["run_attempt"]),
                                     "GITHUB_JOB": "time", "RUNNER_OS": runner_os}
        run, jobs = ci_run(bundle, "workspace", source, "workspace-gates.yml")
        job, = [job for job in jobs if job["name"] == "Linux workspace gates"]
        require(run["conclusion"] == job["conclusion"] == "success" and any(
            step["name"] == "Run workspace gates" and step["conclusion"] == "success" for step in job["steps"]),
            "固定产品 workspace-gates 未通过")
        runs["workspace_gates"] = {"run": run["id"], "attempt": run["run_attempt"], "job": job["id"]}
    return platform_ci, runs


def verify_history(root):
    with ZipFile(root / "history.zip") as bundle:
        summary = {}
        for name, expected in (("first-linux", False), ("first-macos", True), ("second-linux", False), ("second-windows", False)):
            receipt = json.loads(bundle.read(f"{name}/result.json"))
            fixed = FIRST_PRODUCT if name.startswith("first-") else "b0f3edbbfe2f6a3b2bea05b3499692a1a7d978f5"
            require(receipt["passed"] is expected and receipt["source_commit"] == fixed,
                    "前轮平台回执状态或源码身份不符")
            for command in receipt["commands"].values():
                for stream in ("stdout", "stderr"):
                    info = command[stream]
                    require(identity(bundle.read(f"logs/{info['sha256']}.log")) == info, "前轮平台日志摘要不符")
            summary[name] = {"source_commit": fixed, "passed": expected, "ci": receipt["ci"]}
        return summary


def derive(root, prepare):
    matrix = read(root, "matrix.json")
    source = matrix["source_commit"]
    fixed = product(source, root)
    ci, runs = verify_ci(root, source, matrix)
    results = {name: platform(root, name, source, fixed, ci[name]) for name in TARGETS}
    history = verify_history(root)
    gate = read(root, "doc-gates.json")
    docs = gate["source"]["commit"]
    require(matrix["document_commit"] == docs, "平台/CI 矩阵未绑定最终文档提交")
    require(gate["source"] == doc_source(docs) and gate["worktree_matched_before_after"] is True
            and [row["name"] for row in gate["checks"]] == list(GATES)
            and all(row["exit"] == 0 for row in gate["checks"]), "定稿文档门禁未通过")
    for row in gate["checks"]:
        for stream in ("stdout", "stderr"):
            digest = row[f"{stream}_sha256"]
            require(identity(gate["logs"][digest].encode())["sha256"] == digest, "文档门禁原始日志摘要不符")
    document_paths = changed_paths(source, docs)
    require(all(path.startswith("docs/DevDocs/") or path == "core/rust/crates/xiao-runtime-abi/README.md"
                for path in document_paths),
            "定稿提交改动了受检产品")
    expected_files = set(git("ls-tree", "-r", "--name-only", "-z", docs, "--", PREFIX).decode().split("\0")) - {""}
    expected_files -= {f"{PREFIX}/{name}" for name in GENERATED}
    current = inventory(root)
    require({f"{PREFIX}/{name}" for name in current if name not in GENERATED} == expected_files,
            "封存输入文件集合与受检 Git 对象不符")
    for name in sorted(current.keys() - GENERATED):
        require(current[name] == identity(blob(docs, f"{PREFIX}/{name}")), f"封存输入偏离受检 Git 对象：{name}")
    negatives = None if prepare else read(root, "self-negative-result.json")
    if negatives is not None:
        wanted = read(root, "negative-cases.json")
        require([row["case"] for row in negatives] == list(wanted)
                and all(row["before"] == row["after"] and row["exit"] == int(bool(wanted[row["case"]]))
                        and row["error"] == wanted[row["case"]] for row in negatives), "自反例回执不符")
    passed = negatives is not None
    return {
        "scope": "20O-B3-1", "product_commit": source, "document_commit": docs,
        "b3_1_started": True, "b3_1_passed": passed, "b3_2_entry_ready": passed,
        "decisions_approved": True, "no_product_scope_creep": True,
        "b3_0_passed": True, "b3_product_started": True, "old_721_executed": False,
        "b3_1a_entry_and_baseline_verified": True, "b3_1b_time_intrinsics_implemented": True,
        "b3_1c_vm_tac_native_consistent": True, "b3_1d_c_abi_verified": True,
        "b3_1e_time_tutorial_and_boundaries_passed": True, "b3_1f_evidence_audit_passed": passed,
        "versions": {"abi": "1.20", "generator": 23, "stdlib_interface": 1},
        "platforms": results, "ci": runs, "history": history, "baseline": fixed,
        "time_windows_coff": "complete", "registry_status": "planned",
        "tutorial_status": "planned", "public_product_acceptance": False,
        "next": "20O-B3-2" if passed else None, "b3_2_started": False,
        "deferred": ["args_cli_fs_env_log", "B3-7_public_matrix_and_tutorial_registry", "old_721_and_historical_debts"],
        "negatives": None if negatives is None else len(negatives) - 1,
    }


def audit(root, prepare=False):
    actual = inventory(root)
    require(".gitattributes" in actual, "封存缺少 .gitattributes")
    require((root / "evidence.json").read_bytes() == encoded({"schema": 1, "files": actual}),
            "证据清单与实际字节不符")
    status = derive(root, prepare)
    require((root / "closeout.json").read_bytes() == encoded(status), "收口状态与实际结果不符")
    return status | {"audit_files": len(actual)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, default=ROOT)
    parser.add_argument("--prepare", action="store_true")
    args = parser.parse_args()
    try:
        print(encoded(audit(args.evidence_root.resolve(), args.prepare)).decode(), end="")
    except (OSError, RuntimeError, KeyError, TypeError, ValueError, BadZipFile, subprocess.SubprocessError) as error:
        print(f"B3-1 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
