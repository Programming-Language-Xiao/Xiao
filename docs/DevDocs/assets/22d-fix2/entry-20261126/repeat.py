"""重复执行 22D 差分并按轮保存进程回执和原始轨迹。"""

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess


ROOT = Path(__file__).resolve().parents[5]
TEST = "native_concurrency_vm_native_differential_matches_from_same_ir"
LOCK_WAIT = "native-lock-wait-027"
PROBE = f"{LOCK_WAIT}-precheck"
SINGLE_CASES = (LOCK_WAIT, "xcon-bounded-string-success", "spawn-join-str-multiargument")
TERMINAL = {"destroy", "shared_destroy", "weak_release", "lock_guard_release"}
ALIASES = {"strong_release", "shared_release"}
SHARED_COUNTS = {
    "native-queued-cancellation": 2, "native-waiting-cancellation": 2, "native-wait-027": 1,
    LOCK_WAIT: 2, PROBE: 2,
}
SUCCESS_CASES = (
    "spawn-join", "pool-submit-wait", "shared-lock", "shared-read-boundary",
    "finally-return-overrides-child-error", "spawn-join-str-multiargument", "spawn-join-tuple",
    "detach-success", "multi-runtime-parallel", "lock-guard-scope-release", "worker-loop-bounded-stack",
    "xcon-bounded-string-success",
)
ERROR_CASES = (
    "closed-runtime-shared-read", "child-shared-mut-result", "pool-shared-mut-result",
    "child-primary-and-finally-suppressed", "finally-helper-return-preserves-primary",
    "nested-finally-helper-return-preserves-primary-and-suppressed", "native-running-cancellation-finally",
    "native-completed-cancellation", "native-stack-frames", "pool-capacity-zero-032",
    "pool-submit-after-shutdown-030", "native-queued-cancellation", "native-waiting-cancellation",
    "native-wait-027", LOCK_WAIT, "xcon-oversized-029",
)
CASE_EXITS = {**dict.fromkeys(SUCCESS_CASES, 0), **dict.fromkeys(ERROR_CASES, 3), "detached-failure": 0}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def group_trace_actions(raw):
    """校验 TSV 并按对象归并强引用别名，保留原始终态顺序。"""
    objects, terminal = {}, []
    for sequence, line in enumerate(raw.decode("utf-8").splitlines(keepends=True)):
        if line.endswith("\n"):
            line = line[:-1].removesuffix("\r")
        fields = line.split("\t")
        require(len(fields) == 3 and fields[0] == str(sequence) and re.fullmatch(r"[0-9]{1,20}", fields[1])
                and int(fields[1]) <= 2**64 - 1, "释放追踪序号或字段无效")
        _, identity, action = fields
        identity = int(identity)
        if action == "concurrency_release":
            continue
        require(action in TERMINAL or action in ALIASES, "未知释放动作")
        actions = objects.setdefault(identity, [])
        if action in TERMINAL:
            terminal.append((identity, action))
        elif actions and actions[-1] == action:
            continue
        actions.append(action)
    return objects, terminal


def trace_projection(raw):
    """按投影 v2 重放原始字节，保留每个终态动作和共享对象的全部弱释放。"""
    objects, terminal = group_trace_actions(raw)
    signatures = {}
    for identity, actions in objects.items():
        valid = actions == ["strong_release", "destroy"] or actions == ["lock_guard_release"] or (
            actions[:2] == ["shared_release", "shared_destroy"] and len(actions) > 2
            and all(action == "weak_release" for action in actions[2:])
        )
        require(valid, f"释放生命周期无效，对象 {identity} 动作为 {actions}")
        signatures[identity] = tuple(actions)
    return {"objects": sorted(signatures.values()),
            "terminal": [(signatures[identity], action) for identity, action in terminal]}


def compare_trace(label, native, vm):
    """沿用 Rust 差分入口声明的场景范围，对两侧先独立校验，再比较。"""
    shared_count = SHARED_COUNTS.get(label)
    for trace in (native, vm):
        if shared_count is not None:
            shared = [shape for shape in trace["objects"] if shape[0] == "shared_release"]
            require(len(shared) == shared_count, f"{label} 共享对象数量无效")
            require(("lock_guard_release",) in trace["objects"], f"{label} 缺少握手守卫")
        elif label == "native-running-cancellation-finally":
            require(trace["objects"], f"{label} 缺少释放记录")
        elif label == "lock-guard-scope-release":
            require(trace["objects"].count(("lock_guard_release",)) == 2, "作用域 guard 数量无效")
        elif label == "detach-success":
            require(trace["objects"] == [("strong_release", "destroy")] * 2, "必须释放两个独立打印对象")
    if label == "native-running-cancellation-finally":
        return
    if shared_count is not None:
        native = shared_projection(native)
        vm = shared_projection(vm)
    require(native["objects"] == vm["objects"], f"{label} 释放生命周期多重集合不同")
    require(native["terminal"] == vm["terminal"], f"{label} 终态动作流不同")


def shared_projection(trace):
    return {"objects": [shape for shape in trace["objects"] if shape[0] == "shared_release"],
            "terminal": [item for item in trace["terminal"] if item[0][0] == "shared_release"]}


