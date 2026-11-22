"""独立核对 721 个 F10 记录与九组 F-3 样本。"""

from collections import Counter
from importlib.util import module_from_spec, spec_from_file_location
import json
import math
import re
from statistics import median

from measure import ROOT, REPO, base, B1_EXAMPLE, CATALOG, F3_EXAMPLE

F10 = {
    "linux-vm": ("linux", "vm", "linux-f10-vm"),
    "windows-native": ("windows", "native", "windows-f10-native"),
    "linux-native": ("linux", "native", "ci/45013071077/14750741956.zip"),
    "macos-native": ("macos", "native", "ci/45013071077/14751393156.zip"),
}
F3 = {
    "linux": {"vm": 14751101943, "o0": 14751146606, "o3": 14750658070},
    "windows": {"vm": 14751231849, "o0": 14751067163, "o3": 14751271895},
    "macos": {"vm": 14751114706, "o0": 14751315992, "o3": 14751695403},
}
FAILED = {
    14750202991: ("statistics.sum", 3, "f10", "native"),
    14750827874: ("lfloat.to-integer", 3, "f3", "vm"),
    14750878014: ("lint.power", 5, "f3", "o3"),
}
TARGETS = {host: target for host, target, _, _ in base.HOSTS.values()}
MIB64 = 64 * 1024 * 1024
PRODUCT = base.PRODUCT
CASES_FILE = "core/rust/crates/xiao-driver/examples/perf/cases.rs"
REQUIRE = base.require


def blob(path):
    return base.git("show", f"{PRODUCT}:{path}")


def normal(path):
    return path.replace("\\", "/").removeprefix("//?/")


def identity(raw):
    return {"bytes": len(raw), "sha256": base.sha(raw)}


def check_file(value):
    REQUIRE(value["bytes"] > 0 and re.fullmatch(r"[a-f0-9]{64}", value["sha256"]), "缺少实际文件摘要")


def audit_commands(raw, source, host, kind, backend, *, accepted=True):
    read = lambda key: json.loads(raw(key))
    env = read("environment.json")
    report = read("measurement.json")
    target = TARGETS[host]
    task = normal(env["task"])
    runtime = env["overrides"]["XIAO_RUNTIME_LIBRARY"]
    cargo = env["tools"]["cargo"]["path"]
    REQUIRE(env["source"] == source and env["host"] == host and env["target"] == target
            and env["kind"] == kind and env["backend"] == backend
            and env["rustc_verbose"].startswith("rustc 1.96.0 ")
            and f"host: {target}" in env["rustc_verbose"].splitlines()
            and normal(env["overrides"]["XIAO_STDLIB_ROOT"]) == task + "/source/stdlib"
            and normal(env["overrides"]["CARGO_TARGET_DIR"]) == task + "/target"
            and normal(runtime).startswith(task + "/target/release/"), "测量源码或工具链身份不符")
    check_file(env["recipe"])
    for value in env["tools"].values():
        check_file(value)
    rt = read("runtime.json")["runtime_after"]
    check_file(rt)
    REQUIRE(read("worker.json")["runtime_before"] == rt == read(f"{kind}-{backend}.json")["runtime_before"],
            "测量 Runtime 身份不符")
    worker = report["worker"]
    check_file(worker)
    measurement = f"{kind}-{backend}"
    tool = B1_EXAMPLE if kind == "f10" else F3_EXAMPLE
    output = "report.json" if kind == "f10" else "samples"
    selection = ["--case", "all", "--phase", "all"] if kind == "f10" else ["--suite", "all"]
    REQUIRE(normal(worker["path"]) == task + f"/target/release/examples/{tool}" + (".exe" if host == "windows" else ""),
            "测量工具目录不符")
    command = [cargo, "build", *base.LOCKED, "--release"]
    expected = {
        "runtime": [*command, "-p", "xiao-runtime"],
        "worker": [*command, "-p", "xiao-driver", "--example", tool],
        measurement: [worker["path"], "--backend", backend, *selection, "--output", task + f"/evidence/{output}"],
    }
    for name, argv in expected.items():
        row = read(f"{name}.json")
        REQUIRE([normal(item) for item in row["argv"]] == [normal(item) for item in argv]
                and normal(row["cwd"]) == task + "/source" and normal(row["temp"]) == task + "/tmp/" + name
                and row["source_before"] == row["source_after"] == source
                and row["runtime_before"] == (None if name == "runtime" else rt) and row["runtime_after"] == rt
                and type(row["pid"]) is int and row["pid"] > 0
                and row["started_utc"] <= row["finished_utc"]
                and row["exit"] == (0 if accepted or name != measurement else 1)
                and row["log"] == identity(raw(name + ".log")), f"测量命令或原始输出不符：{name}")
    REQUIRE(report["exit"] == (0 if accepted else 1), "退出回执与测量结果不符")
    return env, report, rt


