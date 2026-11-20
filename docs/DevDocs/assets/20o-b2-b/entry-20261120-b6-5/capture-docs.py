"""在已构建的独立 Linux 源码根运行一次定稿仓库门禁。"""

import json
import os
from pathlib import Path
import runpy
import sys

sys.dont_write_bytecode = True
from record import DOC_INPUTS, emit, markdown_identity, require, text_identity


def documents(source):
    return {"files": {name: text_identity((source / name).read_bytes()) for name in DOC_INPUTS},
            "markdown": markdown_identity(source)}


def main():
    task, transfer = (Path(value).resolve(strict=True) for value in sys.argv[1:])
    source, logs = task / "source", task / "linux"
    recipe = runpy.run_path(str(transfer / "capture-linux.py"))
    manifest = (transfer / "product.tree").read_bytes()
    setup = json.loads((logs / "environment.json").read_bytes())
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("XIAO_", "RUST_BIGDECIMAL_", "CARGO_PROFILE_"))}
    env.update(setup["environment"])
    tools = {name: setup["tools"][name] for name in ("bun", "cargo")}
    for name, tool in tools.items():
        require(recipe["identity"](Path(tool["path"])) == {key: tool[key] for key in ("bytes", "sha256")},
                f"定稿仓库门禁工具身份不符：{name}")
    path_prefix = [str(Path(tool["path"]).parent) for tool in tools.values()]
    env["PATH"] = os.pathsep.join([*path_prefix, env["PATH"]])
    before = documents(source)
    result = recipe["capture"](
        logs, "repo-check-final", [tools["bun"]["path"], "tools/repo-check/src/cli.ts", "all", "--format", "json"],
        source, env, Path(env["XIAO_RUNTIME_LIBRARY"]), lambda: recipe["check_source"](source, manifest),
    )
    after = documents(source)
    emit(logs / "docs-source-final.json", {"before": before, "after": after, "tools": tools, "path_prefix": path_prefix})
    require(before == after, "定稿仓库门禁修改了 DevDocs 输入")
    return result


if __name__ == "__main__":
    raise SystemExit(main())
