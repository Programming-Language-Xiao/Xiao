"""从 B2-C 原始回执派生 C0 至 C3 状态。"""

import argparse
from importlib.util import module_from_spec, spec_from_file_location
from io import BytesIO
import json
from pathlib import Path
import posixpath
import sys
import tomllib
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True
from performance_audit import FAILED, F10, F3, TARGETS, f10, f3, identity, normal
from measure import ROOT, REPO, base

ASSETS = "docs/DevDocs/assets"
B7 = f"{ASSETS}/20o-b2-b/entry-20261122-b7"
F7 = f"{ASSETS}/20o-b2-freeze-c/entry-20261105"
PREFIX = ROOT.relative_to(REPO).as_posix()
ANCHOR = "01107ea4df92108014eac4f3487292d8672a445a"
PERF_RUNNER = "6ca7cd28fcfcd5aae14db9fbf5f11a1dff2724e0"
CANDIDATE_RUNNER = "7077b392d87b4b4204252b5c8104a7e13b7bf544"
PERF_RUN = 45013071077
LANGUAGE_RUN = 45016234630
PRODUCT = base.PRODUCT
FIXTURES = ("cmath-library", "b2_explicit_divide", "a5_decimal_arithmetic", "a5_conversions", "cmath-tutorial")
ACCEPTED = {
    14750741956: (137179416348, 1), 14751393156: (137183764509, 2),
    14750658070: (137179416358, 1), 14751067163: (137179416571, 1),
    14751101943: (137179416168, 1), 14751114706: (137183764500, 2),
    14751146606: (137179416448, 1), 14751231849: (137179416306, 1),
    14751271895: (137179416432, 1), 14751315992: (137179416445, 1),
    14751695403: (137183764528, 2),
    14751618515: (137189023226, 1), 14752047523: (137189022992, 1),
    14752137593: (137189023197, 1), 14752207360: (137189023195, 1),
}
FIRST_FAILED = {14750202991: 137179416365, 14750827874: 137179416382, 14750878014: 137179416403}
STAGES = ("c0_snapshot_frozen", "c1_721_passed", "c2_language_archive_passed", "c3_platform_version_passed",
          "c4_registry_tutorial_passed", "c5_closeout_audit_passed")


def load_module(name, path):
    spec = spec_from_file_location(name, path)
    module = module_from_spec(spec)
    previous = sys.path[:]
    previous_audit = sys.modules.pop("audit", None)
    sys.path.insert(0, str(path.parent))
    try:
        spec.loader.exec_module(module)
    finally:
        sys.path[:] = previous
        if previous_audit is not None:
            sys.modules["audit"] = previous_audit
    return module


def blob(commit, path):
    return base.git("show", f"{commit}:{path}")


def derive(flags):
    base.require(all(type(flags[key]) is bool for key in (*STAGES, "b2c_entry_ready")), "阶段状态必须为布尔值")
    return {"b2c_passed": flags["b2c_entry_ready"] and all(flags[key] for key in STAGES)}


