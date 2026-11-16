"""Recompute the diagnostic URL timing evidence without rerunning measurements."""
import argparse
from datetime import datetime
import gzip
import hashlib
import json
import math
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
PAST = REPOSITORY / "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild"
sys.path.insert(0, str(PAST))
import record
from instrument import TEST_PATH, identity, instrument

COMMITS = {
    "A": "cfa79381bbae027cec11136ca380e82f66720c34",
    "B": "5864cbde627f727483ab676c6f4bc1bf0a7819bb",
}
SCHEDULE = ["A", "B", "B", "A", "A", "B", "B", "A"]
PREFIX = ROOT.relative_to(REPOSITORY).as_posix()
MARKER = re.compile(
    "B5_URL_MEASURE_END elapsed_ns=([0-9]+) start_unix_ns=([0-9]+) "
    "end_unix_ns=([0-9]+) pid=([0-9]+) success=(true|false) code=(.*)")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True,
                       indent=2, allow_nan=False) + chr(10)).encode("utf-8")


def write_new(path, value):
    with path.open("xb") as output:
        output.write(json_bytes(value))


def inventory(root):
    result = {}
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(),
                "计时证据含链接")
        require("__pycache__" not in path.parts, "计时证据含Python缓存")
        if path.is_file() and path.relative_to(root).as_posix() != "evidence.json":
            raw = path.read_bytes()
            result[path.relative_to(root).as_posix()] = {"bytes": len(raw), "sha256": sha(raw)}
    return result


def tree(root):
    inventory(root)
    return sha(json_bytes({path.relative_to(root).as_posix(): sha(path.read_bytes())
                           for path in sorted(root.rglob("*")) if path.is_file()}))


def unpack(root, item):
    require(re.fullmatch("objects/sha256/[0-9a-f]{2}/[0-9a-f]{64}[.]bin", item["path"]),
            "计时对象路径不符")
    packed = (root / item["path"]).read_bytes()
    require(len(packed) == item["bytes"] and sha(packed) == item["sha256"]
            and Path(item["path"]).stem == sha(packed), "计时对象摘要不符")
    require(item["encoding"] in {"identity", "gzip"}, "计时对象编码不符")
    raw = gzip.decompress(packed) if item["encoding"] == "gzip" else packed
    require(len(raw) == item["raw_bytes"] and sha(raw) == item["raw_sha256"],
            "计时对象原始摘要不符")
    return raw


def bounds(rows, key):
    values = [row[key] for row in rows if row.get(key) is not None]
    return {"min": min(values), "max": max(values)} if values else None


def telemetry_window(root, receipt):
    raw = json.loads(unpack(root, receipt["telemetry"]))
    samples = raw["samples"]
    require(raw["sample_interval_seconds"] == 0.5 and len(samples) > 2,
            "计时连续采样不符")
    clocks = [row["observed_monotonic_ns"] for row in samples]
    require(clocks == sorted(set(clocks)), "采样时钟未严格递增")
    selected = [row for row in samples
                if row["observed_unix_ns"] >= receipt["driver_start_unix_ns"]
                and receipt["driver_end_unix_ns"] >= row["observed_unix_ns"]]
    require(len(selected) > 2, "URL窗口缺少连续采样")
    first, last = selected[0], selected[-1]
    wall = (last["observed_monotonic_ns"] - first["observed_monotonic_ns"]) / 1e9
    total = last["system_total_100ns"] - first["system_total_100ns"]
    busy = last["system_busy_100ns"] - first["system_busy_100ns"]
    cpu = (last["process_cpu_100ns"] - first["process_cpu_100ns"]) / 1e7
    require(wall > 0 and total > 0 and busy >= 0 and cpu >= 0,
            "URL窗口CPU计数不符")
    require(raw["close_status"] == {"pdh_close_status": 0, "process_handle_closed": True},
            "采样句柄收尾不符")
    gaps = [(b["observed_monotonic_ns"] - a["observed_monotonic_ns"]) / 1e9
            for a, b in zip(samples, samples[1:])]
    return {
        "sample_count": len(samples), "url_window_samples": len(selected),
        "actual_max_sample_gap_seconds": max(gaps),
        "system_cpu_percent": busy / total * 100,
        "whole_resource_process_equivalent_cores": cpu / wall,
        "available_physical_bytes": bounds(selected, "available_physical_bytes"),
        "working_set_bytes": bounds(selected, "working_set_bytes"),
        "private_bytes": bounds(selected, "private_bytes"),
        "processor_queue_length": bounds(selected, "processor_queue_length"),
        "system_context_switches_per_second": bounds(selected, "context_switches_per_second"),
        "scope": "system and whole 17-test process during URL observation window; not URL thread attribution",
    }


