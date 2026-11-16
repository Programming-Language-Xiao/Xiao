"""Run the pre-B5 and B5 default workspaces serially with fixed environments."""
import csv
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parent
REPOSITORY = Path(__file__).resolve().parents[5]
B5_ROOT = REPOSITORY / "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild"
VARIANTS = [
    ("b4", "pre-b5-cfa79381", "cfa79381bbae027cec11136ca380e82f66720c34"),
    ("final3", "b5-idle-replay", "5864cbde627f727483ab676c6f4bc1bf0a7819bb"),
]
COMMAND = [
    "cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml",
    "--workspace", "--no-fail-fast",
]
BUILD_PROCESSES = {"cargo.exe", "rustc.exe", "xiao-driver.exe"}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))


def store(raw):
    digest = sha(raw)
    path = ROOT / "objects" / "sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == raw, "content object collision")
    else:
        with path.open("xb") as output:
            output.write(raw)
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": len(raw), "sha256": digest}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_b5_module(name):
    sys.path.insert(0, str(B5_ROOT))
    module = __import__(name)
    return module


def active_build_processes():
    result = subprocess.run(["tasklist", "/FO", "CSV", "/NH"], check=True,
                            capture_output=True, encoding="mbcs")
    return sorted({row[0] for row in csv.reader(result.stdout.splitlines())
                   if row and row[0].lower() in BUILD_PROCESSES})


def runtime_identity(record, config):
    return record.runtime(config)


def run_one(record, variant, label, expected_commit):
    source = load(B5_ROOT / variant / "source.json")
    config = load(B5_ROOT / variant / "environment.json")
    source_root = Path(config["source_root"])
    require(source["revision"] == expected_commit, variant + " source commit mismatch")
    before = record.worktree_files(source_root)
    require(before == source["files"], variant + " source tree differs from fixed snapshot")
    require({path: row["canonical_sha256"] for path, row in before.items()} ==
            record.fixed_files(REPOSITORY, expected_commit), variant + " fixed Git tree mismatch")
    runtime_before = runtime_identity(record, config)
    require(runtime_before["exists"] and runtime_before["bytes"] > 0,
            variant + " runtime is missing")
    require(not active_build_processes(), "another Cargo/Rust workload is active before " + label)

    task_temp = Path(config["overrides"]["TEMP"])
    task_temp.mkdir(parents=True, exist_ok=True)
    process_env = record.controlled(config)
    started = datetime.now(timezone.utc).isoformat()
    start_clock = __import__("time").monotonic()
    with tempfile.NamedTemporaryFile(prefix="b5-watchdog-", suffix=".log", dir=task_temp,
                                     delete=False) as output:
        temporary_log = Path(output.name)
        process = subprocess.Popen(COMMAND, cwd=source_root, env=process_env,
                                   stdout=output, stderr=subprocess.STDOUT)
        pid = process.pid
        exit_code = process.wait()
    elapsed = __import__("time").monotonic() - start_clock
    raw_log = temporary_log.read_bytes()
    temporary_log.unlink()
    log = store(raw_log)
    after = record.worktree_files(source_root)
    runtime_after = runtime_identity(record, config)
    require(before == after, variant + " product tree changed during workspace")
    require(runtime_before == runtime_after, variant + " runtime changed during workspace")
    finished = datetime.now(timezone.utc).isoformat()
    result = {
        "label": label,
        "variant": variant,
        "expected_product_commit": expected_commit,
        "source_identity_sha256": source["sha256"],
        "cargo_lock_sha256": source["files"]["core/rust/Cargo.lock"]["raw_sha256"],
        "source_root": str(source_root),
        "task_root": config["task_root"],
        "environment_sha256": sha((B5_ROOT / variant / "environment.json").read_bytes()),
        "runner_sha256": sha(Path(__file__).read_bytes()),
        "argv": COMMAND,
        "pid": pid,
        "started_utc": started,
        "finished_utc": finished,
        "elapsed_seconds": round(elapsed, 3),
        "exit_code": exit_code,
        "runtime_before": runtime_before,
        "runtime_after": runtime_after,
        "active_build_processes_before": [],
        "source_unchanged": before == after,
        "watchdog_marker_present": b"X09-DRIVER-002" in raw_log,
        "unicode_budget_test_present": b"url_decoding_reaches_the_unicode_result_budget" in raw_log,
        "log": log,
        "host": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "logical_cpu_count": os.cpu_count(),
            "python": platform.python_version(),
        },
    }
    write_json(ROOT / (label + ".json"), result)
    print(json.dumps({"label": label, "exit_code": exit_code,
                      "elapsed_seconds": result["elapsed_seconds"],
                      "watchdog_marker_present": result["watchdog_marker_present"]}), flush=True)
    return result


def main():
    require(not list(ROOT.glob("*.json")) and not (ROOT / "objects").exists(),
            "attribution evidence directory must be empty before collection")
    record = load_b5_module("record")
    results = [run_one(record, variant, label, commit)
               for variant, label, commit in VARIANTS]
    summary = {
        "same_default_command": results[0]["argv"] == results[1]["argv"] == COMMAND,
        "sequential_idle_replay": True,
        "baseline_commit": results[0]["expected_product_commit"],
        "b5_test_commit": results[1]["expected_product_commit"],
        "baseline_timed_out": results[0]["watchdog_marker_present"],
        "b5_replay_timed_out": results[1]["watchdog_marker_present"],
        "runs": [row["label"] + ".json" for row in results],
    }
    write_json(ROOT / "summary.json", summary)
    print(json.dumps(summary, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