class Evidence:
    def __init__(self, archive):
        self.archive = archive
        self.entries = json.loads(archive.read("evidence.json"))
        objects = {"objects/" + item["sha256"] for item in self.entries.values()}
        base.require(set(archive.namelist()) == objects | {"evidence.json"}
                     and len(archive.namelist()) == len(objects) + 1, "归档对象清单不符")
        for path in self.entries:
            self.raw(path)
        self.unique = len(objects)

    def raw(self, name):
        base.require(name in self.entries, f"归档证据缺失：{name}")
        item = self.entries[name]
        raw = self.archive.read("objects/" + item["sha256"])
        base.require(item == identity(raw), f"证据字节改变：{name}")
        return raw

    def read(self, name):
        return json.loads(self.raw(name))

    def upload(self, run_id, artifact, *, accepted=True):
        if accepted:
            job_id, attempt = ACCEPTED[artifact]
        else:
            job_id, attempt = FIRST_FAILED[artifact], 1
        folder = f"ci/{run_id}"
        run = self.read(f"{folder}/run-{attempt}.json")
        jobs = self.read(f"{folder}/jobs-{attempt}.json")["jobs"]
        job, = [row for row in jobs if row["id"] == job_id]
        api, = [row for row in self.read(f"{folder}/artifacts-{attempt}.json")["artifacts"] if row["id"] == artifact]
        archive = self.raw(f"{folder}/{artifact}.zip")
        sha = base.sha(archive)
        commit = PERF_RUNNER if run_id == PERF_RUN else CANDIDATE_RUNNER
        expected_conclusion = "success" if accepted else "failure"
        base.require(run["head_sha"] == job["head_sha"] == commit and run["id"] == job["run_id"] == run_id
                     and run["run_attempt"] == job["run_attempt"] == attempt and job["conclusion"] == expected_conclusion
                     and run["run_started_at"] <= job["started_at"] < job["completed_at"]
                     and api["digest"] == "sha256:" + sha and api["size_in_bytes"] == len(archive)
                     and api["name"].endswith(f"-{run_id}-{attempt}"), f"CI 作业或上传原件身份不符：{job_id}")
        log = self.raw(f"{folder}/{job_id}.log")
        base.require(str(artifact).encode() in log and sha.encode() in log and commit.encode() in log,
                     f"实际作业日志与上传原件未绑定：{job_id}")
        info = {"run": run_id, "attempt": attempt, "job": job_id, "artifact": artifact, "sha256": sha,
                "conclusion": expected_conclusion}
        return ZipFile(BytesIO(archive)), info


def source_and_dependencies():
    manifest = base.git("ls-tree", "-rz", PRODUCT, "--", *base.PRODUCT_ROOTS)
    source = {"commit": PRODUCT, "tree": base.TREE, "files": len(base.tracked(manifest)),
              "tree_list_sha256": base.sha(manifest),
              "locks": {name: identity(blob(PRODUCT, name)) for name in base.LOCKS}}
    base.require(base.git("cat-file", "-p", PRODUCT).splitlines()[0] == f"tree {base.TREE}".encode()
                 and source["files"] == 1222, "B7 固定产品树不符")
    for revision, allowed in ((PERF_RUNNER, {".github/workflows/b2-c-measurement.yml", f"{PREFIX}/measure.py"}),
                              (CANDIDATE_RUNNER, {".github/workflows/b2-c-candidates.yml", f"{PREFIX}/candidate.py",
                                                  f"{PREFIX}/cmath-tutorial.xiao", f"{PREFIX}/cmath-tutorial.stdout"})):
        prior = ANCHOR if revision == PERF_RUNNER else PERF_RUNNER
        changed = set(base.git("diff", "--name-only", prior, revision).decode().splitlines())
        base.require(changed == allowed, "取证分支差异超出工作流与证据脚本")
    b7 = json.loads(blob(ANCHOR, f"{B7}/closeout.json"))
    base.require(b7["b7_passed"] is b7["b2c_entry_ready"] is True and b7["product"] == source
                 and b7["inherited"]["b6_accepted"] is False and b7["inherited"]["22d_open"] is True,
                 "B7 入口或历史债务状态不符")
    locks = tomllib.loads(blob(PRODUCT, base.LOCKS[0]).decode("utf-8"))["package"]
    freeze = json.loads(blob(ANCHOR, f"{F7}/dependency-review.json"))
    dependencies = {}
    licenses = 0
    for dependency in freeze["packages"]:
        name, version = dependency["name"], dependency["version"]
        actual, = [item for item in locks if item["name"] == name]
        base.require((actual["version"], actual["checksum"]) == (version, dependency["archive_sha256"]),
                     f"F-7 锁定归档不符：{name}")
        for license in dependency["licenses"]:
            path = posixpath.normpath(f"{F7}/{license['file']}")
            base.require(base.sha(blob(ANCHOR, path)) == license["sha256"], f"F-7 许可原文不符：{name}")
            licenses += 1
        dependencies[name] = {"version": version, "sha256": actual["checksum"]}
    base.require(len(dependencies) == 6 and licenses == 11, "F-7 冻结包或许可数量不符")
    return source, b7["inherited"], {"packages": dependencies, "license_files": licenses}