def f10(raw, source, host, backend, *, accepted=True):
    env, observed, runtime = audit_commands(raw, source, host, "f10", backend, accepted=accepted)
    report = json.loads(raw("report.json"))
    rows = report["records"]
    REQUIRE(report["runtime_abi_encoded"] == 65555 and report["codegen_version"] == 22
            and report["backend"] == backend and report["target"] == TARGETS[host]
            and report["phase"] == report["selected"] == "all" and report["rust_debug_assertions"] is False
            and report["complete"] is accepted and report["failed"] is not accepted
            and observed["record_count"] == len(rows) and observed["complete"] is accepted, "F10 报告身份或完成状态不符")
    config = blob(CASES_FILE).decode("utf-8").split("const IDS: &[&str] = &[", 1)[1].split("];", 1)[0]
    ids = re.findall(r'"([a-z0-9._]+)"', config)
    levels = (0, 3) if backend == "native" else (None,)
    expected = [(case, level, scale) for case in ids for level in levels
                for scale in ((None,) if case.startswith("math.") or case == "fmt.format_int" else (None, 4096, 8192, 16384))]
    keys = [(row["case"], row["optimization"], row["scale"]) for row in rows]
    REQUIRE(len(ids) == 31 and len(set(ids)) == 31
            and (keys == expected if accepted else keys == expected[:len(keys)] and len(keys) < len(expected)),
            "F10 负载、规模、平台或优化级别集合不符")
    log = raw(f"f10-{backend}.log").decode("utf-8")
    REQUIRE([json.loads(line) for line in log.splitlines() if line.startswith('{"backend":')] == rows,
            "F10 报告与原始控制进程输出不符")
    product_digests = {module: base.sha(blob(f"stdlib/{module}.xiao")) for module in {row["module"] for row in rows}}
    before = {}
    failures = []
    artifact_root = observed["artifacts"]
    REQUIRE(len(re.findall(r"^B1-PERF-ARTIFACTS (.+)$", log, re.M)) == 1
            and normal(re.findall(r"^B1-PERF-ARTIFACTS (.+)$", log, re.M)[0]) == normal(artifact_root["directory"])
            and normal(artifact_root["directory"]).startswith(normal(env["task"]) + f"/tmp/f10-{backend}/"),
            "F10 独占产物根不符")
    for row, key in zip(rows, keys, strict=True):
        case, level, scale = key
        samples, repeats = row["samples"], row["repeats"]
        calibration = row["calibration"]
        REQUIRE(row["product_sha256"] == product_digests[row["module"]] and row["backend"] == backend
                and row["rust_debug_assertions"] is False and len(samples) == 5 and type(repeats) is int
                and repeats > 0 and row["warmup"]["seconds"] > 0 and calibration[0]["repeats"] == 1
                and calibration[-1]["repeats"] == repeats and calibration[-1]["sample"]["seconds"] >= 0.1,
                f"F10 样本、黄金入口或校准身份不符：{key}")
        for previous, current in zip(calibration, calibration[1:]):
            REQUIRE(previous["sample"]["seconds"] < 0.1
                    and previous["repeats"] < current["repeats"] <= 16 * previous["repeats"], "F10 校准增长不符")
        times = [sample["seconds"] for sample in samples]
        REQUIRE(all(math.isfinite(value) and value > 0 for value in times), "F10 时长无效")
        peak = max(sample["resident_increment_upper_bytes"] for sample in samples)
        current = median(times) / repeats
        REQUIRE(math.isclose(row["median_seconds"], current, rel_tol=1e-12)
                and row["resident_increment_upper_bytes"] == peak and peak >= 0, "F10 中位数或 RSS 重算不符")
        ratio = None if scale in (None, 4096) else current / before[case, level]
        got_ratio = row["doubling_ratio"]
        REQUIRE((ratio is None and got_ratio is None) or
                (ratio is not None and math.isclose(got_ratio, ratio, rel_tol=1e-12)), "F10 翻倍比重算不符")
        if scale is not None:
            before[case, level] = current
        pass_row = min(times) >= 0.05 and current <= (2 if backend == "native" else 10) and peak <= MIB64
        pass_row &= ratio is None or ratio <= 3
        REQUIRE(row["batch_duration_passed"] == (min(times) >= 0.05)
                and row["passed"] is pass_row, "F10 阈值结论不符")
        if not pass_row:
            failures.append({"case": case, "optimization": level, "scale": scale, "ratio": ratio, "limit": 3})
        if backend == "native":
            stem = f"{case}-o{level}-{row['input_bytes_or_items']}-{row['output_bytes_or_items']}"
            files = artifact_root["files"]
            check_file(files[stem + ".ll"])
            check_file(files[stem + (".exe" if host == "windows" else "")])
    REQUIRE(bool(failures) is not accepted, "F10 实测失败集合与报告不符")
    if accepted:
        maximum = [(row["input_bytes_or_items"], row["output_bytes_or_items"])
                   for row in rows if row["case"] == "url.decode_unicode" and row["scale"] is None]
        REQUIRE(maximum == [(196608, 65536)] * len(levels),
                "F10 URL 全转义负载不符")
    return {"records": len(rows), "passed": accepted, "runtime": runtime, "failures": failures,
            "max_ratio": max((row["doubling_ratio"] or 0 for row in rows)),
            "max_seconds": max(row["median_seconds"] for row in rows),
            "max_peak_bytes": max(row["resident_increment_upper_bytes"] for row in rows)}


