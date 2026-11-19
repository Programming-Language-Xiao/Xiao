"""归档并只读复核 B6-3 的固定负载、命令、原始输出与交接结果。"""

import argparse
from collections import Counter
import json
from pathlib import Path
import re
import sys
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True

from guards import GUARD_SOURCE, scripts, variants
from matrix import LANGUAGE_BACKENDS, REGRESSION, catalog
from prepare import (
    BUILD_OPTIONS, INPUTS, PRODUCT, REPO, ROOT, SHARED, SOURCE_SHA, file_identity, git, require, snapshot,
    source_digest, write_json,
)
from run import frames, observe_native, observe_suite, observe_vm, satisfies, suite_command
from b2_b6_macos import sha


RECIPE = "8385893f8b1805c369817fee0c4e5840df28a4b8"
LOCAL_RECIPE = "724ed80dd2e9e4a3f669c50e69172acd1668ab72"
MATRIX_COMMIT = "682beb6f1e538fcf03195c5ad7f71eed03e4ccab"
CLOSEOUT = "55c818fdd9b37369699a55b28441e4d95cbdb4d5"
RUNNER = "ddccdbee82daeb2c406dc7128889a7e921bedfd3"
RUN_ID = 44284131624
JOBS = {"vm": 134732268701, "o0": 134732268646, "o3": 134732268489,
        "runtime": 134732268723, "regression": 134732268807}
REGRESSIONS = ("vm-regressions", "b1-resources", "b1-contract", *REGRESSION)
HOSTS = {"windows": ("x86_64-pc-windows-msvc", "coff", "unverified-coff-exports", "PeakWorkingSetSize", "bytes"),
         "linux": ("x86_64-unknown-linux-gnu", "elf", "complete", "getrusage", "KiB"),
         "macos": ("aarch64-apple-darwin", "macho", "complete", "getrusage", "bytes")}
RAW_SUFFIXES = {".json", ".log", ".stdin", ".stdout", ".stderr", ".xiao", ".sh", ".ps1"}
MESSAGE_IDS = {
    "X06-RUNTIME-002": "runtime.type_mismatch",
    "X06-RUNTIME-009": "runtime.numeric_overflow",
    "X06-RUNTIME-011": "runtime.allocation",
    "X06-RUNTIME-012": "runtime.invalid_value",
    "X06-RUNTIME-013": "runtime.division_by_zero",
    "X06-RUNTIME-033": "runtime.numeric_limit",
    "X06-RUNTIME-034": "runtime.non_terminating_division",
}


def digest(data):
    return {"bytes": len(data), "sha256": sha(data)}


def normalized(path):
    return path.replace(chr(92), "/").removeprefix("//?/")


def valid_identity(item):
    return item["bytes"] > 0 and re.fullmatch(r"[0-9a-f]{64}", item["sha256"]) is not None


def closeout_file(name):
    return git("show", f"{CLOSEOUT}:{ROOT.relative_to(REPO).as_posix()}/{name}")


def verify_repo_check(path):
    require(path.is_file(), "仓库门禁归档缺失")
    with ZipFile(path) as bundle:
        outputs = ("stdout.json", "stderr.log")
        require(sorted(bundle.namelist()) == sorted(("receipt.json", *outputs)), "仓库门禁归档清单不符")
        receipt = json.loads(bundle.read("receipt.json"))
        require(receipt["schema_version"] == 1 and receipt["passed"] is True and receipt["exit_code"] == 0
                and receipt["pid"] > 0 and receipt["started_utc"] <= receipt["finished_utc"]
                and receipt["base_commit"] == RECIPE and receipt["product_commit"] == PRODUCT
                and receipt["argv"][1:] == ["tools/repo-check/src/cli.ts", "all", "--format", "json"],
                "仓库门禁进程回执不符")
        require(receipt["outputs"] == {name: digest(bundle.read(name)) for name in outputs},
                "仓库门禁输出身份不符")
        report = json.loads(bundle.read("stdout.json"))
        diagnostics = Counter(item["severity"] for item in report["diagnostics"])
        require(report["checker"] == "xiao-repo-check" and report["passed"] is True
                and not bundle.read("stderr.log") and not diagnostics["error"]
                and diagnostics == receipt["diagnostics_by_severity"], "仓库门禁实际输出不符")

    changed = set(git("diff", "--name-only", MATRIX_COMMIT + "~1", CLOSEOUT, "--", "docs/DevDocs")
                  .decode("utf-8").splitlines())
    changed.remove((ROOT / "repo-check.zip").relative_to(REPO).as_posix())
    sources = receipt["source_after"]
    require(sources == receipt["source_before"] and set(sources) == changed, "仓库门禁文件清单不符")
    for name, info in sources.items():
        committed = git("show", f"{CLOSEOUT}:{name}")
        require(digest(committed) == info.get("utf8_lf", info), f"仓库门禁固定文件身份不符：{name}")
    require(path.read_bytes() == closeout_file("repo-check.zip"), "仓库门禁归档与固定收口不符")