def samples(book, source):
    f10_reports = {}
    for label, (host, backend, path) in F10.items():
        if path.endswith(".zip"):
            artifact = int(Path(path).stem)
            archive, ci = book.upload(PERF_RUN, artifact)
            with archive:
                f10_reports[label] = f10(archive.read, source, host, backend) | {"ci": ci}
        else:
            f10_reports[label] = f10(lambda key: book.raw(f"{path}/{key}"), source, host, backend)
    base.require(sum(row["records"] for row in f10_reports.values()) == 721, "F10 四份报告合计不是 721")
    f3_reports = {}
    for host, backends in F3.items():
        for backend, artifact in backends.items():
            archive, ci = book.upload(PERF_RUN, artifact)
            with archive:
                f3_reports[f"{host}-{backend}"] = f3(archive.read, source, host, backend) | {"ci": ci}
    failed = []
    for artifact, (case, limit, kind, backend) in FAILED.items():
        archive, ci = book.upload(PERF_RUN, artifact, accepted=False)
        with archive:
            inspect = f10 if kind == "f10" else f3
            result = inspect(archive.read, source, "macos", backend, accepted=False)
        failure, = result["failures"]
        base.require(failure["case"] == case and failure["limit"] == limit and failure["ratio"] > limit,
                     "首轮性能失败被掩盖或改写")
        failed.append(result | {"ci": ci, "kind": kind, "backend": backend})
    return f10_reports, f3_reports, failed


def audit_language(book, source):
    b7_audit = load_module("b7_audit", REPO / B7 / "audit.py")
    folders = {}
    for label, ident, modes, host in (
        ("linux", 14752137593, ("regression",), "linux"),
        ("macos-b1", 14752207360, ("b1",), "macos"),
        ("macos-numeric", 14752047523, ("numeric",), "macos"),
    ):
        uploaded, ci = book.upload(LANGUAGE_RUN, ident)
        with uploaded:
            class Upload:
                def raw(self, name):
                    return uploaded.read(name.split("/", 1)[1])

                def read(self, name):
                    return json.loads(self.raw(name))

            _, runtime, logs = b7_audit.observe(Upload(), label, modes, source, host)
            if "b1-vm" in logs:
                b7_audit.verify_b1(logs)
            if "numeric-vm" in logs:
                b7_audit.verify_numeric(logs, host)
        folders[label] = {"runtime": runtime, "ci": ci}
    _, runtime, logs = b7_audit.observe(book, "windows", ("regression",), source, "windows")
    b7_audit.verify_b1(logs)
    b7_audit.verify_numeric(logs, "windows")
    folders["windows"] = {"runtime": runtime}
    # 守卫只反查本批四份实际输出。Linux/macOS 来源随 CI 节点检查。
    class GuardView:
        def raw(self, name):
            source_file = {"linux/b1-vm.log": (14752137593, "b1-vm.log"),
                           "linux/b1-native.log": (14752137593, "b1-native.log"),
                           "macos-b1/b1-native.log": (14752207360, "b1-native.log")}.get(name)
            if source_file:
                ident, path = source_file
                with ZipFile(BytesIO(book.raw(f"ci/{LANGUAGE_RUN}/{ident}.zip"))) as archive:
                    return archive.read(path)
            return book.raw(name)

        def read(self, name):
            return json.loads(self.raw(name))

        def command(self, folder, stem, host):
            return book.command(folder, stem, host)

    return folders, b7_audit.guards.verify(GuardView())


