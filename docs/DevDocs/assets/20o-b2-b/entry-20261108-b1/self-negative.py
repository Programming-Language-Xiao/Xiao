"""验证审计对清单篡改和删除 .gitattributes 的拒绝行为。"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


DEFAULT_ROOT = Path(__file__).resolve().parent
DEFAULT_SOURCE = DEFAULT_ROOT.parents[4]


def snapshot(root):
    return {
        path.relative_to(root).as_posix(): {
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "bytes": path.stat().st_size,
            "mtime_ns": path.stat().st_mtime_ns,
        }
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def run_audit(audit, root, source):
    env = os.environ.copy()
    env["PYTHONUTF8"] = "1"
    env["PYTHONIOENCODING"] = "utf-8"
    return subprocess.run(
        [sys.executable, "-X", "utf8", str(audit), "--root", str(root), "--source-root", str(source)],
        cwd=source,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="strict",
        env=env,
    )


def main(root, source, write_result):
    audit = root / "audit.py"
    spec = json.loads((root / "negative-cases.json").read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="xiao-b1-audit-negative-") as temporary:
        task_root = Path(temporary).resolve()
        temp = task_root / root.name
        if temp.resolve().parent != task_root:
            raise SystemExit("临时证据目录越界")
        shutil.copytree(root, temp)
        before = snapshot(temp)
        baseline = run_audit(audit, temp, source)
        baseline_read_only = before == snapshot(temp)

        tampered_manifest = json.loads((temp / "evidence.json").read_text(encoding="utf-8"))
        tampered_manifest["files"][0]["sha256"] = "0" * 64
        (temp / "evidence.json").write_text(
            json.dumps(tampered_manifest, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        before = snapshot(temp)
        tampered = run_audit(audit, temp, source)
        tampered_read_only = before == snapshot(temp)

        shutil.copyfile(root / "evidence.json", temp / "evidence.json")
        (temp / ".gitattributes").unlink()
        before = snapshot(temp)
        missing_attributes = run_audit(audit, temp, source)
        missing_read_only = before == snapshot(temp)

    result = {
        "source_root": str(source),
        "baseline_exit_code": baseline.returncode,
        "tampered_manifest_exit_code": tampered.returncode,
        "missing_gitattributes_exit_code": missing_attributes.returncode,
        "baseline_output": baseline.stdout.strip(),
        "tampered_manifest_output": tampered.stdout.strip(),
        "missing_gitattributes_output": missing_attributes.stdout.strip(),
        "read_only": {
            "baseline": baseline_read_only,
            "tampered_manifest": tampered_read_only,
            "missing_gitattributes": missing_read_only,
        },
        "expected": {"baseline": 0, "tampered_manifest": 1, "missing_gitattributes": 1},
        "passed": (
            baseline.returncode == 0
            and tampered.returncode == 1
            and missing_attributes.returncode == 1
            and tampered.stdout.strip() == spec["expected_reason"]
            and missing_attributes.stdout.strip() == spec["missing_gitattributes_reason"]
            and baseline_read_only and tampered_read_only and missing_read_only
        ),
    }
    if not result["passed"]:
        raise SystemExit(json.dumps(result, ensure_ascii=True))
    if write_result:
        (root / "self-negative-result.json").write_text(
            json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
    print(json.dumps(result, ensure_ascii=True))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--write-result", action="store_true", help="显式写入真实运行结果；默认只读")
    args = parser.parse_args()
    main(args.root.resolve(), args.source_root.resolve(), args.write_result)
