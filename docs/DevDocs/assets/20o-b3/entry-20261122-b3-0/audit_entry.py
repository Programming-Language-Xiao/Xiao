"""只读复核 B3-0A 的固定产品、三平台探测及 B1 前置。"""

import argparse
from importlib.util import module_from_spec, spec_from_file_location
from io import BytesIO
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import sys
from zipfile import BadZipFile, ZipFile

sys.dont_write_bytecode = True
from collect import CLI_PROBE, MODULES, ROOT, base

PREFIX = ROOT.relative_to(base.REPO).as_posix()
RUNNER = "0aa8540f9ffcb0ba1ed7a4acf85bc88c8a78ce16"
RUNNER_PARENT = "d3e8b9ed8517d293e526c3ad6c0a0dfc0c985549"
PROBE_RUN = 45033610157
COLLATERAL_RUN = 45033610101
PROBE_JOBS = {"linux": 137242370871, "macos": 137242370630}
ANCHOR = "fb3befee"
B2_C = base.REPO / "docs/DevDocs/assets/20o-b2-c/entry-20261122-b2-c"
B7 = base.REPO / "docs/DevDocs/assets/20o-b2-b/entry-20261122-b7"
B1 = ("b1-contract", "b1-resources", "b1-vm", "b1-native")


def load(directory, stem):
    spec = spec_from_file_location(f"b3_{directory.name}_{stem}", directory / f"{stem}.py")
    module = module_from_spec(spec)
    previous = sys.path[:]
    sys.path.insert(0, str(directory))
    try:
        spec.loader.exec_module(module)
    finally:
        sys.path[:] = previous
    return module


archive_tools = load(B2_C, "entry_audit")
b7 = load(B7, "audit")
require, git = base.require, base.git


def fixed_baseline(root):
    evidence = b7.product_identity()
    b2_path = B2_C.relative_to(base.REPO).as_posix()
    previous = json.loads(git("show", f"{ANCHOR}:{b2_path}/closeout.json"))
    require(previous["b2c_passed"] is True and previous["source"] == evidence
            and previous["public_scope"]["windows_product_coff_verified"] is False, "B2-C 基线或 COFF 历史身份不符")
    changes = set(git("diff", "--name-only", "-z", RUNNER_PARENT, RUNNER).decode().split("\0")) - {""}
    recipes = (".gitattributes", "collect.py", "host_probe.rs")
    require(changes == {".github/workflows/b3-0-probe.yml", *(f"{PREFIX}/{name}" for name in recipes)},
            "Runner 改动越出孤立取证范围")
    for name in recipes:
        require((root / name).read_bytes() == git("show", f"{RUNNER}:{PREFIX}/{name}"),
                f"探测配方固定字节不符：{name}")
    for folder, names in ((B7, ("run.py", "audit.py", "guards.py")),
                          (B2_C, ("entry_audit.py", "measure.py", "performance_audit.py"))):
        for file in names:
            path = folder / file
            path_in_git = path.relative_to(base.REPO).as_posix()
            require(path.read_bytes().replace(b"\r\n", b"\n")
                    == git("show", f"{ANCHOR}:{path_in_git}").replace(b"\r\n", b"\n"),
                    f"继承的取证工具字节不符：{path_in_git}")
    return evidence, previous["inherited"]


def ci(book, number, workflow):
    folder = f"ci/{number}"
    run = book.read(f"{folder}/run.json")
    jobs = book.read(f"{folder}/jobs.json")["jobs"]
    require(run["id"] == number and run["path"] == workflow and run["head_sha"] == RUNNER
            and run["run_attempt"] == 1 and run["status"] == "completed", "CI 运行身份不符")
    for job in jobs:
        job_log = book.raw(f"{folder}/{job['id']}.log")
        require(job["run_id"] == number and job["head_sha"] == RUNNER and job["run_attempt"] == 1
                and job["status"] == "completed" and run["run_started_at"] <= job["started_at"] < job["completed_at"]
                and b"Current runner version:" in job_log and RUNNER.encode() in job_log, "CI 实际作业不符")
    return run, {job["id"]: job for job in jobs}


