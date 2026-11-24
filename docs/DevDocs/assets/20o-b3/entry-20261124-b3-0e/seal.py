"""采集固定文档快照的门禁回执，并生成 B3-0E 清单。"""

import argparse
import json
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
from audit import GATES, GENERATED, PREFIX, PROTECTED, REPO, ROOT, derive, document_source, encoded, git, identity, inventory


def check_worktree(commit):
    scope = ("docs/DevDocs", *PROTECTED)
    files = git("diff", "--name-only", "-z", commit, "--", *scope)
    files += git("ls-files", "--others", "--exclude-standard", "-z", "--", *scope)
    receipts = {f"{PREFIX}/{name}" for name in GENERATED}
    changed = set(filter(None, files.decode("utf-8").split("\0"))) - receipts
    if changed:
        raise ValueError(f"工作树偏离受检快照：{', '.join(sorted(changed))}")


def run_gates(commit):
    bun = shutil.which("bun")
    if not bun:
        raise ValueError("找不到 bun")
    check_worktree(commit)
    logs = {}
    checks = []
    for name in GATES:
        argv = [bun, "run", name]
        result = subprocess.run(argv, cwd=REPO, capture_output=True, encoding="utf-8", timeout=120)
        row = {"name": name, "argv": argv, "exit": result.returncode}
        for stream in ("stdout", "stderr"):
            output = getattr(result, stream)
            sha = identity(output.encode("utf-8"))["sha256"]
            logs.setdefault(sha, output)
            row[f"{stream}_sha256"] = sha
        checks.append(row)
    check_worktree(commit)
    return {"source": document_source(commit), "checks": checks, "logs": logs,
            "worktree_matched_before_after": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--document-commit", help="新建门禁回执时指定受检提交")
    args = parser.parse_args()
    gate = ROOT / "doc-gates.json"
    if args.document_commit:
        commit = git("rev-parse", "--verify", args.document_commit).decode().strip()
        gate.write_bytes(encoded(run_gates(commit)))
    prepare = not (ROOT / "self-negative-result.json").is_file()
    result = derive(ROOT, prepare)
    (ROOT / "closeout.json").write_bytes(encoded(result))
    files = inventory(ROOT)
    (ROOT / "evidence.json").write_bytes(encoded({"schema": 1, "files": files}))
    print(json.dumps({"files": len(files), "b3_0e_passed": result["b3_0e_passed"],
                      "pending_decisions": len(result["pending_decisions"])}))


if __name__ == "__main__":
    main()
