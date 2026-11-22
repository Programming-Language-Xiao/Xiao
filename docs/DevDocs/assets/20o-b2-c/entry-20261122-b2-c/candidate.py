"""用固定 release 核心验证公共语言示例、删源归档及原生候选。"""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from measure import ROOT, base, capture

WIN_LINK_WRAPPER = r"""
#include <process.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <windows.h>

int wmain(int argc, wchar_t **argv) {
    const wchar_t *clang = _wgetenv(L"XIAO_C_REAL_CLANG");
    if (clang == NULL) return 87;
    wchar_t **forward = calloc((size_t)argc + 2, sizeof(*forward));
    wchar_t *option = NULL;
    int used = 1;
    forward[0] = (wchar_t *)clang;
    int assembling = 0;
    for (int i = 1; i < argc; ++i) {
        if (wcscmp(argv[i], L"-c") == 0 || wcscmp(argv[i], L"-S") == 0) assembling = 1;
        forward[used++] = argv[i];
    }
    for (int i = 1; i + 1 < argc && !assembling; ++i) {
        if (wcscmp(argv[i], L"-o") != 0) continue;
        const wchar_t *name = argv[i + 1];
        size_t count = wcslen(name) + 24;
        option = calloc(count, sizeof(*option));
        if (option == NULL) return 8;
        swprintf(option, count, L"-Wl,/MAP:%ls.map", name);
        forward[used++] = option;
        break;
    }
    intptr_t status = _wspawnv(_P_WAIT, clang, (const wchar_t *const *)forward);
    if (status == -1) perror("clang");
    free(option);
    free(forward);
    return status == -1 ? 87 : (int)status;
}
"""
FIXTURES = "tests/fixtures/docs/numeric"
PROTOCOL = "tests/spec/11x0-protocol"
VERSIONS = {"windows": ("coff", ".exe"), "linux": ("elf", ""), "macos": ("macho", "")}
SUPPORTED = ((1 << 16) | 19, 22)


def frame(value):
    raw = json.dumps(value, ensure_ascii=False).encode("utf-8")
    return len(raw).to_bytes(8, "big") + raw


def frames(raw):
    while raw:
        count = int.from_bytes(raw[:8], "big")
        base.require(0 < count <= len(raw) - 8, "核心响应帧不完整")
        yield json.loads(raw[8:8 + count])
        raw = raw[8 + count:]


def output(response):
    return "".join(event["data"]["text"] for event in response["events"] if event["kind"] == "intrinsic_output")


def invoke(executable, argv, directory, env, out, name, *, request=None):
    stdout, stderr = out / f"{name}.stdout", out / f"{name}.stderr"
    started = base.utc()
    wire = None if request is None else b"".join(map(frame, request))
    if wire is not None:
        (out / f"{name}.stdin").write_bytes(wire)
    cmd = [str(executable), *map(str, argv)]
    with stdout.open("xb") as raw_out, stderr.open("xb") as raw_err, subprocess.Popen(
        cmd, cwd=directory, env=env, stdin=subprocess.PIPE if wire is not None else subprocess.DEVNULL,
        stdout=raw_out, stderr=raw_err,
    ) as child:
        try:
            child.communicate(wire, timeout=60)
        except subprocess.TimeoutExpired:
            child.kill()
            child.communicate()
            raise
        finally:
            base.write_json(out / f"{name}.json", {
                "argv": cmd, "pid": child.pid, "cwd": str(directory), "exit": child.returncode,
                "started_utc": started, "finished_utc": base.utc(),
                "stdout": base.identity(stdout), "stderr": base.identity(stderr),
            })
    base.require(child.returncode == 0 and not stderr.read_bytes(), f"候选运行失败：{name}")
    data = stdout.read_bytes()
    if request is None:
        return data
    hello, reply = frames(data)
    base.require(hello["accepted"] and tuple(hello["versions"][key] for key in ("runtime_abi", "codegen")) == SUPPORTED
                 and reply["request_id"] == request[-1]["request_id"], f"核心握手或响应身份不符：{name}")
    return reply


