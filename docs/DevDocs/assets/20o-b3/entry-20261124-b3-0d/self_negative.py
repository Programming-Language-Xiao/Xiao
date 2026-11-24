"""在临时副本复跑 B3-0D 审计，核对反例理由和只读性。"""

import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

from audit import ROOT, REASONS, encoded, identity, inventory


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
        value["b3_0d_passed"] = False
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
    elif case == "forged-negative-report":
        value = json.loads((root / "self-negative-result.json").read_bytes())
        value["cases"][1]["stderr"] = "伪造的拒绝结果\n"
        (root / "self-negative-result.json").write_bytes(encoded(value))
        rewrite_manifest(root)
    else:
        raise ValueError(case)


def replay(parent, case, expected):
    root = parent / case
    shutil.copytree(ROOT, root)
    if case == "forged-negative-report" and not (root / "self-negative-result.json").exists():
        expected_cases = json.loads((ROOT / "negative-cases.json").read_bytes())["cases"]
        seed = {"mode": "sealed", "cases": [
            {"case": item["name"],
             "argv": [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"), "--evidence-root", str(root)],
             "exit": int(item["expected"] != "pass"),
             "stdout": "baseline audit output" if item["expected"] == "pass" else "",
             "stderr": "" if item["expected"] == "pass" else f"B3-0D 拒绝：{item['expected']}\n",
             "read_only": True, "before": "0" * 64, "after": "0" * 64}
            for item in expected_cases
        ]}
        (root / "self-negative-result.json").write_bytes(encoded(seed))
        rewrite_manifest(root)
    if case != "unchanged":
        tamper(root, case)
    before = identity(encoded(snapshot(root)))["sha256"]
    command = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"), "--evidence-root", str(root)]
    result = subprocess.run(command, capture_output=True, encoding="utf-8", timeout=60)
    after = identity(encoded(snapshot(root)))["sha256"]
    if case == "unchanged":
        if result.returncode != 0:
            raise RuntimeError(result.stderr)
    else:
        wanted = f"B3-0D 拒绝：{expected}\n"
        if result.returncode != 1 or result.stderr != wanted:
            raise RuntimeError(f"{case}: {result.returncode} {result.stderr!r}")
    if before != after:
        raise RuntimeError(f"{case}: audit wrote the evidence copy")
    return {"case": case, "argv": command, "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
            "read_only": True, "before": before, "after": after}


def main():
    cases = json.loads((ROOT / "negative-cases.json").read_bytes())["cases"]
    with TemporaryDirectory(prefix="xiao-b3-0d-negative-") as directory:
        rows = [replay(Path(directory), row["name"], REASONS.get(row["name"], "")) for row in cases]
    result = {"mode": "sealed", "cases": rows}
    (ROOT / "self-negative-result.json").write_bytes(encoded(result))
    rewrite_manifest(ROOT)
    print(encoded(result).decode("utf-8"), end="")


if __name__ == "__main__":
    main()
