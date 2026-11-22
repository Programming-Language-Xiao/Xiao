"""用隔离 xiao-core 重生成和重放 B1 七库的可移植产物。"""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from run import PRODUCT, identity, require, source_identity, utc, write_json

LIBRARIES = ("math", "statistics", "fmt", "hex", "base64", "url", "csv")
FORMATS = {"windows": ("coff", ".exe"), "linux": ("elf", ""), "macos": ("macho", "")}


def frame(message):
    raw = json.dumps(message, ensure_ascii=False).encode("utf-8")
    return len(raw).to_bytes(8, "big") + raw


def messages(wire):
    while wire:
        count = int.from_bytes(wire[:8], "big")
        require(0 < count <= len(wire) - 8, "核心响应帧不完整")
        yield json.loads(wire[8:8 + count])
        wire = wire[8 + count:]


def observed_stdout(message):
    return "".join(event["data"]["text"] for event in message["events"] if event["kind"] == "intrinsic_output")


def collect(task, output):
    source = task / "source"
    manifest = (task / "product.tree").read_bytes()
    settings = json.loads((task / "evidence/environment.json").read_bytes())
    require(source_identity(source, manifest) == settings["source"], "回放源码与本批身份不符")
    object_format, suffix = FORMATS[settings["host"]]
    core = task / "target/debug" / ("xiao-core" + suffix)
    runtime = Path(settings["overrides"]["XIAO_RUNTIME_LIBRARY"])
    runtime_before = identity(runtime)
    core_before = identity(core)
    fixture = source / "tests/spec/11x0-protocol"
    hello = json.loads((fixture / "hello-request.json").read_bytes())
    template = json.loads((fixture / "localized-run-request.json").read_bytes())
    template["target"].update(triple=settings["target"], object_format=object_format)
    template["options"]["timeout_ms"] = 60_000
    common = {name: template[name] for name in ("protocol_version", "core_version", "locale")}
    output.mkdir()
    work = task / output.name
    work.mkdir()
    rows = {}
    base_env = {k: v for k, v in os.environ.items() if not k.startswith(("XIAO_", "RUST_BIGDECIMAL_"))}
    for module in LIBRARIES:
        folder = work / module
        folder.mkdir()
        home = folder / "home"
        home.mkdir()
        env = base_env | settings["overrides"] | {name: str(folder) for name in ("TMP", "TEMP", "TMPDIR")}
        env["XIAO_HOME"] = str(home)
        test = source / "tests/fixtures/20o-b1/imp" / module
        fixture_source = test / "product.xiao"
        code = fixture_source.read_bytes()
        app = folder / "main.xiao"
        app.write_bytes(code)
        expected = json.loads((test / "golden.json").read_bytes())["stdout"]
        xar = folder / "program.xar"
        archive_request = {**common, "type": "build_archive", "request_id": "archive-build", "path": str(app),
                           "output": str(xar), "optimization": template["optimization"]}
        run = dict(template, request_id="source-run", source={"module": "main", "path": str(app),
                                                             "text": code.decode("utf-8")})

        def invoke(*requests):
            wire = b"".join(map(frame, (hello, *requests)))
            start = utc()
            with subprocess.Popen([core], cwd=folder, env=env, stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
                stdout, stderr = process.communicate(wire, timeout=180)
            handshake, *replies = messages(stdout)
            require(process.returncode == 0 and not stderr and handshake["accepted"] is True
                    and handshake["versions"]["runtime_abi"] == (1 << 16 | 19)
                    and handshake["versions"]["codegen"] == 22
                    and {row["request_id"] for row in replies} == {row["request_id"] for row in requests},
                    f"核心协议进程失败：{module}")
            require(all(row["exit_code"] == 0 for row in replies), f"产物构建或回放失败：{module} {replies}")
            return {"argv": [str(core)], "pid": process.pid, "cwd": str(folder), "started_utc": start,
                    "finished_utc": utc(), "exit": process.returncode,
                    "requests": list(requests), "responses": replies}

        build = invoke(run, archive_request)
        first = {row["request_id"]: row for row in build["responses"]}["source-run"]
        require(observed_stdout(first) == expected and first["cache"]["verified"] is True
                and first["cache"]["status"] == "written", f"源码执行或字节码物化失败：{module}")
        digest = first["cache"]["digest"]
        require(re.fullmatch("[0-9a-f]{64}", digest), f"字节码摘要无效：{module}")
        cache = [path for path in folder.rglob("*.xiaoc") if identity(path)["sha256"] == digest]
        require(len(cache) == 1 and xar.is_file(), f"新产物缺失或字节码来源不唯一：{module}")
        xiaoc = cache[0]
        app.unlink()
        replay = invoke(
            {**common, "type": "run_xiaoc", "request_id": "xiaoc-replay", "path": str(xiaoc),
             "options": template["options"]},
            {**common, "type": "run_archive", "request_id": "xar-replay", "path": str(xar),
             "options": template["options"]},
        )
        require(all(observed_stdout(row) == expected for row in replay["responses"]), f"回放黄金不符：{module}")
        for filename, path in (("xiaoc", xiaoc), ("xar", xar)):
            (output / f"{module}.{filename}").write_bytes(path.read_bytes())
        rows[module] = {"source": identity(fixture_source), "golden": identity(test / "golden.json"),
                        "xiaoc": identity(xiaoc), "xar": identity(xar), "source_removed": not app.exists(),
                        "build": build, "replay": replay}
        write_json(output / f"{module}.json", rows[module])
        print(f"{module} .xiaoc/.xar 黄金通过", flush=True)
    require(source_identity(source, manifest) == settings["source"] and identity(core) == core_before
            and identity(runtime) == runtime_before, "回放期间产品、核心或 Runtime 变化")
    write_json(output / "identity.json", {"product": PRODUCT, "source": settings["source"],
                                        "runtime": runtime_before, "core": {"path": str(core), **core_before},
                                        "platform": settings["host"], "target": settings["target"]})


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("task", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    collect(args.task.resolve(strict=True), args.output.resolve())
