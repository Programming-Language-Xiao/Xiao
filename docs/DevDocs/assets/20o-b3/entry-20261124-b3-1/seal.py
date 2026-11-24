"""按已提交的文档树采集门禁，生成 B3-1 收口和完整清单。"""

import argparse
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
from audit import GATES, GENERATED, PREFIX, REPO, ROOT, derive, doc_source, encoded, git, identity, inventory, require


def check_worktree(commit):
    diff = git("diff", "--name-only", "-z", commit)
    untracked = git("ls-files", "--others", "--exclude-standard", "-z")
    changed = set(filter(None, (diff + untracked).decode().split("\0")))
    allowed = {f"{PREFIX}/{name}" for name in GENERATED}
    require(not changed - allowed, f"工作树偏离受检快照：{', '.join(sorted(changed - allowed))}")


def document_gates(commit):
    check_worktree(commit)
    bun = shutil.which("bun")
    require(bun is not None, "找不到 bun")
    checks, logs = [], {}
    for name in GATES:
        argv = [bun, "run", name]
        result = subprocess.run(argv, cwd=REPO, capture_output=True, timeout=120)
        row = {"name": name, "argv": argv, "exit": result.returncode}
        for stream, raw in (("stdout", result.stdout), ("stderr", result.stderr)):
            digest = identity(raw)["sha256"]
            logs.setdefault(digest, raw.decode("utf-8"))
            row[f"{stream}_sha256"] = digest
        if result.returncode:
            sys.stderr.write(logs[row["stdout_sha256"]] + logs[row["stderr_sha256"]])
        checks.append(row)
    check_worktree(commit)
    return {"source": doc_source(commit), "checks": checks, "logs": logs, "worktree_matched_before_after": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--document-commit", help="执行文档门禁并登记 Git 树")
    args = parser.parse_args()
    if args.document_commit:
        commit = git("rev-parse", "--verify", args.document_commit).decode().strip()
        (ROOT / "doc-gates.json").write_bytes(encoded(document_gates(commit)))
    status = derive(ROOT, not (ROOT / "self-negative-result.json").is_file())
    (ROOT / "closeout.json").write_bytes(encoded(status))
    (ROOT / "evidence.json").write_bytes(encoded({"schema": 1, "files": inventory(ROOT)}))
    print(encoded({"b3_1_passed": status["b3_1_passed"], "b3_2_entry_ready": status["b3_2_entry_ready"]}).decode(), end="")


if __name__ == "__main__":
    main()
