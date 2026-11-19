"""Prepare fixed diagnostic binaries, then collect all eight default-resource runs."""
import argparse
import csv
from datetime import datetime, timezone
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import time

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
PAST = REPOSITORY / "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild"
sys.path.insert(0, str(PAST))
import record
from instrument import TEST_NAME, TEST_PATH, instrument, identity
from winmetrics import WindowsMetrics

TASK_BASE = Path("D:/project/Xiao/.b5/url-timing-20261116-01")
COMMITS = {
    "A": "cfa79381bbae027cec11136ca380e82f66720c34",
    "B": "5864cbde627f727483ab676c6f4bc1bf0a7819bb",
}
SCHEDULE = ["A", "B", "B", "A", "A", "B", "B", "A"]
SAMPLE_INTERVAL = 0.5
BUILD = [
    "cargo", "+1.96.0", "test", "--locked", "--manifest-path",
    "core/rust/Cargo.toml", "-p", "xiao-driver", "--test",
    "20o_b1_imp_resources", "--no-run", "--message-format=json",
]
PREFIX = ROOT.relative_to(REPOSITORY).as_posix()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def utc():
    return datetime.now(timezone.utc).isoformat()


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True,
                       allow_nan=False, indent=2) + chr(10)).encode("utf-8")


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as output:
        output.write(json_bytes(value))


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == packed, "内容对象损坏")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": len(packed), "sha256": digest,
            "encoding": "gzip" if len(raw) > 8192 else "identity",
            "raw_bytes": len(raw), "raw_sha256": sha(raw)}


def artifact(path):
    raw = path.read_bytes()
    return {"path": str(path), "bytes": len(raw), "sha256": sha(raw)}


def tool_identity():
    commit = record.git(REPOSITORY, "rev-parse", "HEAD").decode().strip()
    files = {}
    for name in ["collect.py", "instrument.py", "winmetrics.py"]:
        raw = (ROOT / name).read_bytes()
        require(raw == record.git(REPOSITORY, "show", commit + ":" + PREFIX + "/" + name),
                "采集工具须先提交定稿：" + name)
        files[name] = sha(raw)
    return {"commit": commit, "files": files}


def environment(label, run_directory):
    task = TASK_BASE / label
    source = task / "source"
    removed = []
    inherited = {}
    for key, value in os.environ.items():
        upper = key.upper()
        excluded = upper.startswith(("XIAO_", "RUST_BIGDECIMAL_", "RUST_TEST_",
                                     "CARGO_PROFILE_", "CARGO_TARGET_"))
        excluded = excluded or upper in {
            "RUSTC_BOOTSTRAP", "RUSTFLAGS", "RUSTDOCFLAGS", "CARGO_ENCODED_RUSTFLAGS",
            "RUSTC", "RUSTDOC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
            "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTFLAGS", "CARGO_BUILD_RUSTC",
        }
        if excluded:
            removed.append(key)
        else:
            inherited[key] = value
    overrides = {
        "RUSTUP_TOOLCHAIN": "1.96.0",
        "CARGO_TARGET_DIR": str(task / "target"),
        "CARGO_BUILD_JOBS": "4",
        "XIAO_STDLIB_ROOT": str(source / "stdlib"),
        "XIAO_HOME": str(run_directory / "home"),
        "TEMP": str(run_directory / "temp"),
        "TMP": str(run_directory / "temp"),
        "XIAO_B1_F10": "0",
        "XIAO_B2_A_PERF": "0",
        "PYTHONUTF8": "1",
        "PYTHONIOENCODING": "utf-8",
        "CARGO_TERM_COLOR": "never",
    }
    for name in ["home", "temp"]:
        (run_directory / name).mkdir(parents=True, exist_ok=False)
    inherited.update(overrides)
    return inherited, {"overrides": overrides, "removed_names": sorted(removed),
                       "default_test_threads": True}


def source_identity(label):
    return record.worktree_files(TASK_BASE / label / "source")


def ensure_source(label, expected):
    actual = source_identity(label)
    require(actual == expected, "诊断源码发生变化：" + label)
    return record.identity(actual)


