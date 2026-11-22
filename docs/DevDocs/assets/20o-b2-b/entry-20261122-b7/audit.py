"""只读复核 B7 回归、固定来源与交接状态。"""

import argparse
from collections import Counter
from datetime import datetime
from importlib.util import module_from_spec, spec_from_file_location
from io import BytesIO
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import sys
from zipfile import BadZipFile, ZipFile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
# 历史证据模块使用 audit、guards 和 run 作为本地导入名。
spec = spec_from_file_location("b7_guards", ROOT / "guards.py")
guards = module_from_spec(spec)
spec.loader.exec_module(guards)
from evidence import Evidence, clean_log, digest, observed_identity

import history

run = guards.b7_run
git, require = run.git, run.require
PRODUCT, TREE = run.PRODUCT, run.TREE
PREFIX = ROOT.relative_to(run.REPO).as_posix()
RUNNER = "4e009503033b6ae7e33ad1a8546febf220724e38"
RETAINED = "95c0f5b4159c4d9e7b9e3113844d40a44305b608"
LIBRARIES = ("math", "statistics", "fmt", "hex", "base64", "url", "csv")
B1_MARKERS = (*LIBRARIES, "aliases", "string-index")
FAILURES = ("native-lock-wait-027", "xcon-bounded-string-success", "spawn-join-str-multiargument")
COUNTS = re.compile(r"^test result: (?:ok\.|FAILED\.) (\d+) passed; (\d+) failed; (\d+) ignored;", re.M)
WORKSPACE = {"windows": [1758, 0, 55], "linux": [1760, 0, 56], "macos": [1759, 0, 56]}
RUNS = {
    44996207048: (PRODUCT, "maintenance-regression.yml", "failure", 6),
    44996207236: (PRODUCT, "workspace-gates.yml", "failure", 1),
    44998495908: (RUNNER, "b2-b7-evidence.yml", "success", 3),
    44998495938: (RUNNER, "maintenance-regression.yml", "success", 6),
    44998495850: (RUNNER, "workspace-gates.yml", "success", 1),
    45003168952: (RETAINED, "b2-b7-artifacts.yml", "success", 1),
    45003168769: (RETAINED, "workspace-gates.yml", "success", 1),
}
BASIC = ("b1_regression_passed", "stage_reconciliation_passed", "language_registry_boundary_passed",
         "ci_platform_gates_passed")
B6_FIELDS = ("b6_accepted", "b6_1_stage_passed", "b6_3_final_tree_covered", "b6_4_passed",
             "original_passed", "followup_passed", "windows_coff_verified", "22d_open")


def blob(commit, path):
    return git("show", f"{commit}:{path}")


def product_identity():
    manifest = git("ls-tree", "-rz", PRODUCT, "--", *run.PRODUCT_ROOTS)
    require(git("cat-file", "-p", PRODUCT).splitlines()[0] == f"tree {TREE}".encode(), "固定产品提交树不符")
    require("tests/spec/11x0-protocol/hello-request.json" in run.tracked(manifest), "固定产品缺少协议夹具")
    base = {"commit": PRODUCT, "tree": TREE, "files": len(run.tracked(manifest)), "tree_list_sha256": run.sha(manifest),
            "locks": {name: digest(blob(PRODUCT, name)) for name in run.LOCKS}}
    intrinsics = json.loads(blob(PRODUCT, "core/rust/crates/xiao-intrinsics/intrinsics.json"))["intrinsics"]
    require(sorted((row["id"], row["status"]) for row in intrinsics)
            == [(number, "active") for number in range(1, 49)] + [(100, "tombstone")], "活动 intrinsic 身份不符")
    crates = "core/rust/crates"
    versions = (
        ("xiao-runtime-abi", b"ABI_MAJOR_VERSION: u32 = 1;"),
        ("xiao-runtime-abi", b"ABI_MINOR_VERSION: u32 = 19;"),
        ("xiao-codegen-llvm", b"CODEGEN_VERSION: u32 = 22;"),
        ("xiao-standard-library", b"STDLIB_INTERFACE_VERSION: u32 = 1;"),
    )
    require(all(marker in blob(PRODUCT, f"{crates}/{crate}/src/lib.rs") for crate, marker in versions),
            "ABI 或生成器身份不符")
    library_sources = [f"stdlib/{name}.xiao" for name in LIBRARIES]
    require(not git("diff", "--name-only", "106b99c7", PRODUCT, "--", *library_sources),
            "B1 七库源码发生变化")
    return base


