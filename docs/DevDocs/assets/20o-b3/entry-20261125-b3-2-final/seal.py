"""在定稿 Git 对象上运行文档门禁并封存 B3-2 收口。"""

from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

sys.dont_write_bytecode = True
from audit import GATES, GENERATED, HERE, PREFIX, audit, derive, doc_source, encoded, git, identity, require
from run import ROOT
from self_negative import run_cases, write_manifest


def check_worktree(commit):
    require(git("rev-parse", "HEAD").decode().strip() == commit, "文档提交发生变化")
    changed = git("diff", "--name-only", "-z", commit) + git("ls-files", "--others", "--exclude-standard", "-z")
    names = set(filter(None, changed.decode().split("\0")))
    require(names <= {f"{PREFIX}/{name}" for name in GENERATED}, "工作树出现未定稿文件")


def document_gates(commit):
    check_worktree(commit)
    bun = shutil.which("bun")
    require(bun is not None, "找不到 bun")
    checks, logs = [], {}
    for name in GATES:
        argv = [bun, "run", name]
        process = subprocess.run(argv, cwd=ROOT, capture_output=True, timeout=120)
        row = {"name": name, "argv": argv, "exit": process.returncode}
        for stream in ("stdout", "stderr"):
            raw = getattr(process, stream)
            row[stream] = identity(raw)["sha256"]
            logs.setdefault(row[stream], raw.decode("utf-8"))
        require(process.returncode == 0, f"{name} 失败\n{logs[row['stdout']]}{logs[row['stderr']]}")
        checks.append(row)
    check_worktree(commit)
    return {"source": doc_source(commit), "matched_before_after": True, "checks": checks, "logs": logs}


def main():
    commit = git("rev-parse", "HEAD").decode().strip()
    gates = document_gates(commit)
    with TemporaryDirectory(prefix="xiao-b3-2-seal-") as directory:
        staged = Path(directory) / "evidence"
        shutil.copytree(HERE, staged, ignore=shutil.ignore_patterns(*GENERATED))
        (staged / "doc-gates.json").write_bytes(encoded(gates))
        (staged / "closeout.json").write_bytes(encoded(derive(staged, prepare=True)))
        write_manifest(staged)
        (staged / "self-negative-result.json").write_bytes(encoded(run_cases(staged)))
        (staged / "closeout.json").write_bytes(encoded(derive(staged, prepare=False)))
        write_manifest(staged)
        status = audit(staged)
        check_worktree(commit)
        for name in sorted(GENERATED - {"evidence.json"}) + ["evidence.json"]:
            (HERE / name).write_bytes((staged / name).read_bytes())
    print(encoded({"b3_2_passed": status["b3_2_passed"], "b3_3_entry_ready": status["b3_3_entry_ready"],
                   "audit_files": status["audit_files"], "negatives": status["negatives"]}).decode(), end="")


if __name__ == "__main__":
    main()