def remote(book, number, job, host):
    folder = f"ci/{number}"
    name = f"b3-0-{host}-{number}-1"
    (artifact,) = [row for row in book.read(f"{folder}/artifacts.json")["artifacts"] if row["name"] == name]
    raw = book.raw(f"{folder}/{artifact['id']}.zip")
    sha = base.sha(raw)
    log = book.raw(f"{folder}/{job['id']}.log")
    require(job["name"] == f"B3-0 {host}" and job["conclusion"] == "success"
            and artifact["digest"] == f"sha256:{sha}" and artifact["size_in_bytes"] == len(raw)
            and str(artifact["id"]).encode() in log and sha.encode() in log, f"原始上传身份不符：{host}")
    return ZipFile(BytesIO(raw)), {"run": number, "job": job["id"], "attempt": 1,
                                  "artifact": artifact["id"], "sha256": sha}


class LocalB1:
    def __init__(self, raw):
        self.raw_file = raw

    def raw(self, name):
        return self.raw_file(name.split("/", 1)[1])

    def read(self, name):
        result = json.loads(self.raw(name))
        return {key: result["exit"][key] for key in B1} if name.endswith("/results.json") else result


class GuardEvidence(b7.Evidence):
    def __init__(self, zipped, raw_sources):
        self.raw_sources = raw_sources
        super().__init__(zipped)

    def raw(self, name):
        if name in self.raw_sources:
            return self.raw_sources[name]
        return super().raw(name)


def inspect_host(root, raw, product, host):
    bundle = LocalB1(raw)
    env, runtime, b1_logs = b7.observe(bundle, host, ("b1",), product, host)
    b7.verify_b1(b1_logs)
    target = env["target"]
    path = PureWindowsPath if host == "windows" else PurePosixPath
    suffix = ".exe" if host == "windows" else ""
    task = path(env["task"])
    tools = env["tools"]
    require(env["probe"] == base.identity(root / "host_probe.rs")
            and env["collector"] == base.identity(root / "collect.py")
            and {"bun", "rustc", "cargo", "linker", "clang", "llvm-as", "llc", "llvm-strip"} <= tools.keys()
            and all(tool["version"] and tool["path"] for tool in tools.values()), f"工具或探测程序源码身份不符：{host}")
    wanted = {"runtime", "probe-build", *MODULES, *B1}
    results = json.loads(raw("results.json"))
    require(results["exit"].keys() == wanted and all(value == 0 for value in results["exit"].values()),
            f"探测/回归执行全集不符：{host}")
    b7.check_identity(results["probe_executable"])
    executable = task / f"host-probe{suffix}"
    probe = json.loads(raw("probe-build.json"))["argv"]
    require(probe[:3] == [tools["rustc"]["path"], "--edition", "2024"]
            and path(probe[3]).as_posix().endswith(f"/{PREFIX}/host_probe.rs")
            and probe[4:] == ["-o", str(executable)], f"探测程序编译命令不符：{host}")
    commands = {"probe-build": probe, "cli": [tools["bun"]["path"], "-e", CLI_PROBE]}
    for label in MODULES:
        if label != "cli":
            commands[label] = [str(executable), label]
    commands["fs"].append(str(task / "tmp/fs/work"))

    observed = {}
    for label, command in commands.items():
        receipt = json.loads(raw(f"{label}.json"))
        log = raw(f"{label}.log")
        b7.check_process(receipt)
        require(receipt["argv"] == command and receipt["log"] == archive_tools.identity(log)
                and receipt["source_before"] == receipt["source_after"] == product
                and receipt["runtime_before"] == receipt["runtime_after"] == runtime
                and path(receipt["temp"]) == task / "tmp" / label and path(receipt["cwd"]) == task / "source",
                f"命令、原始输出或独占任务根不符：{host}/{label}")
        if label != "probe-build":
            observed[label] = log.decode("utf-8").splitlines()
    verify_observations(observed, host)
    return {"target": target, "runtime": runtime, "observed": observed,
            "b1": {"entries": 20, "resource_targets": 18, "vm_exact": 1, "native_exact": 1}}