def check_identity(value):
    require(type(value["bytes"]) is int and value["bytes"] > 0
            and re.fullmatch(r"[0-9a-f]{64}", value["sha256"]), "文件摘要无效")


def check_process(row):
    started, ended = (datetime.fromisoformat(row[key]) for key in ("started_utc", "finished_utc"))
    require(type(row["pid"]) is int and row["pid"] > 0 and type(row["exit"]) is int and row["exit"] == 0
            and started.utcoffset() is not None and ended.utcoffset() is not None and started <= ended,
            "命令未执行成功或进程身份无效")


def observe(bundle, folder, suite, product, host):
    env = bundle.read(f"{folder}/environment.json")
    target, lib = next(value[1:3] for value in run.HOSTS.values() if value[0] == host)
    path = PureWindowsPath if host == "windows" else PurePosixPath
    task = path(env["task"])
    runtime_path = task / "target" / "release" / lib
    overrides = env["overrides"]
    require(task.is_absolute() and env["host"] == host and env["target"] == target and env["source"] == product
            and env["rustc_verbose"].startswith("rustc 1.96.0 ")
            and f"host: {target}" in env["rustc_verbose"].splitlines()
            and overrides["XIAO_B1_F10"] == overrides["XIAO_B2_A_PERF"] == "0"
            and path(overrides["XIAO_RUNTIME_LIBRARY"]) == runtime_path
            and path(overrides["XIAO_STDLIB_ROOT"]) == task / "source/stdlib"
            and path(overrides["XIAO_HOME"]) == task / "home"
            and path(overrides["BUN_INSTALL_CACHE_DIR"]) == task / "cache/bun"
            and path(overrides["CARGO_TARGET_DIR"]) == task / "target", f"平台或任务目录身份不符：{folder}")
    for item in env["tools"].values():
        check_identity(item)
    require(set(env["tools"]) >= {"cargo", "rustc", "clang", "llvm-as", "llc", "linker", "clippy-driver"},
            f"平台工具缺失：{folder}")
    cargo = env["tools"]["cargo"]["path"]
    expected = {"runtime": (cargo, "build", *run.LOCKED, "--release", "-p", "xiao-runtime")}
    for group in suite:
        expected.update(run.commands(cargo, group, host))
    results = bundle.read(f"{folder}/results.json")
    require(results.keys() == expected.keys() - {"runtime"} and all(type(exit_code) is int and exit_code == 0
                                                                    for exit_code in results.values()),
            f"回归入口集合不符：{folder}")
    runtime = bundle.read(f"{folder}/runtime.json")["runtime_after"]
    check_identity(runtime)
    sample = {}
    for name, argv in expected.items():
        row = bundle.read(f"{folder}/{name}.json")
        raw = bundle.raw(f"{folder}/{name}.log")
        check_process(row)
        require(tuple(row["argv"]) == argv and row["log"] == digest(raw)
                and row["source_before"] == product == row["source_after"]
                and row["runtime_before"] == (None if name == "runtime" else runtime)
                and row["runtime_after"] == runtime and path(row["cwd"]) == task / "source"
                and path(row["temp"]) == task / "tmp" / name, f"命令或原始输出身份不符：{folder}/{name}")
        log = raw.decode("utf-8").replace("\r\n", "\n")
        totals = [list(map(int, total)) for total in COUNTS.findall(log)]
        if name in run.EXACT:
            test = run.EXACT[name][1]
            require(totals == [[1, 0, 0]] and f"test {test} ... ok" in log
                    and any("target" in path(key).parts for key in row["artifacts"]), f"精确入口不符：{folder}/{name}")
        if totals:
            require(all(total[1] == 0 for total in totals), f"测试日志包含失败：{folder}/{name}")
        sample[name] = log
        if name == "workspace":
            require(len(totals) == 192 and [sum(column) for column in zip(*totals)] == WORKSPACE[host],
                    f"默认工作区计数不符：{folder}")
        if name in ("b1-contract", "b1-resources"):
            require(totals == [[9 if name == "b1-contract" else 18, 0, 0]], f"B1 目标计数不符：{folder}/{name}")
    return env, runtime, sample


