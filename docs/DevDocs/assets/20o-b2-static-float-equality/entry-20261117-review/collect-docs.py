"""采集一次最终文档门禁；绑定DevDocs字节并按内容摘要去重。"""
import argparse
from datetime import datetime, timezone
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def identity(raw):
    return {"bytes": len(raw), "sha256": sha(raw)}


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def inputs():
    paths = set((REPOSITORY / "docs/DevDocs").rglob("*.md"))
    for directory in ("docs/UseDocs", "tools/repo-check/src", "stdlib"):
        paths.update(path for path in (REPOSITORY / directory).rglob("*") if path.is_file())
    paths.update(path for path in ROOT.iterdir()
                 if path.is_file() and path.suffix in (".py", ".rs"))
    paths.update(REPOSITORY / name for name in (
        "package.json", "bun.lock", "docs/module-registry.json"))
    paths.add(ROOT / ".gitattributes")
    return {path.relative_to(REPOSITORY).as_posix(): identity(path.read_bytes())
            for path in sorted(paths)}


def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == packed, "文档内容对象损坏")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {
        "path": path.relative_to(ROOT).as_posix(), **identity(packed),
        "encoding": "gzip" if len(raw) > 8192 else "identity",
        "raw_bytes": len(raw), "raw_sha256": sha(raw),
    }


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--label", default="", help="本轮唯一后缀，仅允许ASCII字母、数字和连字符")
args = parser.parse_args()
require(not args.label or all(char in "abcdefghijklmnopqrstuvwxyz0123456789-"
                              for char in args.label), "文档回执后缀非法")
suffix = "-" + args.label if args.label else ""
receipt_path = ROOT / ("docs-gate" + suffix + ".json")
require(not receipt_path.exists(), "最终文档回执禁止覆盖")
build = json.loads((ROOT / "probe-build.json").read_text(encoding="utf-8"))
task = Path(build["task_root"])
require(task.is_dir(), "原探针任务根不存在")
bun = shutil.which("bun")
require(bun is not None, "缺少bun，不能跳过门禁")
argv = [bun, "tools/repo-check/src/cli.ts", "all"]
before = inputs()
started = datetime.now(timezone.utc).isoformat()
environment = dict(os.environ)
environment.update(PYTHONUTF8="1", PYTHONIOENCODING="utf-8")
log_path = task / ("final-docs" + suffix + ".log")
with log_path.open("xb") as log:
    child = subprocess.Popen(argv, cwd=REPOSITORY, env=environment,
                             stdin=subprocess.DEVNULL,
                             stdout=log, stderr=subprocess.STDOUT)
    status = child.wait()
finished = datetime.now(timezone.utc).isoformat()
after = inputs()
raw_log = log_path.read_bytes()
receipt = {
    "scope": "本批最终文档和仓库布局门禁，不是产品验收",
    "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
    "started_utc": started, "finished_utc": finished, "exit_code": status,
    "tool": {"path": bun, **identity(Path(bun).read_bytes())},
    "source_before": sha(canonical(before)),
    "source_after": sha(canonical(after)), "inputs_unchanged": before == after,
    "inputs": before, "log": store(raw_log), "repair_applied": False,
}
with receipt_path.open("xb") as output:
    output.write((json.dumps(receipt, ensure_ascii=False, indent=2) + chr(10)).encode("utf-8"))
print(json.dumps({
    "exit_code": status, "inputs_unchanged": before == after, "input_files": len(before),
    "log": receipt["log"], "tail": raw_log.decode("utf-8", errors="replace")[-2500:],
}, ensure_ascii=False))
require(status == 0, "最终文档门禁失败，真实日志已保留")
require(before == after, "门禁运行期间受测文档或工具发生变化")
