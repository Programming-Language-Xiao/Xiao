"""在相同 A0 工具环境中复跑文档门禁，记录依赖路径和完整输出。"""

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", action="store_true", help="检查收口文档所在主工作区")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    bootstrap = json.loads((root / "bootstrap.json").read_text(encoding="utf-8"))
    setup = json.loads((root / "environment.json").read_text(encoding="utf-8"))
    source = Path(bootstrap["source_root"])
    repository = Path(bootstrap["repository_root"])
    name = "workspace-doc-gate" if args.workspace else "repo-all-retry"
    assert not (root / (name + ".json")).exists(), "禁止覆盖历史运行"
    # Bun 安装的 typescript 符号链接相对指向根 node_modules/.bun，两个层级都必须齐备。
    node_root = source / "node_modules"
    node_target = repository / "node_modules"
    if not node_root.exists():
        subprocess.run(["cmd", "/c", "mklink", "/J", str(node_root), str(node_target)], check=True)
    assert node_root.resolve() == node_target.resolve()
    env = {key: value for key, value in os.environ.items() if not key.startswith("XIAO_")}
    env.update(setup["msvc"])
    env.update(setup["overrides"])
    commands = json.loads((root / "commands.json").read_text(encoding="utf-8"))
    original = next(item for item in commands if item["name"] == "repo-all")
    cwd = repository if args.workspace else source
    record = {
        "name": name, "argv": original["argv"], "cwd": str(cwd),
        "environment": "environment.json", "node_root_junction": str(node_root),
        "node_root_target": str(node_target), "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "log": "logs/" + name + ".log",
        "reason": "主工作区最终文档检查" if args.workspace else "补齐 Bun 相对符号链接所需根 node_modules 后复跑；原失败保留在 repo-all.log",
    }
    start = time.monotonic()
    with (root / record["log"]).open("wb") as output:
        process = subprocess.Popen(record["argv"], cwd=cwd, env=env, stdout=output, stderr=subprocess.STDOUT)
        record["pid"] = process.pid
        record["exit_code"] = process.wait()
    record.update(finished_utc=dt.datetime.now(dt.timezone.utc).isoformat(),
                  duration_seconds=round(time.monotonic() - start, 3), passed=record["exit_code"] == 0,
                  sha256=hashlib.sha256((root / record["log"]).read_bytes()).hexdigest())
    (root / (name + ".json")).write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(record, ensure_ascii=False, indent=2))
    return record["exit_code"]


if __name__ == "__main__":
    raise SystemExit(main())