def verify_b1(sample):
    vm = re.findall(r"^20O-B1-IMP-PROBE case=(\S+) backend=vm result=success$", sample["b1-vm"], re.M)
    native = re.findall(r"^20O-B1-IMP-PROBE case=(\S+) backend=vm/native optimization=([03]) result=success$",
                        sample["b1-native"], re.M)
    require(Counter(vm) == Counter(B1_MARKERS)
            and Counter(native) == Counter((name, level) for name in B1_MARKERS for level in ("0", "3")),
            "B1 两个精确目标的七库或辅助夹具缺失")
    names = ("public_calls_enforce_every_frozen_parameter_and_require_imports",
             "products_execute_and_export_only_the_frozen_names", "public_std_aliases_execute_through_the_official_loader",
             "every_public_function_requires_its_real_implementation",
             "products_replay_from_real_archives_without_bundled_library_bodies",
             "products_keep_the_official_namespace_out_of_ordinary_packages",
             "product_tutorials_embed_the_executed_sources_and_match_registry")
    require(all(f"test {name} ... ok" in sample["b1-contract"] for name in names), "B1 契约子集冒充全量")
    require(all(f"test {name} ... ok" in sample["b1-resources"] for name in (
        "url_decoding_reaches_the_unicode_result_budget", "url_decoding_resets_utf8_state_between_escaped_codepoints")),
        "URL Unicode 预算或转义复位缺项")


def verify_numeric(sample, host):
    for name in ("a1_identity", "a1_admission_error", "a1_pending_finally"):
        require(f"case={name} backend=vm result=success" in sample["a1-vm"]
                and all(f"case={name} backend=native optimization={level} " in sample["a1-native"] for level in (0, 3)),
                f"A1 示例缺失：{host}/{name}")
    require("test a5_complete_numeric_language_matrix_matches_independent_golden ... ok" in sample["numeric-vm"]
            and "test a5_complete_vm_entry_covers_resources_and_portable_replay ... ok" in sample["numeric-vm"]
            and "test aliases_replay_from_xiaoc_and_xar_after_application_source_removal ... ok" in sample["numeric-vm"]
            and "test frozen_abi18_baselines_are_rejected_before_entry ... ok" in sample["numeric-vm"],
            f"高精度、归档或版本回归缺项：{host}")
    require(all(f"B2-A5-ALLOCATION optimization={level} code=X06-RUNTIME-011 out=unchanged" in sample["a5-native"]
                for level in (0, 3)), f"A5 受控分配失败证据缺项：{host}")
    require("B2-B2-DIVIDE backend=native optimization=0,3 intrinsic=33 result=success" in sample["divide-native"],
            f"显式除法原生缺项：{host}")
    verification = "UnverifiedCoffExports" if host == "windows" else "Complete"
    rows = re.findall(r"^B4-4-NATIVE path=(\S+) optimization=([03]) .+ verification=(\S+) .+"
                      r"stdout=golden errors=009 result=passed$", sample["cmath-native"], re.M)
    require(Counter(rows) == Counter((alias, level, verification) for alias in ("std.cmath", "stdlib.cmath")
                                    for level in ("0", "3")), f"Cmath 四格或组件观察不符：{host}")
    require("FLOAT-EQUALITY cases=62 native=248 llvm_as=248 vm=62 result=passed" in sample["float-equality"]
            and "B4-3-NATIVE language=actual-exit,catch A5=7-fixtures O0,O3 result=passed" in sample["cmath-llvm"],
            f"静态/动态原生证据不符：{host}")
    refs = dict(re.findall(r"^B3-CMATH-ABI function=(\S+) cases=(\d+) result=success$", sample["f9-runtime"], re.M))
    require(len(refs) == 15 and sum(map(int, refs.values())) == 17243
            and "reference_inputs=17243 facades=15 kernel_identity=exact" in sample["f9-runtime"],
            f"正式参考向量缺项：{host}")