def no_other_build():
    result = subprocess.run(["tasklist", "/FO", "CSV", "/NH"], check=True,
                            capture_output=True, encoding="mbcs")
    active = [row[0] for row in csv.reader(result.stdout.splitlines())
              if row and row[0].lower() in {"cargo.exe", "rustc.exe", "xiao-driver.exe"}]
    require(not active, "存在其它编译或驱动作业：" + ", ".join(active))
    return active


def prepare():
    require(not (ROOT / "plan.json").exists(), "准备回执不可覆盖")
    require(TASK_BASE.is_absolute() and not TASK_BASE.exists(), "任务根必须全新")
    require(TASK_BASE.parent.resolve() == Path("D:/project/Xiao/.b5").resolve(),
            "任务根超出指定目录")
    tools = tool_identity()
    no_other_build()
    compiler_tools = {}
    for name, flag in [("rustc", "-Vv"), ("cargo", "-V")]:
        located = subprocess.run(["rustup", "which", "--toolchain", "1.96.0", name],
                                 capture_output=True, encoding="utf-8", check=True)
        binary = Path(located.stdout.strip())
        version = subprocess.run([str(binary), flag], capture_output=True,
                                 encoding="utf-8", check=True)
        compiler_tools[name] = {
            "binary": artifact(binary), "version_argv": [str(binary), flag],
            "version_stdout": version.stdout, "version_stderr": version.stderr,
        }
    TASK_BASE.mkdir(parents=True)
    write_json(ROOT / "plan.json", {
        "commits": COMMITS, "schedule": SCHEDULE, "tools": tools,
        "task_root": str(TASK_BASE), "sample_interval_seconds": SAMPLE_INTERVAL,
        "method": "instrumented resource target; original 17 tests and default concurrency",
        "timeout_seconds": 60, "watchdog_changed": False,
        "created_utc": utc(), "compiler_tools": compiler_tools,
        "host": {"platform": platform.platform(), "processor": platform.processor(),
                 "logical_cpu_count": os.cpu_count(), "python": platform.python_version()},
    })
    for label, commit in COMMITS.items():
        task = TASK_BASE / label
        task.mkdir()
        source = task / "source"
        record.git(REPOSITORY, "worktree", "add", "--detach", str(source), commit)
        original_files = source_identity(label)
        fixed = record.fixed_files(REPOSITORY, commit)
        require({name: row["canonical_sha256"] for name, row in original_files.items()} == fixed,
                "工作副本偏离固定Git产品树")
        original = record.git(REPOSITORY, "show", commit + ":" + TEST_PATH)
        patched = instrument(original)
        (source / TEST_PATH).write_bytes(patched)
        diagnostic_files = source_identity(label)
        expected = dict(fixed)
        expected[TEST_PATH] = sha(patched)
        require({name: row["canonical_sha256"] for name, row in diagnostic_files.items()} == expected,
                "诊断副本存在额外产品改动")
        write_json(ROOT / "sources" / (label + ".json"), {
            "commit": commit, "files": diagnostic_files,
            "identity": record.identity(diagnostic_files), "patch": identity(original),
            "original_test": store(original), "diagnostic_test": store(patched),
        })
        env, env_record = environment(label, task / "build")
        stdout_path = task / "build" / "cargo.log"
        started = utc()
        print(json.dumps({"phase": "build_started", "label": label, "commit": commit}), flush=True)
        with stdout_path.open("xb") as output:
            child = subprocess.Popen(BUILD, cwd=source, env=env,
                                     stdout=output, stderr=subprocess.STDOUT)
            status = child.wait()
        raw = stdout_path.read_bytes()
        messages = []
        for line in raw.decode("utf-8").splitlines():
            if line.startswith("{"):
                try:
                    messages.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
        binaries = [message["executable"] for message in messages
                    if message.get("reason") == "compiler-artifact"
                    and message["target"]["name"] == "20o_b1_imp_resources"
                    and message.get("executable")]
        runtime_paths = [Path(name) for message in messages
                         if message.get("reason") == "compiler-artifact"
                         and message["target"]["name"] == "xiao_runtime"
                         for name in message["filenames"] if name.endswith((".rlib", ".lib"))]
        receipt = {
            "label": label, "commit": commit, "argv": BUILD, "pid": child.pid,
            "cwd": str(source), "started_utc": started, "finished_utc": utc(),
            "exit_code": status, "environment": env_record, "log": store(raw),
            "source_before": record.identity(diagnostic_files),
            "source_after": ensure_source(label, diagnostic_files),
            "binary": artifact(Path(binaries[0])) if len(binaries) == 1 else None,
            "runtime_objects": [artifact(path) for path in runtime_paths],
        }
        write_json(ROOT / "builds" / (label + ".json"), receipt)
        require(status == 0 and len(binaries) == 1 and runtime_paths,
                "预编译未生成唯一资源测试及Runtime对象")
        binary = Path(binaries[0])
        require(binary.resolve().is_relative_to((task / "target").resolve()),
                "测试二进制不在独立target内")
        listed = subprocess.run([str(binary), "--list"], cwd=source, env=env,
                                capture_output=True, check=True)
        names = [line.removesuffix(": test") for line in listed.stdout.decode("utf-8").splitlines()
                 if line.endswith(": test")]
        require(sorted(names) == sorted(identity(original)["tests"]), "编译后测试集合不符")
        write_json(ROOT / "builds" / (label + "-list.json"), {
            "argv": [str(binary), "--list"], "exit_code": listed.returncode,
            "stdout": store(listed.stdout), "stderr": store(listed.stderr), "tests": names,
        })
        print(json.dumps({"phase": "build_finished", "label": label, "exit_code": status}), flush=True)


