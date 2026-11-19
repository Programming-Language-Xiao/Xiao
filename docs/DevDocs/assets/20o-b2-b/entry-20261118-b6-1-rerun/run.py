"""在各平台全新任务根内逐命令记录真实执行、源码/Runtime/工具/产物身份。"""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True
import common as c


def output(argv, env):
    return subprocess.run(argv, capture_output=True, check=True, encoding="utf-8",
                          errors="strict", env=env).stdout.strip()


def environment(task, source, stage, linux):
    env = os.environ.copy()
    removed = []
    for name in list(env):
        if name.upper().startswith(("XIAO_", "RUST_BIGDECIMAL_")) or name.upper() in {
            "CARGO_TARGET_DIR", "RUSTC", "RUST_TEST_THREADS", "CC", "CXX"}:
            removed.append(name)
            env.pop(name)
    if linux:
        env.update(CARGO_HOME="/home/xiaoczx/.cargo", RUSTUP_HOME="/home/xiaoczx/.rustup",
                   PATH="/home/xiaoczx/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
    temp = task / "temp" / stage
    temp.mkdir()
    overrides = {
        "RUSTUP_TOOLCHAIN":"1.96.0", "CARGO_TARGET_DIR":str(task/"target"),
        "CARGO_BUILD_JOBS":"4", "CARGO_TERM_COLOR":"never",
        "TEMP":str(temp), "TMP":str(temp), "XIAO_HOME":str(task/"home"),
        "XIAO_STDLIB_ROOT":str(source/"stdlib"),
        "XIAO_RUNTIME_LIBRARY":str(task/"target/release"/(
            "libxiao_runtime.a" if linux else "xiao_runtime.lib")),
        "XIAO_TARGET_TRIPLE":"x86_64-unknown-linux-gnu" if linux else "x86_64-pc-windows-msvc",
        "XIAO_CLANG":"/usr/bin/clang" if linux else "D:/msys64/ucrt64/bin/clang.exe",
        "XIAO_LLVM_AS":"/usr/bin/llvm-as" if linux else "D:/msys64/ucrt64/bin/llvm-as.exe",
        "XIAO_LLC":"/usr/bin/llc" if linux else "D:/msys64/ucrt64/bin/llc.exe",
        "XIAO_B1_F10":"0", "XIAO_B2_A_PERF":"0",
        "PYTHONUTF8":"1", "PYTHONIOENCODING":"utf-8",
    }
    if linux:
        overrides["TMPDIR"] = str(temp)
    env.update(overrides)
    rustc = output(["rustup", "which", "--toolchain", "1.96.0", "rustc"], env)
    overrides["RUSTC"] = rustc
    env["RUSTC"] = rustc
    for key in ("CARGO_TARGET_DIR", "TEMP", "TMP", "XIAO_HOME", "XIAO_RUNTIME_LIBRARY", "XIAO_STDLIB_ROOT"):
        c.require(Path(env[key]).resolve().is_relative_to(task.resolve()), "RERUN环境路径越界："+key)
    return env, overrides, sorted(removed)


def tools(env):
    values = {}
    for name in ("rustc", "cargo"):
        path = output(["rustup", "which", "--toolchain", "1.96.0", name], env)
        values[name] = {"binary":c.identity(path), "version":output([path, "-Vv"], env)}
    for key in ("XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLC"):
        values[key] = {"binary":c.identity(env[key]), "version":output([env[key],"--version"],env)}
    return values


def artifact_snapshot(task, temp):
    paths = list(temp.rglob("*"))
    deps = task/"target/release/deps"
    if deps.exists():
        paths.extend(p for p in deps.iterdir() if p.name.startswith(("20o_", "n0_", "xiao_runtime-")))
    rows = []
    for p in sorted(set(paths)):
        if p.is_file() and (p.suffix.lower() in {".exe", ".ll", ".o", ".obj", ".xiaoc", ".xar", ".lib"}
                            or os.access(p,os.X_OK) and p.suffix == ""):
            rows.append(c.identity(p))
    return rows


def run(stage):
    linux = sys.platform != "win32"
    name = ("linux" if linux else "windows") + "-" + stage
    task = Path(c.LINUX_TASK if linux else c.WINDOWS_TASK)
    source = task/"source"
    receipt_path = c.ROOT/(name+".json")
    c.require(not receipt_path.exists(), "RERUN回执禁止覆盖："+name)
    env, overrides, removed = environment(task,source,stage,linux)
    expected = c.read(c.ROOT,"source.json")["files"]
    before = c.source_snapshot(source)
    c.require(before == expected,"RERUN执行前源码不等于固定Git字节")
    fixed_tools = tools(env)
    runtime = Path(env["XIAO_RUNTIME_LIBRARY"])
    runtime_before = c.identity(runtime) if runtime.exists() else None
    c.require((runtime_before is None) == (stage=="runtime-build"), "RERUN必须先冷构建Runtime")
    argv = c.command(stage)
    c.require(argv == c.read(c.ROOT,"commands.json")["linux" if linux else "windows"][stage],
              "RERUN命令偏离开工前冻结清单")
    log = task/"logs"/(name+".log")
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    with log.open("xb") as stream:
        child = subprocess.Popen(argv,cwd=source,env=env,stdin=subprocess.DEVNULL,
                                 stdout=stream,stderr=subprocess.STDOUT)
        code = child.wait()
    elapsed = time.monotonic()-clock
    finished = datetime.now(timezone.utc).isoformat()
    after = c.source_snapshot(source)
    runtime_after = c.identity(runtime) if runtime.exists() else None
    receipt = {
        "label":name,"platform":"linux" if linux else "windows", "stage":stage,
        "code_commit":c.CODE, "task_root":str(task), "source_root":str(source),
        "argv":argv, "cwd":str(source),"pid":child.pid,"exit_code":code,
        "started_utc":started,"finished_utc":finished,"elapsed_seconds":elapsed,
        "os":{"system":platform.system(),"release":platform.release(),"machine":platform.machine(),
              "distribution":platform.freedesktop_os_release() if linux else platform.win32_ver()},
        "environment_overrides":overrides,"removed_environment_names":removed,
        "libtest_threads":"repository .cargo/config.toml RUST_TEST_THREADS=4; no override",
        "tools_before":fixed_tools,"tools_after":tools(env),
        "source_before":c.store(c.canonical(before)),"source_after":c.store(c.canonical(after)),
        "source_unchanged":before==after,
        "runtime_before":runtime_before,"runtime_after":runtime_after,
        "artifacts_after":c.store(c.canonical(artifact_snapshot(task,Path(env["TEMP"])))),
        "recorder_sha256":c.sha(Path(__file__).read_bytes()),
        "common_sha256":c.sha(Path(c.__file__).read_bytes()),
        "log":c.store(log.read_bytes()),
    }
    c.write_new(receipt_path,receipt)
    print(json.dumps({k:receipt[k] for k in ("label","pid","exit_code","elapsed_seconds","source_unchanged")},ensure_ascii=False),flush=True)
    c.require(before==after and receipt["tools_before"]==receipt["tools_after"],"RERUN源码或工具在运行期间变化")
    c.require(runtime_after and (runtime_before is None or runtime_before==runtime_after),"RERUN Runtime身份变化")
    return code


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    stages = sys.argv[1:]
    c.require(stages,"用法：run.py stage [stage ...]")
    for stage in stages:
        code = run(stage)
        if code:
            raise SystemExit(code)