def output(response):
    return "".join(row["data"]["text"] for row in response["events"] if row["kind"] == "intrinsic_output")


def artifact_identities(bundle, root, env, runtime):
    common = bundle.read(f"{root}/identity.json")
    require(common["product"] == PRODUCT and common["source"] == env["source"] and common["runtime"] == runtime
            and common.get("platform", common.get("host")) == env["host"], f"产物采集身份不符：{root}")
    check_identity(common["core"])
    return common


def verify_portable(bundle, folder, env, runtime):
    common = artifact_identities(bundle, folder, env, runtime)
    for module in LIBRARIES:
        row = bundle.read(f"{folder}/{module}.json")
        fixture = f"tests/fixtures/20o-b1/imp/{module}"
        source, golden = (blob(PRODUCT, f"{fixture}/{filename}") for filename in ("product.xiao", "golden.json"))
        require(row["source_removed"] is True and row["source"] == digest(source) and row["golden"] == digest(golden),
                f"删源回放的源码或黄金不符：{module}")
        for kind in ("xiaoc", "xar"):
            require(row[kind] == digest(bundle.raw(f"{folder}/{module}.{kind}")), f"可移植产物字节不符：{module}/{kind}")
        build, replay = row["build"], row["replay"]
        for call in (build, replay):
            check_process(call)
            require(call["argv"] == [common["core"]["path"]] and all(
                reply["exit_code"] == 0 for reply in call["responses"]), f"产物进程不符：{module}")
        require([request["type"] for request in build["requests"]] == ["run", "build_archive"]
                and [request["type"] for request in replay["requests"]] == ["run_xiaoc", "run_archive"]
                and len(replay["responses"]) == len(build["responses"]) == 2
                and {reply["request_id"] for reply in replay["responses"]} == {"xiaoc-replay", "xar-replay"}
                and all(output(reply) == json.loads(golden)["stdout"] for reply in replay["responses"]),
                f"可移植产物回放不完整：{module}")
        first = next(response for response in build["responses"] if response["request_id"] == "source-run")
        require(first["cache"]["status"] == "written" and first["cache"]["verified"] is True
                and first["cache"]["digest"] == row["xiaoc"]["sha256"], f"字节码物化摘要不符：{module}")
    return 14