def observe_markers(raw_path, seen):
    raw = raw_path.read_bytes()
    for line in raw.splitlines():
        for marker in ["B5_URL_MEASURE_BEGIN", "B5_URL_MEASURE_END"]:
            if marker.encode() in line and marker not in seen:
                seen[marker] = {"observed_monotonic_ns": time.perf_counter_ns(),
                                "line": line.decode("utf-8")}


def run_one(index, label):
    source_record = load(ROOT / "sources" / (label + ".json"))
    build = load(ROOT / "builds" / (label + ".json"))
    before = ensure_source(label, source_record["files"])
    binary = Path(build["binary"]["path"])
    require(artifact(binary) == build["binary"], "诊断测试二进制身份改变")
    for runtime in build["runtime_objects"]:
        require(artifact(Path(runtime["path"])) == runtime, "调试Runtime对象身份改变")
    active = no_other_build()
    task = TASK_BASE / label
    run_directory = task / "runs" / ("%02d" % index)
    env, env_record = environment(label, run_directory)
    argv = [str(binary), "--nocapture", "--color", "never"]
    raw_path = run_directory / "output.log"
    samples, markers = [], {}
    started, start_clock = utc(), time.perf_counter_ns()
    print(json.dumps({"phase": "run_started", "index": index, "label": label}), flush=True)
    with raw_path.open("xb") as output:
        child = subprocess.Popen(argv, cwd=task / "source", env=env,
                                 stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT)
        metrics = None
        try:
            metrics = WindowsMetrics(child.pid)
            while True:
                samples.append(metrics.sample())
                observe_markers(raw_path, markers)
                try:
                    status = child.wait(timeout=SAMPLE_INTERVAL)
                    break
                except subprocess.TimeoutExpired:
                    pass
            samples.append(metrics.sample())
            observe_markers(raw_path, markers)
        except Exception:
            if child.poll() is None:
                child.terminate()
                child.wait()
            raise
        finally:
            if metrics:
                close_status = metrics.close()
    end_clock, finished = time.perf_counter_ns(), utc()
    raw = raw_path.read_bytes()
    text = raw.decode("utf-8")
    end_lines = [line for line in text.splitlines() if "B5_URL_MEASURE_END " in line]
    matches = [re.search(
        "B5_URL_MEASURE_END elapsed_ns=([0-9]+) start_unix_ns=([0-9]+) end_unix_ns=([0-9]+) pid=([0-9]+) success=(true|false) code=(.*)", line)
        for line in end_lines]
    valid_marker = len(matches) == 1 and matches[0] is not None
    elapsed_ns = int(matches[0][1]) if valid_marker else None
    driver_start = int(matches[0][2]) if valid_marker else None
    driver_end = int(matches[0][3]) if valid_marker else None
    marker_pid = int(matches[0][4]) if valid_marker else None
    success = matches[0][5] == "true" if valid_marker else None
    code = matches[0][6] if valid_marker else None
    summaries = re.findall(
        "test result: (ok|FAILED)[.] ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored; ([0-9]+) measured; ([0-9]+) filtered out;",
        text)
    counts = None
    if len(summaries) == 1:
        counts = dict(zip(["passed", "failed", "ignored", "measured", "filtered"],
                          map(int, summaries[0][1:])))
    target_complete = bool(counts and counts["passed"] + counts["failed"] == 17
                           and counts["ignored"] == counts["measured"] == counts["filtered"] == 0)
    valid_timing = bool(valid_marker and len(markers) == 2 and marker_pid == child.pid
                        and driver_start > 0 and driver_end >= driver_start and elapsed_ns > 0)
    require(artifact(binary) == build["binary"], "运行后测试二进制身份改变")
    result = {
        "index": index, "label": label, "commit": COMMITS[label], "argv": argv,
        "cwd": str(task / "source"), "pid": child.pid, "environment": env_record,
        "started_utc": started, "finished_utc": finished, "process_wall_ns": end_clock - start_clock,
        "exit_code": status, "binary": build["binary"], "active_build_processes_before": active,
        "source_before": before, "source_after": ensure_source(label, source_record["files"]),
        "measurement_valid": valid_timing and target_complete,
        "resource_target_complete": target_complete, "resource_target_counts": counts,
        "driver_start_unix_ns": driver_start, "driver_end_unix_ns": driver_end,
        "marker_pid": marker_pid,
        "driver_elapsed_ns": elapsed_ns, "driver_success": success, "driver_code_debug": code,
        "right_censored_at_original_timeout": bool(code and "X09-DRIVER-002" in code),
        "log": store(raw),
        "telemetry": store(json_bytes({
            "sample_interval_seconds": SAMPLE_INTERVAL,
            "samples": samples, "marker_observations": markers, "close_status": close_status,
            "scope": "whole resource-test process and system; marker timestamps are polling observations",
        })),
    }
    write_json(ROOT / "runs" / ("%02d-%s.json" % (index, label)), result)
    print(json.dumps({
        "phase": "run_finished", "index": index, "label": label, "exit_code": status,
        "driver_seconds": elapsed_ns / 1e9 if elapsed_ns is not None else None,
        "right_censored": result["right_censored_at_original_timeout"],
    }), flush=True)
    return result


