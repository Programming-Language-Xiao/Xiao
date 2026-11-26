"""重复执行 22D 差分并按轮保存进程回执和原始轨迹。"""

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[5]
TEST = "native_concurrency_vm_native_differential_matches_from_same_ir"
LOCK_WAIT = "native-lock-wait-027"
PROBE = f"{LOCK_WAIT}-precheck"
SINGLE_CASES = {LOCK_WAIT: 3, "xcon-bounded-string-success": 0, "spawn-join-str-multiargument": 0}
TERMINAL = {"destroy", "shared_destroy", "weak_release", "lock_guard_release"}


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def identify(path):
    path = Path(path).resolve(strict=True)
    with path.open("rb") as stream:
        return {"bytes": path.stat().st_size, "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}


def command(*argv):
    return subprocess.check_output(argv, cwd=ROOT, text=True, encoding="utf-8").strip()


def file_receipt(path, root):
    rows = [line.split("\t") for line in path.read_text(encoding="utf-8").splitlines()]
    objects = {row[1] for row in rows if len(row) == 3 and row[2] != "concurrency_release"}
    actions = Counter(row[2] for row in rows if len(row) == 3 and row[2] in TERMINAL)
    return {
        "path": path.relative_to(root).as_posix(), **identify(path), "objects": len(objects),
        "terminal": dict(actions),
        "invalid_rows": [index for index, row in enumerate(rows) if len(row) != 3 or row[0] != str(index)],
    }


def repeat(output, case, number, executable):
    task = output / (case or "full") / f"{number:03}"
    trace_dir = task / "trace"
    temp_dir = task / "temp"
    for directory in (trace_dir, temp_dir):
        directory.mkdir(parents=True)
    env = os.environ.copy()
    env.update(XIAO_22D_TRACE_DIR=str(trace_dir), TEMP=str(temp_dir), TMP=str(temp_dir), TMPDIR=str(temp_dir))
    if case:
        env["XIAO_22D_CASE"] = case
    else:
        env.pop("XIAO_22D_CASE", None)
    started = datetime.now(timezone.utc).isoformat()
    argv = [str(executable), "--exact", "--ignored", "--nocapture", TEST]
    process = subprocess.run(argv, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log = task / "output.log"
    log.write_bytes(process.stdout)
    text = process.stdout.decode("utf-8", errors="replace")
    start = re.findall(r"(?m)^22D-CASE (\S+): start\s*$", text)
    cases = re.findall(r"(?m)^22D-CASE (\S+): ok\s*$", text)
    summaries = re.findall(r"(?m)^22D-TRACE (\S+) projection=2 vm_objects=(\d+) vm_terminal=(\d+) "
                           r"native_objects=(\d+) native_terminal=(\d+)\s*$", text)
    receipts = re.findall(r"(?m)^22D-NATIVE (\S+) exit=Some\((\d+)\) verification=(\S+)\s*$", text)
    traces = sorted(trace_dir.glob("*/*.tsv"))
    files = [file_receipt(path, output) for path in traces]
    summary_cases = [row[0] for row in summaries]
    native_cases = [row[0] for row in receipts]
    expected_files = {f"{label}.{backend}-release-trace.tsv" for label in cases for backend in ("vm", "native")}
    if not case:
        expected_files.add(f"{PROBE}.native-release-trace.tsv")
    observed_exit = {label: int(status) for label, status, _ in receipts}
    scope_passed = (
        cases == [case] and observed_exit.get(case) == SINGLE_CASES[case]
        if case else len(set(cases)) == 29
        and Counter(observed_exit.values()) == {0: 13, 3: 16}
        and all(observed_exit.get(label) == status for label, status in SINGLE_CASES.items())
        and observed_exit.get("detached-failure") == 0
        and f"22D-REGRESSION {PROBE}: ok" in text
        and f"22D-PRECHECK {PROBE} exit=Some(3)" in text
    )
    expected_summaries = Counter(cases if case else [*cases, PROBE])
    row = {
        "case": case, "round": number, "command": argv, "started_at": started, "exit": process.returncode,
        "saved_log": log.relative_to(output).as_posix(), "log_sha256": identify(log)["sha256"],
        "case_start": start, "case_ok": cases,
        "summaries": [dict(zip(("case", "vm_objects", "vm_terminal", "native_objects", "native_terminal"),
                               (label, *map(int, counts)), strict=True)) for label, *counts in summaries],
        "native": [{"case": label, "exit": int(status), "verification": grade} for label, status, grade in receipts],
        "traces": files,
        "passed": process.returncode == 0 and scope_passed and start == cases == native_cases
        and "test result: ok. 1 passed; 0 failed; 0 ignored;" in text
        and Counter(summary_cases) == expected_summaries
        and len(traces) == len(expected_files) and {path.name for path in traces} == expected_files
        and all(not item["invalid_rows"] for item in files),
    }
    write_json(task / "receipt.json", row)
    print(f"22D-ROUND {case or 'full'} {number:03}: {'ok' if row['passed'] else 'FAILED'}", flush=True)
    return {"case": case, "round": number, "passed": row["passed"],
            "receipt": (task / "receipt.json").relative_to(output).as_posix()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--profile", choices=("matrix", "f1", "smoke"), default="matrix")
    parser.add_argument("--workers", type=int, default=1)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True)
    git = ("cmd", "/c", "git") if os.name == "nt" else ("git",)
    rust = command("rustc", "-vV")
    target = re.search(r"(?m)^host: (\S+)$", rust)[1]
    os.environ["XIAO_TARGET_TRIPLE"] = target
    runtime = os.environ["XIAO_RUNTIME_LIBRARY"]
    report = {
        "product": command(*git, "rev-parse", "HEAD"), "tree": command(*git, "rev-parse", "HEAD:"),
        "target": target, "task_root": str(output), "profile": args.profile, "rust": rust.splitlines(),
        "runtime": {"path": runtime, **identify(runtime)},
        "tools": {key: {"path": os.environ[key], **identify(os.environ[key]),
                        "version": command(os.environ[key], "--version").splitlines()[0]}
                  for key in ("XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLC")},
        "ci": {key: os.environ.get(key) for key in
               ("GITHUB_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "GITHUB_JOB", "RUNNER_OS", "ImageOS", "ImageVersion")},
        "passed": False,
    }
    write_json(output / "matrix.json", report)
    # Cargo 返回实际编译出的测试入口，避免从缓存目录猜测二进制文件名。
    argv = ["cargo", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-driver",
            "--test", "22d_llvm_concurrency", "--no-run", "--message-format=json"]
    result = subprocess.run(argv, cwd=ROOT, stdout=subprocess.PIPE, check=True)
    executable, = [Path(row["executable"]).resolve() for row in map(json.loads, result.stdout.splitlines())
                   if row.get("reason") == "compiler-artifact" and row.get("executable")
                   and row["target"]["name"] == "22d_llvm_concurrency"]
    report["executable"] = {"path": str(executable), **identify(executable)}
    if args.profile == "matrix":
        single_count = 50 if os.name == "nt" else 30
        counts = {None: 10, **dict.fromkeys(SINGLE_CASES, single_count)}
    elif args.profile == "f1":
        counts = {LOCK_WAIT: 100}
    else:
        counts = {None: 1, **dict.fromkeys(SINGLE_CASES, 1)}
    rounds = [(case, number) for case, count in counts.items() for number in range(1, count + 1)]
    with ThreadPoolExecutor(max_workers=args.workers) as workers:
        report["rounds"] = list(workers.map(lambda item: repeat(output, *item, executable), rounds))
    report["passed"] = all(row["passed"] for row in report["rounds"])
    report["runtime_unchanged"] = identify(runtime)["sha256"] == report["runtime"]["sha256"]
    write_json(output / "matrix.json", report)
    return 0 if report["passed"] and report["runtime_unchanged"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
