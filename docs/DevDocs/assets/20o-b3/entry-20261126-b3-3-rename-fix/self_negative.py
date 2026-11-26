"""在临时副本删除排他重命名回执，验证只读审计会拒绝。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

from audit import HERE, MARKERS, checks, identity, manifest


def run_audit(root):
    return subprocess.run([sys.executable, "-B", "-X", "utf8", str(HERE / "audit.py"), "--root", str(root)],
                          capture_output=True, text=True, encoding="utf-8")


def snapshot(root):
    return manifest(root), (root / "evidence.json").read_bytes()


def remove_case(root):
    bundle = root / "runs.zip"
    original = bundle.rename(root / "original.zip")
    report_name = "platforms/windows.json"
    with ZipFile(original) as source, ZipFile(bundle, "x", ZIP_DEFLATED) as output:
        report = checks.read_json(source, report_name)
        command = report["commands"]["runtime-and-abi"]
        raw = checks.read_object(source, command["stdout"])
        assert MARKERS[0] in raw
        changed = raw.replace(MARKERS[0], b"")
        command["stdout"] = identity(changed)
        for name in source.namelist():
            if name == report_name:
                output.writestr(name, json.dumps(report))
            else:
                output.writestr(name, source.read(name))
        output.writestr("objects/" + command["stdout"]["sha256"], changed)
    original.unlink()
    (root / "evidence.json").write_text(json.dumps({"files": manifest(root)}), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--temp-root", type=Path)
    args = parser.parse_args()
    original = snapshot(HERE)
    baseline = run_audit(HERE)
    assert baseline.returncode == 0 and not baseline.stderr, baseline.stderr
    with TemporaryDirectory(dir=args.temp_root, prefix="xiao-b3-3-rename-audit-") as tmp:
        root = Path(tmp) / "evidence"
        shutil.copytree(HERE, root)
        remove_case(root)
        before = snapshot(root)
        rejected = run_audit(root)
        reason = "B3-3 复验拒绝：排他重命名回执缺项"
        assert (rejected.returncode, rejected.stdout, rejected.stderr) == (1, "", reason + "\n"), rejected
        assert snapshot(root) == before, "审计改写临时副本"
    assert snapshot(HERE) == original, "审计改写封存目录"
    print(json.dumps({"baseline_passed": True, "read_only": True, "rejection": reason}, ensure_ascii=False))


if __name__ == "__main__":
    main()
