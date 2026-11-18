"""在GitHub macOS作业中验证B6-1并保存可对账的执行回执。"""
import argparse
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tarfile
import tempfile


REPO = Path(__file__).resolve().parents[2]
BASELINE = Path(__file__).with_name("b2-b6-baseline.json")
SOURCE_ROOTS = (".cargo", ".gitattributes", "LICENSE", "core/rust", "stdlib",
                "tests", "docs/UseDocs", "docs/module-registry.json")
REQUIRED = ("core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock", "stdlib/cmath.xiao",
            "tests/fixtures/docs/cmath/reference-vectors.tsv",
            "tests/spec/11x0-protocol/hello-request.json",
            "docs/UseDocs/language/basics/types/conversions.md",
            "docs/UseDocs/language/basics/p1-expressions/operators.md")
DRIVER_TESTS = ("20o_b2_b6_resources", "20o_b2_a_resources", "20o_b2_a_contract",
                "20o_b2_b2_divide", "20o_b2_b4_cmath_vm", "20o_b2_b4_cmath_library",
                "20o_b2_b4_cmath_boundary", "20o_b2_b4_numeric_range_routing")
NATIVE_TESTS = (
    ("wrappers", "20o_b2_b4_cmath_library", "cmath_library_matches_native_o0_o3"),
    ("core", "20o_b2_b4_cmath_llvm", "native_cmath_static_dynamic_o0_o3_and_shared_pow"),
    ("errors", "20o_b2_a_contract", "a5_language_values_match_native_golden_outputs"),
    ("divide", "20o_b2_b2_divide", "lfloat_divide_matches_native_golden_outputs"),
    ("b1", "20o_b1_probe", "b1_capabilities_match_native_golden_outputs"),
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def git(*args):
    command = ["cmd", "/c", "git"] if os.name == "nt" else ["git"]
    return subprocess.check_output([*command, *args], cwd=REPO)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def file_identity(path):
    with path.open("rb") as stream:
        return {"bytes": path.stat().st_size, "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}


def write_json(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as output:
        json.dump(value, output, indent=2, ensure_ascii=False, sort_keys=True)
        output.write("\n")


def source_digest(files):
    return sha(json.dumps(files, sort_keys=True, separators=(",", ":")).encode("utf-8"))


def snapshot(revision, destination=None):
    # Windows Git 的 autocrlf 会影响 archive，固定 LF 以复核同一 Git 对象。
    raw = git("-c", "core.autocrlf=false", "-c", "core.eol=lf", "archive", "--format=tar",
              revision, "--", *SOURCE_ROOTS)
    with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
        files = {m.name: sha(archive.extractfile(m).read()) for m in archive if m.isfile()}
        require(all(name in files for name in REQUIRED), "固定源码缺少锁、标准库或编译夹具")
        if destination is not None:
            archive.extractall(destination, filter="data")
    return files


def live_source(root):
    return {path.relative_to(root).as_posix(): file_identity(path)["sha256"]
            for path in root.rglob("*") if path.is_file()}


def command_output(argv, env):
    return subprocess.check_output(argv, env=env).decode("utf-8").strip()


def tool_identity(path, version_arg, env):
    path = path.resolve(strict=True)
    return {"path": str(path), **file_identity(path),
            "version": command_output([str(path), version_arg], env)}


def suite_commands(cargo, suite):
    base = ["--locked", "--manifest-path", "core/rust/Cargo.toml", "--release"]
    yield "runtime-build", [cargo, "build", *base, "-p", "xiao-runtime"]
    if suite == "native":
        for label, target, name in NATIVE_TESTS:
            yield label, [cargo, "test", *base, "-p", "xiao-driver", "--test", target,
                          "--", "--ignored", "--exact", name, "--nocapture"]
        return
    yield "runtime", [cargo, "test", *base, "-p", "xiao-runtime", "--", "--nocapture"]
    targets = [part for target in DRIVER_TESTS for part in ("--test", target)]
    yield "vm", [cargo, "test", *base, "-p", "xiao-driver", *targets, "--", "--nocapture"]
    yield "codegen", [cargo, "test", *base, "-p", "xiao-codegen-llvm", "--", "--nocapture"]
    yield "driver", [cargo, "test", *base, "-p", "xiao-driver", "--test", "20o_b2_b4_cmath_llvm",
                     "--test", "n0_b_dynamic_native", "--", "--nocapture"]
    yield "b1-vm", [cargo, "test", *base, "-p", "xiao-driver", "--test", "20o_b1_probe",
                    "--", "--exact", "b1_capabilities_have_vm_golden_outputs", "--nocapture"]
    yield "fmt", [cargo, "fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"]
    yield "clippy", [cargo, "clippy", *base, "-p", "xiao-codegen-llvm", "-p", "xiao-driver",
                     "--all-targets", "--", "-D", "warnings"]


def wrappers(log, temp):
    rows = []
    pattern = (r"B4-4-NATIVE path=(\S+) optimization=([03]) executable=(.+) verification=(\S+)"
               r" declared=(\[.*?\]) observed=(\[.*?\]) stdout=golden errors=009 result=passed")
    for line in log.splitlines():
        if not line.startswith("B4-4-NATIVE "):
            continue
        match = re.fullmatch(pattern, line)
        require(match is not None, "macOS包装回执格式不符")
        module, level, path, verification, declared, observed = match.groups()
        declared, observed = json.loads(declared), json.loads(observed)
        require(verification == "Complete" and declared == observed and "containers" in declared,
                "Mach-O组件验证未通过")
        executable = Path(path).resolve(strict=True)
        require(executable.is_relative_to(temp.resolve()), "包装执行文件不在本次任务根")
        with executable.open("rb") as artifact:
            require(artifact.read(4) == b"\xcf\xfa\xed\xfe", "包装执行文件不是64位Mach-O")
        rows.append({"module": module, "optimization": int(level), "verification": verification,
                     "declared": declared, "observed": observed, "path": path,
                     "executable": file_identity(executable), "llvm_ir": file_identity(executable.with_suffix(".ll"))})
    require(len(rows) == 4 and {(r["module"], r["optimization"]) for r in rows} ==
            {(m, o) for m in ("std.cmath", "stdlib.cmath") for o in (0, 3)}, "macOS包装缺少四格")
    return rows


def verify_log(label, log, temp):
    if label in {"wrappers", "core", "errors", "divide", "b1", "b1-vm"}:
        summaries = re.findall(r"^test result: .*", log, re.M)
        require(len(summaries) == 1 and summaries[0].startswith("test result: ok. 1 passed; 0 failed; 0 ignored;"),
                "精确测试未通过一项：" + label)
    if label == "wrappers":
        return {"native_rows": wrappers(log, temp)}
    if label == "runtime":
        cases = re.findall(r"B3-CMATH-ABI function=(\S+) cases=(\d+) result=success", log)
        require(len({name for name, _ in cases}) == 15 and sum(int(count) for _, count in cases) == 17243
                and "reference_inputs=17243 facades=15 kernel_identity=exact" in log,
                "macOS正式向量覆盖不完整")
        return {"formal_reference_cases": 17243}
    if label == "vm":
        require(all("B6-1-LANGUAGE module=" + name in log for name in ("std.cmath", "stdlib.cmath"))
                and "test result: ok. 8 passed; 0 failed; 0 ignored;" in log, "macOS资源或双别名证据缺项")
    if label == "errors":
        require(all(f"B2-A5-ALLOCATION optimization={level} code=X06-RUNTIME-011 out=unchanged source_unchanged=true" in log
                    for level in (0, 3)), "macOS原生011或原子性回执缺项")
    if label == "divide":
        require("B2-B2-DIVIDE backend=native optimization=0,3 intrinsic=33 result=success" in log,
                "macOS显式除法原生回执缺项")
    return {}


def utc_now():
    return datetime.now(timezone.utc).isoformat()


def execute(argv, label, source, temp, env, out, runtime, files):
    temp.mkdir()
    env = dict(env, TMPDIR=str(temp), TEMP=str(temp), TMP=str(temp))
    before = file_identity(runtime) if runtime.is_file() else None
    log = out / (label + ".log")
    started = utc_now()
    with log.open("xb") as stream:
        child = subprocess.Popen(argv, cwd=source, env=env, stdout=stream, stderr=subprocess.STDOUT)
        exit_code = child.wait()
    contents = log.read_text(encoding="utf-8", errors="replace")
    after = file_identity(runtime) if runtime.is_file() else None
    row = {"argv": argv, "pid": child.pid, "started_utc": started, "finished_utc": utc_now(),
           "exit_code": exit_code, "cwd": str(source), "temp": str(temp), "runtime_before": before,
           "runtime_after": after, "log": {"name": log.name, **file_identity(log)},
           "source_after_sha256": source_digest(live_source(source))}
    try:
        require(exit_code == 0, f"{label}退出{exit_code}")
        require(row["source_after_sha256"] == source_digest(files), "测量源码发生变化")
        require(after and (before is None if label == "runtime-build" else before == after), "Runtime身份漂移")
        row.update(verify_log(label, contents, temp), passed=True)
    except ValueError as error:
        row.update(passed=False, reason=str(error))
        print(contents[-12000:], file=sys.stderr, flush=True)
    write_json(out / (label + ".json"), row)
    print(f"B6-1-MACOS {label} pid={child.pid} exit={exit_code} passed={row['passed']}", flush=True)
    if not row["passed"]:
        raise SystemExit(row["reason"])


def run(suite, llvm_bin, out):
    require(sys.platform == "darwin" and platform.machine() == "arm64", "需要原生arm64 macOS作业")
    checkout = git("rev-parse", "HEAD").decode().strip()
    require(checkout == os.environ["GITHUB_SHA"], "检出的提交与作业不符")
    fixed = json.loads(BASELINE.read_bytes())
    out.mkdir(parents=True)
    task = Path(tempfile.mkdtemp(prefix="xiao-b6-1-", dir=os.environ["RUNNER_TEMP"]))
    source, target = task / "source", task / "target"
    source.mkdir()
    files = snapshot(checkout, source)
    write_json(out / "source.json", files)
    actual = live_source(source)
    require(source_digest(files) == fixed["sha256"] and len(files) == fixed["files"],
            f"macOS归档清单不符，实际{len(files)}项/{source_digest(files)}，预期{fixed}")
    changed = sorted(name for name in files.keys() | actual.keys() if files.get(name) != actual.get(name))
    require(not changed, "macOS解包内容变化：" + ", ".join(changed[:10]))
    env = {name: value for name, value in os.environ.items()
           if not name.startswith(("XIAO_", "RUST_BIGDECIMAL_")) and name != "RUST_TEST_THREADS"}
    rustc = Path(command_output(["rustup", "which", "--toolchain", "1.96.0", "rustc"], env))
    cargo = rustc.with_name("cargo")
    runtime = target / "release/libxiao_runtime.a"
    env.update(RUSTUP_TOOLCHAIN="1.96.0", RUSTC=str(rustc), CARGO_TERM_COLOR="never",
               PATH=str(rustc.parent) + os.pathsep + env["PATH"], CARGO_TARGET_DIR=str(target),
               XIAO_HOME=str(task / "home"), XIAO_STDLIB_ROOT=str(source / "stdlib"),
               XIAO_RUNTIME_LIBRARY=str(runtime), XIAO_TARGET_TRIPLE="aarch64-apple-darwin",
               XIAO_B1_F10="0", XIAO_B2_A_PERF="0")
    (task / "home").mkdir()
    tools = {"rustc": tool_identity(rustc, "-Vv", env), "cargo": tool_identity(cargo, "-Vv", env)}
    require("host: aarch64-apple-darwin" in tools["rustc"]["version"]
            and tools["rustc"]["version"].startswith("rustc 1.96.0"), "Rust目标或版本不符")
    for label, binary in (("XIAO_CLANG", "clang"), ("XIAO_LLVM_AS", "llvm-as"), ("XIAO_LLC", "llc")):
        tools[label] = tool_identity(llvm_bin / binary, "--version", env)
        env[label] = tools[label]["path"]
    write_json(out / "environment.json", {"suite": suite, "runner_commit": checkout, "fixed": fixed,
               "runner_image": os.environ.get("ImageVersion"), "run_id": os.environ["GITHUB_RUN_ID"],
               "run_attempt": os.environ["GITHUB_RUN_ATTEMPT"], "job": os.environ["GITHUB_JOB"],
               "macos": platform.mac_ver()[0], "task_root": str(task), "source_root": str(source),
               "target": env["XIAO_TARGET_TRIPLE"], "tools": tools, "stdlib": env["XIAO_STDLIB_ROOT"],
               "runtime": str(runtime), "cargo_target_dir": str(target)})
    for label, argv in suite_commands(str(cargo), suite):
        execute(argv, label, source, task / label, env, out, runtime, files)
    write_json(out / "result.json", {"passed": True, "suite": suite, "runner_commit": checkout,
                                     "source_sha256": fixed["sha256"], "runtime": file_identity(runtime)})


def main():
    args = argparse.ArgumentParser(description=__doc__)
    commands = args.add_subparsers(dest="command", required=True)
    baseline = commands.add_parser("baseline")
    baseline.add_argument("revision")
    runner = commands.add_parser("run")
    runner.add_argument("--suite", choices=("native", "resources"), required=True)
    runner.add_argument("--llvm-bin", type=Path, required=True)
    runner.add_argument("--output", type=Path, required=True)
    options = args.parse_args()
    if options.command == "baseline":
        revision = git("rev-parse", "--verify", options.revision).decode().strip()
        require(git("cat-file", "-t", revision).strip() == b"commit", "基线必须是Git提交")
        files = snapshot(revision)
        print(json.dumps({"product_commit": revision, "files": len(files), "sha256": source_digest(files)}, indent=2))
    else:
        run(options.suite, options.llvm_bin, options.output)


if __name__ == "__main__":
    main()
