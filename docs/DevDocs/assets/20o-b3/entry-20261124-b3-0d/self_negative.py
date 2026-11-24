"""在临时副本复跑 B3-0D 审计，核对反例理由和只读性。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import NamedTemporaryFile, TemporaryDirectory, gettempdir

sys.dont_write_bytecode = True

from audit import ROOT, encoded, identity, inventory


def rewrite_manifest(root):
    (root / "evidence.json").write_bytes(encoded({"schema": 1, "files": inventory(root)}))


def snapshot(root):
    return {"files": inventory(root), "evidence": identity((root / "evidence.json").read_bytes())}


def tamper(root, case):
    if case == "manifest-tampered":
        value = json.loads((root / "evidence.json").read_bytes())
        name = next(iter(value["files"]))
        value["files"][name]["sha256"] = "0" * 64
        (root / "evidence.json").write_bytes(encoded(value))
    elif case == "missing-attributes":
        (root / ".gitattributes").unlink()
        rewrite_manifest(root)
    elif case == "forged-status":
        value = json.loads((root / "closeout.json").read_bytes())
        value["b3_1_entry_ready"] = True
        (root / "closeout.json").write_bytes(encoded(value))
        rewrite_manifest(root)
    elif case == "forged-decision":
        value = json.loads((root / "decisions.json").read_bytes())
        value["decisions"]["B3-0D-ID"]["status"] = "pending"
        (root / "decisions.json").write_bytes(encoded(value))
        rewrite_manifest(root)
    elif case == "forged-version":
        value = json.loads((root / "version-matrix.json").read_bytes())
        value["b3_target"]["generator"] = 24
        (root / "version-matrix.json").write_bytes(encoded(value))
        rewrite_manifest(root)
    elif case == "forged-coff":
        value = json.loads((root / "coff-policy.json").read_bytes())
        value["status"] = "complete"
        (root / "coff-policy.json").write_bytes(encoded(value))
        rewrite_manifest(root)
    elif case == "forged-probe-observation":
        value = json.loads((root / "coff-probe.json").read_bytes())
        value["observations"][0]["observed"]["retained_coff_symbol_table"] = True
        (root / "coff-probe.json").write_bytes(encoded(value))
        rewrite_manifest(root)
    elif case == "forged-coff-criteria":
        value = json.loads((root / "coff-criteria-revision.json").read_bytes())
        value["effect"]["historical_probe_products_complete"] = True
        (root / "coff-criteria-revision.json").write_bytes(encoded(value))
        rewrite_manifest(root)
    elif case == "forged-negative-report":
        path = root / "self-negative-result.json"
        if path.is_file():
            value = json.loads(path.read_bytes())
            value["cases"][1]["stderr"] = "伪造的拒绝结果\n"
        else:
            value = {"mode": "sealed", "cases": []}
        path.write_bytes(encoded(value))
        rewrite_manifest(root)
    else:
        raise ValueError(case)


def replay(parent, case, expected):
    root = parent / case
    shutil.copytree(ROOT, root)
    if expected != "pass":
        tamper(root, case)
    before = identity(encoded(snapshot(root)))["sha256"]
    command = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"), "--evidence-root", str(root)]
    result = subprocess.run(command, capture_output=True, encoding="utf-8", timeout=60)
    after = identity(encoded(snapshot(root)))["sha256"]
    if expected == "pass":
        if result.returncode != 0 or result.stderr:
            raise RuntimeError(f"{case} 原样副本审计失败：{result.stderr}")
    else:
        wanted = f"B3-0D 拒绝：{expected}\n"
        if result.returncode != 1 or result.stderr != wanted:
            raise RuntimeError(f"{case}: {result.returncode} {result.stderr!r}")
    if before != after:
        raise RuntimeError(f"{case} 审计改写了证据副本")
    return {"case": case, "argv": command, "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
            "read_only": True, "before": before, "after": after}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="显式写入完整自反例回执的路径")
    args = parser.parse_args()
    temporary_root = Path(gettempdir()).resolve()
    if temporary_root.is_relative_to(ROOT):
        parser.error("系统临时目录须位于封存证据目录之外")
    before = snapshot(ROOT)
    cases = json.loads((ROOT / "negative-cases.json").read_bytes())["cases"]
    with TemporaryDirectory(prefix="xiao-b3-0d-negative-", dir=temporary_root) as directory:
        rows = [replay(Path(directory), row["name"], row["expected"]) for row in cases]
    if snapshot(ROOT) != before:
        raise RuntimeError("复跑期间封存证据字节发生变化")
    result = {"mode": "sealed", "cases": rows}
    if args.output is None:
        with NamedTemporaryFile(prefix="xiao-b3-0d-self-negative-", suffix=".json", dir=temporary_root, delete=False) as report:
            report.write(encoded(result))
            output = Path(report.name)
    else:
        output = args.output.resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(encoded(result))
    print(encoded({"report": str(output), "cases": len(rows),
                   "rejected_cases": sum(row["exit"] == 1 for row in rows),
                   "read_only": all(row["read_only"] and row["before"] == row["after"] for row in rows),
                   "evidence_root_written": output.is_relative_to(ROOT)}).decode("utf-8"), end="")


if __name__ == "__main__":
    main()
