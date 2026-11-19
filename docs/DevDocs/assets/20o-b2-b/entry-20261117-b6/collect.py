"""按真实命令回执生成B6-1当前结果与Windows/Linux产物摘要。"""
import gzip
import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True
import record

ROOT, REPOSITORY, TASK = record.ROOT, record.REPOSITORY, record.TASK
BASELINE = "67319d61c18b2f02a57f974883c57600f70c724d"
CODE = "a067bd90b6ff34b050e2a39efaa40f3b692d75fd"
B6_TEST = "core/rust/crates/xiao-driver/tests/20o_b2_b6_resources.rs"
WINDOWS_ACCEPTED = [
    "runtime-release", "runtime-staticlib", "driver-b6-language-r3",
    "windows-vm-regressions-final-r2", "a6-resource-driver",
    "native-cmath-o0-o3",
    "native-a5-errors-o0-o3", "native-lfloat-divide-o0-o3",
    "windows-clippy", "windows-b6-language-final-r2",
    "windows-native-cmath-wrappers-final",
]
LINUX_ACCEPTED = [
    "linux-toolchain", "linux-runtime-tests",
    "linux-driver-vm-regressions-final", "linux-native-cmath-o0-o3",
    "linux-native-lfloat-divide", "linux-native-a5-errors",
    "linux-native-component-diagnostic-r2",
]
LINUX_BLOCKER = "linux-native-cmath-wrappers-final"
LINUX_FINAL_LABELS = {
    "linux-toolchain": "linux-toolchain-final-r2",
    "linux-runtime-tests": "linux-runtime-tests-final-r2",
    "linux-driver-vm-regressions-final": "linux-driver-vm-regressions-final-r2",
    "linux-native-cmath-o0-o3": "linux-native-cmath-o0-o3-final-r2",
    "linux-native-lfloat-divide": "linux-native-lfloat-divide-final-r2",
    "linux-native-a5-errors": "linux-native-a5-errors-final-r2",
    "linux-native-component-diagnostic-r2": "linux-native-component-diagnostic-final-r2",
}
RETAINED_ATTEMPTS = [
    "driver-b6-language", "driver-b6-language-r2",
    "linux-linux-runtime-build", "linux-linux-native-cmath-wrappers",
]


def load(name):
    return json.loads((ROOT / name).read_text(encoding="utf-8"))


def raw_object(reference):
    packed = (ROOT / reference["path"]).read_bytes()
    record.require(record.sha(packed) == reference["sha256"], "B6日志对象摘要错误")
    return gzip.decompress(packed) if reference["encoding"] == "gzip" else packed


def summary(raw):
    rows = re.findall(
        rb"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",
        raw,
    )
    return {
        "targets": len(rows),
        "passed": sum(int(row[0]) for row in rows),
        "failed": sum(int(row[1]) for row in rows),
        "ignored": sum(int(row[2]) for row in rows),
    }


def receipt(platform, label, expected=0):
    name = (("linux-" + label) if platform == "linux" else label) + ".json"
    row = load(name)
    code = row.get("wsl_exit_code") if platform == "linux" else row["exit_code"]
    record.require(code == expected and row["source_unchanged"], "命令状态或源码身份不符：" + name)
    record.require(row.get("pid") and row["elapsed_seconds"] > 0,
                   "缺PID或命令时长：" + name)
    for field in ("source_before", "source_after", "log"):
        raw_object(row[field])
    if platform == "windows":
        runtime = row.get("runtime_after")
        if runtime is not None:
            record.require(row["runtime_before"] is None
                           or runtime["sha256"] == row["runtime_before"]["sha256"],
                           "Windows Runtime身份发生漂移")
    return {
        "receipt": name,
        "platform": platform,
        "exit_code": code,
        "pid": row["pid"],
        "elapsed_seconds": row["elapsed_seconds"],
        "source_before": row["source_before"],
        "source_after": row["source_after"],
        "log": row["log"],
        "runtime_before": row.get("runtime_before"),
        "runtime_after": row.get("runtime_after"),
    }