def f3(raw, source, host, backend, *, accepted=True):
    env, observed, runtime = audit_commands(raw, source, host, "f3", backend, accepted=accepted)
    old = REPO / "docs/DevDocs/assets/20o-b2-b/entry-20261118-b6-2/audit.py"
    spec = spec_from_file_location("b6_sampler_audit", old)
    audit = module_from_spec(spec)
    spec.loader.exec_module(audit)
    read = lambda name: json.loads(raw("samples/" + name))
    catalog_raw = blob(CATALOG)
    expected = json.loads(catalog_raw)["loads"]
    details = read("run.json")
    report = read("result.json")
    rows = report["records"]
    REQUIRE(len(expected) == len(rows) == observed["record_count"] == 183 and observed["complete"] is True
            and [row["load"] for row in rows] == details["expected"] == expected and details["suite"] == "all"
            and details["batches"] == 5 and details["calibration_limit"] == 2 ** 24
            and details["abi"] == 65555 and details["codegen"] == 22 and details["backend"] == backend
            and details["catalog_sha256"] == base.sha(catalog_raw) and report["complete"] is True
            and report["passed"] is accepted, "F-3 完整负载或报告身份不符")
    worker = observed["worker"]
    prior, processes = {}, set()
    root = normal(env["task"]) + "/evidence/samples"
    for index, row in enumerate(rows):
        label = f"{index:03}"
        REQUIRE(read(f"{label}/result.json") == row, "F-3 原始负载报告与汇总不符")
        audit.verify_row(row, f"samples/{label}", f"{root}/{label}", worker, backend, host, raw, prior, processes)
    counts = Counter(row["status"] for row in rows)
    REQUIRE((counts == {"passed": 183}) is accepted and len(processes) == 915, "F-3 独立进程或阈值状态不符")
    failures = [{"case": row["load"]["case"], "profile": row["load"]["profile"],
                 "ratio": row.get("doubling_ratio"), "limit": row["load"]["ratio_limit"]}
                for row in rows if row["status"] != "passed"]
    return {"records": len(rows), "processes": len(processes), "passed": accepted, "runtime": runtime,
            "failures": failures}