def parse_log(raw):
    text = raw.decode("utf-8", errors="replace")
    summaries = re.findall(r"(?m)^22D-TRACE (\S+) projection=2 vm_objects=(\d+) vm_terminal=(\d+) "
                           r"native_objects=(\d+) native_terminal=(\d+)\s*$", text)
    native = re.findall(r"(?m)^22D-NATIVE (\S+) exit=Some\((\d+)\) verification=(\S+)\s*$", text)
    return {
        "case_start": re.findall(r"(?m)^22D-CASE (\S+): start\s*$", text),
        "case_ok": re.findall(r"(?m)^22D-CASE (\S+): ok\s*$", text),
        "summaries": [dict(zip(("case", "vm_objects", "vm_terminal", "native_objects", "native_terminal"),
                               (label, *map(int, counts)), strict=True)) for label, *counts in summaries],
        "native": [{"case": label, "exit": int(status), "verification": grade} for label, status, grade in native],
    }


def validate_round(row, raw_log, traces):
    """从原始日志和 TSV 重新计算结果，核对回执里的每个数量与场景。"""
    parsed = parse_log(raw_log)
    require(all(row[key] == value for key, value in parsed.items()), "进程回执与原始日志不符")
    case = row["case"]
    require(case is None or case in SINGLE_CASES, "单夹具身份无效")
    cases = [case] if case else list(CASE_EXITS)
    require(parsed["case_start"] == parsed["case_ok"] == cases
            == [record["case"] for record in parsed["native"]], "差分场景缺失或重复")
    require(row["exit"] == 0 and b"test result: ok. 1 passed; 0 failed; 0 ignored;" in raw_log,
            "差分进程未通过")
    require(all(record["exit"] == CASE_EXITS[record["case"]] for record in parsed["native"]), "原生退出码不符")
    labels = cases if case else [*cases, PROBE]
    require(Counter(summary["case"] for summary in parsed["summaries"]) == Counter(labels), "轨迹摘要缺项或重复")
    expected_files = {f"{label}.{backend}-release-trace.tsv" for label in cases for backend in ("native", "vm")}
    if not case:
        expected_files.add(f"{PROBE}.native-release-trace.tsv")
        require(f"22D-REGRESSION {PROBE}: ok".encode() in raw_log
                and f"22D-PRECHECK {PROBE} exit=Some(3)".encode() in raw_log, "受控取消回归未通过")
    require(len(traces) == len(row["traces"]), "原始 TSV 路径重复")
    projected = {}
    for receipt in row["traces"]:
        path = receipt["path"]
        name = PurePosixPath(path).name
        require(name not in projected, f"重复的原始 TSV：{path}")
        trace = trace_projection(traces[path])
        require(receipt["objects"] == len(trace["objects"]) and not receipt["invalid_rows"]
                and receipt["terminal"] == dict(Counter(action for _, action in trace["terminal"])),
                f"轨迹文件回执与 TSV 不符：{path}")
        projected[name] = trace
    require(projected.keys() == expected_files, "原始 TSV 缺项或重复")
    for summary in parsed["summaries"]:
        label = summary["case"]
        vm_label = LOCK_WAIT if label == PROBE else label
        native = projected[f"{label}.native-release-trace.tsv"]
        vm = projected[f"{vm_label}.vm-release-trace.tsv"]
        require(all(summary[f"{backend}_{field}"] == len(trace[field])
                    for backend, trace in (("native", native), ("vm", vm)) for field in ("objects", "terminal")),
                "轨迹摘要数量与 TSV 不符")
        compare_trace(label, native, vm)


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def identify(path):
    path = Path(path).resolve(strict=True)
    with path.open("rb") as stream:
        return {"bytes": path.stat().st_size, "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}


def command(*argv):
    return subprocess.check_output(argv, cwd=ROOT, text=True, encoding="utf-8").strip()


def trace_receipt(path, raw):
    rows = [line.split("\t") for line in raw.decode("utf-8", errors="replace").splitlines()]
    objects = {row[1] for row in rows if len(row) == 3 and row[2] != "concurrency_release"}
    actions = Counter(row[2] for row in rows if len(row) == 3 and row[2] in TERMINAL)
    return {
        "path": path, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest(), "objects": len(objects),
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
    traces = {path.relative_to(output).as_posix(): path.read_bytes() for path in sorted(trace_dir.glob("*/*.tsv"))}
    row = {
        "case": case, "round": number, "command": argv, "started_at": started, "exit": process.returncode,
        "saved_log": log.relative_to(output).as_posix(), "log_sha256": identify(log)["sha256"],
        **parse_log(process.stdout), "traces": [trace_receipt(path, raw) for path, raw in traces.items()],
        "passed": False,
    }
    try:
        validate_round(row, process.stdout, traces)
        row["passed"] = True
    except ValueError as error:
        row["failure"] = str(error)
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
    git = ("cmd", "/c", "git") if os.name == "nt" else ("git",)
    require(not output.is_relative_to(ROOT), "矩阵输出必须位于仓库之外")
    require(not command(*git, "status", "--porcelain=v1"), "矩阵必须从干净的固定源码执行")
    output.mkdir(parents=True)
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
    report["source_unchanged"] = (report["product"] == command(*git, "rev-parse", "HEAD")
                                  and not command(*git, "status", "--porcelain=v1"))
    report["runtime_unchanged"] = identify(runtime)["sha256"] == report["runtime"]["sha256"]
    report["passed"] = report["source_unchanged"] and report["runtime_unchanged"] and all(
        row["passed"] for row in report["rounds"]
    )
    write_json(output / "matrix.json", report)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