def inspect(root, archive_file):
    source, inherited, dependencies = source_and_dependencies()
    with ZipFile(archive_file) as archive:
        book = Evidence(archive)
        measured, numeric, failed = samples(book, source)
        # 复用上一批的归档进程读取器来核对守卫的原始 stdin/stdout/stderr。
        b7 = load_module("b7_guard_reader", REPO / B7 / "audit.py")
        book.command = b7.Evidence(archive).command
        language, guards = audit_language(book, source)
        observed = audit_candidates(book, source)
        archive_id = identity(archive_file.read_bytes()) | {"paths": len(book.entries), "objects": book.unique}
    flags = {"b2c_entry_ready": True, "c0_snapshot_frozen": True, "c1_721_passed": True,
             "c2_language_archive_passed": True, "c3_platform_version_passed": True,
             "c4_registry_tutorial_passed": False, "c5_closeout_audit_passed": False}
    return flags | derive(flags) | {
        "b2c_started": True, "source": source, "inherited": inherited, "f10": measured, "f3": numeric,
        "first_attempt_failures": failed, "language": language, "guards": guards, "candidates": observed,
        "dependencies": dependencies, "archive": archive_id,
    }


def audit_candidates(book, source):
    from candidate import frames, output
    from coff import verify
    observed = {}
    base.require(all(f"{host}-candidates/environment.json" in book.entries for host in ("linux", "windows")),
                 "公开候选缺平台")
    archives = {"linux": lambda name: book.raw(f"linux-candidates/{name}"),
                "windows": lambda name: book.raw(f"windows-candidates/{name}")}
    upload, ci = book.upload(LANGUAGE_RUN, 14751618515)
    with upload:
        archives["macos"] = upload.read
        for host, raw in archives.items():
            env = json.loads(raw("environment.json"))
            build_runtime = json.loads(raw("runtime.json"))
            build_core = json.loads(raw("core.json"))
            runtime = build_runtime["runtime_after"]
            base.require(env["host"] == host and env["target"] == TARGETS[host] and env["source"] == source
                         and env["rustc_verbose"].startswith("rustc 1.96.0 ")
                         and f"host: {TARGETS[host]}" in env["rustc_verbose"].splitlines()
                         and build_runtime["exit"] == build_core["exit"] == 0
                         and build_runtime["source_before"] == build_runtime["source_after"] == source
                         and build_core["source_before"] == build_core["source_after"] == source
                         and build_core["runtime_before"] == build_core["runtime_after"] == runtime,
                         f"公开候选源码或 Runtime 身份不符：{host}")
            rows = {}
            for name in FIXTURES:
                attempt = "candidates" if host == "macos" else (
                    "candidates-tutorial-final" if name == "cmath-tutorial" else "candidates-selection")
                path = f"{attempt}/{name}"
                fixture = PREFIX if name == "cmath-tutorial" else "tests/fixtures/docs/numeric"
                revision = CANDIDATE_RUNNER if name == "cmath-tutorial" else PRODUCT
                code = blob(revision, f"{fixture}/{name}.xiao")
                golden = blob(revision, f"{fixture}/{name}.stdout").replace(b"\r\n", b"\n")
                row = json.loads(raw(f"{path}/result.json"))
                base.require(row["source_removed"] is True and row["source"] == identity(code)
                             and row["golden"] == identity(golden)
                             and row["xiaoc"] == identity(raw(f"{path}/program.xiaoc"))
                             and row["xar"] == identity(raw(f"{path}/program.xar")), "删源候选身份不符")
                for label, operation in (("vm", "run"), ("xiaoc-replay", "run_xiaoc"),
                                         ("archive-replay", "run_archive")):
                    handshake, reply = frames(candidate_output(raw, path, label))
                    base.require(handshake["versions"]["runtime_abi"] == 65555 and handshake["versions"]["codegen"] == 22
                                 and reply["operation"] == operation and reply["exit_code"] == 0
                                 and output(reply).encode() == golden,
                                 f"VM 或归档黄金不符：{host}/{name}/{label}")
                _, archive_reply = frames(candidate_output(raw, path, "archive"))
                archived = json.loads(archive_reply["value"]["value"])
                base.require(archive_reply["operation"] == "build_archive" and archive_reply["exit_code"] == 0
                             and archived["verified"] is True and archived["digest"] == row["xar"]["sha256"],
                             f"真实归档构建回执不符：{host}/{name}")
                if name.startswith("cmath-"):
                    expected_modules = ["stdlib.cmath"]
                    if name == "cmath-library":
                        expected_modules.extend(("stdlib.math", "stdlib.statistics"))
                    base.require(archived["stdlib_modules"] == expected_modules
                                 and all(not item["module"].startswith(("std.", "stdlib.")) for item in archived["modules"]),
                                 f"归档未使用固定 cmath 官方身份：{host}/{name}")
                base.require([entry["optimization"] for entry in row["native"]] == [0, 3], "候选缺 O0/O3")
                for entry in row["native"]:
                    level = entry["optimization"]
                    stem = f"native-o{level}"
                    suffix = ".exe" if host == "windows" else ""
                    base.require(entry["executable"] == identity(raw(f"{path}/{stem}{suffix}"))
                                 and entry["llvm"] == identity(raw(f"{path}/{stem}.ll"))
                                 and candidate_output(raw, path, stem) == golden
                                 and entry["verification"] == ("unverified-coff-exports" if host == "windows" else "complete"),
                                 f"实际原生候选字节、输出或验证等级不符：{host}/{name}/O{level}")
                    _, build = frames(candidate_output(raw, path, f"build-o{level}"))
                    verification = build["artifact"]["artifact_runtime"]
                    base.require(build["exit_code"] == 0 and build["artifact"]["optimization_level"] == level
                                 and build["artifact"]["object_digest"] == entry["executable"]["sha256"]
                                 and verification["verification"] == entry["verification"]
                                 and (host == "windows" or
                                      verification["declared_components"] == verification["observed_components"]),
                                 f"产品实际 Runtime 组件对账不符：{host}/{name}/O{level}")
                rows[name] = row
            observed[host] = {"target": env["target"], "runtime": runtime, "fixtures": len(rows), "native": 10}
    observed["macos"]["ci"] = ci
    coff = book.read("coff/result.json")["candidate_rows"]
    base.require(coff.keys() == {f"{name}-o{level}" for name in FIXTURES for level in (0, 3)},
                 "同链接 PE 候选集合不符")
    for name in FIXTURES:
        attempt = "candidates-tutorial-final" if name == "cmath-tutorial" else "candidates-selection"
        for level in (0, 3):
            folder = f"windows-candidates/{attempt}/{name}"
            result = verify(book.raw(f"{folder}/native-o{level}.exe"),
                            book.raw(f"{folder}/native-o{level}.exe.map"), book.read(f"{folder}/coff-o{level}.json"),
                            book.raw(f"coff/{name}-o{level}.readobj.log"), book.raw(f"{folder}/build-o{level}.stdout"))
            base.require(result == coff[f"{name}-o{level}"], "同链接 PE 原件与摘要不符")
    observed["windows"]["same_link_coff_rows"] = len(coff)
    observed["versions"] = audit_versions(book, source)
    return observed