def verify_closeout(result, repo_check):
    summary = closeout_file("summary.json")
    require((ROOT / "summary.json").read_bytes() == summary, "发布汇总与固定收口不符")
    require(result == json.loads(summary), "边界归档与固定收口汇总不符")
    verify_repo_check(repo_check)


def collect(archive, sources):
    entries = {}
    objects = set()
    with ZipFile(archive, "x", compression=ZIP_DEFLATED, compresslevel=6) as bundle:
        for label, root in sources:
            require(root.is_dir(), f"证据目录不存在：{root}")
            for path in sorted(root.rglob("*")):
                name = path.relative_to(root)
                if not path.is_file() or "home" in name.parts or path.suffix not in RAW_SUFFIXES:
                    continue
                key = f"{label}/{name.as_posix()}"
                require(key not in entries, f"证据路径重复：{key}")
                data = path.read_bytes()
                info = digest(data)
                entries[key] = info
                member = "objects/" + info["sha256"]
                if member not in objects:
                    bundle.writestr(member, data)
                    objects.add(member)
        bundle.writestr("evidence.json", json.dumps(entries, sort_keys=True, indent=2) + "\n")


class Evidence:
    def __init__(self, bundle):
        self.bundle = bundle
        self.entries = json.loads(bundle.read("evidence.json"))
        objects = {"objects/" + item["sha256"] for item in self.entries.values()}
        require(set(bundle.namelist()) == objects | {"evidence.json"}
                and len(bundle.namelist()) == len(objects) + 1, "证据包清单不符")
        for name in self.entries:
            self.raw(name)
        self.unique = len(objects)

    def raw(self, name):
        require(name in self.entries, f"证据缺失：{name}")
        identity = self.entries[name]
        raw = self.bundle.read("objects/" + identity["sha256"])
        require(digest(raw) == identity, f"证据字节改变：{name}")
        return raw

    def read(self, name):
        return json.loads(self.raw(name))

    def command(self, folder, stem, host):
        row = self.read(f"{folder}/{stem}.json")
        require(row["pid"] > 0 and not row["timed_out"] and row["started_utc"] <= row["finished_utc"],
                f"进程回执不符：{folder}/{stem}")
        files = [f"{stem}.{stream}" for stream in ("stdin", "stdout", "stderr")]
        require(row["files"] == {name: self.entries[f"{folder}/{name}"] for name in files},
                f"进程原始文件身份不符：{folder}/{stem}")
        rss = row["rss"]
        if rss is not None:
            method, unit = HOSTS[host][3:]
            require(rss["method"] == method and rss["unit"] == unit
                    and (rss["peak"] is None or rss["peak"] > 0), f"OS 内存单位不符：{folder}/{stem}")
        return row, *(self.raw(f"{folder}/{name}") for name in files)


def recipe_files(revision):
    prefix = ROOT.relative_to(REPO).as_posix()
    paths = {name: f"{prefix}/{name}" for name in INPUTS} | {name: name for name in SHARED}
    return {name: digest(git("show", f"{revision}:{path}")) for name, path in paths.items()}


