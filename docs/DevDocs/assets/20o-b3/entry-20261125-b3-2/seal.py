"""执行文档门禁并生成 B3-2A 的基线回执。"""

import shutil
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

sys.dont_write_bytecode = True
from audit import GATES, GENERATED, GENERATED_PATHS, NEGATIVE_RECEIPT, REPO, ROOT, audit, derive, doc_source
from audit import encoded, git, identity, inventory, require
from self_negative import run_cases


def check_worktree(commit):
    require(git(REPO, "rev-parse", "HEAD").decode().strip() == commit, "门禁执行期间文档提交发生变化")
    tracked = git(REPO, "diff", "--name-only", "-z", commit)
    untracked = git(REPO, "ls-files", "--others", "--exclude-standard", "-z")
    changed = set(filter(None, (tracked + untracked).decode().split("\0")))
    require(changed <= GENERATED_PATHS, "工作树偏离待核验的文档提交")


def main():
    commit = git(REPO, "rev-parse", "HEAD").decode().strip()
    check_worktree(commit)
    checks, logs = [], {}
    bun = shutil.which("bun")
    require(bun, "找不到 bun")
    for name in GATES:
        argv = [bun, "run", name]
        result = subprocess.run(argv, cwd=REPO, capture_output=True, timeout=120)
        row = {"name": name, "argv": argv, "exit": result.returncode}
        for stream in ("stdout", "stderr"):
            raw = getattr(result, stream)
            row[stream] = identity(raw)["sha256"]
            logs.setdefault(row[stream], raw.decode("utf-8"))
        require(result.returncode == 0, f"{name} 失败\n{logs[row['stdout']]}{logs[row['stderr']]}")
        checks.append(row)
    check_worktree(commit)
    gates = {"source": doc_source(commit), "worktree_matched_before_after": True, "checks": checks, "logs": logs}
    with TemporaryDirectory(prefix="xiao-b3-2a-seal-") as directory:
        staged = Path(directory) / "evidence"
        shutil.copytree(ROOT, staged, ignore=shutil.ignore_patterns(*GENERATED))
        (staged / "doc-gates.json").write_bytes(encoded(gates))
        status = derive(staged, gates, inventory(staged))
        (staged / "closeout.json").write_bytes(encoded(status))
        manifest = staged / "evidence.json"
        manifest.write_bytes(encoded({"schema": 1, "files": inventory(staged)}))
        (staged / NEGATIVE_RECEIPT).write_bytes(encoded(run_cases(staged)))
        manifest.write_bytes(encoded({"schema": 1, "files": inventory(staged)}))
        audit(staged)
        check_worktree(commit)
        for name in ("doc-gates.json", "closeout.json", NEGATIVE_RECEIPT, "evidence.json"):
            (ROOT / name).write_bytes((staged / name).read_bytes())
    print(encoded({"b3_2a_entry_and_baseline_verified": status["b3_2a_entry_and_baseline_verified"],
                   "b3_2_started": status["b3_2_started"]}).decode(), end="")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"B3-2A 封存失败：{error}", file=sys.stderr)
        raise SystemExit(1) from None