def current_source():
    return record.source()


def artifacts():
    windows = {}
    target = TASK / "target/release/deps"
    if target.is_dir():
        for path in sorted(target.iterdir()):
            if path.is_file() and (
                path.name.startswith("20o_") or path.name.startswith("xiao_runtime")
            ) and path.suffix.lower() in (".exe", ".pdb", ".lib", ".dll"):
                windows[path.relative_to(TASK).as_posix()] = record.identity(path)
    temporary = TASK / "temp"
    if temporary.is_dir():
        for path in sorted(temporary.rglob("*")):
            if path.is_file() and path.suffix.lower() in (
                ".exe", ".ll", ".o", ".obj", ".xiao", ".xiaoc", ".xar", ".meta"
            ):
                windows[path.relative_to(TASK).as_posix()] = record.identity(path)

    linux = {}
    pattern = re.compile(rb"^([0-9a-f]{64})  (/home/xiaoczx/\.b6/.*)$")
    for path in sorted(ROOT.glob("linux-*.json")):
        receipt = load(path.name)
        raw = raw_object(receipt["log"])
        runtime_line = receipt.get("runtime_line")
        runtime_sha = receipt.get("runtime_sha256")
        if runtime_line and runtime_sha:
            runtime_path = runtime_line.split(" ", 1)[1]
            linux[runtime_path] = runtime_sha
        for line in raw.splitlines():
            match = pattern.match(line) or re.match(rb"^([0-9a-f]{64})  (.*)$", line)
            if match and match.group(2).startswith(b"/home/xiaoczx/.b6/"):
                linux[match.group(2).decode("utf-8")] = match.group(1).decode("ascii")
    return {"windows": windows, "linux": linux}