def verify_setup(evidence, root, host, source, recipe):
    environment = evidence.read(f"{root}/environment.json")
    built = evidence.read(f"{root}/build.json")
    target = HOSTS[host][0]
    task = normalized(environment["task_root"])
    suffix = ".exe" if host == "windows" else ""
    library = "xiao_runtime.lib" if host == "windows" else "libxiao_runtime.a"
    source_root = f"{task}/source"
    runtime = f"{task}/target/release/{library}"
    overrides = environment["environment"]
    tools = environment["tools"]
    require(environment["platform"] == host and environment["target"] == target
            and environment["product"] == PRODUCT and environment["source_sha256"] == SOURCE_SHA
            and evidence.read(f"{root}/source.json") == source
            and environment["inputs"] == recipe and normalized(environment["source_root"]) == source_root
            and normalized(environment["runtime"]) == normalized(overrides["XIAO_RUNTIME_LIBRARY"]) == runtime
            and normalized(environment["core"]) == f"{task}/bin/xiao-core{suffix}"
            and normalized(overrides["CARGO_TARGET_DIR"]) == f"{task}/target"
            and normalized(overrides["XIAO_STDLIB_ROOT"]) == f"{source_root}/stdlib"
            and normalized(overrides["XIAO_HOME"]) == f"{task}/home"
            and overrides["RUSTUP_TOOLCHAIN"] == "1.96.0" and overrides["XIAO_TARGET_TRIPLE"] == target
            and overrides["RUSTC"] == tools["rustc"]["path"]
            and all(overrides[name] == tools[name]["path"] for name in ("XIAO_CLANG", "XIAO_LLC", "XIAO_LLVM_AS"))
            and overrides["XIAO_B1_F10"] == overrides["XIAO_B2_A_PERF"] == "0",
            f"执行身份或任务根不符：{root}")
    require(tools["rustc"]["version"].startswith("rustc 1.96.0")
            and f"host: {target}" in tools["rustc"]["version"]
            and tools["cargo"]["version"].startswith("cargo 1.96.0")
            and all(valid_identity(tool) for tool in tools.values())
            and all(valid_identity(built[name]) for name in ("runtime", "core")),
            f"工具链身份不符：{root}")
    for label, before, target_args in (
        ("runtime-build", None, ["-p", "xiao-runtime"]),
        ("core-build", built["runtime"], ["-p", "xiao-driver", "--bin", "xiao-core"]),
    ):
        command = evidence.read(f"{root}/{label}.json")
        require(command["exit_code"] == 0 and command["pid"] > 0
                and command["started_utc"] <= command["finished_utc"]
                and command["argv"] == [tools["cargo"]["path"], "build", *BUILD_OPTIONS, *target_args]
                and normalized(command["temp"]) == f"{task}/temp-{label}"
                and command["source_after_sha256"] == SOURCE_SHA and normalized(command["cwd"]) == source_root
                and command["runtime_before"] == before and command["runtime_after"] == built["runtime"]
                and command["log"] == {"name": label + ".log", **evidence.entries[f"{root}/{label}.log"]},
                f"构建回执不符：{root}/{label}")
    return environment, built


def verify_summary(summary, environment, built, mode):
    require(summary["platform"] == environment["platform"] and summary["product"] == PRODUCT
            and summary["source_sha256"] == SOURCE_SHA and summary["mode"] == mode
            and summary["matrix"] == environment["inputs"]["matrix.json"] and summary["build"] == built,
            "采集器汇总身份不符")


def wrapper_span(actual, response, source):
    wrapper = (response.get("report") or {}).get("stack", [])
    if wrapper:
        name = wrapper[0]["function"].split("::")[-1]
    else:
        match = re.search(r"at stdlib\.cmath::(\w+) \(User\)", actual["diagnostic"])
        name = match[1] if match else ""
    token = f"cmath_{name}(".encode()
    require(source.count(token) == 1, "cmath 包装错误缺少源码归因")
    start = source.index(token)
    return [start, source.index(b")", start) + 1]