def verify_native(bundle, folder, env, runtime):
    common = artifact_identities(bundle, folder, env, runtime)
    expected_format = {"windows": "coff", "linux": "elf", "macos": "macho"}[env["host"]]
    verification = "unverified-coff-exports" if env["host"] == "windows" else "complete"
    for module in LIBRARIES:
        fixture = f"tests/fixtures/20o-b1/imp/{module}"
        golden = json.loads(blob(PRODUCT, f"{fixture}/golden.json"))["stdout"].encode()
        for level in (0, 3):
            case = f"{module}-o{level}"
            row = bundle.read(f"{folder}/{case}.json")
            request = row["request"]
            artifact = row["artifact"]
            observed = artifact["artifact_runtime"]
            for key in ("build", "run"):
                check_process(row[key])
            require(row["module"] == module and row["optimization"] == level == artifact["optimization_level"]
                    and request["source"]["text"].encode() == blob(PRODUCT, f"{fixture}/product.xiao")
                    and request["target"]["triple"] == env["target"]
                    and request["toolchain"]["runtime_library"] == env["overrides"]["XIAO_RUNTIME_LIBRARY"]
                    and row["build"]["argv"] == [common["core"]["path"]]
                    and row["run"]["argv"] == [artifact["executable"]]
                    and artifact["object_digest"] == row["executable"]["sha256"]
                    and observed["object_format"] == expected_format and observed["verification"] == verification
                    and observed["declared_components"] == observed["observed_components"]
                    and bundle.raw(f"{folder}/{case}.stdout") == golden, f"B1 原生回执不符：{folder}/{case}")
            check_identity(row["executable"])
            check_identity(row["llvm_ir"])
    return 14


def verify_ci(bundle):
    expected_paths = {
        ".github/workflows/b2-b7-evidence.yml", ".github/workflows/maintenance-regression.yml",
        ".github/workflows/workspace-gates.yml",
        "docs/DevDocs/assets/20o-b2-b/entry-20261119-b6-4/ci_identity.py",
        "docs/DevDocs/assets/20o-b2-b/entry-20261119-b6-4/linux_runtime.sh", f"{PREFIX}/run.py",
    }
    changes = lambda first, second: set(git("diff", "--name-only", first, second).decode().splitlines())
    require(changes(PRODUCT, RUNNER) == expected_paths and changes(RUNNER, RETAINED) == {
        ".github/workflows/b2-b7-artifacts.yml", f"{PREFIX}/portable.py", f"{PREFIX}/native-artifacts.py"},
        "Runner 取证与固定产品的差异超界")
    reused = expected_paths - {".github/workflows/b2-b7-evidence.yml", f"{PREFIX}/run.py"}
    require(all(blob(RUNNER, path) == blob("e1191445", path) for path in reused), "Runner 环境补验来源不符")
    runs, logs = {}, {}
    for ident, (commit, workflow, conclusion, count) in RUNS.items():
        root = f"ci/{ident}"
        receipt = bundle.read(f"{root}/run.json")
        jobs = bundle.read(f"{root}/jobs.json")
        require(receipt["id"] == ident and receipt["head_sha"] == commit and receipt["run_attempt"] == 1
                and receipt["status"] == "completed" and receipt["conclusion"] == conclusion
                and receipt["path"] == f".github/workflows/{workflow}" and jobs["total_count"] == count
                and len(jobs["jobs"]) == count, f"CI 运行身份不符：{ident}")
        rows = {}
        for job in jobs["jobs"]:
            job_id = job["id"]
            log = clean_log(bundle.raw(f"{root}/{job_id}.log"))
            require(job["head_sha"] == commit and job["run_id"] == ident and job["run_attempt"] == 1
                    and job["status"] == "completed" and f"Complete job name: {job['name']}" in log
                    and commit in log and (conclusion != "success" or job["conclusion"] == "success"),
                    f"CI 作业身份不符：{job_id}")
            rows[str(job_id)] = {"name": job["name"], "conclusion": job["conclusion"], "attempt": 1}
            if "B6-4-IDENTITY before " in log:
                evidence = observed_identity(log, receipt, job)
                require(evidence and set(evidence) == {"before", "native", "after"},
                        f"CI Runtime 或工具身份缺项：{job_id}")
                rows[str(job_id)]["target"] = evidence["native"]["target"]
                rows[str(job_id)]["runtime_sha256"] = evidence["native"]["runtime"]["sha256"]
            logs[job_id] = log
        runs[str(ident)] = {"commit": commit, "workflow": workflow, "conclusion": conclusion, "jobs": rows}
    for job in (137127688235, 137127688828):
        require("b6_diagnoses_linux_native_cmath_runtime_component_observation ... FAILED" in logs[job]
                and "20o_b2_b6_resources.rs:123:62:" in logs[job], "Linux 首跑环境失败记录不符")
    for job in (137127688341, 137127688503, 137127688532):
        require("XIAO_FLOAT_EQUALITY_OUTPUT" in logs[job]
                and "native_static_dynamic_comparisons_match_vm_o0_o3 ... FAILED" in logs[job],
                "三平台首轮浮点输出目录失败记录不符")
    for job in (137134646791, 137134646829, 137134646951):
        require("F7-CARGO command=test locked=true" in logs[job] and all(
            f"22D-CASE {name}: ok" in logs[job] for name in FAILURES), f"远端差分未执行：{job}")
    require("test b1_capabilities_have_vm_golden_outputs ... ok" in logs[137134646851]
            and "F7-CARGO command=clippy locked=true" in logs[137134646338]
            and logs[137134646338].count("F7-CARGO command=check locked=true") >= 2,
            "远端默认回归或工程门禁命令缺项")
    return runs


