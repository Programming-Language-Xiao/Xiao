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
    name = "workspace-doc-gate" if args.workspace else "repo-all-local-copy"
    assert not (root / (name + ".json")).exists(), "禁止覆盖历史运行"
    module = source / "tools/doc-coverage/node_modules/typescript"
    assert module.resolve().is_relative_to(source.resolve()), "必须使用工作树内实体依赖"
    assert json.loads((module / "package.json").read_text(encoding="utf-8"))["version"] == "5.9.3"
    env = {key: value for key, value in os.environ.items() if not key.startswith("XIAO_")}
    env.update(setup["msvc"])
    env.update(setup["overrides"])
    commands = json.loads((root / "commands.json").read_text(encoding="utf-8"))
    original = next(item for item in commands if item["name"] == "repo-all")
    cwd = repository if args.workspace else source
    resolved_module = (cwd / "tools/doc-coverage/node_modules/typescript").resolve()
    record = {
        "name": name, "argv": original["argv"], "cwd": str(cwd),
        "environment": "environment.json", "physical_typescript_root": str(resolved_module),
        "dependency_identity": "javascript-dependency-local-copy.json", "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "log": "logs/" + name + ".log",
        "reason": "主工作区最终文档检查" if args.workspace else "工作树内实体复制同一 TypeScript 5.9.3 后复跑；前两次失败日志保留",
    }
    start = time.monotonic()
    with (root / record["log"]).open("wb") as output:
        process = subprocess.Popen(record["argv"], cwd=cwd, env=env, stdout=output, stderr=subprocess.STDOUT)
        record["pid"] = process.pid
        (root / (name + ".json")).write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        record["exit_code"] = process.wait()
    record.update(finished_utc=dt.datetime.now(dt.timezone.utc).isoformat(),
                  duration_seconds=round(time.monotonic() - start, 3), passed=record["exit_code"] == 0,
                  sha256=hashlib.sha256((root / record["log"]).read_bytes()).hexdigest())
    (root / (name + ".json")).write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(record, ensure_ascii=False, indent=2))
    return record["exit_code"]


if __name__ == "__main__":
    raise SystemExit(main())