def verify_language(evidence, root, host, environment, mode, backend, specs, retained, cmath):
    ids = set(specs)
    require({row["case"] for row in retained} == ids and len(retained) == len(ids)
            and all(row["backend"] == backend for row in retained), f"语言负载清单不全：{host}/{backend}")
    result = Counter()
    verify = HOSTS[host][2] if backend != "vm" else None
    missing_rss = 0
    for row in retained:
        name = row["case"]
        spec = specs[name]
        folder = f"{root}/{backend}/{name}"
        directory = f"{normalized(environment['task_root'])}/evidence/{mode}/{backend}/{name}"
        require(evidence.read(f"{folder}/result.json") == row, f"单项与汇总不符：{folder}")
        source = evidence.raw(f"{folder}/input.xiao")
        require(sha(source) == spec["source_sha256"] and digest(source) == row["source"],
                f"边界输入不符：{folder}")
        command, wire, stdout, stderr = evidence.command(folder, "protocol", host)
        hello, request = frames(wire)
        reply, response = frames(stdout)
        require(command["exit"] == 0 and not stderr and command["argv"] == [environment["core"]]
                and reply["accepted"] and reply["request_id"] == hello["request_id"]
                and (reply["versions"]["runtime_abi"], reply["versions"]["codegen"]) == (65555, 22)
                and request["request_id"] == response["request_id"] == name
                and request["source"]["text"].encode() == source
                and normalized(command["cwd"]) == directory
                and normalized(request["source"]["path"]) == directory + "/input.xiao"
                and request["target"]["triple"] == environment["target"]
                and request["target"]["object_format"] == HOSTS[host][1],
                f"真实编译/执行请求不符：{folder}")
        if backend == "vm":
            require(request["type"] == response["operation"] == "run", f"VM 入口不符：{folder}")
            actual = observe_vm(response)
            message_id = (response.get("report") or {}).get("message_id")
        else:
            program, inp, out, err = evidence.command(folder, "program", host)
            executable = directory + ("/program.exe" if host == "windows" else "/program")
            llvm = directory + "/program.ll"
            artifact = response["artifact"]
            composition = artifact["artifact_runtime"]
            require(response["exit_code"] == 0 and request["type"] == response["operation"] == "build"
                    and request["toolchain"]["runtime_library"] == environment["runtime"]
                    and artifact["object_digest"] == row["executable"]["sha256"]
                    and valid_identity(row["executable"]) and valid_identity(row["llvm"])
                    and artifact["optimization_level"] == request["optimization"]["level"] == int(backend[1])
                    and normalized(request["output"]) == normalized(artifact["executable"]) == executable
                    and normalized(request["llvm_ir_output"]) == normalized(artifact["llvm_ir_output"]) == llvm
                    and not inp and list(map(normalized, program["argv"])) == [executable]
                    and normalized(program["cwd"]) == directory
                    and composition == row["artifact"] and composition["verification"] == verify
                    and composition["object_format"] == HOSTS[host][1]
                    and (verify != "complete" or composition["declared_components"] == composition["observed_components"]),
                    f"原生产物或组件核对不符：{folder}")
            actual = observe_native(program, out, err)
            message_id = actual["message_id"]
            command = program
        code = spec["expected"]["code"]
        require(message_id == MESSAGE_IDS.get(code), f"F-12 消息标识不符：{folder}")
        if code and spec["expected"]["span"] is None:
            require(actual["span"] == wrapper_span(actual, response, cmath), f"cmath 包装错误位置不符：{folder}")
        passed = satisfies(actual, spec["expected"])
        require(actual == row["actual"] and row["status"] == ("passed" if passed else "failed"),
                f"语言结论与原始输出不符：{folder}")
        result[row["status"]] += 1
        if command["rss"] is None or command["rss"]["peak"] is None:
            missing_rss += 1
    return {"counts": dict(result), "native_verification": verify, "missing_peak_samples": missing_rss}


def verify_suite(evidence, root, label, environment):
    host = environment["platform"]
    folder = f"{root}/{label}"
    row, inp, raw, err = evidence.command(folder, "cargo", host)
    require(not inp and not err and row["argv"] == suite_command(label, environment["tools"]["cargo"]["path"])
            and row["cwd"] == environment["source_root"], f"Cargo 回执与命令不符：{folder}")
    if label == "cmath-native":
        text = raw.decode("utf-8").replace("\r\n", "\n")
        matrices = re.findall(
            r"^B4-3-MATRIX dynamic=(false|true) optimization=([03]) executable=.+ "
            r"B4-3-NATIVE cases=18318 error_cases=\d+ pow_inputs=1075 bit_or_code=exact$",
            text, re.M,
        )
        require(Counter(matrices) == Counter((path, level) for path in ("false", "true")
                                             for level in ("0", "3")),
                f"原生正式向量或共享幂回执不符：{host}")
    return observe_suite(label, row["exit"], raw)


