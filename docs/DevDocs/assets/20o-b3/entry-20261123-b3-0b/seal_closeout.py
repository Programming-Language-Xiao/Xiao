"""采集定稿文档门禁并生成 B3-0B 的收口回执和清单。"""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
from audit_closeout import (CONTRACT, DOCS, DOCUMENT_CHECKS, MANIFEST, ROOT, base, derive,
                            document_snapshot, git, identity, inventory, json_bytes, require)


def checked_snapshot(checkout):
    require(git("rev-parse", "HEAD", repo=checkout).strip().decode() == CONTRACT
            and not git("status", "--porcelain", "--untracked-files=all", repo=checkout),
            "文档门禁需要干净的固定检出")
    observed = document_snapshot()
    for path in DOCS:
        raw = (checkout / path).read_bytes().replace(b"\r\n", b"\n")
        require(identity(raw) == observed["markdown"][path], f"定稿 Markdown 与固定提交字节不符：{path}")
    return observed


def clean_boundary():
    require(not git("diff", "--name-only", "-z", CONTRACT, "--", *base.PRODUCT_ROOTS)
            and not git("ls-files", "--others", "--exclude-standard", "-z", "--", *base.PRODUCT_ROOTS),
            "最终门禁时发现候选提交之外的产品或夹具改动")


def document_gates(receipt, checkout):
    before = checked_snapshot(checkout)
    executable = shutil.which("bun")
    require(executable is not None, "缺少 Bun")
    bun = Path(executable).resolve(strict=True)
    checks = []
    for gate in DOCUMENT_CHECKS:
        command = [str(bun), "run", gate]
        started = base.utc()
        with subprocess.Popen(command, cwd=checkout, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              encoding="utf-8") as process:
            try:
                stdout, stderr = process.communicate(timeout=90)
            except subprocess.TimeoutExpired:
                process.kill()
                stdout, stderr = process.communicate()
        checks.append({"argv": command, "pid": process.pid, "exit": process.returncode, "stdout": stdout,
                       "stderr": stderr, "started_utc": started, "finished_utc": base.utc()})
        print(f"{gate} exit={process.returncode}", flush=True)
    after = checked_snapshot(checkout)
    report = {"cwd": str(checkout), "tool": base.identity(bun) | {"path": str(bun)}, "checks": checks,
              "before": before, "after": after}
    with receipt.open("xb") as retained:
        retained.write(json_bytes(report))
    require(before == after and all(row["exit"] == 0 for row in checks),
            f"定稿文档门禁失败，原始回执见 {receipt}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", action="store_true")
    parser.add_argument("--collect-docs", type=Path, metavar="RECEIPT")
    parser.add_argument("--docs-root", type=Path, default=base.REPO, help="受检提交的干净检出路径")
    args = parser.parse_args()
    if args.collect_docs:
        document_gates(args.collect_docs, args.docs_root.resolve())
        raise SystemExit(0)
    clean_boundary()
    (ROOT / "closeout.json").write_bytes(json_bytes(derive(ROOT, args.preflight)))
    (ROOT / MANIFEST).write_bytes(json_bytes({"schema": 1, "files": inventory(ROOT)}))
