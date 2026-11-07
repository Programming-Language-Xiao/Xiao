"""验证审计对清单篡改和删除 .gitattributes 的拒绝行为。"""

import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]
AUDIT = ROOT / "audit.py"


def run_audit(root):
    return subprocess.run(
        [sys.executable, str(AUDIT), "--root", str(root), "--source-root", str(SOURCE)],
        cwd=SOURCE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )


with tempfile.TemporaryDirectory(prefix="xiao-b1-audit-negative-") as temporary:
    temp = Path(temporary) / ROOT.name
    shutil.copytree(ROOT, temp)
    baseline = run_audit(temp)

    tampered_manifest = json.loads((temp / "evidence.json").read_text(encoding="utf-8"))
    tampered_manifest["files"][0]["sha256"] = "0" * 64
    (temp / "evidence.json").write_text(
        json.dumps(tampered_manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    tampered = run_audit(temp)

    shutil.copyfile(ROOT / "evidence.json", temp / "evidence.json")
    (temp / ".gitattributes").unlink()
    missing_attributes = run_audit(temp)

result = {
    "baseline_exit_code": baseline.returncode,
    "tampered_manifest_exit_code": tampered.returncode,
    "missing_gitattributes_exit_code": missing_attributes.returncode,
    "expected": {
        "baseline": 0,
        "tampered_manifest": 1,
        "missing_gitattributes": 1,
    },
    "passed": baseline.returncode == 0 and tampered.returncode != 0 and missing_attributes.returncode != 0,
}
if not result["passed"]:
    raise SystemExit(json.dumps(result, ensure_ascii=False))
(ROOT / "self-negative-result.json").write_text(
    json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
)
print(json.dumps(result, ensure_ascii=False))
