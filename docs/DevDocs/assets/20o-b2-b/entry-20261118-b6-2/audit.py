"""归档并只读复核 B6-2 的固定目录、进程记录及性能结论。"""

import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import statistics
import sys
from zipfile import ZIP_DEFLATED, ZipFile


ROOT = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT.parents[4] / "tools/gates"))

from b2_b6_macos import file_identity, git, require, snapshot, source_digest


CATALOG = "tests/fixtures/20o-b2-b6/catalog.json"
BASELINE_COMMIT = "659a023c544e6ea54ed547ee42c226b2c448a646"
BASELINE_PATH = "tools/gates/b2-b6-performance-baseline.json"
BACKENDS = ("vm", "o0", "o3")
MACOS_JOBS = {"vm": 134616425164, "o0": 134616425573, "o3": 134616425472}
POW_CALLS = ("std.cmath.pow", "stdlib.cmath.pow", "operator.cmath.pow")
POW_VECTORS = ("v16177", "v16181", "v16544")
UNITS = {"windows": ("working-set/peak-working-set", "bytes", 1),
         "linux": ("proc-vmrss-vmhwm", "KiB", 1024),
         "macos": ("resident-size/getrusage-ru-maxrss", "bytes", 1)}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def normalized_path(path):
    return path.replace(chr(92), "/").removeprefix("//?/")


def collect(archive, sources):
    files = {}
    objects = set()
    with ZipFile(archive, "x", compression=ZIP_DEFLATED, compresslevel=6) as out:
        for label, root in sources:
            require(root.is_dir(), f"证据目录不存在：{root}")
            for path in sorted(root.rglob("*")):
                if not path.is_file():
                    continue
                key = f"{label}/{path.relative_to(root).as_posix()}"
                require(key not in files, f"重复回执：{key}")
                data = path.read_bytes()
                digest = sha(data)
                files[key] = {"sha256": digest, "bytes": len(data)}
                if digest not in objects:
                    out.writestr("objects/" + digest, data)
                    objects.add(digest)
        out.writestr("evidence.json", json.dumps(files, sort_keys=True, indent=2) + "\n")


def number(actual, expected, name):
    require(math.isfinite(actual) and math.isclose(actual, expected, rel_tol=1e-12, abs_tol=1e-15),
            f"无法复算{name}")


def memory(value, os_name):
    if value is None:
        return None
    method, unit, factor = UNITS[os_name]
    require(value["method"] == method and value["unit"] == unit
            and 0 < value["current_raw"] <= value["peak_raw"]
            and value["current_raw"] * factor == value["current_bytes"]
            and value["peak_raw"] * factor == value["peak_bytes"], "驻留内存单位或读数不符")
    return value


def verify_batch(base, row, directory, batch, index, os_name, raw, processes):
    load = row["load"]
    label = f"{load['case']}-{load['profile']}-batch{index}"
    started = json.loads(raw(f"{base}/{label}.process.json"))
    key = (started["pid"], int(started["started_unix_nanoseconds"]))
    require(key not in processes and key == (batch["pid"], batch["started_unix_nanoseconds"])
            and started["case"] == load["case"] and started["profile"] == load["profile"]
            and started["batch"] == index, "批次 PID、时间或负载身份不符")
    processes.add(key)
    stdout_name = f"{label}.stdout.jsonl"
    stderr_name = f"{label}.stderr.log"
    actual_args = [normalized_path(arg) for arg in started["argv"]]
    expected_args = ["--worker-vm", f"{directory}/program.xiaoc"] if row["backend"] == "vm" else []
    require(normalized_path(started["executable"]) == normalized_path(row["executable"]["path"])
            and actual_args == expected_args
            and normalized_path(batch["stdout"]) == normalized_path(started["stdout"]) == f"{directory}/{stdout_name}"
            and normalized_path(batch["stderr"]) == normalized_path(started["stderr"]) == f"{directory}/{stderr_name}"
            and not raw(f"{base}/{stderr_name}"), "测量进程输出路径或错误输出不符")
    events = [json.loads(line) for line in raw(f"{base}/{stdout_name}").splitlines()]
    calibration = batch["calibration"]
    kinds = ["baseline", "validated", "ready"] + ["sample", "validated"] * len(calibration) + ["done"]
    require([event["event"] for event in events] == kinds
            and events[0]["pid"] == events[2]["pid"] == key[0] and len(calibration) > 0,
            "初始化、预热、黄金或正式样本事件不符")
    baseline = memory(batch["baseline"], os_name)
    require(events[0]["memory"] == baseline, "进程 RSS 基线不符")
    resident = [baseline]
    repeats = 1
    for i, sample in enumerate(calibration):
        observed = events[3 + 2 * i]
        duration = sample["seconds"]
        require(sample["repeats"] == repeats and 0 <= duration
                and observed["memory"] == sample["memory"], "校准次数、时长或内存读数不符")
        number(duration, observed["seconds"], "原始单调钟时长")
        resident.append(memory(sample["memory"], os_name))
        last = ((repeats == 1 and duration >= 0.05) or duration >= 0.1 or repeats == 2 ** 24)
        require(last == (i == len(calibration) - 1), "校准停止点不符")
        if not last:
            factor = 16 if duration <= 0.1 / 16 else max(2, min(16, math.ceil(0.1 / duration)))
            repeats = min(2 ** 24, repeats * factor)
    sample = calibration[-1]
    require(batch["sample"] == sample and batch["calibrated"] == (sample["seconds"] >= 0.05),
            "正式样本或校准状态不符")
    peak = (max(item["peak_bytes"] for item in resident) - baseline["current_bytes"]
            if all(item is not None for item in resident) else None)
    require(batch["resident_increment_upper_bytes"] == peak, "批次 RSS 增量不符")
    return sample["seconds"] / sample["repeats"], peak