def verify_abi(evidence, root, host, fixed, retained, environment):
    folder = f"{root}/abi"
    link, link_in, _, _ = evidence.command(folder, "link", host)
    normal, normal_in, out, err = evidence.command(folder, "normal", host)
    fatal, fatal_in, fatal_out, fatal_err = evidence.command(folder, "fatal", host)
    mode = root.rsplit("/", 1)[-1]
    directory = f"{normalized(environment['task_root'])}/evidence/{mode}/abi"
    executable = directory + ("/abi_probe.exe" if host == "windows" else "/abi_probe")
    prefix = [environment["environment"]["XIAO_CLANG"], "-target", environment["target"], "-std=c11", "-Werror"]
    require(link["exit"] == normal["exit"] == 0 and not (link_in or normal_in or fatal_in or err)
            and link["argv"][:5] == prefix and normalized(link["argv"][5]).endswith("/abi_probe.c")
            and link["argv"][6] == environment["runtime"] and link["argv"][-2] == "-o"
            and normalized(link["argv"][-1]) == executable
            and list(map(normalized, normal["argv"])) == [executable]
            and list(map(normalized, fatal["argv"])) == [executable, "fatal"]
            and all(normalized(row["cwd"]) == directory for row in (link, normal, fatal))
            and valid_identity(retained["executable"]),
            "C ABI 探针链接或进程身份不符")
    observed = {}
    for line in out.decode().splitlines():
        name, status, code, before, after, value, ownership = line.split("\t")
        require(name not in observed and name in fixed["c_abi"], f"C ABI 观察编号不符：{name}")
        status, before, after = map(int, (status, before, after))
        expected = fixed["c_abi"][name]
        require([status, value if status == 0 else code] == expected
                and value == ("0.5" if status == 0 else "old-out") and ownership == "owned"
                and after > 0 and (before == 0 if status == 6 else before == after)
                and (status != 0 or code == "B6_SENTINEL"), f"C ABI 状态、输出或挂起槽不符：{name}")
        observed[name] = {"status": status, "code": code, "before_id": before,
                          "after_id": after, "out": value, "ownership": ownership}
    require(observed.keys() == fixed["c_abi"].keys() - {"fatal"} and observed == retained["rows"], "C ABI 观察缺项")
    fields = fatal_out.decode().strip().split("\t")
    status, code = fixed["c_abi"]["fatal"]
    require(len(fields) == 6 and fields[:3] == ["fatal", str(status), code]
            and fields[3] == fields[4] != "0" and fields[5] == "4" and fatal["exit"] == 4
            and f"class=fatal code={code} ".encode() in fatal_err
            and retained["fatal"] == {"status": "passed", "observation": fields}
            and retained["status"] == "passed", "Fatal/8 独立观测不符")
    return {"passed": len(observed) + 1, "statuses": sorted({row["status"] for row in observed.values()} | {status}),
            "fatal": {"abi_status": status, "process_exit": fatal["exit"], "code": code}}


def verify_wrappers(raw, host):
    pattern = (r"^B4-4-NATIVE path=(\S+) optimization=([03]) executable=.+ verification=(\S+)"
               r" declared=(\[.*?\]) observed=(\[.*?\]) stdout=golden errors=009 result=passed$")
    rows = re.findall(pattern, raw.decode().replace("\r\n", "\n"), re.M)
    status = "UnverifiedCoffExports" if host == "windows" else "Complete"
    require(Counter((name, level) for name, level, *_ in rows) ==
            Counter((name, level) for name in ("std.cmath", "stdlib.cmath") for level in ("0", "3"))
            and all(verification == status and json.loads(declared) == json.loads(observed)
                    == ["containers", "rc", "value", "weak"] for _, _, verification, declared, observed in rows),
            f"B4 包装四格回执不符：{host}")


def verify_regressions(evidence, root, retained, environment, replay):
    host = environment["platform"]
    require(retained.keys() == set(REGRESSIONS), f"回归命令缺项：{host}")
    rows = {}
    for label in REGRESSIONS:
        actual = verify_suite(evidence, root, label, environment)
        original = {key: retained[label][key] for key in actual}
        if replay and label in ("b1-vm", "b1-native"):
            require(original["status"] == "failed" and actual["status"] == "passed"
                    and original | {"status": "passed"} == actual, f"旧采集误判无法复核：{host}/{label}")
        else:
            require(actual == original and actual["status"] == "passed", f"回归实际失败：{host}/{label}")
        rows[label] = actual["test_counts"]
    verify_wrappers(evidence.raw(f"{root}/cmath-wrappers/cargo.stdout"), host)
    return rows