def verify_macos_archive(bundle, folder):
    ident = 45003168952 if folder == "retained" else 44998495908
    name = f"b7-macos-{folder}-{ident}-1"
    rows = [row for row in bundle.read(f"ci/{ident}/artifacts.json")["artifacts"] if row["name"] == name]
    require(len(rows) == 1, f"macOS 上传证据缺失：{folder}")
    row = rows[0]
    raw = bundle.raw(f"ci/{ident}/{row['id']}.zip")
    require(row["digest"] == f"sha256:{run.sha(raw)}", f"macOS 上传原件摘要不符：{folder}")
    prefix = f"macos-{folder}/"
    with ZipFile(BytesIO(raw)) as upload:
        paths = {item.filename for item in upload.infolist() if not item.is_dir()}
        require(paths == {path.removeprefix(prefix) for path in bundle.entries if path.startswith(prefix)},
                f"macOS 上传原件子集冒充全量：{folder}")
        require(all(bundle.raw(prefix + path) == upload.read(path) for path in paths),
                f"macOS 上传原件实际字节不符：{folder}")
    config = bundle.read(prefix + "environment.json")["runner"]
    require(config["GITHUB_RUN_ID"] == str(ident) and config["GITHUB_RUN_ATTEMPT"] == "1"
            and config["GITHUB_SHA"] == (RETAINED if folder == "retained" else RUNNER)
            and config["RUNNER_OS"] == "macOS" and config["RUNNER_ARCH"] == "ARM64",
            f"macOS 专项任务身份不符：{folder}")


def derive(values):
    require(all(type(values[key]) is bool for key in (*BASIC, "open_items_assigned")), "状态值必须为布尔值")
    passed = all(values[key] for key in BASIC)
    return {"b7_passed": passed, "b2c_entry_ready": passed and values["open_items_assigned"]}


def verify_handoff(root, inherited):
    handoff = json.loads((root / "handoff.json").read_bytes())
    require(handoff["product"] == PRODUCT and handoff["b2_c_started"] is False
            and handoff["required_final_checks"] == {
                "b1_f10_new_snapshot": 721, "public_artifacts": ["windows", "linux", "macos"],
                "final_tutorial_and_registry": "pending", "b6_numeric_workloads": "pending"}
            and handoff["open_items"] == {
                "22d": {"owner": "B7", "cases": list(FAILURES)},
                "windows_coff": {"owner": "B2-C", "verification": "UnverifiedCoffExports"},
                "final_tree_artifact_verification": {"owner": "B2-C", "pending": ["windows", "macos"]},
                "b6_1_linux_clippy": {"owner": "B7-4", "historical": "not_run", "b7_gate": "linux/clippy.json"}}
            and inherited["22d_open"] is True and all(inherited[name] is False for name in B6_FIELDS if name != "22d_open"),
            "B2-C 入口或 B6 受限状态不符")
    return handoff


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode()


