"""核对教程正文和黄金，采集本机 Xiao 解析器的实际结果。"""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import subprocess
import sys

sys.dont_write_bytecode = True
from audit_baseline import ROOT, REPO, encoded, identity, require


MODULES = ("fs", "env", "log")
INPUT_MODULES = ("env", "log")
PATHS = {module: f"tests/fixtures/docs/{module}/b3-0c-candidate" for module in MODULES}
SOURCES = [f"candidates/{PATHS[module]}.xiao" for module in MODULES]


def data(path):
    return json.loads(path.read_bytes())


def verify_examples(root):
    matrix = data(root / "tutorials.json")
    require(matrix["status"] == "planned" and matrix["executed"] is False
            and matrix["cases"].keys() == PATHS.keys() and matrix["candidate_root"] == "candidates"
            and matrix["module_aliases"] == ["std", "stdlib"] and matrix["native_optimizations"] == [0, 3]
            and matrix["platforms"] == ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu", "aarch64-apple-darwin"]
            and matrix["expected_cargo_counts"] == [1, 0, 0] and matrix["expected_exit"] == 0
            and matrix["windows_product_coff_policy"] == "complete_required"
            and matrix["windows_product_coff_status"] == "pending", "教程精确入口或执行状态不符")
    snapshots = {}
    for module, prefix in PATHS.items():
        page = (root / "tutorials" / f"{module}.md").read_bytes()
        snapshots[f"tutorials/{module}.md"] = identity(page)
        for ext, language in (("xiao", "xiao"), ("stdout", "text")):
            path = f"candidates/{prefix}.{ext}"
            raw = (root / path).read_bytes()
            fence = b"```"
            require(re.findall(rb"(?m)^" + fence + language.encode() + rb"\n(.*?)^" + fence + rb"$", page, re.S) == [raw],
                    f"教程代码块与固定夹具不符：{path}")
            snapshots[path] = identity(raw)
        if module in INPUT_MODULES:
            path = f"{prefix}.input.json"
            require(matrix["cases"][module]["input"] == path, f"合成运行输入未登记：{path}")
            snapshots[f"candidates/{path}"] = identity((root / "candidates" / path).read_bytes())

    env_path = root / "candidates" / f"{PATHS['env']}.input.json"
    clock_path = root / "candidates" / f"{PATHS['log']}.input.json"
    require(data(env_path)["environment"] == [["XIAO_B3_TUTORIAL_VALUE", "initial"]], "环境合成输入不符")
    _, initial = data(env_path)["environment"][0]
    seconds, millis = divmod(data(clock_path)["wall_clock_unix_ms"], 1000)
    stamp = datetime.fromtimestamp(seconds, timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")
    stamp = f"{stamp}.{millis:03}Z"
    goldens = {
        "fs": [*sorted(("z.txt", "a.txt"), key=str.encode), "FS_EXISTS", "1", str(len(b"hello")), "hello"],
        "env": [f"snapshot=[{initial}]", "empty=[]", "updated=[local]", "true", "false"],
        "log": [f"{stamp} INFO task started", "X06-RUNTIME-012", f"{stamp} ERROR task stopped"],
    }
    for module, lines in goldens.items():
        require((root / "candidates" / f"{PATHS[module]}.stdout").read_bytes() == ("\n".join(lines) + "\n").encode(),
                f"独立黄金复核失败：{module}")
    return snapshots


def command(argv, cwd):
    start = datetime.now(timezone.utc).isoformat()
    with subprocess.Popen(argv, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, encoding="utf-8") as process:
        try:
            stdout, stderr = process.communicate(timeout=60)
        except subprocess.TimeoutExpired:
            process.kill()
            process.communicate()
            raise
    return {"argv": argv, "pid": process.pid, "exit": process.returncode, "stdout": stdout, "stderr": stderr,
            "started_utc": start, "finished_utc": datetime.now(timezone.utc).isoformat()}


def verify_report(root, report):
    paths = verify_examples(root)
    require(report["examples"] == paths and report["product_execution"] is False, "教程回执的候选或执行状态不符")
    rows = report["commands"]
    expected = "".join(f"{path}: diagnostics=0\n" for path in SOURCES)
    require(set(rows) == {"rustc", "build", "parse"}
            and all(row["pid"] > 0 and row["exit"] == 0 and row["started_utc"] <= row["finished_utc"]
                    for row in rows.values())
            and "release: 1.96.0\n" in rows["rustc"]["stdout"]
            and "host: x86_64-pc-windows-msvc\n" in rows["rustc"]["stdout"]
            and rows["parse"]["argv"][1:] == SOURCES and rows["parse"]["stdout"] == expected
            and not rows["parse"]["stderr"], "教程解析回执不符")
    require(report["probe"] == identity((root / "tutorial_parse.rs").read_bytes())
            and set(report["libraries"]) == {"xiao_source", "xiao_syntax"}
            and all(entry["bytes"] > 0 and re.fullmatch(r"[a-f0-9]{64}", entry["sha256"])
                    and f"{key}={entry['path']}" in rows["build"]["argv"]
                    for key, entry in report["libraries"].items()), "教程解析器输入身份不符")
    return {"status": "planned", "parsed": len(SOURCES), "goldens": len(MODULES), "product_execution": False,
            "examples": paths}


def collect(deps, task, output):
    require(not task.is_relative_to(REPO), "探针任务目录须独立于仓库")
    task.mkdir()
    libraries = {}
    for crate in ("xiao_source", "xiao_syntax"):
        matches = list(deps.glob(f"lib{crate}-*.rlib"))
        require(len(matches) == 1, f"解析库身份不唯一：{crate}")
        path = matches[0]
        libraries[crate] = identity(path.read_bytes()) | {"path": str(path)}
    exe = str(task / "tutorial_parse.exe")
    version = command(["rustc", "+1.96.0", "--version", "--verbose"], ROOT)
    build = command(["rustc", "+1.96.0", "--edition=2024", "tutorial_parse.rs", "-L", f"dependency={deps}",
                     "--extern", f"xiao_source={libraries['xiao_source']['path']}",
                     "--extern", f"xiao_syntax={libraries['xiao_syntax']['path']}", "-o", exe], ROOT)
    require(build["exit"] == 0, f"教程探针编译失败：{build['stderr']}")
    parse = command([exe, *SOURCES], ROOT)
    report = {"cwd": str(ROOT), "probe": identity((ROOT / "tutorial_parse.rs").read_bytes()), "libraries": libraries,
              "executable": identity(Path(exe).read_bytes()), "examples": verify_examples(ROOT),
              "commands": {"rustc": version, "build": build, "parse": parse}, "product_execution": False}
    with output.open("xb") as receipt:
        receipt.write(encoded(report))
    return verify_report(ROOT, report)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--deps", type=Path, required=True)
    parser.add_argument("--task", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = collect(args.deps.resolve(), args.task.resolve(), args.output.resolve())
    print(encoded(report).decode(), end="")