def verify_ci(evidence):
    prefix = f"ci-{RUN_ID}"
    workflow = evidence.read(f"{prefix}/workflow.json")
    artifacts = evidence.read(f"{prefix}/artifact-api.json")["artifacts"]
    require(workflow["databaseId"] == RUN_ID and workflow["headSha"] == RUNNER and workflow["attempt"] == 1
            and workflow["status"] == "completed" and workflow["conclusion"] == "success"
            and len(workflow["jobs"]) == len(JOBS)
            and {job["name"]: job["databaseId"] for job in workflow["jobs"]
                 if job["status"] == "completed" and job["conclusion"] == "success"}
            == {f"B6-3 macOS {mode}": identifier for mode, identifier in JOBS.items()}
            and {item["name"] for item in artifacts} == {f"b6-3-macos-{mode}-{RUN_ID}-1" for mode in JOBS},
            "macOS Runner 作业或产物身份不符")
    require(snapshot(RUNNER) == snapshot(PRODUCT), "macOS Runner 受测 Git 对象不符")
    for name, info in recipe_files(RECIPE).items():
        path = name if name in SHARED else f"{ROOT.relative_to(REPO).as_posix()}/{name}"
        require(digest(git("show", f"{RUNNER}:{path}")) == info, f"macOS Runner 采集入口不符：{name}")
    require(len(artifacts) == len(JOBS) and all(
        item["workflow_run"]["id"] == RUN_ID and item["workflow_run"]["head_sha"] == RUNNER
        and re.fullmatch(r"sha256:[0-9a-f]{64}", item["digest"]) for item in artifacts
    ), "macOS 产物 API 来源不符")
    for run_id, job_id, reason in (
        (44279794451, 134717138346, "ValueError: 执行后源码或产物身份变化"),
        (44281869728, 134724364070, '{"platform": "macos", "mode": "regression", "counts": {"passed": 7, "failed": 2}}'),
    ):
        old = evidence.read(f"ci-{run_id}/workflow.json")
        log = evidence.raw(f"ci-{run_id}/regression-failure.log").decode("utf-8")
        require(old["databaseId"] == run_id and old["conclusion"] == "failure" and old["attempt"] == 1
                and {row["databaseId"] for row in old["jobs"] if row["conclusion"] == "failure"} == {job_id}
                and any(line.endswith("Z " + reason) for line in log.splitlines()), f"历史失败缺失：{run_id}")
    return {"run": RUN_ID, "attempt": 1, "runner": RUNNER, "jobs": JOBS}


def verify_macos_artifacts(archive, directory):
    with ZipFile(archive) as bundle:
        entries = json.loads(bundle.read("evidence.json"))
        api = entries[f"ci-{RUN_ID}/artifact-api.json"]
        artifacts = json.loads(bundle.read("objects/" + api["sha256"]))["artifacts"]
    rows = {}
    for item in artifacts:
        name = item["name"]
        path = directory / f"{item['id']}.zip"
        identity = file_identity(path)
        require(identity == {"bytes": item["size_in_bytes"], "sha256": item["digest"].removeprefix("sha256:")},
                f"macOS 原始产物摘要不符：{name}")
        prefix = f"macos/{name}/"
        archived = {key.removeprefix(prefix): info for key, info in entries.items() if key.startswith(prefix)}
        with ZipFile(path) as artifact:
            members = [info for info in artifact.infolist() if not info.is_dir()]
            observed = {info.filename: digest(artifact.read(info)) for info in members}
        require(len(members) == len(observed) and observed == archived, f"macOS 原件与归档文件不符：{name}")
        rows[name] = {"artifact_id": item["id"], "zip": identity, "files": len(observed),
                      "manifest_sha256": sha(json.dumps(observed, sort_keys=True, separators=(",", ":")).encode())}
    return {"run": RUN_ID, "attempt": 1, "runner": RUNNER, "artifacts": rows,
            "files": sum(row["files"] for row in rows.values())}


