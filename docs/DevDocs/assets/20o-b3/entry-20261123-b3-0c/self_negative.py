"""在证据副本运行实际审计，核对拒绝理由和字节只读性。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

sys.dont_write_bytecode = True
from audit_closeout import (
    DOC_GATE, FS_DEFERRED, MANIFEST, NEGATIVE_REPORT, PREFLIGHT_REPORT, PREFLIGHT_REASONS, REASONS, ROOT, START, STATUS,
    add_anchor_arguments, blob, digest, encoded, identity, inventory, require,
)
from check_tutorials import data


def tamper(root, name, sealed):
    if name == "manifest-tampered":
        with (root / MANIFEST).open("ab") as output:
            output.write(b"\n")
        return
    if name == "missing-attributes":
        (root / ".gitattributes").unlink()
    elif name in {"forged-status", "missing-fs-deferral"}:
        path = root / STATUS
        report = data(path)
        if name == "forged-status":
            report["b3_0d_entry_ready"] = not report["b3_0d_entry_ready"]
        else:
            report["handoff"]["pending"].remove(FS_DEFERRED)
        path.write_bytes(encoded(report))
    elif name == "forged-observation":
        path = root / "baseline.json"
        report = data(path)
        row = report["commands"]["0a-audit"]
        observed = json.loads(row["stdout"])
        observed["platforms"]["windows"]["observed"]["fs"] = {}
        row["stdout"] = encoded(observed).decode()
        path.write_bytes(encoded(report))
    elif name == "direction-as-freeze":
        path = root / "decisions.json"
        report = data(path)
        report["detail_decisions"] = {}
        for row in report["direction_decisions"].values():
            row["status"] = "frozen"
        path.write_bytes(encoded(report))
    elif name == "replaced-doc-blob":
        path = root / DOC_GATE
        report = data(path)
        plan = "docs/DevDocs/20o-b3-0c-fs-env-log-plan.md"
        for moment in ("before", "after"):
            report[moment]["markdown"][plan] = identity(blob(START, plan))
        path.write_bytes(encoded(report))
    elif name == "forged-negative-report":
        path = root / (NEGATIVE_REPORT if sealed else PREFLIGHT_REPORT)
        report = data(path)
        row = report["cases"][1]
        row["before"] = row["after"] = "0" * 64
        path.write_bytes(encoded(report))
    else:
        raise ValueError(f"未知负例：{name}")
    (root / MANIFEST).write_bytes(encoded({"schema": 1, "files": inventory(root)}))


def snapshot(root):
    return digest(encoded(inventory(root) | {MANIFEST: identity((root / MANIFEST).read_bytes())}))


def replay(parent, name, reason, extra, sealed):
    root = parent / name
    shutil.copytree(ROOT, root)
    if reason:
        tamper(root, name, sealed)
    before = snapshot(root)
    argv = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit_closeout.py"), "--evidence-root", str(root), *extra]
    result = subprocess.run(argv, capture_output=True, encoding="utf-8", timeout=60)
    after = snapshot(root)
    expected = f"B3-0C 拒绝：{reason}\n" if reason else ""
    require(result.returncode == int(bool(reason)) and result.stderr == expected and before == after,
            f"{name} 的拒绝或只读性不符：{result.returncode} {result.stderr}")
    if reason:
        require(not result.stdout, f"拒绝时出现成功输出：{name}")
    else:
        status = json.loads(result.stdout)
        require(status["b3_0c_passed"] is status["b3_0d_entry_ready"] is sealed,
                "原样副本的收口状态不符")
    return {"case": name, "argv": argv, "exit": result.returncode, "stderr": result.stderr,
            "read_only": True, "before": before, "after": after}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_anchor_arguments(parser)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    mode, extra = "sealed", []
    if args.document_commit:
        mode, extra = "preflight", ["--document-commit", args.document_commit]
    elif args.receipt_commit:
        mode, extra = "receipt", ["--receipt-commit", args.receipt_commit]
    cases = PREFLIGHT_REASONS if mode == "preflight" else REASONS
    with TemporaryDirectory(prefix="xiao-b3-0c-negative-") as temporary:
        rows = [replay(Path(temporary), name, reason, extra, mode == "sealed") for name, reason in cases.items()]
    raw = encoded({"mode": mode, "cases": rows})
    if args.output:
        with args.output.open("xb") as output:
            output.write(raw)
    print(raw.decode(), end="")
