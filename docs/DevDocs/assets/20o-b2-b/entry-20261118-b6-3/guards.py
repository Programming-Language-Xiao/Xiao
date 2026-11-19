"""回放四条实际 B1 输出和同源派生的计数反例。"""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import sys

sys.dont_write_bytecode = True

from prepare import git, require, write_json
from run import job, observe_suite
from b2_b6_macos import sha


WORKFLOW = ".github/workflows/maintenance-regression.yml"
GUARD_SOURCE = "8385893f8b1805c369817fee0c4e5840df28a4b8"


def step(workflow, name):
    lines = workflow.splitlines()
    marker = f"      - name: {name}"
    require(lines.count(marker) == 1, f"维护工作流步骤不唯一：{name}")
    start = lines.index(marker) + 1
    end = next((i for i in range(start, len(lines)) if lines[i].startswith("      - ")), len(lines))
    body = lines[start:end]
    run = body.index("        run: |") + 1
    return "\n".join(line[10:] for line in body[run:] if line.startswith("          ")) + "\n"


def scripts():
    raw = git("show", f"{GUARD_SOURCE}:{WORKFLOW}")
    workflow = raw.decode("utf-8")
    shells = {"default": step(workflow, "Run B1 VM probe exactly once")}
    win = step(workflow, "Build runtime and run native gates")
    block = re.search(r"(?ms)^\$b1Output = cargo test.*?^\}", win)
    require(block is not None, "维护工作流 Windows B1 块缺失")
    shells["windows"] = "$ErrorActionPreference = 'Stop'\n" + block.group() + "\n"
    for platform in ("Linux", "macOS"):
        body = step(workflow, f"Build runtime and run {platform} native gates")
        block = re.search(r"(?ms)^run_exact_native_test\(\) \{.*?^\}", body)
        call = "run_exact_native_test b1_capabilities_match_native_golden_outputs 20o_b1_probe"
        require(block and call in body.splitlines(), f"维护工作流 {platform} B1 块缺失")
        shells[platform.lower()] = f"set -euo pipefail\n{block.group()}\n{call}\n"
    return raw, shells


def variants(source):
    text = source.decode("utf-8").replace("\r\n", "\n")
    matches = re.findall(r"^test result: .*", text, re.M)
    require(len(matches) == 1 and matches[0].startswith("test result: ok. 1 passed; 0 failed; 0 ignored;"),
            "回放来源不是唯一实际通过的 B1 测试")
    summary = matches[0]
    for name, replacement, exit_code in (
        ("actual", summary, 0),
        ("zero", summary.replace("1 passed", "0 passed"), 0),
        ("two", summary.replace("1 passed", "2 passed"), 0),
        ("ignored", summary.replace("1 passed", "0 passed").replace("0 ignored", "1 ignored"), 0),
        ("failed", summary.replace("ok.", "FAILED.").replace("1 passed", "0 passed").replace("0 failed", "1 failed"), 101),
        ("failed-exit", summary, 101),
        ("duplicate", summary + "\n" + summary, 0),
    ):
        yield name, text.replace(summary, replacement).encode("utf-8"), exit_code


def replay(bash, root, sources):
    pwsh = shutil.which("pwsh")
    require(pwsh, "缺少 PowerShell 7")
    workflow, bodies = scripts()
    root = root.resolve()
    root.mkdir()
    env = os.environ | {"PATH": str(bash.parent) + os.pathsep + os.environ["PATH"]}
    rows = {}
    for path, body in bodies.items():
        log = sources[path]
        raw = log.read_bytes()
        receipt = json.loads(log.with_suffix(".json").read_bytes())
        label = "b1-vm" if path == "default" else "b1-native"
        require(receipt["files"][log.name]["sha256"] == sha(raw) and observe_suite(label, receipt["exit"], raw)["status"]
                == "passed", f"守卫来源未通过：{path}")
        path_root = root / path
        path_root.mkdir()
        for case, sample, exit_code in variants(raw):
            folder = path_root / case
            folder.mkdir()
            (folder / "cargo-output.log").write_bytes(sample)
            if path == "windows":
                script = folder / "guard.ps1"
                mock = f"function cargo {{ Get-Content -LiteralPath 'cargo-output.log'; $global:LASTEXITCODE = {exit_code} }}\n"
                argv = [pwsh, "-NoProfile", "-File", str(script)]
            else:
                script = folder / "guard.sh"
                mock = f"cargo() {{ cat cargo-output.log; return {exit_code}; }}\n"
                argv = [str(bash), script.name]
            script.write_text(mock + body, encoding="utf-8", newline="\n")
            result, _, _ = job(argv, folder, env, "guard", timeout=20)
            accepted = result["exit"] == 0
            require(not result["timed_out"] and accepted == (case == "actual"),
                    f"维护工作流计数判断不符：{path}/{case}")
            rows[f"{path}/{case}"] = {"accepted": accepted, "expected": case == "actual",
                                     "exit": result["exit"], "source_sha256": sha(raw),
                                     "derived_input_sha256": sha(sample), "cargo_exit": exit_code}
    summary = {"passed": len(rows), "actual": 4, "derived": 24, "workflow_sha256": sha(workflow),
               "source_commit": GUARD_SOURCE, "rows": rows, "scope": "maintenance guard replay"}
    write_json(root / "summary.json", summary)
    print(f"B1 guard replay {len(rows)}/28; 4 actual outputs and 24 derived cases")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("bash", "output", "vm", "windows", "linux", "macos"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    replay(args.bash.resolve(strict=True), args.output, {
        "default": args.vm, "windows": args.windows, "linux": args.linux, "macos": args.macos,
    })


if __name__ == "__main__":
    main()