def check(root=ROOT, sealed=True):
    before = tree(root)
    if (root / "evidence.json").exists():
        require(load(root / "evidence.json") == {"files": inventory(root)}, "计时清单不符")
    elif sealed:
        raise ValueError("计时证据未封存")
    plan = load(root / "plan.json")
    timing = load(root / "timing.json")
    require(plan["commits"] == COMMITS and timing["plan"] == plan
            and plan["schedule"] == SCHEDULE, "计时计划不符")
    require(plan["timeout_seconds"] == 60 and plan["watchdog_changed"] is False
            and timing["budget_or_watchdog_changed"] is False, "计时预算被改动")
    require(timing["causal_attribution_proven"] is False and timing["b5_closed"] is False
            and timing["b6_started"] is False and timing["all_measurements_valid"] is True,
            "计时阶段状态不符")
    require(not (root / "collection-failure.json").exists(), "计时采集存在异常")
    require(set(plan["tools"]["files"]) == {"collect.py", "instrument.py", "winmetrics.py"},
            "计时工具集合不符")
    for name, digest in plan["tools"]["files"].items():
        require(name in {"collect.py", "instrument.py", "winmetrics.py"}
                and sha((root / name).read_bytes()) == digest
                and sha(record.git(REPOSITORY, "show",
                    plan["tools"]["commit"] + ":" + PREFIX + "/" + name)) == digest,
                "计时工具身份不符")
    sources, builds = {}, {}
    for label, commit in COMMITS.items():
        source = load(root / "sources" / (label + ".json"))
        original = record.git(REPOSITORY, "show", commit + ":" + TEST_PATH)
        patched = instrument(original)
        expected = record.fixed_files(REPOSITORY, commit)
        expected[TEST_PATH] = sha(patched)
        require(source["commit"] == commit and source["patch"] == identity(original)
                and source["identity"] == record.identity(source["files"])
                and {name: row["canonical_sha256"] for name, row in source["files"].items()} == expected
                and unpack(root, source["original_test"]) == original
                and unpack(root, source["diagnostic_test"]) == patched,
                "计时源码身份不符")
        build = load(root / "builds" / (label + ".json"))
        listing = load(root / "builds" / (label + "-list.json"))
        require(build["commit"] == commit and build["exit_code"] == 0
                and build["source_before"] == build["source_after"] == source["identity"]
                and build["runtime_objects"] and build["binary"]["sha256"]
                and sorted(listing["tests"]) == sorted(source["patch"]["tests"])
                and listing["exit_code"] == 0, "计时预编译不符")
        messages = []
        for line in unpack(root, build["log"]).decode("utf-8").splitlines():
            if line.startswith("{"):
                try:
                    messages.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
        require(any(row.get("reason") == "compiler-artifact"
                    and row.get("executable") == build["binary"]["path"] for row in messages),
                "计时二进制缺少真实构建来源")
        sources[label], builds[label] = source, build
    results = timing["results"]
    require(len(results) == 8 and [row["label"] for row in results] == SCHEDULE,
            "计时样本顺序或数量不符")
    rows, previous_end = [], None
    build_end = max(datetime.fromisoformat(row["finished_utc"]) for row in builds.values())
    for index, receipt in enumerate(results, 1):
        label = receipt["label"]
        source, build = sources[label], builds[label]
        require(load(root / "runs" / ("%02d-%s.json" % (index, label))) == receipt,
                "计时逐次回执与汇总不符")
        require(receipt["index"] == index and receipt["commit"] == COMMITS[label]
                and receipt["source_before"] == receipt["source_after"] == source["identity"]
                and receipt["binary"] == build["binary"]
                and receipt["argv"] == [build["binary"]["path"], "--nocapture", "--color", "never"]
                and receipt["environment"]["default_test_threads"] is True
                and receipt["active_build_processes_before"] == [],
                "计时运行身份不符")
        started = datetime.fromisoformat(receipt["started_utc"])
        ended = datetime.fromisoformat(receipt["finished_utc"])
        require(started > build_end and ended > started
                and (previous_end is None or started > previous_end), "计时运行与构建重叠")
        previous_end = ended
        log = unpack(root, receipt["log"]).decode("utf-8")
        ends = [match for line in log.splitlines() if (match := MARKER.search(line))]
        require(len(ends) == 1 and log.count("B5_URL_MEASURE_BEGIN pid=") == 1,
                "计时起止标记不唯一")
        marker = ends[0]
        elapsed, start_ns, end_ns, pid = map(int, marker.groups()[:4])
        success = marker[5] == "true"
        code = marker[6]
        censored = code == 'Some("X09-DRIVER-002")'
        require(elapsed == receipt["driver_elapsed_ns"] and elapsed > 0
                and start_ns == receipt["driver_start_unix_ns"] and start_ns > 0
                and end_ns == receipt["driver_end_unix_ns"] and end_ns >= start_ns
                and pid == receipt["pid"] == receipt["marker_pid"]
                and success == receipt["driver_success"]
                and code == receipt["driver_code_debug"]
                and censored == receipt["right_censored_at_original_timeout"]
                and receipt["measurement_valid"] is True, "计时单项结果与日志不符")
        summaries = re.findall(
            "test result: (ok|FAILED)[.] ([0-9]+) passed; ([0-9]+) failed; "
            "([0-9]+) ignored; ([0-9]+) measured; ([0-9]+) filtered out;", log)
        require(len(summaries) == 1, "资源目标汇总不唯一")
        counts = dict(zip(["passed", "failed", "ignored", "measured", "filtered"],
                          map(int, summaries[0][1:])))
        require(receipt["resource_target_counts"] == counts
                and counts["passed"] + counts["failed"] == 17
                and counts["ignored"] == counts["measured"] == counts["filtered"] == 0
                and receipt["resource_target_complete"] is True,
                "资源目标计数不符")
        require((success and not censored and code == "None" and receipt["exit_code"] == 0)
                or (not success and censored and receipt["exit_code"] == 101),
                "资源目标退出与URL结果不符")
        rows.append({
            "index": index, "snapshot": label, "driver_seconds": elapsed / 1e9,
            "completed": success, "right_censored": censored, "resource_counts": counts,
            "url_window_telemetry": telemetry_window(root, receipt),
        })
    groups = {}
    for label in COMMITS:
        selected = [row for row in rows if row["snapshot"] == label]
        completed = [row["driver_seconds"] for row in selected if row["completed"]]
        groups[label] = {
            "runs": len(selected), "completed": len(completed),
            "right_censored": sum(row["right_censored"] for row in selected),
            "observations": [{"seconds": row["driver_seconds"], "right_censored": row["right_censored"]}
                             for row in selected],
            "completed_only_min_seconds": min(completed) if completed else None,
            "completed_only_max_seconds": max(completed) if completed else None,
            "completed_only_median_seconds": statistics.median(completed) if completed else None,
            "warning": "completed-only statistics exclude censored results and are not an unbiased A/B comparison",
        }
    result = {
        "evidence_valid": True, "stage_passed": False, "blocked": True,
        "b5_closed": False, "b6_started": False, "causal_attribution_proven": False,
        "watchdog_changed": False, "scope": "instrumented default-concurrency resource target; not full workspace",
        "schedule": SCHEDULE, "groups": groups, "runs": rows,
        "conclusion": "both snapshots are close to the original 60-second deadline; no causal attribution",
    }
    if sealed:
        require(load(root / "summary.json") == result, "计时统计与原始数据不符")
        validate_seal(root)
    require(tree(root) == before, "计时审计改写证据")
    return result


