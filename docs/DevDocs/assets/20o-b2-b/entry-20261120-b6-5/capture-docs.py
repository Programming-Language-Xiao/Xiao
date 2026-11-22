"""在 B6-5 Linux 任务根复跑仓库门禁，记录间接 Cargo 的实际参数。"""

import json
import os
from pathlib import Path
import runpy
import sys

sys.dont_write_bytecode = True
from record import DOC_INPUTS, collect, emit, markdown_identity, require, text_identity

LOCKED_DOC_GATE = "repo-check-locked"
CARGO_WRAPPER = """import json
import os
from pathlib import Path
import runpy
import subprocess
import sys

prepare = runpy.run_path(str(Path.cwd() / "tools/gates/controlled-cargo.py"))["prepare"]
arguments, environment, _, _, _ = prepare(sys.argv[1:], os.environ)
argv = [environment["XIAO_B6_5_CARGO"], *arguments]
with subprocess.Popen(argv, env=environment) as child:
    row = {"argv": argv, "cwd": os.getcwd(), "pid": child.pid,
           "gate_pid": os.getppid(), "exit_code": child.wait()}
with open(environment["XIAO_B6_5_CARGO_LOG"], "a", encoding="utf-8") as log:
    log.write(json.dumps(row) + "\\n")
raise SystemExit(row["exit_code"])
"""


def documents(source):
    return {"files": {name: text_identity((source / name).read_bytes()) for name in DOC_INPUTS},
            "markdown": markdown_identity(source)}


def main():
    task, transfer = (Path(value).resolve(strict=True) for value in sys.argv[1:3])
    archive = Path(sys.argv[3]).resolve()
    source, logs = task / "source", task / LOCKED_DOC_GATE
    recipe = runpy.run_path(str(transfer / "capture-linux.py"))
    manifest = (transfer / "product.tree").read_bytes()
    setup = json.loads((task / "linux/environment.json").read_bytes())
    require(not archive.exists(), "复验须使用新归档")
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("XIAO_", "RUST_BIGDECIMAL_", "CARGO_PROFILE_"))}
    env.update(setup["environment"])
    tools = {name: setup["tools"][name] for name in ("bun", "cargo")}
    for name, tool in tools.items():
        require(recipe["identity"](Path(tool["path"])) == {key: tool[key] for key in ("bytes", "sha256")},
                f"定稿仓库门禁工具身份不符：{name}")
    logs.mkdir()
    bin_dir = logs / "bin"
    bin_dir.mkdir()
    wrapper = bin_dir / "cargo"
    wrapper.write_text(f"#!{sys.executable}\n{CARGO_WRAPPER}", encoding="utf-8")
    wrapper.chmod(0o755)
    env["XIAO_B6_5_CARGO"] = tools["cargo"]["path"]
    env["XIAO_B6_5_CARGO_LOG"] = str(logs / "cargo.log")
    path_prefix = [str(bin_dir), *(str(Path(tool["path"]).parent) for tool in tools.values())]
    env["PATH"] = os.pathsep.join([*path_prefix, env["PATH"]])
    before = documents(source)
    result = recipe["capture"](
        logs, LOCKED_DOC_GATE, [tools["bun"]["path"], "tools/repo-check/src/cli.ts", "all", "--format", "json"],
        source, env, Path(env["XIAO_RUNTIME_LIBRARY"]), lambda: recipe["check_source"](source, manifest),
    )
    after = documents(source)
    emit(logs / "docs-source.json", {"before": before, "after": after, "tools": tools,
                                    "wrapper": recipe["identity"](wrapper), "path_prefix": path_prefix})
    collect(archive, [("linux", logs)])
    require(before == after, "定稿仓库门禁修改了 DevDocs 输入")
    return result


if __name__ == "__main__":
    raise SystemExit(main())
