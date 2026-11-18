"""Execute the frozen B6-3 boundary cases through the product entry points."""

import argparse
from collections import Counter
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

from matrix import REGRESSION, REUSE, cases, catalog, output
from prepare import (
    BUILD_OPTIONS, PRODUCT, ROOT, SOURCE_SHA, file_identity, input_identities, live_source, process_env,
    require, source_digest, utc_now, write_json,
)


REPORT = re.compile(
    r"^xiao-error class=(\S+) code=(\S+) message_id=(\S+) params=.* span=(\d+\.\.\d+|<none>) exit_code=(\d+)$",
    re.M,
)
TEST_SUMMARY = re.compile(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", re.M)
LIBRARY_MARKERS = {
    "cmath-native": ("B4-3-NATIVE language=actual-exit,catch A5=7-fixtures O0,O3 result=passed",),
    "numeric-native": tuple(f"B2-A5-ALLOCATION optimization={level} code=X06-RUNTIME-011 out=unchanged"
                            for level in (0, 3)),
    "divide-native": ("B2-B2-DIVIDE backend=native optimization=0,3 intrinsic=33 result=success",),
}


def frame(value):
    body = json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()
    return len(body).to_bytes(8, "big") + body


def frames(data):
    values = []
    while data:
        length = int.from_bytes(data[:8], "big")
        require(0 < length <= len(data) - 8, "核心响应帧不完整")
        values.append(json.loads(data[8:8 + length]))
        data = data[8 + length:]
    return values


def system_peak(process, timeout):
    if os.name != "nt":
        deadline = time.monotonic() + timeout
        while True:
            pid, status, usage = os.wait4(process.pid, os.WNOHANG)
            if pid:
                process.returncode = os.waitstatus_to_exitcode(status)
                return {"method": "getrusage", "unit": "bytes" if sys.platform == "darwin" else "KiB",
                        "peak": usage.ru_maxrss}
            if time.monotonic() >= deadline:
                raise subprocess.TimeoutExpired(process.args, timeout)
            time.sleep(0.01)
    import ctypes

    class Counters(ctypes.Structure):
        _fields_ = [("size", ctypes.c_uint32), ("faults", ctypes.c_uint32),
                    ("peak_working_set", ctypes.c_size_t), ("unused", ctypes.c_size_t * 7)]

    sample = Counters(size=ctypes.sizeof(Counters))
    read = ctypes.windll.psapi.GetProcessMemoryInfo
    read.argtypes = (ctypes.c_void_p, ctypes.POINTER(Counters), ctypes.c_uint32)
    peak = None
    deadline = time.monotonic() + timeout
    while True:
        if read(int(process._handle), ctypes.byref(sample), sample.size) and sample.peak_working_set:
            peak = max(peak or 0, sample.peak_working_set)
        if process.poll() is not None:
            return {"method": "PeakWorkingSetSize", "unit": "bytes", "peak": peak}
        if time.monotonic() >= deadline:
            raise subprocess.TimeoutExpired(process.args, timeout)
        time.sleep(0.01)


def job(argv, directory, env, stem, *, cwd=None, wire=None, timeout=60, merge=False):
    args = [str(arg) for arg in argv]
    stdin = directory / (stem + ".stdin")
    stdout = directory / (stem + ".stdout")
    stderr = directory / (stem + ".stderr")
    stdin.write_bytes(wire or b"")
    started = utc_now()
    with stdin.open("rb") as inp, stdout.open("xb") as out, stderr.open("xb") as err:
        child = subprocess.Popen(args, cwd=cwd or directory, env=env, stdin=inp, stdout=out,
                                 stderr=subprocess.STDOUT if merge else err)
        peak = None
        timed_out = False
        try:
            peak = system_peak(child, timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            child.kill()
            child.wait()
    receipt = {"argv": args, "cwd": str(cwd or directory), "pid": child.pid, "started_utc": started,
               "finished_utc": utc_now(), "exit": child.returncode, "timed_out": timed_out, "rss": peak,
               "files": {path.name: file_identity(path) for path in (stdin, stdout, stderr)}}
    write_json(directory / (stem + ".json"), receipt)
    return receipt, stdout.read_bytes(), stderr.read_bytes()


def observe_vm(response):
    text = "".join(item["data"]["text"] for item in response.get("events", ())
                   if item["kind"] == "intrinsic_output")
    report = response.get("report") or {}
    location = report.get("location")
    return {"exit": response["exit_code"], "stdout": output(text), "code": report.get("code"),
            "span": [location["start"], location["end"]] if location else None,
            "diagnostic": report.get("message", ""), "class": report.get("class")}


def observe_native(receipt, stdout, stderr):
    text = stderr.decode("utf-8").replace("\r\n", "\n")
    reports = REPORT.findall(text)
    require(len(reports) <= 1, "原生用例输出多个机器错误")
    require(reports or not text, "原生产物有未识别的错误输出")
    category, code, message_id, span, exit_code = reports[0] if reports else (None, None, None, "<none>", 0)
    require(int(exit_code) == receipt["exit"], "原生退出与机器错误报告不符")
    return {"exit": receipt["exit"], "stdout": output(stdout.decode("utf-8").replace("\r\n", "\n")),
            "code": code, "span": list(map(int, span.split(".."))) if span != "<none>" else None,
            "diagnostic": text, "class": category, "message_id": message_id}


def satisfies(actual, expected):
    return (
        all(actual[field] == expected[field] for field in ("exit", "stdout", "code"))
        and (expected["span"] is None or actual["span"] == expected["span"])
        and (expected["code"] is None or actual["class"] == "recoverable")
        and (expected["diagnostic_detail"] is None or expected["diagnostic_detail"] in actual["diagnostic"])
    )


def protocol_request(hello, source, request, directory, environment):
    wire = frame(hello) + frame(request)
    receipt, stdout, stderr = job([environment["core"]], directory, process_env(environment), "protocol", wire=wire)
    require(receipt["exit"] == 0 and not receipt["timed_out"] and not stderr, "核心协议进程异常")
    reply, response = frames(stdout)
    require(reply["accepted"] and reply["versions"]["runtime_abi"] == (1 << 16 | 19)
            and reply["versions"]["codegen"] == 22 and response["request_id"] == request["request_id"],
            "核心握手或响应身份不符")
    require(source.is_file(), "边界源码缺失")
    return response


def run_case(spec, code, backend, directory, environment, hello, template, suffix):
    directory.mkdir()
    source = directory / "input.xiao"
    source.write_bytes(code.encode())
    require(file_identity(source)["sha256"] == spec["source_sha256"], "边界源码摘要不符")
    # Give each program its own cache and temporary paths.
    environment = {**environment, "environment": {**environment["environment"]}}
    overrides = environment["environment"]
    for field in ("TMPDIR", "TEMP", "TMP"):
        overrides[field] = str(directory)
    home = directory / "home"
    home.mkdir()
    overrides["XIAO_HOME"] = str(home)
    request = dict(template, request_id=spec["id"], source={"module": "main", "path": str(source), "text": code})
    try:
        receipt = {"case": spec["id"], "backend": backend, "source": file_identity(source)}
        if backend == "vm":
            response = protocol_request(hello, source, request, directory, environment)
            actual = observe_vm(response)
        else:
            executable = directory / ("program" + suffix)
            llvm = directory / "program.ll"
            tools = environment["tools"]
            toolchain = {"clang": overrides["XIAO_CLANG"], "llvm_as": overrides["XIAO_LLVM_AS"],
                         "llc": overrides["XIAO_LLC"], "runtime_library": environment["runtime"],
                         "native_static_libraries": [], "rustc": overrides["RUSTC"],
                         "versions": {"clang": tools["XIAO_CLANG"]["version"].splitlines()[0],
                                      "llvm_as": tools["XIAO_LLVM_AS"]["version"].splitlines()[0],
                                      "llc": tools["XIAO_LLC"]["version"].splitlines()[0]}}
            request.pop("options")
            request.update(type="build", output=str(executable), llvm_ir_output=str(llvm),
                           optimization=dict(template["optimization"], level=int(backend[1])), toolchain=toolchain)
            response = protocol_request(hello, source, request, directory, environment)
            artifact = response.get("artifact")
            if response["exit_code"] != 0 or not artifact:
                return receipt | {"status": "failed", "build_diagnostic": response}
            require(artifact["object_digest"] == file_identity(executable)["sha256"]
                    and artifact["optimization_level"] == int(backend[1]), "原生产物或优化等级不符")
            receipt.update(artifact=artifact["artifact_runtime"], executable=file_identity(executable),
                           llvm=file_identity(llvm))
            process, stdout, stderr = job([executable], directory, process_env(environment), "program")
            require(not process["timed_out"], "边界原生产物触发看门狗")
            actual = observe_native(process, stdout, stderr)
        return receipt | {"actual": actual, "status": "passed" if satisfies(actual, spec["expected"]) else "failed"}
    except (ValueError, OSError, KeyError, subprocess.SubprocessError) as error:
        return {"case": spec["id"], "backend": backend, "status": "failed", "reason": str(error)}


def native_libraries(directory, environment, env):
    source = directory / "link_probe.rs"
    source.write_bytes(b"pub fn native_dependency_probe() {}\n")
    suffix = "lib" if environment["platform"] == "windows" else "a"
    argv = [env["RUSTC"], "--crate-type=staticlib", "--print=native-static-libs", "--target",
            environment["target"], "-o", directory / ("link_probe." + suffix), source]
    receipt, stdout, stderr = job(argv, directory, env, "rustc")
    matches = re.findall(rb"native-static-libs: ([^\r\n]+)", stdout + stderr)
    require(receipt["exit"] == 0 and len(matches) == 1, "Rust native-static-libs 探测失败")
    libs = []
    for token in matches[0].decode().split():
        if token.lower().startswith(("/defaultlib:", "-defaultlib:")):
            continue
        libs.append("-l" + Path(token).stem if token.lower().endswith(".lib") else token)
    return libs


def run_abi(root, environment, fixed, suffix):
    directory = root / "abi"
    directory.mkdir()
    env = process_env(environment) | {key: str(directory) for key in ("TEMP", "TMP", "TMPDIR")}
    libraries = native_libraries(directory, environment, env)
    exe = directory / ("abi_probe" + suffix)
    argv = [env["XIAO_CLANG"], "-target", environment["target"], "-std=c11", "-Werror",
            ROOT / "abi_probe.c", environment["runtime"], *libraries, "-o", exe]
    receipt, _, stderr = job(argv, directory, env, "link")
    if receipt["exit"] != 0:
        return {"status": "failed", "reason": stderr.decode("utf-8", errors="replace")}
    receipt, stdout, stderr = job([exe], directory, env, "normal")
    observed = {}
    for line in stdout.decode().splitlines():
        name, status, code, before, after, result, ownership = line.split("\t")
        observed[name] = {"status": int(status), "code": code, "before_id": int(before),
                          "after_id": int(after), "out": result, "ownership": ownership}
    fatal, stdout, stderr_fatal = job([exe, "fatal"], directory, env, "fatal")
    fatal_fields = stdout.decode().strip().split("\t")
    fatal_expected = fixed["c_abi"]["fatal"]
    fatal_ok = (fatal["exit"] == 4 and len(fatal_fields) == 6
                and fatal_fields[:3] == ["fatal", str(fatal_expected[0]), fatal_expected[1]]
                and fatal_fields[3] == fatal_fields[4] != "0" and fatal_fields[5] == "4"
                and b"class=fatal code=X07-FATAL-006" in stderr_fatal)
    normal = {key: value for key, value in fixed["c_abi"].items() if key != "fatal"}
    good = receipt["exit"] == 0 and not stderr and observed.keys() == normal.keys()
    for name, row in observed.items():
        status = row["status"]
        good &= (name in normal and [status, row["out"] if status == 0 else row["code"]] == normal[name]
                 and row["ownership"] == "owned" and row["after_id"] != 0
                 and (row["before_id"] == 0 if status == 6 else row["before_id"] == row["after_id"]))
    return {"status": "passed" if good and fatal_ok else "failed", "rows": observed,
            "fatal": {"status": "passed" if fatal_ok else "failed", "observation": fatal_fields},
            "executable": file_identity(exe)}


def suite_command(label, cargo):
    base = [cargo, "test", *BUILD_OPTIONS, "--no-fail-fast"]
    if label == "runtime":
        return [*base, "-p", "xiao-runtime", "--", "--show-output"]
    if label == "vm-regressions":
        targets = ("20o_b2_a_resources", "20o_b2_a_contract", "20o_b2_b6_resources",
                   "20o_b2_b2_divide", "20o_b2_b4_cmath_vm", "20o_b2_b4_cmath_library",
                   "20o_b2_b4_cmath_boundary", "20o_b2_b4_cmath_llvm", "20o_b2_b4_numeric_range_routing")
        return [*base, "-p", "xiao-driver", *(arg for target in targets for arg in ("--test", target)),
                "--", "--show-output"]
    if label in ("b1-resources", "b1-contract"):
        target = {"b1-resources": "20o_b1_imp_resources", "b1-contract": "20o_b1_imp_contract"}[label]
        return [*base, "-p", "xiao-driver", "--test", target, "--", "--show-output"]
    target, test, ignored = REGRESSION[label]
    return [*base, "-p", "xiao-driver", "--test", target, "--", *(["--ignored"] if ignored else []),
            "--exact", test, "--nocapture"]


def artifact_identities(directory, source, text):
    paths = set()
    for match in re.findall(r"^\s*Running .+? \((.+)\)\s*$", text, re.M):
        binary = source / match
        if binary.is_file():
            paths.add(binary)
    for path in directory.rglob("*"):
        if path.is_file() and (path.suffix in {".exe", ".ll", ".xiaoc", ".xar"}
                               or path.suffix == "" and os.access(path, os.X_OK)):
            paths.add(path)
    return {str(path): file_identity(path) for path in sorted(paths)}


def run_suite(label, root, environment):
    directory = root / label
    directory.mkdir()
    env = process_env(environment) | {key: str(directory) for key in ("TEMP", "TMP", "TMPDIR")}
    source = Path(environment["source_root"])
    cargo = environment["tools"]["cargo"]["path"]
    argv = suite_command(label, cargo)
    receipt, raw, _ = job(argv, directory, env, "cargo", cwd=source, timeout=900, merge=True)
    text = raw.decode("utf-8", errors="replace").replace("\r\n", "\n")
    totals = [list(map(int, row)) for row in TEST_SUMMARY.findall(text)]
    expected_tests = {name: bool(re.search(rf"^test (?:[\w:]+::)?{name} \.\.\. ok$", text, re.M))
                      for name in REUSE.get(label, {})}
    good = receipt["exit"] == 0 and bool(totals) and all(expected_tests.values())
    if label in REGRESSION:
        good &= totals == [[1, 0, 0]]
    if label == "b1-resources":
        good &= totals == [[18, 0, 0]]
    if label == "b1-contract":
        good &= totals == [[9, 0, 0]]
    if label == "runtime":
        good &= "reference_inputs=17243 facades=15 kernel_identity=exact" in text
    if label == "b1-vm":
        good &= len(set(re.findall(r"^20O-B1-IMP-PROBE case=(\S+) backend=vm result=success$", text, re.M))) == 7
    if label == "b1-native":
        marks = set(re.findall(r"^20O-B1-IMP-PROBE case=(\S+) backend=vm/native optimization=([03]) result=success$",
                               text, re.M))
        good &= len(marks) == 14 and len({name for name, _ in marks}) == 7
    good &= all(marker in text for marker in LIBRARY_MARKERS.get(label, ()))
    if label == "cmath-wrappers":
        good &= len(set(re.findall(r"^B4-4-NATIVE path=(\S+) optimization=([03])", text, re.M))) == 4
    return {"status": "passed" if good else "failed", "test_counts": totals,
            "named_assertions": expected_tests, "exit": receipt["exit"],
            "artifacts": artifact_identities(directory, source, text)}


def run(task, mode):
    task = task.resolve(strict=True)
    evidence = task / "evidence"
    environment = json.loads((evidence / "environment.json").read_bytes())
    source = task / "source"
    runtime, core = map(Path, (environment["runtime"], environment["core"]))
    for path in (source, runtime, core):
        require(path.is_relative_to(task), "任务路径越界")
    require(environment["product"] == PRODUCT and environment["source_sha256"] == SOURCE_SHA,
            "源提交不符")
    built = json.loads((evidence / "build.json").read_bytes())
    require(built == {"runtime": file_identity(runtime), "core": file_identity(core)}, "本次构建产物身份变化")
    require(environment["inputs"] == input_identities(), "边界输入发生变化")
    for tool in environment["tools"].values():
        require(file_identity(Path(tool["path"])) == {key: tool[key] for key in ("bytes", "sha256")},
                "工具链二进制身份变化")
    before = live_source(source)
    require(source_digest(before) == SOURCE_SHA, "实际源码不符")
    fixed = json.loads((ROOT / "matrix.json").read_bytes())
    require(fixed == json.loads(json.dumps(catalog())), "生成的边界矩阵与冻结文件不符")
    suffix = ".exe" if environment["platform"] == "windows" else ""
    result = {"platform": environment["platform"], "product": PRODUCT, "mode": mode,
              "source_sha256": SOURCE_SHA, "matrix": environment["inputs"]["matrix.json"], "build": built}
    location = evidence / mode
    location.mkdir()
    if mode in ("language", "vm", "o0", "o3", "all"):
        keys = {"windows": "coff", "linux": "elf", "macos": "macho"}
        fixtures = source / "tests/spec/11x0-protocol"
        hello = json.loads((fixtures / "hello-request.json").read_bytes())
        template = json.loads((fixtures / "localized-run-request.json").read_bytes())
        template["target"].update(triple=environment["target"], object_format=keys[environment["platform"]])
        template["options"]["timeout_ms"] = 60000
        rows = []
        for backend in ("vm", "o0", "o3") if mode in ("all", "language") else (mode,):
            (location / backend).mkdir()
            for spec, code in cases():
                directory = location / backend / spec["id"]
                row = run_case(spec, code, backend, directory, environment, hello, template, suffix)
                rows.append(row)
                write_json(directory / "result.json", row)
                print(f"{environment['platform']} {backend} {spec['id']} {row['status']}", flush=True)
        result["language"] = rows
    if mode in ("runtime", "all"):
        try:
            result["abi"] = run_abi(location, environment, fixed, suffix)
        except (ValueError, OSError, KeyError, subprocess.SubprocessError) as error:
            result["abi"] = {"status": "failed", "reason": str(error)}
        result["runtime"] = run_suite("runtime", location, environment)
    if mode in ("regression", "all"):
        result["regressions"] = {label: run_suite(label, location, environment)
                                 for label in ("vm-regressions", "b1-resources", "b1-contract", *REGRESSION)}
    statuses = [row["status"] for row in result.get("language", ())]
    statuses += [row["status"] for row in result.get("regressions", {}).values()]
    statuses += [result[key]["status"] for key in ("abi", "runtime") if key in result]
    result["counts"] = dict(Counter(statuses))
    result["passed"] = bool(statuses) and all(status == "passed" for status in statuses)
    require(live_source(source) == before and built == {"runtime": file_identity(runtime), "core": file_identity(core)},
            "执行后源码或产物身份变化")
    write_json(location / "summary.json", result)
    print(json.dumps({"platform": environment["platform"], "mode": mode, "counts": result["counts"]}), flush=True)
    return 0 if result["passed"] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", type=Path, required=True)
    parser.add_argument("--mode", choices=("all", "language", "vm", "o0", "o3", "runtime", "regression"), default="all")
    args = parser.parse_args()
    sys.exit(run(args.task, args.mode))


if __name__ == "__main__":
    main()