def verify_row(row, base, directory, worker, backend, os_name, raw, prior, processes):
    load = row["load"]
    if "reason" in row:
        require(row["status"] == "failed" and row["reason"], "构建或采样错误被错误标记")
        return
    artifacts = {normalized_path(item["path"]): item["sha256"] for item in row["artifacts"]}
    executable = row["executable"]
    require(artifacts[f"{directory}/program.xiao"] == load["source_sha256"] and row["fingerprint"]
            and (artifacts.get(f"{directory}/program.xiaoc") == row["fingerprint"] and executable == worker
                 if backend == "vm" else artifacts.get(normalized_path(executable["path"])) == executable["sha256"]),
            "源程序或实际执行文件身份不符")
    batches = row["samples"]
    errors = row["batch_errors"]
    error_indices = {error["batch"] for error in errors}
    require(len(batches) + len(errors) == 5 and error_indices <= set(range(5)),
            "独立进程数量不符")
    values = [verify_batch(base, row, directory, batch, index, os_name, raw, processes)
              for index, batch in zip((i for i in range(5) if i not in error_indices), batches, strict=True)]
    seconds = sorted(value for value, _ in values)
    require(len(seconds) == len(row["normalized_seconds_sorted"]), "归一化样本数不符")
    for actual, expected in zip(row["normalized_seconds_sorted"], seconds, strict=True):
        number(actual, expected, "归一化时间")
    median = statistics.median(seconds) if len(seconds) == 5 else None
    if median is not None:
        number(row["median_seconds"], median, "中位数")
    else:
        require(row["median_seconds"] is None, "不完整批次包含中位数")
    peaks = [peak for _, peak in values]
    peak = max(peaks) if peaks and None not in peaks else None
    require(row["resident_increment_upper_bytes"] == peak, "五批 RSS 峰值不符")
    calibrated = len(batches) == 5 and all(batch["calibrated"] for batch in batches)
    limit = 10 if backend == "vm" else 2
    require(row.get("calibrated", False) == calibrated and row["seconds_limit"] == limit
            and row["resident_limit_bytes"] == 64 * 1024 * 1024, "预算或校准状态不符")
    failed = bool(errors) or median is not None and median > limit or peak is not None and peak > 64 * 1024 * 1024
    unverifiable = not calibrated or peak is None
    if load["double_of"] is not None:
        previous = prior.get((load["case"], load["double_of"]))
        if calibrated and previous is not None and previous > 0:
            ratio = median / previous
            number(row["doubling_ratio"], ratio, "翻倍比")
            failed |= ratio > load["ratio_limit"]
        else:
            require(row["doubling_ratio"] is None, "缺少合法翻倍比较样本")
            unverifiable = True
    if calibrated:
        prior[load["case"], load["profile"]] = median
    require(row["status"] == ("failed" if failed else "unverifiable" if unverifiable else "passed"),
            "负载结论与原始测量不符")


def compare_pow(records):
    rows = {(row["load"]["profile"], row["load"]["case"]): row for row in records
            if row["load"]["case"] in POW_CALLS}
    require(set(rows) == {(vector, call) for vector in POW_VECTORS for call in POW_CALLS},
            "cmath.pow 与浮点幂缺少同输入对照")
    return {vector: {call: {key: rows[vector, call].get(key)
                            for key in ("status", "median_seconds", "resident_increment_upper_bytes")}
                     for call in POW_CALLS}
            for vector in POW_VECTORS}