def verify_guards(evidence, sources):
    summary = evidence.read("guards/summary.json")
    workflow, bodies = scripts()
    require(summary["source_commit"] == GUARD_SOURCE and summary["workflow_sha256"] == sha(workflow)
            and summary["passed"] == 28
            and (summary["actual"], summary["derived"]) == (4, 24) and len(summary["rows"]) == 28,
            "计数守卫范围不符")
    for path, source in sources.items():
        raw = evidence.raw(f"{source}/cargo.stdout")
        for case, adjusted, cargo_exit in variants(raw):
            name = f"{path}/{case}"
            folder = f"guards/{name}"
            script = "guard.ps1" if path == "windows" else "guard.sh"
            prefix = (f"function cargo {{ Get-Content -LiteralPath 'cargo-output.log'; $global:LASTEXITCODE = {cargo_exit} }}\n"
                      if path == "windows" else f"cargo() {{ cat cargo-output.log; return {cargo_exit}; }}\n")
            row, inp, stdout, stderr = evidence.command(folder, "guard", "windows")
            recorded = summary["rows"][name]
            cwd = normalized(row["cwd"])
            shell = normalized(row["argv"][0]).rsplit("/", 1)[-1].lower()
            arguments = list(map(normalized, row["argv"][1:]))
            require(cwd.endswith(f"/{name}") and not inp
                    and (shell == "pwsh.exe" and arguments == ["-NoProfile", "-File", f"{cwd}/{script}"]
                         if path == "windows" else shell == "bash.exe" and arguments == [script])
                    and stdout.replace(b"\r\n", b"\n").rstrip(b"\n") == adjusted.rstrip(b"\n"),
                    f"计数守卫命令或输出不符：{name}")
            expected_exit = cargo_exit if path == "windows" and cargo_exit else int(case != "actual")
            if case != "actual" and cargo_exit == 0 and path == "windows":
                reason = stderr.decode("utf-8").splitlines()[-1:]
                reason_ok = [line.strip() for line in reason] == [
                    "| B1 native probe must pass exactly one case without ignoring it"]
            elif case != "actual" and cargo_exit == 0 and path in ("linux", "macos"):
                reason_ok = stderr.strip() == b"Native test did not execute exactly one case: b1_capabilities_match_native_golden_outputs"
            else:
                reason_ok = not stderr
            require(row["exit"] == expected_exit and reason_ok, f"计数守卫拒绝原因不符：{name}")
            require(evidence.raw(f"{folder}/cargo-output.log") == adjusted
                    and evidence.raw(f"{folder}/{script}") == (prefix + bodies[path]).encode()
                    and recorded == {"accepted": row["exit"] == 0, "expected": case == "actual", "exit": row["exit"],
                                     "cargo_exit": cargo_exit, "source_sha256": sha(raw),
                                     "derived_input_sha256": sha(adjusted)}
                    and recorded["accepted"] == recorded["expected"], f"计数守卫回放不符：{name}")
    return {"passed": 28, "actual": 4, "derived": 24}


def counts(tests):
    return dict(zip(("passed", "failed", "ignored"), map(sum, zip(*tests))))