GENERATED = {"evidence.json", "seal.json", "final-self-negative.json"}


def docs_snapshot(revision=None):
    if revision:
        names = record.git(REPOSITORY, "ls-tree", "-r", "--name-only", "-z",
                           revision, "--", "docs/DevDocs")
    else:
        names = record.git(REPOSITORY, "ls-files", "-z", "--cached", "--others",
                           "--exclude-standard", "--", "docs/DevDocs")
    paths = sorted({name.decode("utf-8") for name in names.split(bytes([0])) if name})
    paths = [path for path in paths if "/assets/" not in path or (
        path.startswith(PREFIX + "/")
        and path.removeprefix(PREFIX + "/") not in GENERATED
        and "/objects/" not in path and "/gates/" not in path)]
    if not revision:
        return {path: sha(record.canonical((REPOSITORY / path).read_bytes())) for path in paths}
    requests = b"".join((revision + ":" + path + chr(10)).encode("utf-8") for path in paths)
    stream = record.git(REPOSITORY, "cat-file", "--batch", data=requests)
    result, offset = {}, 0
    for path in paths:
        end = stream.index(bytes([10]), offset)
        header = stream[offset:end].split()
        require(header[1] == b"blob", "封存文档Git对象无效")
        length = int(header[2])
        result[path] = sha(record.canonical(stream[end + 1:end + length + 1]))
        offset = end + length + 2
    require(offset == len(stream), "封存文档Git流长度不符")
    return result


