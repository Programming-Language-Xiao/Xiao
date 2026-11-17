"""文档定稿后运行唯一最终门禁，再按实际字节生成B6清单。"""
from datetime import datetime, timezone
import gzip
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode = True
import record

ROOT, REPOSITORY, TASK = record.ROOT, record.REPOSITORY, record.TASK
DOC_FILES = ["docs/DevDocs/" + name for name in (
    "20o-b2-b-implementation-plan.md", "20o-b2-b4-5-closeout-plan.md",
    "20o-b2-b5-version-matrix-plan.md", "20o-b2-b6-resource-performance-plan.md",
    "20o-b2-b7-closeout-plan.md", "20o-b2-c-final-closeout-plan.md",
    "20o-b2-numeric-foundation-plan.md", "20o-official-library-program.md", "README.md",
)]


def docs_source():
    paths = [REPOSITORY / name for name in DOC_FILES]
    paths += [
        path for path in ROOT.rglob("*")
        if path.is_file()
        and path.relative_to(ROOT).parts[0] != "objects"
        and not path.name.startswith("docs-final")
        and path.name != "evidence.json"
    ]
    result = {}
    for path in sorted(paths):
        raw = path.read_bytes()
        binary = path.suffix == ".bin" or b"\0" in raw
        result[path.relative_to(REPOSITORY).as_posix()] = {
            "bytes": len(raw), "sha256": record.sha(raw),
            "lf_sha256": record.sha(raw if binary else raw.replace(b"\r\n", b"\n")),
            "binary": binary,
        }
    return result


def docs_gate():
    receipt_path = ROOT / "docs-final-r8.json"
    log_path = TASK / "logs/docs-final-r8.log"
    record.require(not receipt_path.exists() and not log_path.exists(),
                   "B6最终文档门禁回执禁止覆盖")
    before_docs, before_product = docs_source(), record.source()
    env, overrides, removed = record.environment()
    bun = shutil.which("bun")
    record.require(bun is not None, "找不到Bun")
    version = subprocess.run([bun, "--version"], capture_output=True,
                             encoding="utf-8", check=True, env=env).stdout.strip()
    argv = [bun, "tools/repo-check/src/cli.ts", "all", "--format", "json"]
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    with log_path.open("xb") as output:
        child = subprocess.Popen(argv, cwd=REPOSITORY, env=env,
                                 stdin=subprocess.DEVNULL, stdout=output,
                                 stderr=subprocess.STDOUT)
        code = child.wait()
    elapsed = time.monotonic() - clock
    finished = datetime.now(timezone.utc).isoformat()
    after_docs, after_product = docs_source(), record.source()
    report = json.loads(log_path.read_text(encoding="utf-8"))
    record.require(code == 0 and report["passed"], "B6最终文档门禁失败")
    receipt = {
        "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished,
        "elapsed_seconds": elapsed, "exit_code": code,
        "environment_overrides": overrides, "removed_environment_names": removed,
        "bun": {"binary": record.identity(bun), "version": version},
        "docs_before": record.store(record.canonical(before_docs)),
        "docs_after": record.store(record.canonical(after_docs)),
        "product_before": record.store(record.canonical(before_product)),
        "product_after": record.store(record.canonical(after_product)),
        "source_unchanged": before_docs == after_docs and before_product == after_product,
        "log": record.store(log_path.read_bytes()),
    }
    record.write_new(receipt_path, receipt)
    record.require(receipt["source_unchanged"], "文档门禁运行前后源码或文档变化")
    print(json.dumps({"docs_passed": True, "pid": child.pid,
                      "elapsed_seconds": elapsed}, ensure_ascii=False))


def seal():
    value = json.loads((ROOT / "docs-final-r8.json").read_text(encoding="utf-8"))
    packed_docs = (ROOT / value["docs_after"]["path"]).read_bytes()
    record.require(value["exit_code"] == 0 and value["source_unchanged"]
                   and record.sha(packed_docs) == value["docs_after"]["sha256"]
                   and gzip.decompress(packed_docs) == record.canonical(docs_source()),
                   "B6文档门禁之后仍有改动")
    result = json.loads((ROOT / "result-final-r5.json").read_text(encoding="utf-8"))
    record.require(result["stage_passed"] is False, "B6阻断状态不可改成通过")
    files = {}
    for path in sorted(ROOT.rglob("*")):
        if path.is_file() and path.name != "evidence.json":
            raw = path.read_bytes()
            files[path.relative_to(ROOT).as_posix()] = {
                "bytes": len(raw), "sha256": record.sha(raw),
            }
    record.require(".gitattributes" in files, "B6清单遗漏.gitattributes")
    manifest = {
        "file_count_excluding_manifest": len(files), "files": files,
        "generated_from_actual_bytes": True,
        "generator": "finish.py seal", "docs_receipt": "docs-final-r8.json",
        "result": "result-final-r5.json",
    }
    manifest_path = ROOT / "evidence.json"
    temporary_path = ROOT / ".evidence.json.tmp"
    record.require(not temporary_path.exists(), "B6临时清单路径已存在")
    record.write_new(temporary_path, manifest)
    os.replace(temporary_path, manifest_path)
    print(json.dumps({"sealed_files_excluding_manifest": len(files)}))


if __name__ == "__main__":
    record.require(len(sys.argv) == 2 and sys.argv[1] in ("docs", "seal"),
                   "用法：finish.py docs | seal")
    {"docs": docs_gate, "seal": seal}[sys.argv[1]]()