def write_wrapper(task, settings, env, runtime, manifest, batch):
    source = task / "tmp" / f"coff-clang-{batch}.c"
    with source.open("x", encoding="utf-8", newline="\n") as output_file:
        output_file.write(WIN_LINK_WRAPPER)
    compiler = settings["tools"]["clang"]["path"]
    wrapper = task / f"coff-clang-{batch}.exe"
    argv = (compiler, "-target", settings["target"], str(source), "-O2", "-o", str(wrapper))
    base.require(capture(task, f"coff-wrapper-{batch}", argv, env, runtime, manifest) == 0, "同链接取证代理构建失败")
    return wrapper


def map_symbols(path):
    text = path.read_text(encoding="utf-8", errors="replace")
    rows = re.findall(r"(?m)^\s*([0-9a-fA-F]{4}:[0-9a-fA-F]{8})\s+"
                      r"(xiao_runtime_\w+)\s+([0-9a-fA-F]+)\s+[^\r\n]*$", text)
    base.require(rows and "Publics by Value" in text and "Preferred load address is" in text, "PE 链接映射缺失")
    return sorted({name for _, name, _ in rows})


def candidate_rows(task, source, runtime, env, settings, core, manifest, batch, only):
    host = settings["host"]
    kind, suffix = VERSIONS[host]
    before = base.source_identity(source, manifest)
    core_id, rt_id = base.identity(core), base.identity(runtime)
    tools = settings["tools"]
    env = env | {"PATH": str(Path(tools["linker"]["path"]).parent) + os.pathsep + env["PATH"]}
    clang = tools["clang"]["path"]
    wrapper = write_wrapper(task, settings, env, runtime, manifest, batch) if host == "windows" else None
    if wrapper:
        env["XIAO_C_REAL_CLANG"] = clang
    protocol = source / PROTOCOL
    hello = json.loads((protocol / "hello-request.json").read_bytes())
    run = json.loads((protocol / "localized-run-request.json").read_bytes())
    run["target"].update(triple=settings["target"], object_format=kind)
    run["options"]["timeout_ms"] = 60_000
    protocol_versions = {name: run[name] for name in ("protocol_version", "core_version", "locale")}
    fixture_root = source / FIXTURES
    inputs = [(name, fixture_root / f"{name}.xiao", fixture_root / f"{name}.stdout") for name in (
        "cmath-library", "b2_explicit_divide", "a5_decimal_arithmetic", "a5_conversions",
    )]
    inputs.append(("cmath-tutorial", ROOT / "cmath-tutorial.xiao", ROOT / "cmath-tutorial.stdout"))
    if only:
        inputs = [entry for entry in inputs if entry[0] in only]
        base.require(len(inputs) == len(only), "候选示例选择缺失或重复")
    rows = {}
    evidence = task / "evidence" / batch
    evidence.mkdir()
    for name, fixture, expected_file in inputs:
        folder = evidence / name
        folder.mkdir()
        cache = task / "home" / batch / name
        cache.parent.mkdir(exist_ok=True)
        cache.mkdir()
        temp = task / "tmp" / batch / name
        temp.parent.mkdir(exist_ok=True)
        temp.mkdir()
        process_env = env | {"XIAO_HOME": str(cache), **{key: str(temp) for key in ("TEMP", "TMP", "TMPDIR")}}
        code = fixture.read_bytes()
        expected = expected_file.read_text(encoding="utf-8")
        app = folder / "program.xiao"
        app.write_bytes(code)
        request = dict(run, request_id=f"{name}-source", source={"module": "main", "path": str(app),
                                                                "text": code.decode("utf-8")})
        result = invoke(core, (), folder, process_env, folder, "vm", request=(hello, request))
        base.require(result["exit_code"] == 0 and output(result) == expected, f"VM 黄金失败：{name} {result}")
        archive_file = folder / "program.xar"
        request_archive = {**protocol_versions, "type": "build_archive", "request_id": f"{name}-archive",
                           "path": str(app), "output": str(archive_file), "optimization": run["optimization"]}
        archive = invoke(core, (), folder, process_env, folder, "archive", request=(hello, request_archive))
        base.require(archive["exit_code"] == 0 and archive_file.is_file(), f"归档构建失败：{name}")
        cache_id = result["cache"]["digest"]
        base.require(result["cache"]["verified"] and result["cache"]["status"] == "written", f"字节码物化失败：{name}")
        matching = [path for path in (*cache.rglob("*.xiaoc"), *folder.rglob("*.xiaoc"))
                    if base.identity(path)["sha256"] == cache_id]
        base.require(len(matching) == 1, f"字节码物化身份不唯一：{name}")
        xiaoc = folder / "program.xiaoc"
        xiaoc.write_bytes(matching[0].read_bytes())
        native_rows = []
        native_request = request
        if name == "cmath-library":
            # B4 的原生公开面为选择导入。模块值语料仍原样用于 VM 与两种可移植产物。
            symbols = ("sqrt", "floor", "ceil", "round", "trunc", "abs", "min", "max",
                       "pow", "sin", "cos", "tan", "exp", "ln", "log10")
            native_code = code.decode("utf-8").removeprefix("import std.cmath as math\n")
            for index, symbol in enumerate(symbols):
                native_code = native_code.replace(f"math.{symbol}(", f"f{index}(")
            base.require("math." not in native_code, "原生选择导入语料不完整")
            native_request = dict(request, source=dict(request["source"], text=native_code))
            (folder / "native-input.xiao").write_bytes(native_code.encode("utf-8"))
        for level in (0, 3):
            executable = folder / (f"native-o{level}" + suffix)
            llvm = folder / f"native-o{level}.ll"
            spec = {"clang": str(wrapper or clang), "llvm_as": tools["llvm-as"]["path"],
                    "llc": tools["llc"]["path"], "runtime_library": str(runtime), "rustc": tools["rustc"]["path"],
                    "native_static_libraries": [],
                    "versions": {"clang": tools["clang"]["version"], "llvm_as": tools["llvm-as"]["version"],
                                 "llc": tools["llc"]["version"], "rustc": tools["rustc"]["version"]}}
            build = dict(native_request, type="build", request_id=f"{name}-o{level}", output=str(executable),
                         llvm_ir_output=str(llvm), optimization=dict(run["optimization"], level=level), toolchain=spec)
            build.pop("options")
            status = invoke(core, (), folder, process_env, folder, f"build-o{level}", request=(hello, build))
            base.require(status["exit_code"] == 0, f"原生候选构建失败：{name}/O{level} {status}")
            artifact = status["artifact"]
            own = base.identity(executable)
            base.require(artifact["optimization_level"] == level and artifact["object_digest"] == own["sha256"],
                         f"原生候选身份不符：{name}/O{level}")
            linked = artifact["artifact_runtime"]
            if host == "windows":
                link_map = executable.with_suffix(".exe.map")
                observed = map_symbols(link_map)
                # 映射来自产生这份 PE 的同一链接器进程，要求所有直接调用的 Runtime 函数和可见导出存在。
                direct = set(re.findall(rb"\bcall\b[^\n]*?@(xiao_runtime_\w+)\(", llvm.read_bytes()))
                missing = {name.decode() for name in direct} | set(linked["runtime_symbols"])
                missing -= set(observed)
                base.require(not missing, f"同链接映射缺少符号：{name}/O{level} {sorted(missing)}")
                (folder / f"coff-o{level}.json").write_text(json.dumps({
                    "map": base.identity(link_map), "exe": own, "direct": sorted(value.decode() for value in direct),
                    "actual_runtime_symbols": observed, "product_verification": linked["verification"],
                }, indent=2) + "\n", encoding="utf-8")
            else:
                base.require(linked["verification"] == "complete"
                             and linked["observed_components"] == linked["declared_components"],
                             f"原生候选组件不完整：{name}/O{level}")
            observed = invoke(executable, (), folder, process_env, folder, f"native-o{level}")
            base.require(observed.decode("utf-8") == expected and base.identity(executable) == own,
                         f"原生候选黄金或实际字节不符：{name}/O{level}")
            native_rows.append({"optimization": level, "executable": own, "llvm": base.identity(llvm),
                                "verification": linked["verification"]})

        app.unlink()
        for label, path in (("xiaoc", xiaoc), ("archive", archive_file)):
            command = {**protocol_versions, "type": "run_xiaoc" if label == "xiaoc" else "run_archive",
                       "request_id": f"{name}-{label}-replay", "path": str(path), "options": run["options"]}
            status = invoke(core, (), folder, process_env, folder, f"{label}-replay", request=(hello, command))
            base.require(status["exit_code"] == 0 and output(status) == expected,
                         f"删源回放失败：{name}/{label} {status}")
        rows[name] = {"source": base.identity(fixture), "golden": base.identity(expected_file),
                      "xiaoc": base.identity(xiaoc), "xar": base.identity(archive_file),
                      "source_removed": not app.exists(), "native": native_rows}
        base.write_json(folder / "result.json", rows[name])
        print(f"{name}: VM、双归档、O0/O3 黄金通过", flush=True)
    base.require(before == base.source_identity(source, manifest) and rt_id == base.identity(runtime)
                 and core_id == base.identity(core), "候选执行期间产品、核心或 Runtime 变化")
    base.write_json(evidence / "result.json", {"source": before, "host": host, "target": settings["target"],
                                             "core": core_id, "runtime": rt_id, "fixtures": rows})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", type=Path, required=True)
    parser.add_argument("--llvm", type=Path, required=True)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--existing", action="store_true", help="在同一固定 Runtime 上补测，仍使用新回执目录")
    parser.add_argument("--batch", default="candidates")
    parser.add_argument("--only", action="append")
    args = parser.parse_args()
    task = args.task.resolve()
    base.require(re.fullmatch(r"[a-z][a-z0-9-]*", args.batch) is not None, "候选回执目录名不合法")
    if args.existing:
        source = task / "source"
        manifest = (task / "product.tree").read_bytes()
        settings = json.loads((task / "evidence/environment.json").read_bytes())
        runtime = Path(settings["overrides"]["XIAO_RUNTIME_LIBRARY"])
        env = {name: value for name, value in os.environ.items()
               if not name.startswith(("XIAO_", "RUST_BIGDECIMAL_"))}
        env.update(settings["overrides"])
        suffix = VERSIONS[settings["host"]][1]
        candidate_rows(task, source, runtime, env, settings, task / "target/release" / ("xiao-core" + suffix),
                       manifest, args.batch, args.only)
        return 0
    source, manifest, exported = base.exported(task, args.archive, args.manifest)
    settings = {"source": base.source_identity(source, manifest)}
    runtime, env = base.setup(task, source, settings, args.llvm, exported)
    base.write_json(task / "evidence/environment.json", settings)
    cargo = settings["tools"]["cargo"]["path"]
    build = (cargo, "build", *base.LOCKED, "--release")
    dest = task / "target/release"
    suffix = VERSIONS[settings["host"]][1]
    for label, argv in (("runtime", (*build, "-p", "xiao-runtime")),
                        ("core", (*build, "-p", "xiao-driver", "--bin", "xiao-core"))):
        if capture(task, label, argv, env, runtime, manifest):
            return 1
    candidate_rows(task, source, runtime, env, settings, dest / ("xiao-core" + suffix), manifest, args.batch, args.only)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