def audit(archive):
    matrix = git("show", f"{MATRIX_COMMIT}:{ROOT.relative_to(REPO).as_posix()}/matrix.json")
    fixed = json.loads(matrix)
    require(matrix == (ROOT / "matrix.json").read_bytes() and fixed == json.loads(json.dumps(catalog())),
            "固定边界矩阵与生成规则不符")
    specs = {row["id"]: row for row in fixed["language"]}
    require(len(specs) == len(fixed["language"]), "固定边界矩阵有重复用例")
    source = snapshot(PRODUCT)
    require(source_digest(source) == SOURCE_SHA, "固定产品摘要不符")
    cmath = git("show", f"{PRODUCT}:stdlib/cmath.xiao")
    with ZipFile(archive) as bundle:
        evidence = Evidence(bundle)
        ci = verify_ci(evidence)
        local_recipe, remote_recipe = recipe_files(LOCAL_RECIPE), recipe_files(RECIPE)
        platforms = {}
        for host in ("windows", "linux", "macos"):
            groups = {mode: f"macos/b6-3-macos-{mode}-{RUN_ID}-1" for mode in JOBS} if host == "macos" else {"all": host}
            language, regressions, abi, runtime = {}, {}, None, None
            for mode, root in groups.items():
                environment, built = verify_setup(evidence, root, host, source,
                                                  remote_recipe if host == "macos" else local_recipe)
                if host == "macos":
                    require(environment["checkout"] == RUNNER and environment["github"] ==
                            {"RUN_ID": str(RUN_ID), "RUN_ATTEMPT": "1", "JOB": "verify"}, "macOS 作业回执不符")
                base = f"{root}/{mode}"
                summary = evidence.read(f"{base}/summary.json")
                verify_summary(summary, environment, built, mode)
                if mode == "all" or mode in LANGUAGE_BACKENDS:
                    for backend in LANGUAGE_BACKENDS if mode == "all" else (mode,):
                        retained = [row for row in summary["language"] if row["backend"] == backend]
                        language[backend] = verify_language(evidence, base, host, environment, mode, backend,
                                                            specs, retained, cmath)
                if mode in ("all", "runtime"):
                    abi = verify_abi(evidence, base, host, fixed, summary["abi"], environment)
                    runtime = verify_suite(evidence, base, "runtime", environment)
                    require(runtime["status"] == "passed" and all(runtime[key] == summary["runtime"][key] for key in runtime),
                            f"Runtime 断言未通过：{host}")
                if mode in ("all", "regression"):
                    regressions = verify_regressions(evidence, base, summary["regressions"], environment, replay=mode == "all")
                if mode == "all":
                    require(summary["passed"] is False and summary["counts"] == {"passed": 264, "failed": 2},
                            f"本地采集器历史结论不符：{host}")
                else:
                    require(summary["passed"] is True, f"macOS 专项未通过：{mode}")
            require(set(language) == set(LANGUAGE_BACKENDS)
                    and all(row["counts"] == {"passed": len(specs)} for row in language.values()),
                    f"边界负载执行/通过集合不全：{host}")
            platforms[host] = {"language": language, "runtime": counts(runtime["test_counts"]),
                               "existing_runtime_assertions": len(runtime["named_assertions"]), "c_abi": abi,
                               "vm_regressions": counts(regressions["vm-regressions"]),
                               "b1_contract": counts(regressions["b1-contract"]),
                               "b1_resources": counts(regressions["b1-resources"]),
                               "exact_regressions": len(REGRESSION),
                               "raw_collector_false_negatives": ["b1-vm", "b1-native"] if host != "macos" else []}
        guards = verify_guards(evidence, {
            "default": "windows/all/b1-vm", "windows": "windows/all/b1-native", "linux": "linux/all/b1-native",
            "macos": f"macos/b6-3-macos-regression-{RUN_ID}-1/regression/b1-native",
        })
        return {"verified": True, "boundary_passed": True, "source_commit": PRODUCT, "source_sha256": SOURCE_SHA,
                "matrix": digest(matrix), "language_cases": len(specs),
                "language_runs": len(specs) * len(HOSTS) * len(LANGUAGE_BACKENDS),
                "new_performance_samples": 0, "b6_1_status": "restricted", "b6_1_stage_passed": False,
                "windows_export_integrity": "unverifiable", "linux_clippy": "not_run",
                "platforms": platforms, "macos_ci": ci, "guard_replay": guards,
                "archive": {**file_identity(archive), "files": len(evidence.entries), "unique_objects": evidence.unique}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, default=ROOT / "boundaries.zip")
    parser.add_argument("--repo-check", type=Path, default=ROOT / "repo-check.zip", help="复核仓库门禁归档副本")
    parser.add_argument("--macos-artifacts", type=Path, help="将产物 ID 命名的 ZIP 原件与收口归档逐文件对账")
    parser.add_argument("--collect", nargs="+", metavar="LABEL=DIR")
    parser.add_argument("--report", type=Path, help="独占生成汇总回执")
    args = parser.parse_args()
    if args.collect:
        sources = [(label, Path(path)) for label, path in (entry.split("=", 1) for entry in args.collect)]
        collect(args.archive, sources)
    result = audit(args.archive)
    if not args.collect:
        verify_closeout(result, args.repo_check)
    if args.macos_artifacts:
        result["macos_artifacts"] = verify_macos_artifacts(args.archive, args.macos_artifacts)
    if args.report:
        write_json(args.report, result)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