def validate_seal(root):
    seal = load(root / "seal.json")
    relative = seal["docs_receipt"]
    require(re.fullmatch("gates/[a-z0-9-]+[.]json", relative), "文档门禁回执路径不符")
    gate = load(root / relative)
    documents = json.loads(unpack(root, gate["documents"]))
    products = json.loads(unpack(root, gate["products"]))
    require(seal["product_commit"] == COMMITS["B"]
            and gate["argv"] == ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"]
            and gate["exit_code"] == 0 and gate["pid"] > 0
            and gate["docs_before"] == gate["docs_after"] == record.identity(documents)
            and gate["source_before"] == gate["source_after"] == record.identity(products)
            and gate["docs_unchanged"] is True and gate["source_unchanged"] is True
            and documents == docs_snapshot(gate["tools_commit"]),
            "最终文档门禁身份不符")
    require({name: item["canonical_sha256"] for name, item in products.items()}
            == record.fixed_files(REPOSITORY, COMMITS["B"]), "文档门禁产品快照不符")
    require(datetime.fromisoformat(gate["finished_utc"]) >
            datetime.fromisoformat(gate["started_utc"]), "文档门禁时间不符")
    require(gate["runner_sha256"] == sha((root / "seal.py").read_bytes()),
            "封存采集器身份不符")
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        relative_path = path.relative_to(root).as_posix()
        if (relative_path in GENERATED or relative_path.startswith("objects/")
                or relative_path.startswith("gates/")):
            continue
        require(sha(record.canonical(path.read_bytes())) ==
                documents.get(PREFIX + "/" + relative_path), "封存稳定输入偏离门禁")
    output = json.loads(unpack(root, gate["log"]))
    require(output["passed"] is True
            and not any(row["severity"] == "error" for row in output["diagnostics"]),
            "最终文档门禁未通过")
    require(seal["self_negative_receipt"] == "final-self-negative.json",
            "封存自反例引用不符")
    proof = load(root / "final-self-negative.json")
    reasons = {"baseline": "", "budget": "计时预算被改动",
               "source": "计时源码身份不符", "object": "计时对象摘要不符"}
    require(proof["passed"] is True and proof["audit_sha256"] == sha((root / "audit.py").read_bytes())
            and [row["case"] for row in proof["cases"]] == list(reasons),
            "最终计时反例集合或身份不符")
    for row in proof["cases"]:
        reason = reasons[row["case"]]
        require(row["exit_code"] == (1 if reason else 0)
                and row["stderr"].strip() == reason and row["expected_reason"] == reason
                and row["read_only"] is True and row["before_sha256"] == row["after_sha256"],
                "最终计时反例结果不符")


def negatives(root):
    reasons = {"baseline": "", "budget": "计时预算被改动", "source": "计时源码身份不符",
               "object": "计时对象摘要不符"}
    rows = []
    with tempfile.TemporaryDirectory(prefix="b5-url-timing-audit-") as temporary:
        for name, reason in reasons.items():
            copy = Path(temporary) / name
            shutil.copytree(root, copy)
            if name == "budget":
                value = load(copy / "timing.json")
                value["budget_or_watchdog_changed"] = True
                (copy / "timing.json").write_bytes(json_bytes(value))
            elif name == "source":
                value = load(copy / "sources/A.json")
                value["commit"] = COMMITS["B"]
                (copy / "sources/A.json").write_bytes(json_bytes(value))
            elif name == "object":
                value = load(copy / "sources/A.json")["original_test"]
                target = copy / value["path"]
                target.write_bytes(target.read_bytes() + b"tampered")
            if (copy / "evidence.json").exists():
                (copy / "evidence.json").write_bytes(json_bytes({"files": inventory(copy)}))
            before = tree(copy)
            process = subprocess.run(
                [sys.executable, "-B", "-X", "utf8", str(Path(__file__).resolve()),
                 "--root", str(copy), "--core"], capture_output=True, encoding="utf-8")
            after = tree(copy)
            require(process.returncode == (1 if reason else 0)
                    and process.stderr.strip() == reason and before == after,
                    "计时反例未按预期只读拒绝：" + name)
            rows.append({"case": name, "exit_code": process.returncode, "stderr": process.stderr,
                         "expected_reason": reason, "before_sha256": before,
                         "after_sha256": after, "read_only": True})
    return {"passed": True, "audit_sha256": sha(Path(__file__).read_bytes()), "cases": rows}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--core", action="store_true")
    parser.add_argument("--self-negative", action="store_true")
    parser.add_argument("--collect-summary", action="store_true")
    args = parser.parse_args()
    try:
        if args.collect_summary:
            value = check(args.root, sealed=False)
            write_new(args.root / "summary.json", value)
            write_new(args.root / "self-negative.json", negatives(args.root))
        else:
            value = negatives(args.root) if args.self_negative else check(args.root, sealed=not args.core)
        print(json.dumps(value, ensure_ascii=False, allow_nan=False))
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
