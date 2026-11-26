"""派生收口状态，按最终字节封存清单。"""

import argparse
import json
import shutil
import subprocess

from audit import GATES, HERE, REPO, derive, document_identity, generated_paths, git, identity, manifest, require


def write(name, value):
    (HERE / name).write_bytes((json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode())


def check_worktree(commit):
    require(git(REPO, "rev-parse", "HEAD").decode().strip() == commit, "文档提交发生变化")
    changed = (git(REPO, "diff", "--name-only", "-z", commit)
               + git(REPO, "ls-files", "--others", "--exclude-standard", "-z"))
    require(set(filter(None, changed.decode().split("\0"))) <= generated_paths(), "存在未定稿文件")


def document_gates():
    commit = git(REPO, "rev-parse", "HEAD").decode().strip()
    check_worktree(commit)
    bun = shutil.which("bun")
    require(bun is not None, "找不到 Bun")
    rows, logs = [], {}
    for name in GATES:
        command = [bun, "run", name]
        result = subprocess.run(command, capture_output=True, cwd=REPO, timeout=120)
        row = {"name": name, "argv": command, "exit": result.returncode}
        for stream in ("stdout", "stderr"):
            raw = getattr(result, stream)
            digest = identity(raw)["sha256"]
            row[stream] = digest
            logs.setdefault(digest, raw.decode("utf-8"))
        require(result.returncode == 0, name + " 未通过\n" + logs[row["stdout"]] + logs[row["stderr"]])
        rows.append(row)
    check_worktree(commit)
    write("doc-gates.json", {"source": document_identity(REPO, commit), "matched_before_after": True,
                             "checks": rows, "logs": logs})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare", action="store_true")
    args = parser.parse_args()
    if args.prepare:
        document_gates()
    write("closeout.json", derive(HERE, REPO, args.prepare))
    write("evidence.json", {"files": manifest(HERE)})


if __name__ == "__main__":
    main()
