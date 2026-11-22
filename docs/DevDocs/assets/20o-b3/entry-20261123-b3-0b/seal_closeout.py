"""采集定稿文档门禁并生成 B3-0B 的收口回执和清单。"""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
from audit_closeout import (CONTRACT, DOCS, DOCUMENT_CHECKS, MANIFEST, ROOT, base, derive,
                            git, identity, inventory, json_bytes, require)


def markdown():
    result = {}
    for path in DOCS:
        raw = (base.REPO / path).read_bytes().replace(b"\r\n", b"\n")
        blob = git("rev-parse", f":{path}").strip().decode()
        require(raw == git("cat-file", "blob", blob), f"定稿 Markdown 与暂存字节不符：{path}")
        result[path] = {"blob": blob, "text": identity(raw)}
    return result


def clean_boundary():
    require(not git("diff", "--name-only", "-z", CONTRACT, "--", *base.PRODUCT_ROOTS)
            and not git("ls-files", "--others", "--exclude-standard", "-z", "--", *base.PRODUCT_ROOTS),
            "最终门禁时发现候选提交之外的产品或夹具改动")


def document_gates(receipt):
    clean_boundary()
    before = markdown()
    executable = shutil.which("bun")
    require(executable is not None, "缺少 Bun")
    bun = Path(executable).resolve(strict=True)
    checks = []
    for gate in DOCUMENT_CHECKS:
        command = [str(bun), "run", gate]
        started = base.utc()
        with subprocess.Popen(command, cwd=base.REPO, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              encoding="utf-8") as process:
            try:
                stdout, stderr = process.communicate(timeout=90)
            except subprocess.TimeoutExpired:
                process.kill()
                stdout, stderr = process.communicate()
        checks.append({"argv": command, "pid": process.pid, "exit": process.returncode, "stdout": stdout,
                       "stderr": stderr, "started_utc": started, "finished_utc": base.utc()})
        print(f"{gate} exit={process.returncode}", flush=True)
    report = {"contract": CONTRACT, "cwd": str(base.REPO), "tool": base.identity(bun), "checks": checks,
              "markdown_before": before, "markdown_after": markdown()}
    raw = json_bytes(report)
    with receipt.open("xb") as retained:
        retained.write(raw)
    clean_boundary()
    require(before == report["markdown_after"] and all(row["exit"] == 0 for row in checks),
            f"定稿文档门禁失败，原始回执见 {receipt}")
    (ROOT / "doc-gate.json").write_bytes(raw)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", action="store_true")
    parser.add_argument("--collect-docs", type=Path, metavar="RECEIPT")
    args = parser.parse_args()
    if args.collect_docs:
        document_gates(args.collect_docs)
    (ROOT / "closeout.json").write_bytes(json_bytes(derive(ROOT, args.preflight)))
    (ROOT / MANIFEST).write_bytes(json_bytes({"schema": 1, "files": inventory(ROOT)}))