def result():
    source = current_source()
    b5_raw = record.git(
        "show",
        "88a14834:docs/DevDocs/assets/20o-b2-b/entry-20261117-b5-closeout/closeout.json",
    )
    b5 = json.loads(b5_raw)
    record.require(b5["b5_closed"] is True and b5["b6_entry_open"] is True,
                   "固定B5收口对象未通过")

    windows_rows = {label: receipt("windows", label) for label in WINDOWS_ACCEPTED}
    linux_rows = {
        label: receipt("linux", LINUX_FINAL_LABELS[label]) for label in LINUX_ACCEPTED
    }
    blocker_row = receipt("linux", LINUX_BLOCKER + "-r2", expected=101)
    blocker_log = raw_object(load("linux-" + LINUX_BLOCKER + "-r2.json")["log"])
    record.require(b"ArtifactVerification" in blocker_log
                   and b"containers" in blocker_log, "Linux原生包装失败身份不符")

    runtime_windows = summary(raw_object(load("runtime-release.json")["log"]))
    runtime_linux = summary(raw_object(load("linux-linux-runtime-tests-final-r2.json")["log"]))
    a6_resources = summary(raw_object(load("a6-resource-driver.json")["log"]))
    windows_vm = summary(raw_object(load("windows-vm-regressions-final-r2.json")["log"]))
    linux_vm = summary(raw_object(load("linux-linux-driver-vm-regressions-final-r2.json")["log"]))

    diagnostic = raw_object(load("linux-linux-native-component-diagnostic-final-r2.json")["log"])
    diagnostic_line = next(
        (line.decode("utf-8", errors="replace") for line in diagnostic.splitlines()
         if b"B6-NATIVE-DIAGNOSTIC" in line), None,
    )
    record.require(diagnostic_line is not None
                   and 'declared=["rc", "value", "weak"]' in diagnostic_line
                   and "xiao_runtime_array_new" in diagnostic_line,
                   "Linux Runtime组件诊断不符")

    result = {
        "stage": "B6-1",
        "evidence_valid": True,
        "stage_passed": False,
        "status": "blocked",
        "b6_started": True,
        "b6_closed": False,
        "b6_2_started": False,
        "b6_3_started": False,
        "b6_4_started": False,
        "b7_started": False,
        "b2_c_started": False,
        "baseline_commit": BASELINE,
        "code_commit": CODE,
        "final_head": record.git("rev-parse", "HEAD").decode().strip(),
        "runtime_abi": "1.19",
        "codegen_version": 22,
        "stdlib_interface_version": 1,
        "runtime_summaries": {"windows": runtime_windows, "linux": runtime_linux},
        "driver_vm_summaries": {"windows": windows_vm, "linux": linux_vm},
        "b5_closeout": {
            "commit": "88a14834f1dd6389b6d37374d5a89d9176642339",
            "b5_closed": b5["b5_closed"],
            "b6_entry_open": b5["b6_entry_open"],
        },
        "product_changes": sorted(set(record.git(
            "diff", "--name-only", BASELINE, CODE, "--", *record.PRODUCT_ROOTS
        ).decode().splitlines())),
        "current_source_files": len(source),
        "windows": {
            "status": "passed",
            "runtime_tests": runtime_windows,
            "driver_vm_regressions": windows_vm,
            "a6_resource_and_atomicity": a6_resources,
            "language_aliases": ["std.cmath", "stdlib.cmath"],
            "native_cmath_core_o0_o3": "passed",
            "native_cmath_wrappers_o0_o3": "passed",
            "native_a5_errors_o0_o3": "passed",
            "native_lfloat_divide_o0_o3": "passed",
            "clippy_all_targets": "passed",
            "fmt": "passed",
        },
        "linux": {
            "status": "blocked",
            "host": load("linux-linux-runtime-tests-final-r2.json")["host"],
            "runtime_tests": runtime_linux,
            "driver_vm_regressions": linux_vm,
            "native_cmath_core_o0_o3": "passed",
            "native_cmath_wrappers_o0_o3": {
                "passed": False,
                "receipt": "linux-" + LINUX_BLOCKER + "-r2.json",
                "exit_code": blocker_row["exit_code"],
                "reason": "ArtifactVerification拒绝IR未登记的Runtime组件containers",
                "diagnostic": diagnostic_line,
            },
            "native_a5_errors_o0_o3": "passed",
            "native_lfloat_divide_o0_o3": "passed",
        },
        "macos": {
            "status": "unverifiable",
            "reason": "当前执行环境无macOS主机或已配置远程Mac Runner",
        },
        "macos_status": "unverifiable",
        "linux_native_wrapper_exit": blocker_row["exit_code"],
        "linux_error": "containers",
        "performance": {
            "status": "not_started",
            "url_decode_unicode_full_escaped_maximum": {
                "required_input_utf8_bytes": 196608,
                "required_output_utf8_bytes": 65536,
                "measured": False,
            },
            "five_batch_samples": "not_started",
            "os_resident_peak": "not_started",
        },
        "remote_ci": "not_run",
        "full_721_f10": "not_run",
        "retained_attempts": RETAINED_ATTEMPTS,
        "accepted_windows_receipts": windows_rows,
        "accepted_linux_receipts": linux_rows,
        "linux_blocker_receipt": blocker_row,
        "artifact_identities": artifacts(),
        "retained_debts": ["B1原件缺失与来源标记重建", "22C", "22D", "19/11/22"],
    }
    record.require(result["product_changes"] == [B6_TEST],
                   "B6产品差异超出本批新测试文件")
    record.write_new(ROOT / "artifacts-final-r5.json", result["artifact_identities"])
    record.write_new(ROOT / "result-final-r5.json", result)
    print(json.dumps({
        "stage": result["stage"], "stage_passed": result["stage_passed"],
        "status": result["status"], "windows_runtime": runtime_windows,
        "linux_runtime": runtime_linux,
        "linux_native_wrapper_failure": result["linux"]["native_cmath_wrappers_o0_o3"],
        "macos": result["macos"],
    }, ensure_ascii=False))


if __name__ == "__main__":
    record.require(len(sys.argv) == 2 and sys.argv[1] == "result-final",
                   "用法：collect.py result-final")
    result()