def verify_docs(bundle, root):
    gate = bundle.read("docs/receipt.json")
    check_process(gate)
    report = bundle.read("docs/repo-check.stdout")
    require(gate["argv"][1:] == ["tools/repo-check/src/cli.ts", "all", "--format", "json"]
            and gate["outputs"] == {stream: digest(bundle.raw(f"docs/repo-check.{stream}"))
                                    for stream in ("stdout", "stderr")}
            and report["checker"] == "xiao-repo-check" and report["schemaVersion"] == 1 and report["passed"] is True
            and all(row["severity"] != "error" for row in report["diagnostics"]),
            "定稿完整仓库门禁或锁定 Cargo 证据不符")
    commit = gate["commit"]
    for name, argv in (("install", ["install", "--frozen-lockfile"]), ("check-lock", ["run", "check:lock"])):
        row = bundle.read(f"docs/{name}.json")
        check_process(row)
        require(row["argv"] == [gate["argv"][0], *argv] and row["cwd"] == gate["cwd"]
                and row["outputs"] == {stream: digest(bundle.raw(f"docs/{name}.{stream}"))
                                       for stream in ("stdout", "stderr")}, f"定稿门禁命令或日志不符：{name}")
    cargo_log = bundle.raw("docs/cargo.log")
    calls = [json.loads(line) for line in cargo_log.splitlines()]
    require(gate["cargo_calls"] == digest(cargo_log) and [(row["step"], row["argv"][1]) for row in calls]
            == [("check-lock", "check"), ("check-lock", "check"), ("repo-check", "metadata"), ("repo-check", "run")]
            and all(row["argv"][0] == gate["cargo"]["path"] and row["argv"][2] == "--locked"
                    and row["pid"] > 0 and row["exit"] == 0 and row["cwd"] == gate["cwd"] for row in calls)
            and all(row["parent_pid"] == gate["pid"] for row in calls if row["step"] == "repo-check"),
            "定稿门禁间接 Cargo 参数或退出状态不符")
    require(gate["locks_before"] == gate["locks_after"]
            == {path: digest(blob(PRODUCT, path)) for path in run.LOCKS}, "定稿门禁锁文件身份不符")
    markdown = git("ls-tree", "-rz", commit, "--", "docs/DevDocs", "docs/UseDocs")
    expected = {path: oid for path, oid in run.tracked(markdown).items() if path.endswith(".md")}
    require(gate["markdown_before"] == gate["markdown_after"] == expected, "定稿 Markdown 路径及字节集合不符")
    require(not git("diff", "--name-only", PRODUCT, commit, "--",
                    *(path for path in run.PRODUCT_ROOTS if path != "docs/UseDocs")),
            "定稿门禁改变产品、测试或登记")
    registry = json.loads(blob(commit, "docs/module-registry.json"))["modules"]
    statuses = {row["id"]: row["status"] for row in registry}
    require(statuses["stdlib.cmath"] == "planned" and statuses["rust.xiao-error-code"] == "verified"
            and all(statuses[f"stdlib.{name}"] == "verified" for name in LIBRARIES)
            and b"status: planned" in blob(commit, "docs/UseDocs/library/cmath.md"),
            "教程或登记提前改为已发布")
    require(blob(commit, f"{PREFIX}/handoff.json") == (root / "handoff.json").read_bytes(),
            "交接文件未与定稿 Git 对象绑定")
    return {"commit": commit, "markdown_files": len(expected), "exit": gate["exit"],
            "locked_cargo_calls": [call["argv"][1] for call in calls],
            "warnings": dict(Counter(row["code"] for row in report["diagnostics"] if row["severity"] == "warning"))}