def verify_observations(observed, host):
    time = observed["time"]
    require(len(time) == 7 and re.fullmatch(r"wall_unix_ns=[1-9]\d+", time[0])
            and re.fullmatch(r"monotonic_min_observed_ns=Some\([1-9]\d*\) regressions=\d+", time[1])
            and re.fullmatch(r"host_wakeable_wait_received=true observed_ns=\d+", time[5])
            and time[6] == "host_wait_deadline=true", f"时钟或唤醒观察不完整：{host}")
    for requested, line in zip((0, 1, 10), time[2:5], strict=True):
        prefix = f"sleep_requested_ms={requested} observed_ns="
        require(line.startswith(prefix), f"sleep 样本请求不符：{host}")
        durations = json.loads(line.removeprefix(prefix))
        require(len(durations) == 5 and all(type(value) is int and value >= 0 for value in durations),
                f"sleep 原始样本不完整：{host}")
    require(observed["argv"] == ["argv0_present=true argv_count=10 valid_exact=true malformed_roundtrip=true"],
            f"原始参数观察不符：{host}")
    cli = [json.loads(line) for line in observed["cli"]]
    require(cli == [
        {"argv": ["run", "probe.xiao"], "kind": "run",
         "publicKeys": ["file", "kind", "optimizationExplicit", "optimizationLevel", "options"]},
        {"argv": ["run", "probe.xiao", "--", "", "two words"], "code": "X11-CLI-ARG-001"},
        {"argv": ["run", "probe.xiao", "--", "--help"], "kind": "help", "publicKeys": ["kind", "options"]},
        {"argv": ["-xar", "probe.xar", "--", "--flag"], "code": "X11-CLI-ARG-001"},
    ], f"现有 CLI 分隔符观察不符：{host}")
    fs = observed["fs"]
    require(any(line.startswith("same_directory_replace=") for line in fs)
            and any(line.startswith("replace_directory=") for line in fs)
            and any(line.startswith("readonly_open_for_write=") for line in fs)
            and any(line.startswith("directory_link=") for line in fs), f"文件系统观察不完整：{host}")
    case = "true" if host == "windows" else "false"
    require(observed["env"] == [
        "child_with_values", f"value_present=true case_folded={case} empty_present=true absent=true malformed_roundtrip=true",
        "child_with_value_removed", f"value_present=false case_folded={case} empty_present=true absent=true malformed_roundtrip=true",
    ], f"隔离环境变量观察不符：{host}")
    log = observed["log"]
    require(len(log) == 3 and log[0].startswith("concurrent_stdout_records=48 exact=true stderr_separate=true ")
            and log[1].startswith("closed_stdout_write=") and log[2].startswith("closed_stream_child="),
            f"并发记录或关闭流观察不完整：{host}")


def inspect(root):
    product, inherited = fixed_baseline(root)
    path = root / "observations.zip"
    with ZipFile(path) as zipped:
        book = archive_tools.Evidence(zipped)
        run, jobs = ci(book, PROBE_RUN, ".github/workflows/b3-0-probe.yml")
        require(run["conclusion"] == "success" and jobs.keys() == set(PROBE_JOBS.values()), "三平台探测作业缺项")
        collateral, _ = ci(book, COLLATERAL_RUN, ".github/workflows/workspace-gates.yml")
        platforms = {"windows": inspect_host(root, lambda name: book.raw(f"windows/{name}"), product, "windows")}
        originals = {}
        for host, job_id in PROBE_JOBS.items():
            uploaded, receipt = remote(book, PROBE_RUN, jobs[job_id], host)
            with uploaded:
                platforms[host] = inspect_host(root, uploaded.read, product, host) | {"ci": receipt}
                folder = "macos-b1" if host == "macos" else "linux"
                for source in b7.guards.SOURCES.values():
                    if source.startswith(folder + "/"):
                        originals[source] = uploaded.read(source.split("/", 1)[1])
        guards = b7.guards.verify(GuardEvidence(zipped, originals))
        archive = base.identity(path) | {"paths": len(book.entries), "objects": book.unique}
    return {
        "product": product, "inherited": inherited, "platforms": platforms, "guards": guards, "archive": archive,
        "runner": RUNNER, "collateral_workspace": {"run": COLLATERAL_RUN, "conclusion": collateral["conclusion"]},
        "b3_0a_baseline_frozen": True, "b3_0_passed": False, "b3_1_entry_ready": False,
        "decisions_approved": False, "b3_product_started": False, "registry_verified": False, "old_721_executed": False,
    }


def inventory(root):
    return {item.relative_to(root).as_posix(): base.identity(item)
            for item in sorted(root.rglob("*")) if item.is_file() and item.name != "evidence.json"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, default=ROOT)
    options = parser.parse_args()
    root = options.evidence_root.resolve()
    try:
        files = inventory(root)
        expected = {"schema": 1, "files": files}
        require((root / "evidence.json").read_bytes() == archive_tools.json_bytes(expected)
                and ".gitattributes" in files, "清单字节或身份不符")
        summary = inspect(root)
        require((root / "entry.json").read_bytes() == archive_tools.json_bytes(summary), "入口回执与原始观察不符")
        print(archive_tools.json_bytes(summary | {"audit_files": len(files)}).decode(), end="")
    except (ValueError, OSError, KeyError, BadZipFile) as error:
        print(f"B3-0A 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