def candidate_output(raw, folder, stem):
    prefix = f"{folder}/{stem}"
    receipt = json.loads(raw(f"{prefix}.json"))
    stdout, stderr = (raw(f"{prefix}.{name}") for name in ("stdout", "stderr"))
    base.require(receipt["exit"] == 0 and receipt["pid"] > 0 and not stderr
                 and receipt["stdout"] == identity(stdout) and receipt["stderr"] == identity(stderr),
                 f"候选实际进程输出不符：{prefix}")
    return stdout


def audit_versions(book, source):
    versions = book.read("versions/result.json")
    base.require(versions["source"] == source and
                 versions["runtime"] == book.read("windows-candidates/runtime.json")["runtime_after"],
                 "版本验证器的源码或 Runtime 不符")

    def command(stem):
        prefix = f"versions/{stem}"
        receipt = book.read(f"{prefix}.receipt.json")
        output = book.raw(f"{prefix}.stdout")
        base.require(receipt["exit"] == 0 and receipt["pid"] > 0 and
                     receipt["stdout_sha256"] == base.sha(output) and
                     receipt["stderr_sha256"] == base.sha(book.raw(f"{prefix}.stderr")),
                     f"版本命令的真实输出不符：{stem}")
        return json.loads(output)

    accepted = {}
    for kind in ("xiaoc", "xar", "meta"):
        accepted[f"current/{kind}"] = row = command(f"current-{kind}")
        base.require(row["accepted"] is True and row["input_unchanged"] is True
                     and row["input_sha256"] == base.sha(book.raw(f"versions/generated/main.{kind}")),
                     f"当前候选验证身份不符：{kind}")
        for side in ("b1", "b4"):
            old = blob(PRODUCT, f"tests/fixtures/compatibility/b5/{side}-main.{kind}")
            new = book.raw(f"versions/generated/main.{kind}")
            for direction, raw in ((f"{side}-to-current", old), (f"current-to-{side}", new)):
                row = command(f"{direction}-{kind}")
                base.require(row["accepted"] is row["executed"] is False and row["input_unchanged"] is True
                             and row["input_sha256"] == base.sha(raw), f"跨 ABI 执行前拒绝不符：{direction}/{kind}")
                accepted[f"{direction}/{kind}"] = row
    for direction, validator, generator, success in (("21-validates-22", 21, 22, False),
                                                       ("22-validates-21", 22, 21, True)):
        accepted[direction] = row = command(direction)
        base.require(row["accepted"] is success and row["entry_executed"] is False
                     and (row["actual_validator"], row["artifact_generator"]) == (validator, generator),
                     f"生成器验证方向不符：{direction}")
    base.require(accepted == versions["validators"], "版本原始验收记录与汇总不符")
    for mode in ("static", "dynamic"):
        for level in (0, 3):
            key = f"{mode}-o{level}"
            rows = [command(f"{key}-{phase}") for phase in ("old", "new", "hit")]
            base.require(rows == versions["cache"][key]
                         and [row["actual_codegen"] for row in rows] == [21, 22, 22]
                         and [row["cache_hit"] for row in rows] == [False, False, True]
                         and rows[0]["request_key"] != rows[1]["request_key"] == rows[2]["request_key"]
                         and rows[1]["sha256"] == rows[2]["sha256"], f"生成器缓存失配或复用不符：{key}")
    return {"validator_rows": len(accepted), "cache_rows": 12}


def json_bytes(doc):
    return (json.dumps(doc, indent=2, ensure_ascii=False, sort_keys=True) + "\n").encode()


def collect(output, groups):
    entries = {}
    objects = set()
    with ZipFile(output, "x", compression=ZIP_DEFLATED, compresslevel=6) as archive:
        for group, folder in groups:
            base.require(folder.is_dir(), f"证据目录缺失：{folder}")
            for item in sorted(folder.rglob("*")):
                if not item.is_file():
                    continue
                name = f"{group}/{item.relative_to(folder).as_posix()}"
                base.require(name not in entries, f"重复证据路径：{name}")
                data = item.read_bytes()
                row = identity(data)
                entries[name] = row
                if row["sha256"] not in objects:
                    archive.writestr("objects/" + row["sha256"], data)
                    objects.add(row["sha256"])
        archive.writestr("evidence.json", json_bytes(entries))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, default=ROOT / "execution.zip")
    parser.add_argument("--collect", nargs="*", metavar="GROUP=PATH")
    args = parser.parse_args()
    if args.collect:
        collect(args.archive, [(label, Path(item)) for label, item in (v.split("=", 1) for v in args.collect)])
    print(json_bytes(inspect(ROOT, args.archive)).decode(), end="")
