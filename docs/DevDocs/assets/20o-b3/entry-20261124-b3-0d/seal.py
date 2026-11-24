"""生成 B3-0D 的确定性收口回执和实际文件清单。"""

import json
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True

from audit import (
    FREEZE_COMMIT, GENERATED_FILES, OPTIONAL_GENERATED_FILES, PREFIX, PRODUCT_ROOTS, REPO, ROOT,
    derive, doc_snapshot, encoded, git, identity, inventory,
)


def run_gate(bun, name):
    command = [bun, "run", name]
    result = subprocess.run(command, cwd=REPO, capture_output=True, encoding="utf-8", timeout=120)
    return {"name": name, "argv": command, "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
            "stdout_sha256": identity(result.stdout.encode("utf-8"))["sha256"],
            "stderr_sha256": identity(result.stderr.encode("utf-8"))["sha256"]}


def verify_gate_worktree():
    roots = ("docs/DevDocs", *PRODUCT_ROOTS)
    changed = git("diff", "--name-only", "-z", FREEZE_COMMIT, "--", *roots)
    untracked = git("ls-files", "--others", "--exclude-standard", "-z", "--", *roots)
    exceptions = {f"{PREFIX}/{name}" for name in GENERATED_FILES | OPTIONAL_GENERATED_FILES | {"audit.py"}}
    differing = (set((changed + untracked).decode("utf-8").split("\0")) - {""}) - exceptions
    if differing:
        raise RuntimeError(f"门禁工作树偏离固定输入：{', '.join(sorted(differing))}")


def main():
    receipt = ROOT / "doc-gates.json"
    source = doc_snapshot()
    previous = json.loads(receipt.read_bytes()) if receipt.is_file() else {}
    if previous.get("source") != source or not previous.get("passed"):
        bun = shutil.which("bun")
        if not bun:
            raise RuntimeError("找不到 bun，无法执行文档门禁")
        verify_gate_worktree()
        checks = [run_gate(bun, name) for name in ("check:docs", "check:usedocs", "check:layout")]
        verify_gate_worktree()
        logs = {}
        for row in checks:
            logs.setdefault(row["stdout_sha256"], row["stdout"])
            logs.setdefault(row["stderr_sha256"], row["stderr"])
        receipt.write_bytes(encoded({"source": source, "checks": checks, "logs": logs,
                                     "worktree_verified_before_after": True,
                                     "passed": all(row["exit"] == 0 for row in checks)}))
    closeout = derive(ROOT)
    (ROOT / "closeout.json").write_bytes(encoded(closeout))
    files = inventory(ROOT)
    (ROOT / "evidence.json").write_bytes(encoded({"schema": 1, "files": files}))
    print(json.dumps({"files": len(files), "doc_gates_passed": closeout["doc_gates_passed"],
                      "b3_0d_passed": closeout["b3_0d_passed"]}, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
