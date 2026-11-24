"""生成 B3-0D 的确定性收口回执和实际文件清单。"""

import json
from pathlib import Path
import shutil
import subprocess
import sys

from audit import ROOT, REPO, STATIC_FILES, GENERATED_FILES, audit, encoded, identity, inventory, derive


def run_gate(name):
    bun = shutil.which("bun")
    if not bun:
        return {"name": name, "exit": None, "stdout": "", "stderr": "bun not found"}
    command = [bun, "run", name]
    result = subprocess.run(command, cwd=REPO, capture_output=True, encoding="utf-8", timeout=120)
    return {"name": name, "argv": command, "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
            "stdout_sha256": identity(result.stdout.encode("utf-8"))["sha256"],
            "stderr_sha256": identity(result.stderr.encode("utf-8"))["sha256"]}


def write_json(path, value):
    path.write_bytes(encoded(value))


def main():
    checks = [run_gate(name) for name in ("check:docs", "check:usedocs", "check:layout")]
    write_json(ROOT / "doc-gates.json", {"checks": checks, "passed": all(row["exit"] == 0 for row in checks)})
    closeout = derive(ROOT)
    write_json(ROOT / "closeout.json", closeout)
    files = inventory(ROOT)
    write_json(ROOT / "evidence.json", {"schema": 1, "files": files})
    print(json.dumps({"files": len(inventory(ROOT)), "doc_gates_passed": closeout["doc_gates_passed"]}, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