def audit(archive, allow_partial):
    with ZipFile(archive) as bundle:
        files = json.loads(bundle.read("evidence.json"))
        objects = {f"objects/{entry['sha256']}" for entry in files.values()}
        require(set(bundle.namelist()) == objects | {"evidence.json"}
                and len(bundle.namelist()) == len(objects) + 1, "证据包清单不符")

        def raw(name):
            entry = files[name]
            data = bundle.read("objects/" + entry["sha256"])
            require(sha(data) == entry["sha256"] and len(data) == entry["bytes"], f"证据字节改变：{name}")
            return data

        def read(name):
            return json.loads(raw(name))

        for name in files:
            raw(name)
        ci_manifest = "ci/workflow.json"
        if ci_manifest in files:
            remote = read(ci_manifest)
            jobs = {item["name"]: item for item in remote["jobs"]}
            artifacts = read("ci/artifact-api.json")["artifacts"]
            require(remote["databaseId"] == 44249501458 and remote["attempt"] == 1
                    and remote["headSha"] == "6eddee2d1b2677fac5d2665a15cb56166a638fd0"
                    and remote["status"] == "completed"
                    and {name: job["databaseId"] for name, job in jobs.items()} ==
                    {f"B6-2 macOS {backend}": job for backend, job in MACOS_JOBS.items()}
                    and {item["name"] for item in artifacts} ==
                    {f"b6-2-macos-{backend}-44249501458-1" for backend in BACKENDS},
                    "macOS Runner 的作业或产物身份不符")
        else:
            require(allow_partial, "macOS Runner 回执缺失")
        environments = [name.removesuffix("/environment.json") for name in files
                        if name.endswith("/environment.json")]
        coverage = {}
        process_count = 0
        fixed = json.loads(git("show", f"{BASELINE_COMMIT}:{BASELINE_PATH}"))
        commit = fixed["source_commit"]
        source = snapshot(commit)
        catalog_raw = git("show", f"{commit}:{CATALOG}")
        require(len(source) == fixed["files"] and source_digest(source) == fixed["sha256"]
                and sha(catalog_raw) == fixed["catalog_sha256"], "固定源码清单不符")
        catalog = json.loads(catalog_raw)
        for root in environments:
            environment = read(root + "/environment.json")
            source_root = normalized_path(environment["source_root"])
            task_root = normalized_path(environment["task_root"])
            require(environment["source"] == fixed and read(root + "/source.json") == source
                    and source_root == task_root + "/source"
                    and normalized_path(environment["stdlib"]) == source_root + "/stdlib",
                    "执行源码清单或标准库加载根不符")
            host = environment["platform"]
            suite = environment["suite"]
            run_summary = read(root + "/result.json") if root + "/result.json" in files else None
            require(run_summary is not None or allow_partial, "平台汇总回执缺失")
            loads = [load for load in catalog["loads"] if suite == "all" or load["suite"] == suite]
            for backend in environment["backends"]:
                base = f"{root}/{backend}"
                receipt = base + "/result.json"
                if receipt not in files:
                    coverage[base] = {"platform": host, "backend": backend, "complete": False}
                    continue
                run = read(base + "/run.json")
                result = read(receipt)
                identity = run["identity"]
                require(identity["target"] == environment["target"]
                        and normalized_path(identity["repository"]) == source_root
                        and normalized_path(identity["runtime"]["path"]) == normalized_path(environment["runtime"])
                        and all(source.get(name) == digest for name, digest in identity["files"].items())
                        and "core/rust/Cargo.lock" in identity["files"]
                        and "tests/benchmarks/Cargo.lock" in identity["files"]
                        and identity["files"][CATALOG] == fixed["catalog_sha256"]
                        and all(tool["file"]["sha256"] == environment["tools"][name]["sha256"]
                                for name, tool in identity["tools"].items()), "运行输入或工具身份不符")
                require(run["catalog_sha256"] == fixed["catalog_sha256"] and run["expected"] == loads
                        and run["abi"] == (1 << 16 | 19) and run["codegen"] == 22
                        and run["batches"] == 5 and run["calibration_limit"] == 2 ** 24
                        and run["backend"] == result["backend"] == backend
                        and result["complete"] and result["selected"] == len(loads)
                        and [row["load"] for row in result["records"]] == loads, "实际负载偏离固定清单")
                for label in ("runtime-build", "measure-build", backend):
                    cmd = read(f"{root}/{label}.json")
                    require(cmd["source_after_sha256"] == fixed["sha256"]
                            and cmd["log"]["sha256"] == files[f"{root}/{label}.log"]["sha256"]
                            and cmd["runtime_after"]["sha256"] == run["identity"]["runtime"]["sha256"]
                            and cmd["runtime_before"] == (None if label == "runtime-build" else cmd["runtime_after"])
                            and normalized_path(cmd["cwd"]) == source_root
                            and normalized_path(cmd["temp"]) == f"{task_root}/temp-{label}"
                            and cmd["exit_code"] == (1 if label == backend and not result["passed"] else 0),
                            "命令回执、Runtime 或日志不符")
                output = f"{task_root}/{backend}"
                require(normalized_path(cmd["argv"][0]) == normalized_path(identity["worker"]["path"])
                        and normalized_path(cmd["argv"][-1]) == output, "控制器或独占输出路径不符")
                prior, processes = {}, set()
                for index, row in enumerate(result["records"]):
                    cell = f"{base}/{index:03}"
                    require(read(cell + "/result.json") == row and row["backend"] == backend,
                            "测量单项与汇总回执不符")
                    verify_row(row, cell, f"{output}/{index:03}", identity["worker"],
                               backend, host, raw, prior, processes)
                counts = Counter(row["status"] for row in result["records"])
                require(result["passed"] == (counts["passed"] == len(loads)), "后端结论不符")
                if run_summary is not None:
                    summary = run_summary["backends"][backend]
                    require(summary["complete"] and summary["passed"] == result["passed"]
                            and summary["exit_code"] == cmd["exit_code"]
                            and all(summary["counts"][status] == counts[status]
                                    for status in ("passed", "failed", "unverifiable"))
                            and all(run_summary["worker"][key] == identity["worker"][key]
                                    for key in ("sha256", "bytes")), "平台汇总结论不符")
                if host == "macos" and ci_manifest in files:
                    job = jobs[f"B6-2 macOS {backend}"]
                    require(environment["github"]["RUN_ID"] == str(remote["databaseId"])
                            and environment["github"]["RUN_ATTEMPT"] == str(remote["attempt"])
                            and environment["runner_checkout"] == remote["headSha"]
                            and job["conclusion"] == ("success" if result["passed"] else "failure"),
                            "macOS 结果与 CI 作业结论不符")
                url_rows = [row for row in result["records"] if row["load"]["case"] == "b1.url.decode_unicode"]
                coverage[base] = {"platform": host, "backend": backend, "complete": True,
                                  "passed": result["passed"], "counts": dict(counts),
                                  "source_commit": commit, "full_catalog": suite == "all",
                                  "url": {row["load"]["profile"]: {
                                      "status": row["status"], "median_seconds": row.get("median_seconds"),
                                      "peak_bytes": row.get("resident_increment_upper_bytes"),
                                      "ratio": row.get("doubling_ratio"),
                                  } for row in url_rows}}
                if suite in ("all", "cmath"):
                    coverage[base]["pow"] = compare_pow(result["records"])
                process_count += len(processes)
        require(coverage, "证据包中没有测量")
        executed = [(row["platform"], row["backend"]) for row in coverage.values()
                    if row["complete"] and row.get("full_catalog")]
        required = {(host, backend) for host in UNITS for backend in BACKENDS}
        matrix_complete = set(executed) == required and len(executed) == len(coverage) == len(required)
        require(allow_partial or matrix_complete, "缺少或重复三平台、VM/O0/O3测量")
        return {"verified": True, "files": len(files), "unique_objects": len(objects),
                "successful_processes": process_count, "coverage": coverage,
                "matrix_complete": matrix_complete,
                "perf_pass": matrix_complete and all(row["passed"] for row in coverage.values()),
                "archive": file_identity(archive)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, default=ROOT / "measurements.zip")
    parser.add_argument("--collect", nargs="+", metavar="LABEL=DIR")
    parser.add_argument("--allow-partial", action="store_true")
    options = parser.parse_args()
    if options.collect:
        sources = [(label, Path(path)) for label, path in (entry.split("=", 1) for entry in options.collect)]
        collect(options.archive, sources)
    report = audit(options.archive, options.allow_partial)
    if options.archive.resolve() == ROOT / "measurements.zip" and not options.collect:
        require(report == json.loads((ROOT / "summary.json").read_bytes()), "归档摘要与复算不符")
    print(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
