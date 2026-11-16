"""显式采集定稿文档门禁；默认只读审计由 audit.py 提供。"""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from audit import (ROOT, REPOSITORY, PAST, check, docs_snapshot, inventory,
                   load, record_module, require, self_negative, sha)


def write_json(path, value):
    with path.open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2) + chr(10)).encode("utf-8"))


def store(raw):
    digest = sha(raw)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == raw, "封存内容对象损坏")
    else:
        with path.open("xb") as output:
            output.write(raw)
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": len(raw), "sha256": digest}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seal", action="store_true", required=True)
    parser.parse_args()
    outputs = ["docs-final.json", "review.json", "evidence.json", "self-negative.json"]
    require(not any((ROOT / name).exists() for name in outputs), "封存输出不可覆盖")
    result = check(sealed=False)
    record = record_module()
    commit = record.git(REPOSITORY, "rev-parse", "HEAD").decode().strip()
    documents_before = docs_snapshot(record)
    require(documents_before == docs_snapshot(record, commit), "文档与工具须先提交定稿")
    source_before = record.worktree_files(REPOSITORY)
    require(source_before == load(REPOSITORY / PAST / "final3/source.json")["files"],
            "对照后产品树已改变")
    config = load(REPOSITORY / PAST / "final3/environment.json")
    argv = ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"]
    started = datetime.now(timezone.utc).isoformat()
    with tempfile.TemporaryDirectory(prefix="b5-watchdog-docs-") as temporary:
        with (Path(temporary) / "output.log").open("xb") as output:
            process = subprocess.Popen(argv, cwd=REPOSITORY, env=record.controlled(config),
                                       stdout=output, stderr=subprocess.STDOUT)
            status = process.wait()
        raw_log = (Path(temporary) / "output.log").read_bytes()
    documents_after = docs_snapshot(record)
    source_after = record.worktree_files(REPOSITORY)
    write_json(ROOT / "docs-final.json", {
        "argv": argv, "cwd": str(REPOSITORY), "pid": process.pid,
        "started_utc": started, "finished_utc": datetime.now(timezone.utc).isoformat(),
        "exit_code": status, "tools_commit": commit,
        "runner_sha256": sha(Path(__file__).read_bytes()),
        "source_before": record.identity(source_before), "source_after": record.identity(source_after),
        "source_unchanged": source_before == source_after,
        "docs_before": record.identity(documents_before), "docs_after": record.identity(documents_after),
        "docs_unchanged": documents_before == documents_after,
        "documents": store((json.dumps(documents_before, ensure_ascii=False, sort_keys=True)
                            + chr(10)).encode("utf-8")),
        "log": store(raw_log),
    })
    gate = json.loads(raw_log)
    require(status == 0 and gate["passed"] is True and source_before == source_after
            and documents_before == documents_after, "文档门禁失败或输入发生变化；回执已保留")
    with tempfile.TemporaryDirectory(prefix="b5-watchdog-seal-") as temporary:
        copy = Path(temporary) / "evidence"
        shutil.copytree(ROOT, copy)
        write_json(copy / "evidence.json", {"files": inventory(copy)})
        proof = self_negative(copy)
    write_json(ROOT / "self-negative.json", proof)
    result["docs_gate_passed"] = True
    result["self_negative_passed"] = True
    write_json(ROOT / "review.json", result)
    write_json(ROOT / "evidence.json", {"files": inventory(ROOT)})
    check()
    print(json.dumps({"evidence_sealed": True, "b5_closed": False, "b6_started": False,
                      "files": len(inventory(ROOT))}, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
