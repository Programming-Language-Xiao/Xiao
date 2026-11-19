"""采集最终仓库文档门禁，并绑定完整产品源码树及DevDocs文本。"""
from datetime import datetime, timezone
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
TASK = Path(json.loads((ROOT / "environment.json").read_text(encoding="utf-8"))["task_root"])
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def identity(raw):
    return {"bytes": len(raw), "sha256": sha(raw)}


def git(*arguments):
    result = subprocess.run(["cmd", "/c", "git", "-C", str(REPOSITORY), *arguments],
                            capture_output=True, check=True)
    return result.stdout


def inputs():
    paths = set()
    product_names = git("ls-files", "-z", "--cached", "--others", "--exclude-standard",
                        "--", *PRODUCT_ROOTS).decode("utf-8").split(chr(0))
    paths.update(REPOSITORY / name for name in product_names if name)
    devdocs = REPOSITORY / "docs/DevDocs"
    paths.update(path for path in devdocs.rglob("*.md") if path.is_file())
    usedocs = REPOSITORY / "docs/UseDocs"
    paths.update(path for path in usedocs.rglob("*") if path.is_file())
    for directory in ("stdlib", "tools/repo-check/src"):
        base = REPOSITORY / directory
        paths.update(path for path in base.rglob("*") if path.is_file())
    paths.update(path for path in ROOT.iterdir()
                 if path.is_file() and path.suffix in (".py", ".rs", ".md"))
    paths.add(ROOT / ".gitattributes")
    result = {}
    for path in sorted(paths):
        if path.is_file():
            raw = path.read_bytes()
            result[path.relative_to(REPOSITORY).as_posix()] = {
                **identity(raw), "lf_sha256": sha(raw.replace(b"\r\n", b"\n")),
            }
    return result


def store(raw):
    packed = gzip.compress(raw, mtime=0) if len(raw) > 8192 else raw
    digest = sha(packed)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == packed, "文档日志内容对象摘要冲突")
    else:
        with path.open("xb") as output:
            output.write(packed)
    return {
        "path": path.relative_to(ROOT).as_posix(), **identity(packed),
        "encoding": "gzip" if len(raw) > 8192 else "identity",
        "raw_bytes": len(raw), "raw_sha256": sha(raw),
    }


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("label")
args = parser.parse_args()
require(args.label and all(char in "abcdefghijklmnopqrstuvwxyz0123456789-"
                           for char in args.label), "文档门禁标签非法")
receipt_path = ROOT / ("docs-gate-" + args.label + ".json")
require(not receipt_path.exists(), "文档门禁回执禁止覆盖")
before = inputs()
bun = subprocess.run(["where.exe", "bun"], capture_output=True, encoding="utf-8",
                     errors="strict", check=True).stdout.splitlines()[0]
environment = dict(os.environ)
environment.update(PYTHONUTF8="1", PYTHONIOENCODING="utf-8")
argv = [bun, "tools/repo-check/src/cli.ts", "all"]
started = datetime.now(timezone.utc).isoformat()
log_path = TASK / "logs" / ("docs-" + args.label + ".log")
with log_path.open("xb") as output:
    child = subprocess.Popen(argv, cwd=REPOSITORY, env=environment, stdin=subprocess.DEVNULL,
                             stdout=output, stderr=subprocess.STDOUT)
    status = child.wait()
finished = datetime.now(timezone.utc).isoformat()
after = inputs()
raw_log = log_path.read_bytes()
receipt = {
    "scope": "本次最终DevDocs、UseDocs与完整产品源码树门禁，不代表三平台或CI",
    "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
    "started_utc": started, "finished_utc": finished, "exit_code": status,
    "tool": {"path": bun, **identity(Path(bun).read_bytes())},
    "source_before": sha(canonical(before)), "source_after": sha(canonical(after)),
    "source_unchanged": before == after, "inputs": before, "log": store(raw_log),
}
with receipt_path.open("xb") as output:
    output.write((json.dumps(receipt, ensure_ascii=False, indent=2) + chr(10)).encode("utf-8"))
print(json.dumps({
    "label": args.label, "exit_code": status, "inputs_unchanged": before == after,
    "input_files": len(before), "log": receipt["log"],
    "tail": raw_log.decode("utf-8", errors="replace")[-1800:],
}, ensure_ascii=False))
require(status == 0, "最终文档门禁失败；回执和原日志已保留")
require(before == after, "文档门禁运行期间源码字节发生变化")