def measure():
    plan = load(ROOT / "plan.json")
    require(plan["commits"] == COMMITS and plan["schedule"] == SCHEDULE, "固定采集计划不符")
    require(plan["tools"]["files"] == tool_identity()["files"], "采集工具发生变化")
    require(not (ROOT / "runs").exists() and not (ROOT / "timing.json").exists(),
            "测量结果不可覆盖；已开始的采集须保留")
    require(all((ROOT / "builds" / (label + "-list.json")).exists() for label in COMMITS),
            "两份快照须先完成预编译")
    results = [run_one(index, label) for index, label in enumerate(SCHEDULE, 1)]
    write_json(ROOT / "timing.json", {
        "plan": plan, "results": results, "causal_attribution_proven": False,
        "budget_or_watchdog_changed": False, "b5_closed": False, "b6_started": False,
        "scope": "diagnostic resource target, not a default full workspace acceptance",
        "all_measurements_valid": all(row["measurement_valid"] for row in results),
    })
    print(json.dumps({"phase": "collection_finished", "runs": len(results),
                      "b5_closed": False, "b6_started": False}), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=["prepare", "measure"])
    args = parser.parse_args()
    try:
        prepare() if args.phase == "prepare" else measure()
    except Exception as error:
        if not (ROOT / "collection-failure.json").exists():
            write_json(ROOT / "collection-failure.json", {
                "phase": args.phase, "time_utc": utc(),
                "exception": type(error).__name__, "message": str(error),
                "stage_passed": False, "b5_closed": False,
            })
        raise