def inspect(root, *, prepare=False):
    product = product_identity()
    raw = (root / "records.zip").read_bytes()
    with ZipFile(BytesIO(raw)) as zip_file:
        bundle = Evidence(zip_file)
        previous, inherited = history.inspect(bundle, git, digest, require)
        handoff = verify_handoff(root, inherited)
        ci = verify_ci(bundle)
        platforms = {}
        for host in ("windows", "linux"):
            env, runtime, sample = observe(bundle, host, ("regression", "workspace"), product, host)
            verify_b1(sample)
            verify_numeric(sample, host)
            native = verify_native(bundle, f"native-{host}", env, runtime)
            platforms[host] = {"workspace": WORKSPACE[host], "runtime": runtime, "b1_native_artifacts": native}
            platforms[host]["portable_artifacts"] = verify_portable(
                bundle, "portable" if host == "windows" else "portable-linux", env, runtime,
            )
            if host == "windows":
                require(all(f"22D-CASE {name}: ok" in sample["windows-22d"] for name in FAILURES),
                        "Windows 本机 22D 完整回归缺项")
        for mode, suites in (("b1", ("b1",)), ("numeric", ("numeric",)), ("workspace", ("workspace",)),
                             ("retained", ("b1",))):
            verify_macos_archive(bundle, mode)
            env, runtime, sample = observe(bundle, f"macos-{mode}", suites, product, "macos")
            if mode in ("b1", "retained"):
                verify_b1(sample)
            if mode == "numeric":
                verify_numeric(sample, "macos")
            if mode == "retained":
                platforms["macos"] = {"workspace": WORKSPACE["macos"], "retained_runtime": runtime,
                                      "b1_native_artifacts": verify_native(bundle, "macos-retained/native", env, runtime),
                                      "portable_artifacts": verify_portable(bundle, "macos-retained/portable", env, runtime)}
        guard = guards.verify(bundle)
        docs = verify_docs(bundle, root)
        archive = digest(raw) | {"paths": len(bundle.entries), "objects": bundle.unique}
    state = {key: True for key in BASIC} | {"open_items_assigned": bool(handoff["open_items"])}
    result = state | derive(state) | {
        "product": product, "inherited": {name: inherited[name] for name in B6_FIELDS},
        "b1_entries": 20, "b1_resource_cases": 18, "b1_guards": guard, "platforms": platforms,
        "historical_audits": len(previous), "history": previous, "ci": ci, "docs": docs, "archive": archive,
        "old_721_executed": False, "b2_c_started": False, "cmath": "planned",
    }
    if not prepare:
        require(json_bytes(result) == json_bytes(json.loads((root / "closeout.json").read_bytes())),
                "封存状态与实际证据派生结果不符")
    return result


def verify_manifest(root):
    stored = json.loads((root / "evidence.json").read_bytes())
    observed = {path.relative_to(root).as_posix(): digest(path.read_bytes()) for path in root.rglob("*")
                if path.is_file() and path.name != "evidence.json"}
    require(".gitattributes" in observed and stored == observed, "B7 封存文件清单或字节不符")


def verify_seal(root):
    path = f"{PREFIX}/evidence.json"
    # 首次提交封存清单的对象是证据身份。后续提交可以推进规划而不改变本批回执。
    introduced = git("log", "--reverse", "--format=%H", "--diff-filter=A", "--all", "--", path).decode().splitlines()
    require(introduced, "B7 清单尚无固定 Git 对象")
    require((root / "evidence.json").read_bytes() == blob(introduced[0], path), "B7 清单与固定 Git 对象不符")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--prepare", action="store_true", help="只派生待封存汇总")
    args = parser.parse_args()
    try:
        if not args.prepare:
            verify_manifest(args.root)
        result = inspect(args.root, prepare=args.prepare)
        if not args.prepare:
            verify_seal(args.root)
        print(json_bytes(result).decode("utf-8"), end="")
    except (BadZipFile, KeyError, ValueError, OSError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
