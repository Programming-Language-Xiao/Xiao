"""归档 B6-4 原始 CI、守卫回放和定稿门禁。默认只读复核固定 Git 对象。"""

import argparse
from collections import Counter
from io import BytesIO
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import shutil
import sys
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT.with_name("entry-20261118-b6-3")))

from audit import Evidence, collect, digest
from guards import step, variants
from prepare import git, require, write_json
from run import job, observe_suite

PRODUCT = "58176ff5299441f921f3b54fbdc300e2e46b9bb0"
PLANNING = "5376f7fe5ae7dbd1081fd799eed374650e041a7e"
FIRST = "5758aa51dd54b30d62fb450b8518c3f92e336909"
RERUN = "97d5f288117d496706d24d5aac304b3c122c6d62"
FIRST_SUPPLEMENT = "9ea348f4ab0dee4fda2b2ee86086bb05df4f3f47"
SUPPLEMENT = "907c4cb35cee1abc70fc5a93943a0e46610ac44d"
CLOSEOUT = "0af1575b9612597a44621477c1610da09814b9c0"
DOCS_SEAL = "e154b9e75d16c42ff83ef3e06be247a89c1e6cff"
WORKSPACE = ".github/workflows/workspace-gates.yml"
MAINTENANCE = ".github/workflows/maintenance-regression.yml"
EXACT = ".github/workflows/b2-b6-b1-supplement.yml"
RUNNER_BRANCH = "codex/b6-4-gates-runner"
SUPPLEMENT_BRANCH = "codex/b6-4-b1-runner"
RUNS = {
    44460712945: (FIRST, WORKSPACE, "push", "failure", (135346523473,)),
    44460712977: (FIRST, MAINTENANCE, "push", "failure",
                  (135346523696, 135346523733, 135346523758, 135346523781, 135346523788, 135346524047)),
    44464422364: (RERUN, WORKSPACE, "workflow_dispatch", "success", (135358660716,)),
    44464422497: (RERUN, MAINTENANCE, "workflow_dispatch", "failure",
                  (135358661136, 135358661515, 135358661585, 135358661593, 135358661595, 135358661620)),
    44467531829: (FIRST_SUPPLEMENT, EXACT, "push", "success", (135368808231, 135368808627)),
    44470256490: (SUPPLEMENT, EXACT, "push", "success", (135377693561, 135377693905)),
}
B1 = {
    "default": (44464422497, 135358661595, "Rust workspace default tests (Linux)"),
    "windows": (44464422497, 135358661620, "Native differential regression (Windows)"),
    "linux": (44470256490, 135377693905, "B6-4 B1 native exact (linux)"),
    "macos": (44470256490, 135377693561, "B6-4 B1 native exact (macos)"),
}
HISTORICAL_RUST = {135368808231: "1.100.0", 135368808627: "1.99.1"}
RUNNER_TARGETS = {
    "ubuntu-24.04": ("Linux", "X64", "x86_64-unknown-linux-gnu"),
    "macos-15": ("macOS", "ARM64", "aarch64-apple-darwin"),
    "windows-2025": ("Windows", "X64", "x86_64-pc-windows-msvc"),
}
LOCKS = ("core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock")
PROTECTED = (
    ".gitattributes", ".cargo", "LICENSE", "core", "stdlib", "tests", "tools", "cli", "resources",
    "package.json", "bun.lock", "tsconfig.json", "docs/UseDocs", "docs/module-registry.json",
)
GUARD = "tools/gates/b1-probe-guards.ps1"
CONTROLLED = "tools/gates/controlled-cargo.py"
GATE = "tools/gates/run.sh"
RECIPE = ROOT.relative_to(REPO).as_posix()
CI_IDENTITY = f"{RECIPE}/ci_identity.py"
LINUX_RUNTIME = f"{RECIPE}/linux_runtime.sh"
DOCS = tuple(f"docs/DevDocs/{name}" for name in (
    "README.md", "20o-official-library-program.md", "20o-b2-b-implementation-plan.md",
    "20o-b2-b6-3-boundary-plan.md", "20o-b2-b6-4-ci-gates-plan.md",
    "20o-b2-b6-resource-performance-plan.md", "20o-b2-b7-closeout-plan.md",
)) + (f"{RECIPE}/README.md",)
ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
TIMESTAMP = re.compile(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z ", re.M)
TEST_COUNTS = re.compile(r"^test result: (?:ok\.|FAILED\.) (\d+) passed; (\d+) failed; (\d+) ignored;", re.M)
DEFAULT_COMMAND = ("cargo test --manifest-path core/rust/Cargo.toml --workspace -- "
                   "--skip b1_capabilities_have_vm_golden_outputs")
CMP_TEST = "native_static_dynamic_comparisons_match_vm_o0_o3"
DIAGNOSTIC_TEST = "b6_diagnoses_linux_native_cmath_runtime_component_observation"
COMPONENT_ERROR = "产物观察到未由 IR 登记的 Runtime 组件：rc"
RELEASE_ERROR = "native-lock-wait-027: 稳定共享对象释放顺序"


def blob(commit, path):
    return git("show", f"{commit}:{path}")


def verify_sealed_archive(raw, name, commit):
    require(raw == blob(commit, f"{RECIPE}/{name}"), f"归档与封存提交不符：{name}")


def source_identity():
    for commit in (PLANNING, FIRST, RERUN, FIRST_SUPPLEMENT, SUPPLEMENT):
        require(not git("diff", "--name-only", PRODUCT, commit, "--", *PROTECTED),
                f"受保护产品树发生变化：{commit}")
    require(set(git("diff", "--name-only", PLANNING, FIRST).decode().splitlines())
            == {WORKSPACE, MAINTENANCE, CI_IDENTITY}, "首轮执行快照范围不符")
    require(set(git("diff", "--name-only", FIRST, RERUN).decode().splitlines())
            == {WORKSPACE, MAINTENANCE, CI_IDENTITY, LINUX_RUNTIME}, "复跑执行快照范围不符")
    for older, newer in ((RERUN, FIRST_SUPPLEMENT), (FIRST_SUPPLEMENT, SUPPLEMENT)):
        require(git("diff", "--name-only", older, newer).decode().splitlines() == [EXACT],
                f"补验超出专用工作流：{newer}")
    for workflow in (WORKSPACE, MAINTENANCE):
        pattern = rb"^[ \t]*(?:timeout-minutes: .*|cancel-in-progress: .*)$"
        require(re.findall(pattern, blob(PLANNING, workflow), re.M)
                == re.findall(pattern, blob(RERUN, workflow), re.M), "工作流时限或并发设置变化")
    return {
        "product": PRODUCT, "planning": PLANNING,
        "locks": {path: digest(blob(PRODUCT, path)) for path in LOCKS},
        "snapshots": {
            commit: {path: digest(blob(commit, path)) for path in paths}
            for commit, paths in (
                (FIRST, (WORKSPACE, MAINTENANCE, CI_IDENTITY, GATE, CONTROLLED, GUARD)),
                (RERUN, (WORKSPACE, MAINTENANCE, CI_IDENTITY, LINUX_RUNTIME, GATE, CONTROLLED, GUARD,
                         "tools/gates/b2-a6-selection.py", "package.json", "bun.lock")),
                (FIRST_SUPPLEMENT, (EXACT, CI_IDENTITY, LINUX_RUNTIME, CONTROLLED)),
                (SUPPLEMENT, (EXACT, MAINTENANCE, CI_IDENTITY, LINUX_RUNTIME, CONTROLLED)),
            )
        },
    }


def clean_log(raw):
    return TIMESTAMP.sub("", ANSI.sub("", raw.decode("utf-8-sig").replace("\r\n", "\n")))


def step_result(job_row, name, expected):
    rows = [step for step in job_row["steps"] if step["name"] == name]
    require(len(rows) == 1 and rows[0]["status"] == "completed" and rows[0]["conclusion"] == expected,
            f"作业步骤结果不符：{job_row['id']}/{name}")
    return rows[0]["number"]


def checked_file(item):
    require(item["bytes"] > 0 and re.fullmatch("[0-9a-f]{64}", item["sha256"])
            and item["path"], "文件身份字段无效")


def observed_identity(log, run, job_row):
    pairs = re.findall(r"^B6-4-IDENTITY (before|native|after) (\{.+\})$", log, re.M)
    values = {phase: json.loads(raw) for phase, raw in pairs}
    require(len(values) == len(pairs), f"Runner 身份阶段重复：{job_row['id']}")
    if not values:
        require(job_row["name"].startswith(("Deterministic parser", "Deterministic artifact")),
                f"主门禁缺少 Runner 身份：{job_row['id']}")
        return values
    head = run["head_sha"]
    devdocs = git("rev-parse", f"{head}:docs/DevDocs").decode().strip()
    tree = git("cat-file", "-p", head).decode().splitlines()[0].removeprefix("tree ")
    labels = job_row["labels"]
    require(len(labels) == 1 and labels[0] in RUNNER_TARGETS, f"Runner 标签不符：{job_row['id']}")
    runner_os, runner_arch, target = RUNNER_TARGETS[labels[0]]
    for row in values.values():
        context = row["context"]
        require(context["GITHUB_SHA"] == head and context["GITHUB_RUN_ID"] == str(run["id"])
                and context["GITHUB_RUN_ATTEMPT"] == "1" and context["GITHUB_EVENT_NAME"] == run["event"]
                and context["GITHUB_REF"] == f"refs/heads/{run['head_branch']}"
                and (context["RUNNER_OS"], context["RUNNER_ARCH"]) == (runner_os, runner_arch)
                and context["ImageVersion"],
                f"Runner 上下文不符：{job_row['id']}")
        if run["path"] != WORKSPACE:
            require(context["XIAO_B1_F10"] == context["XIAO_B2_A_PERF"] == "0",
                    f"性能模式状态不符：{job_row['id']}")
    before, after = values["before"], values["after"]
    for snapshot in (before, after):
        require((snapshot["head"], snapshot["tree"], snapshot["devdocs_tree"]) == (head, tree, devdocs)
                and snapshot["tracked_changes"] == [] and set(snapshot["locks"]) == set(LOCKS),
                f"实际检出或锁文件发生变化：{job_row['id']}")
        for name, info in snapshot["locks"].items():
            fixed = blob(head, name)
            require({key: info[key] for key in ("bytes", "sha256")}
                    in (digest(fixed), digest(fixed.replace(b"\n", b"\r\n"))),
                    f"锁文件与固定 Git 对象不符：{name}")
    rust_version = HISTORICAL_RUST.get(job_row["id"], "1.96.0")
    require(before["locks"] == after["locks"] and before["cargo_target_dir"] == after["cargo_target_dir"]
            and before["rustc"].startswith(f"rustc {rust_version} ")
            and f"host: {target}" in before["rustc"].splitlines()
            and "xiao-controlled-cargo-" in before["controlled_cargo"],
            f"锁、目标目录或 Cargo 工具身份不符：{job_row['id']}")
    checked_file(before["cargo_delegate"])
    before_uses_native = job_row["name"].startswith("Native differential")
    require(("native" in values) == (head != FIRST or before_uses_native),
            f"Runtime 身份回执缺少或阶段不符：{job_row['id']}")
    if "native" in values:
        native = values["native"]
        path_type = PureWindowsPath if runner_os == "Windows" else PurePosixPath
        library = "xiao_runtime.lib" if runner_os == "Windows" else "libxiao_runtime.a"
        target_dir = path_type(before["cargo_target_dir"])
        require(before["runtime"] is None and target_dir.is_absolute()
                and path_type(native["runtime"]["path"]) == target_dir / "release" / library,
                f"Runtime 不在本次构建目标目录，或构建前已存在：{job_row['id']}")
        require(native["runtime"] == after["runtime"] and native["target"] == target
                and set(native["llvm"]) == {"XIAO_CLANG", "XIAO_LLC", "XIAO_LLVM_AS", "XIAO_STRIP"},
                f"Runtime 或工具目标不符：{job_row['id']}")
        checked_file(native["runtime"])
        for tool in native["llvm"].values():
            checked_file(tool)
            require(bool(tool["version"]), "缺少 LLVM 版本")
        if before_uses_native:
            checked_file(native["diagnostics"])
            if head != FIRST:
                require(native["float_equality_output"]["exists"] is False, "浮点比较输出路径已存在")
    return values


def cache_observation(log):
    if "Run Swatinem/rust-cache@v2" in log:
        if re.search(r"^Cache hit for(?: restore-key)?: v0-rust-", log, re.M):
            return "rust-cache:hit"
        require("No cache found." in log, "缺少 rust-cache 命中回执")
        return "rust-cache:miss"
    if "Using a cached version of Bun:" in log:
        return "setup-bun:hit"
    return "no-rust-cache-action"


def read_run(read, run_id, spec):
    head, workflow, event, result, expected_ids = spec
    prefix = f"ci/{run_id}"
    run = json.loads(read(f"{prefix}/run.json"))
    jobs = json.loads(read(f"{prefix}/jobs.json"))
    branch = SUPPLEMENT_BRANCH if workflow == EXACT else RUNNER_BRANCH
    require((run["id"], run["head_sha"], run["path"], run["event"], run["conclusion"],
             run["head_branch"], run["run_attempt"], run["status"])
            == (run_id, head, workflow, event, result, branch, 1, "completed"), f"CI 运行身份不符：{run_id}")
    require(sorted(item["id"] for item in jobs["jobs"]) == list(expected_ids)
            and jobs["total_count"] == len(expected_ids), f"CI 作业ID清单不符：{run_id}")
    observed = {}
    for row in jobs["jobs"]:
        ident = row["id"]
        require((row["run_id"], row["run_attempt"], row["head_sha"], row["head_branch"], row["status"])
                == (run_id, 1, head, branch, "completed"), f"CI 作业身份不符：{ident}")
        path = f"{prefix}/{ident}.log"
        raw = read(path)
        log = clean_log(raw)
        require(f"Complete job name: {row['name']}" in log and f"\n{head}\n" in log,
                f"原日志与作业名或检出提交不符：{ident}")
        failures = [item["name"] for item in row["steps"] if item["conclusion"] == "failure"]
        require(row["conclusion"] == ("failure" if failures else "success"),
                f"作业状态与步骤矛盾：{ident}")
        if workflow == MAINTENANCE:
            for item in row["steps"]:
                if item["name"].startswith(("Measure controlled", "Upload B2-A6", "Upload B1")):
                    require(item["conclusion"] == "skipped", f"性能步骤被执行：{ident}")
        observed[ident] = {
            "api": row, "text": log, "source": path, "sha256": digest(raw)["sha256"],
            "cache": cache_observation(log), "identity": observed_identity(log, run, row),
            "failed_steps": failures,
        }
    return run, observed


def require_parts(log, parts, label):
    matches = [log.find(part) for part in parts]
    require(all(index >= 0 for index in matches) and matches == sorted(matches), f"日志顺序或命令缺失：{label}")


def slice_log(log, begin, end):
    lines = log.splitlines()
    require(lines.count(begin) == lines.count(end) == 1, f"实际执行边界缺失或重复：{begin}")
    first, last = lines.index(begin), lines.index(end)
    require(first < last, f"实际执行边界顺序错误：{begin}")
    return "\n".join(lines[first + 1:last]) + "\n"


def counts(log):
    totals = [tuple(map(int, items)) for items in TEST_COUNTS.findall(log)]
    require(bool(totals), "缺少实际测试计数")
    return dict(zip(("passed", "failed", "ignored"), map(sum, zip(*totals))))


def engineering(observed):
    workspace = observed[135358660716]
    default = observed[135358661595]
    step_result(workspace["api"], "Run workspace gates", "success")
    for name in ("Run default workspace tests", "Run B1 VM probe exactly once",
                 "Run benchmark workspace tests and driver self-test", "Verify B1 exact-count guards"):
        step_result(default["api"], name, "success")
    repo_check = "$ bun tools/repo-check/src/cli.ts all && bun run check:clippy && bun run check:lock"
    clippy = "$ cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings"
    locks = ("$ cargo check --manifest-path core/rust/Cargo.toml --workspace && "
             "cargo check --manifest-path tests/benchmarks/Cargo.toml && git diff --exit-code -- "
             + " ".join(LOCKS))
    log = workspace["text"]
    require_parts(log, ("Run bash tools/gates/run.sh", "-- cargo fmt --check --",
                        "F7-CARGO command=fmt locked=false", "-- bun run check --", repo_check, clippy),
                  "Linux 仓库门禁")
    require_parts(log[log.index(clippy):], (clippy, "F7-CARGO command=clippy locked=true", locks,
                                          "F7-CARGO command=check locked=true",
                                          "-- bun run check:coverage --",
                                          "-- xiao-driver cargo test --no-fail-fast --", "-- bun test --"),
                  "Linux Clippy、锁及测试")
    lock_output = log[log.index(locks):log.index("-- bun run check:coverage --")]
    require(len(re.findall(r"^F7-CARGO command=check locked=true ", lock_output, re.M)) == 2,
            "两个 Cargo 锁命令均须有执行回执")
    # GitHub 合流后，Clippy 的命令回显可能早于 repo-check 最后一条诊断。
    clippy_started = log.index("\nF7-CARGO command=clippy locked=true ", log.index(clippy))
    repo_output = log[log.index(repo_check):clippy_started]
    codes = Counter(re.findall(r"^\[(warning|error)\] (A0-[A-Z]+-\d+)", repo_output, re.M))
    require(set(codes) == {("warning", "A0-COVERAGE-002")}, "仓库门禁诊断超出既有覆盖警告")
    log = default["text"]
    start = f"##[group]Run {DEFAULT_COMMAND}"
    require(log.count(start) == 1 and log.count("\n" + DEFAULT_COMMAND + "\n") == 1, "默认 workspace 命令缺失")
    section = log[log.index(start):log.index("B6-4-B1 begin default")]
    require("F7-CARGO command=test locked=true" in section and f"test {DIAGNOSTIC_TEST} ... ok" in section
            and "test b1_capabilities_have_vm_golden_outputs ... ok" not in section,
            "默认 workspace 的跳过项或 Linux 诊断结果不符")
    totals = counts(section)
    require(totals["failed"] == 0, "Linux 默认 workspace 有失败")
    require(all(log.count(f"B1-GUARDS path={path} passed=7 total=7\n") == 1
                for path in ("default", "Windows", "Linux", "macOS"))
            and log.count("B1-GUARDS passed=28 total=28;") == 1, "模拟守卫报告不完整")
    return {
        "linux_workspace_gates": {"job": workspace["api"]["id"], "status": "passed",
                                  "fmt": "passed", "clippy": "passed", "check_lock": "passed",
                                  "repo_check": "passed", "repo_check_warning_lines": sum(codes.values()),
                                  "commands": [repo_check[2:], clippy[2:], locks[2:]]},
        "linux_workspace_default": {"job": default["api"]["id"], "status": "passed",
                                     "command": DEFAULT_COMMAND, "counts": totals},
        "simulated_guards": {"passed": 28, "job": default["api"]["id"]},
    }


def native_failures(observed):
    for ident in (135346523473, 135346523758):
        log = observed[ident]["text"]
        require(f"thread '{DIAGNOSTIC_TEST}'" in log and "called \x60Option::unwrap()\x60 on a \x60None\x60 value" in log
                and not observed[ident]["identity"]["after"]["runtime"], f"首轮 Linux 缺少环境的证据不符：{ident}")
    for ident in (135346523696, 135346523788, 135346524047):
        log = observed[ident]["text"]
        require(f"thread '{CMP_TEST}'" in log and "\nXIAO_FLOAT_EQUALITY_OUTPUT\n" in log,
                f"首轮原生缺少输出路径的证据不符：{ident}")
    for ident in (135358661515, 135358661593):
        row = observed[ident]
        log = row["text"]
        require_parts(log, (f"thread '{CMP_TEST}'", "original-pow-eq-static-o0", COMPONENT_ERROR), ident)
        require(not re.search(r"^B6-4-B1 begin ", log, re.M) and row["api"]["conclusion"] == "failure",
                f"Unix 原生作业短路状态不符：{ident}")
    win = observed[135358661620]
    require_parts(win["text"], ("FLOAT-EQUALITY cases=62 native=248 llvm_as=248 vm=62 result=passed",
                               "thread 'native_concurrency_vm_native_differential_matches_from_same_ir'",
                               RELEASE_ERROR,
                               'left: [(0, "shared_destroy"), (0, "weak_release")]',
                               'right: [(0, "shared_destroy"), (0, "weak_release"), (1, "shared_destroy"), (1, "weak_release")]'),
                  "Windows 22D")
    for ident in (135358661515, 135358661593, 135358661620):
        require(observed[ident]["api"]["conclusion"] == "failure", f"原生门禁状态不符：{ident}")
    return {
        "linux": {"job": 135358661593, "status": "failed", "case": "original-pow-eq-static-o0",
                  "reason": COMPONENT_ERROR, "owner": "B4 LLVM Runtime 组件"},
        "macos": {"job": 135358661515, "status": "failed", "case": "original-pow-eq-static-o0",
                  "reason": COMPONENT_ERROR, "owner": "B4 LLVM Runtime 组件"},
        "windows": {"job": 135358661620, "status": "failed", "case": "native-lock-wait-027",
                    "reason": RELEASE_ERROR, "owner": "22D"},
    }


def b1_sources(observed):
    sources = {}
    for path, (run_id, ident, job_name) in B1.items():
        row = observed[ident]
        require((row["api"]["name"], row["api"]["run_id"]) == (job_name, run_id), f"B1 入口作业不符：{path}")
        label = "b1-vm" if path == "default" else "b1-native"
        raw = slice_log(row["text"], f"B6-4-B1 begin {path}", f"B6-4-B1 end {path}").encode()
        checked = observe_suite(label, 0, raw)
        require(checked["status"] == "passed" and counts(raw.decode()) == {"passed": 1, "failed": 0, "ignored": 0}
                and re.search(r"^F7-CARGO command=test locked=true ", raw.decode(), re.M),
                f"B1 真实输出不符：{path}")
        case = "have_vm" if path == "default" else "match_native"
        require(f"\ntest b1_capabilities_{case}_golden_outputs ... ok\n" in raw.decode(), f"B1 精确测试名不符：{path}")
        sources[path] = (raw, {"run": run_id, "job": ident, "attempt": 1,
                                "job_log_sha256": row["sha256"], "cargo_output": digest(raw),
                                "markers": 9 if path == "default" else 18})
    return sources


def guard_bodies():
    workflow = blob(RERUN, MAINTENANCE)
    require(workflow == blob(SUPPLEMENT, MAINTENANCE), "补验与原维护计数判断来源不同")
    raw = workflow.decode()
    bodies = {"default": step(raw, "Run B1 VM probe exactly once").replace(
        "printf 'B6-4-B1 begin default\\n'\n", "").replace("printf 'B6-4-B1 end default\\n'\n", "")}
    win = step(raw, "Build runtime and run native gates")
    block = re.search(r"(?ms)^\$b1Output = cargo test.*?^\}", win)
    require(block is not None, "固定 Windows 精确守卫缺失")
    bodies["windows"] = "$ErrorActionPreference = 'Stop'\n" + block.group() + "\n"
    for platform in ("Linux", "macOS"):
        script = step(raw, f"Build runtime and run {platform} native gates")
        helper = re.search(r"(?ms)^run_exact_native_test\(\) \{.*?^\}", script)
        call = "run_exact_native_test b1_capabilities_match_native_golden_outputs 20o_b1_probe"
        require(helper is not None and call in script.splitlines(), f"固定 {platform} 精确守卫缺失")
        bodies[platform.lower()] = f"set -euo pipefail\n{helper.group()}\n{call}\n"
    return bodies


def guard_input(path, body, exit_code):
    if path == "windows":
        prefix = ("function cargo { Get-Content -LiteralPath 'cargo-output.log' -Encoding utf8; "
                  f"$global:LASTEXITCODE = {exit_code} }}\n")
        return "guard.ps1", (prefix + body).encode()
    return "guard.sh", (f"cargo() {{ cat cargo-output.log; return {exit_code}; }}\n" + body).encode()


def run_guards(root, bash, sources, bodies):
    root.mkdir()
    env = os.environ | {"PATH": str(bash.parent) + os.pathsep + os.environ["PATH"]}
    pwsh = shutil.which("pwsh")
    require(pwsh is not None, "缺少 PowerShell 7")
    for path, (raw, _) in sources.items():
        for case, sample, cargo_exit in variants(raw):
            folder = root / path / case
            folder.mkdir(parents=True)
            (folder / "cargo-output.log").write_bytes(sample)
            name, script = guard_input(path, bodies[path], cargo_exit)
            (folder / name).write_bytes(script)
            argv = [pwsh, "-NoProfile", "-File", folder / name] if path == "windows" else [bash, name]
            job(argv, folder, env, "guard", timeout=20)


def audit_guards(bundle, sources, bodies):
    results = {}
    for path, (raw, source) in sources.items():
        for case, expected_input, cargo_exit in variants(raw):
            folder = f"guards/{path}/{case}"
            name, script = guard_input(path, bodies[path], cargo_exit)
            receipt, stdin, stdout, stderr = bundle.command(folder, "guard", "windows")
            shell = Path(receipt["argv"][0].replace("\\", "/")).name.lower()
            args = [arg.replace("\\", "/") for arg in receipt["argv"][1:]]
            require((shell in ("pwsh.exe", "pwsh") and args[:2] == ["-NoProfile", "-File"]
                     and len(args) == 3 and args[2].endswith(f"/{path}/{case}/{name}")
                     if path == "windows" else shell in ("bash.exe", "bash") and args == [name])
                    and not stdin and bundle.raw(f"{folder}/{name}") == script
                    and bundle.raw(f"{folder}/cargo-output.log") == expected_input
                    and clean_log(stdout).rstrip("\n") == expected_input.decode().rstrip("\n"),
                    f"守卫命令、输入或实际输出不符：{path}/{case}")
            expected_exit = cargo_exit if path == "windows" and cargo_exit else int(case != "actual")
            reason = clean_log(stderr).strip()
            if case != "actual" and cargo_exit == 0 and path == "windows":
                reason_ok = reason.splitlines()[-1:] and reason.splitlines()[-1].strip() == \
                    "| B1 native probe must pass exactly one case without ignoring it"
            elif case != "actual" and cargo_exit == 0 and path in ("linux", "macos"):
                reason_ok = reason == "Native test did not execute exactly one case: b1_capabilities_match_native_golden_outputs"
            else:
                reason_ok = not reason
            require(receipt["exit"] == expected_exit and reason_ok, f"计数守卫拒绝原因不符：{path}/{case}")
            results[f"{path}/{case}"] = {"exit": receipt["exit"], "cargo_exit": cargo_exit,
                                         "produced_reason": reason.splitlines()[-1] if reason else None,
                                         "input_sha256": digest(expected_input)["sha256"],
                                         "source_sha256": source["cargo_output"]["sha256"]}
    require(len(results) == 28, "预期四条实际输出与 24 条派生负例")
    return {"passed": 28, "actual": 4, "derived": 24, "results": results,
            "sources": {path: source for path, (_, source) in sources.items()}}


def inspect_ci(read):
    reports, observed = {}, {}
    for run_id, spec in RUNS.items():
        run, jobs = read_run(read, run_id, spec)
        require(not (jobs.keys() & observed.keys()), "多个 CI 运行复用了作业 ID")
        observed.update(jobs)
        reports[run_id] = {key: run[key] for key in (
            "id", "event", "head_sha", "head_branch", "run_attempt", "workflow_id", "path", "conclusion")}
        reports[run_id]["jobs"] = {
            job_id: {"name": row["api"]["name"], "conclusion": row["api"]["conclusion"], "log": row["source"],
                     "log_sha256": row["sha256"], "labels": row["api"]["labels"], "cache": row["cache"],
                     "failed_steps": row["failed_steps"], "identity": row["identity"]}
            for job_id, row in jobs.items()
        }
    return reports, observed


def capture_ci(source, archive, bash, replay_root):
    def read(name):
        return (source / name.removeprefix("ci/")).read_bytes()

    _, observed = inspect_ci(read)
    engineering(observed)
    native_failures(observed)
    run_guards(replay_root, bash, b1_sources(observed), guard_bodies())
    for run, expected in RUNS.items():
        require({path.name for path in (source / str(run)).iterdir()}
                == {"run.json", "jobs.json", *(f"{job_id}.log" for job_id in expected[-1])},
                f"CI 原件路径集合不符：{run}")
    collect(archive, [(f"ci/{run}", source / str(run)) for run in RUNS] + [("guards", replay_root)])


def audit_ci(archive, *, sealed=True):
    raw = archive.read_bytes()
    with ZipFile(BytesIO(raw)) as zipped:
        bundle = Evidence(zipped)
        reports, observed = inspect_ci(bundle.raw)
        guards = audit_guards(bundle, b1_sources(observed), guard_bodies())
        archive_identity = digest(raw) | {"files": len(bundle.entries), "unique_objects": bundle.unique}
    if sealed:
        verify_sealed_archive(raw, "ci.zip", CLOSEOUT)
    return {"verified": True, "b6_4_passed": False, "b6_5_started": False,
            "source": source_identity(), "runs": reports, "archive": archive_identity,
            "engineering": engineering(observed), "native_maintenance": native_failures(observed),
            "guard_replay": guards, "b6_1": {"status": "restricted", "stage_passed": False,
            "windows_coff": "unverifiable", "linux_clippy": "not_run"},
            "b6_2_performance_commit": "79ea0375d7f2d823a5bdde73cb2c9a1c33c3e4c3",
            "b6_3_boundary_commit": PRODUCT}


def docs_bytes(raw):
    return {"raw": digest(raw), "utf8_lf": digest(raw.decode("utf-8").replace("\r\n", "\n").encode())}


def capture_docs(task, archive):
    task.mkdir()
    commit = git("rev-parse", "HEAD").decode().strip()
    require(not git("status", "--porcelain=v1"), "定稿门禁在已提交的干净树运行")
    before = {path: docs_bytes((REPO / path).read_bytes()) for path in DOCS}
    bun = shutil.which("bun")
    require(bun is not None, "缺少 Bun")
    receipt, _, _ = job([bun, "tools/repo-check/src/cli.ts", "all", "--format", "json"], task,
                        os.environ, "repo-check", cwd=REPO, timeout=180)
    after = {path: docs_bytes((REPO / path).read_bytes()) for path in DOCS}
    clean_after = git("rev-parse", "HEAD").decode().strip() == commit and not git("status", "--porcelain=v1")
    write_json(task / "docs-source.json", {"commit": commit, "before": before, "after": after,
                                          "checkout_unchanged": clean_after,
                                          "bun": digest(Path(bun).read_bytes())})
    collect(archive, [("docs", task)])
    require(receipt["exit"] == 0 and before == after and clean_after, "定稿门禁失败或检出内容发生变化")


def audit_docs(archive, *, sealed=True):
    raw = archive.read_bytes()
    with ZipFile(BytesIO(raw)) as zipped:
        bundle = Evidence(zipped)
        sources = bundle.read("docs/docs-source.json")
        require(sources["checkout_unchanged"] is True
                and sources["before"] == sources["after"] and set(sources["before"]) == set(DOCS),
                "定稿文档证据范围或前后摘要不符")
        commit = sources["commit"]
        require(re.fullmatch("[0-9a-f]{40}", commit), "定稿 Git 提交无效")
        if sealed:
            require(commit == CLOSEOUT, "定稿门禁执行提交不符")
        for path, info in sources["before"].items():
            fixed = blob(commit, path).replace(b"\r\n", b"\n")
            require(info["utf8_lf"] == digest(fixed),
                    f"定稿文档未与固定 Git 对象绑定：{path}")
            require(info["raw"] in (digest(fixed), digest(fixed.replace(b"\n", b"\r\n"))),
                    f"定稿文档原始字节摘要不符：{path}")
        receipt, stdin, stdout, stderr = bundle.command("docs", "repo-check", "windows")
        report = json.loads(stdout)
        warnings = Counter(row["code"] for row in report["diagnostics"] if row["severity"] == "warning")
        require(receipt["argv"][1:] == ["tools/repo-check/src/cli.ts", "all", "--format", "json"]
                and receipt["exit"] == 0 and not stdin and not stderr and report["checker"] == "xiao-repo-check"
                and report["passed"] is True and set(warnings) <= {"A0-COVERAGE-002"}
                and all(row["severity"] == "warning" for row in report["diagnostics"]), "定稿门禁结果不符")
    if sealed:
        verify_sealed_archive(raw, "docs-check.zip", DOCS_SEAL)
    return {"commit": commit, "exit": receipt["exit"], "diagnostics": dict(warnings), "docs_files": len(DOCS)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, default=ROOT / "ci.zip")
    parser.add_argument("--summary", type=Path, default=ROOT / "summary.json")
    parser.add_argument("--docs", type=Path, default=ROOT / "docs-check.zip")
    parser.add_argument("--capture-ci", type=Path, metavar="API_DOWNLOADS")
    parser.add_argument("--replay-root", type=Path)
    parser.add_argument("--bash", type=Path)
    parser.add_argument("--capture-docs", type=Path, metavar="NEW_TASK_ROOT")
    parser.add_argument("--ci-only", action="store_true")
    args = parser.parse_args()
    if args.capture_ci:
        require(args.bash and args.replay_root and not args.capture_docs, "采集 CI 须明确 Bash 与独立回放目录")
        require(not args.archive.exists() and not args.summary.exists(), "采集 CI 须使用新归档与汇总路径")
        capture_ci(args.capture_ci, args.archive, args.bash.resolve(strict=True), args.replay_root.resolve())
    result = audit_ci(args.archive, sealed=args.capture_ci is None)
    expected_summary = json.loads(json.dumps(result))
    if args.capture_ci:
        write_json(args.summary, expected_summary)
    else:
        require(expected_summary == json.loads(args.summary.read_bytes()), "公开汇总与原始回执不符")
    if args.capture_docs:
        require(not args.docs.exists(), "采集定稿门禁须使用新归档路径")
        capture_docs(args.capture_docs.resolve(), args.docs)
    outcome = {key: result[key] for key in ("verified", "b6_4_passed", "archive", "engineering", "native_maintenance")}
    outcome["guard_replay"] = {key: result["guard_replay"][key] for key in ("passed", "actual", "derived")}
    if args.capture_docs or not args.ci_only:
        outcome["final_docs"] = audit_docs(args.docs, sealed=args.capture_docs is None)
    print(json.dumps(outcome, ensure_ascii=False))


if __name__ == "__main__":
    main()
