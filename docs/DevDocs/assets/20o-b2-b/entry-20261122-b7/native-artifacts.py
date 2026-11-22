"""保留并核对 B1 七库通过普通原生入口生成的 O0/O3 产物身份。"""

import argparse
import json
import os
from pathlib import Path
import subprocess

from portable import FORMATS, LIBRARIES, frame, messages
from run import PRODUCT, identity, require, source_identity, utc, write_json


def collect(task, output):
    source = task / "source"
    manifest = (task / "product.tree").read_bytes()
    settings = json.loads((task / "evidence/environment.json").read_bytes())
    before = source_identity(source, manifest)
    require(before == settings["source"], "原生取证产品身份不符")
    object_format, suffix = FORMATS[settings["host"]]
    core = task / "target/debug" / ("xiao-core" + suffix)
    core_id = identity(core)
    runtime = Path(settings["overrides"]["XIAO_RUNTIME_LIBRARY"])
    runtime_id = identity(runtime)
    protocol = source / "tests/spec/11x0-protocol"
    hello = json.loads((protocol / "hello-request.json").read_bytes())
    template = json.loads((protocol / "localized-run-request.json").read_bytes())
    template.pop("options")
    template["target"].update(triple=settings["target"], object_format=object_format)
    tools = settings["tools"]
    template["toolchain"] = {
        "clang": tools["clang"]["path"], "llvm_as": tools["llvm-as"]["path"], "llc": tools["llc"]["path"],
        "rustc": tools["rustc"]["path"], "runtime_library": str(runtime), "native_static_libraries": [],
        "versions": {"clang": tools["clang"]["version"], "llvm_as": tools["llvm-as"]["version"],
                     "llc": tools["llc"]["version"], "rustc": tools["rustc"]["version"]},
    }
    template["type"] = "build"
    env = {name: value for name, value in os.environ.items() if not name.startswith(("XIAO_", "RUST_BIGDECIMAL_"))}
    env.update(settings["overrides"])
    env["PATH"] = os.pathsep.join((str(Path(tools["linker"]["path"]).parent),
                                   str(Path(tools["clang"]["path"]).parent),
                                   str(Path(tools["rustc"]["path"]).parent), env["PATH"]))
    root = task / "native-artifact-observations"
    root.mkdir()
    output.mkdir()
    for module in LIBRARIES:
        fixture = source / "tests/fixtures/20o-b1/imp" / module
        code = (fixture / "product.xiao").read_text(encoding="utf-8")
        expected = json.loads((fixture / "golden.json").read_bytes())["stdout"].encode("utf-8")
        for level in (0, 3):
            case = f"{module}-o{level}"
            folder = root / case
            folder.mkdir()
            home = folder / "home"
            home.mkdir()
            step_env = env | {name: str(folder) for name in ("TEMP", "TMP", "TMPDIR")}
            step_env["XIAO_HOME"] = str(home)
            executable = folder / ("program" + suffix)
            llvm = folder / "program.ll"
            request = dict(template, request_id=case, output=str(executable), llvm_ir_output=str(llvm),
                           source={"module": "main", "path": str(folder / "main.xiao"), "text": code},
                           optimization=dict(template["optimization"], level=level))
            (folder / "main.xiao").write_text(code, encoding="utf-8", newline="\n")
            started = utc()
            with subprocess.Popen([core], cwd=folder, env=step_env, stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
                wire, stderr = process.communicate(frame(hello) + frame(request), timeout=60)
            handshake, response = messages(wire)
            require(process.returncode == 0 and not stderr and handshake["accepted"] and response["exit_code"] == 0
                    and response["request_id"] == case, f"B1 原生产物构建失败：{case} {response}")
            artifact = response["artifact"]
            executable_id = identity(executable)
            require(artifact["object_digest"] == executable_id["sha256"] and artifact["optimization_level"] == level,
                    f"B1 原生产物身份不符：{case}")
            run_started = utc()
            with subprocess.Popen([executable], cwd=folder, env=step_env, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE) as child:
                stdout, stderr = child.communicate(timeout=60)
            require(child.returncode == 0 and stdout == expected and not stderr, f"B1 原生黄金不符：{case}")
            require(identity(executable) == executable_id, f"B1 原生执行期间产物发生变化：{case}")
            (output / f"{case}.stdout").write_bytes(stdout)
            write_json(output / f"{case}.json", {
                "module": module, "optimization": level, "cwd": str(folder), "request": request,
                "artifact": artifact, "build": {"argv": [str(core)], "pid": process.pid, "started_utc": started,
                                                "finished_utc": run_started, "exit": process.returncode},
                "run": {"argv": [str(executable)], "pid": child.pid, "started_utc": run_started,
                        "finished_utc": utc(), "exit": child.returncode},
                "executable": executable_id, "llvm_ir": identity(llvm),
            })
            print(f"{case} 原生黄金通过", flush=True)
    require(identity(core) == core_id and identity(runtime) == runtime_id
            and source_identity(source, manifest) == before, "原生取证期间产品、核心或 Runtime 变化")
    write_json(output / "identity.json", {"product": PRODUCT, "source": before, "runtime": runtime_id,
                                         "core": {"path": str(core), **core_id}, "host": settings["host"]})


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("task", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    collect(args.task.resolve(strict=True), args.output.resolve())
