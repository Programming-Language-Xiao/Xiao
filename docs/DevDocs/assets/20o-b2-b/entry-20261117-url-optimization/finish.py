"""文档定稿后单次采集门禁；按实际字节生成清单，与只读审计分离。"""
from datetime import datetime, timezone
import json
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True
import record

DOC_FILES = [
    "docs/DevDocs/" + name for name in (
        "20o-b2-b-implementation-plan.md", "20o-b2-b5-version-matrix-plan.md",
        "20o-b2-b6-resource-performance-plan.md", "20o-b2-b7-closeout-plan.md",
        "20o-b2-c-final-closeout-plan.md", "20o-official-library-program.md", "README.md",
    )
]


def docs_source():
    paths = [record.REPOSITORY / name for name in DOC_FILES]
    paths += [path for path in record.ROOT.iterdir() if path.is_file()
              and path.name not in ("evidence.json", "docs-final.json")]
    return {
        path.relative_to(record.REPOSITORY).as_posix(): {
            "bytes": len(raw := path.read_bytes()), "sha256": record.sha(raw),
            "lf_sha256": record.sha(raw.replace(b"\r\n", b"\n")),
        }
        for path in sorted(paths)
    }


def docs():
    receipt = record.ROOT / "docs-final.json"
    log = record.TASK / "docs-final.log"
    record.require(not receipt.exists() and not log.exists(), "文档门禁回执禁止覆盖")
    before, product_before = docs_source(), record.source()
    env, overrides, removed = record.environment()
    bun = shutil.which("bun")
    record.require(bun is not None, "找不到Bun")
    version = subprocess.run([bun, "--version"], capture_output=True, check=True,
                             encoding="utf-8", env=env).stdout.strip()
    argv = [bun, "tools/repo-check/src/cli.ts", "all", "--format", "json"]
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    with log.open("xb") as output:
        child = subprocess.Popen(argv, cwd=record.REPOSITORY, env=env,
                                 stdin=subprocess.DEVNULL, stdout=output,
                                 stderr=subprocess.STDOUT)
        code = child.wait()
    elapsed = time.monotonic() - clock
    finished = datetime.now(timezone.utc).isoformat()
    after, product_after = docs_source(), record.source()
    result = {
        "argv": argv, "cwd": str(record.REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished,
        "elapsed_seconds": elapsed, "exit_code": code,
        "environment_overrides": overrides, "removed_environment_names": removed,
        "bun": {"binary": record.identity(bun), "version": version},
        "docs_before": record.store(record.canonical(before)),
        "docs_after": record.store(record.canonical(after)),
        "product_before": record.store(record.canonical(product_before)),
        "product_after": record.store(record.canonical(product_after)),
        "source_unchanged": before == after and product_before == product_after,
        "log": record.store(log.read_bytes()),
    }
    record.write_new(receipt, result)
    print(json.dumps(result, ensure_ascii=False))
    record.require(result["source_unchanged"], "文档门禁运行中源码或文档变化")
    report = json.loads(log.read_text(encoding="utf-8"))
    record.require(report["passed"] is True and not any(
        item["severity"] == "error" for item in report["diagnostics"]),
        "文档门禁实际报告未通过")
    raise SystemExit(code)


def seal():
    import collect
    receipt = json.loads((record.ROOT / "docs-final.json").read_text(encoding="utf-8"))
    record.require(receipt["exit_code"] == 0 and receipt["source_unchanged"],
                   "最终文档门禁未通过")
    record.require(json.loads(collect.raw_object(receipt["docs_after"])) == docs_source(),
                   "文档门禁后定稿字节发生变化")
    record.require(json.loads(collect.raw_object(receipt["product_after"])) == record.source(),
                   "文档门禁后产品字节发生变化")
    files = {
        path.relative_to(record.ROOT).as_posix(): {
            "bytes": len(raw := path.read_bytes()), "sha256": record.sha(raw),
        }
        for path in sorted(record.ROOT.rglob("*"))
        if path.is_file() and path.name != "evidence.json"
    }
    record.require(".gitattributes" in files, "清单遗漏.gitattributes")
    record.write_new(record.ROOT / "evidence.json", {
        "file_count_excluding_manifest": len(files), "files": files,
        "generator": "finish.py seal", "docs_receipt": "docs-final.json",
    })
    print(json.dumps({"sealed": True, "files_excluding_manifest": len(files)}))


if __name__ == "__main__":
    record.require(len(sys.argv) == 2 and sys.argv[1] in ("docs", "seal"),
                   "用法：finish.py docs | seal")
    {"docs": docs, "seal": seal}[sys.argv[1]]()
