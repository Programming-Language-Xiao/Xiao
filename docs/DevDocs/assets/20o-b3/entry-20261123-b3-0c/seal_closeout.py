"""采集 B3-0C 文档门禁，按实际字节生成收口状态和清单。"""

import argparse
from pathlib import Path
import shutil
import sys

sys.dont_write_bytecode = True
from audit_baseline import verify_worktree
from audit_closeout import (
    GATES, MANIFEST, REPO, ROOT, STATUS,
    add_anchor_arguments, derive, digest, document_snapshot, encoded, frozen_files, git, identity, inputs,
    inventory, require,
)
from check_tutorials import command


def checked_snapshot(checkout, commit):
    require(git("rev-parse", "HEAD", repo=checkout).strip().decode() == commit
            and not git("status", "--porcelain", "--untracked-files=all", repo=checkout),
            "文档门禁需要干净的固定检出")
    snapshot = document_snapshot(commit)
    for path, expected in snapshot["markdown"].items():
        raw = (checkout / path).read_bytes().replace(b"\r\n", b"\n")
        require(identity(raw) == expected, f"受检文档字节不符：{path}")
    return snapshot


def document_gates(checkout, commit, output):
    require(checkout != REPO and not output.is_relative_to(checkout), "文档门禁使用独立检出，回执放在检出之外")
    before = checked_snapshot(checkout, commit)
    executable = shutil.which("bun")
    require(executable is not None, "找不到 Bun")
    bun = Path(executable).resolve(strict=True)
    version = command([str(bun), "--version"], checkout)
    require(version["exit"] == 0, "无法取得 Bun 版本")
    tool = identity(bun.read_bytes()) | {"path": str(bun), "version": version["stdout"].strip()}
    checks, logs = [], {}
    for gate in GATES:
        row = command([str(bun), "run", gate], checkout)
        for stream in ("stdout", "stderr"):
            raw = row.pop(stream)
            sha = digest(raw.encode())
            logs.setdefault(sha, raw)
            row[f"{stream}_sha256"] = sha
        print(f"{gate} exit={row['exit']}", flush=True)
        checks.append(row)
    after = checked_snapshot(checkout, commit)
    with output.open("xb") as receipt:
        receipt.write(encoded({"cwd": str(checkout), "before": before, "after": after,
                               "bun": tool, "checks": checks, "logs": logs}))
    require(before == after and all(row["exit"] == 0 for row in checks), f"文档门禁失败：{output}")


def seal(args):
    verify_worktree()
    document, receipt, final = inputs(args.document_commit, args.receipt_commit)
    result = derive(ROOT, document, receipt, final, frozen_files(document))
    (ROOT / STATUS).write_bytes(encoded(result))
    files = inventory(ROOT)
    (ROOT / MANIFEST).write_bytes(encoded({"schema": 1, "files": files}))
    print(encoded({"audit_files": len(files), "b3_0c_passed": result["b3_0c_passed"],
                   "b3_0d_entry_ready": result["b3_0d_entry_ready"]}).decode(), end="")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_anchor_arguments(parser)
    parser.add_argument("--collect-docs", type=Path, metavar="RECEIPT")
    parser.add_argument("--docs-root", type=Path)
    args = parser.parse_args()
    if args.collect_docs:
        if not args.docs_root or not args.document_commit:
            parser.error("采集门禁须指定 --docs-root 与 --document-commit")
        document_gates(args.docs_root.resolve(), args.document_commit, args.collect_docs.resolve())
    else:
        if args.docs_root:
            parser.error("封存参数冲突")
        seal(args)
