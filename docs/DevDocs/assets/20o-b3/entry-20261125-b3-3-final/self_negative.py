"""在任务临时副本验证收口审计的拒绝原因和只读性。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from zipfile import ZIP_DEFLATED, ZipFile

from audit import BASE, HERE, NEGATIVE_REASONS, REPO, identity, manifest


def json_bytes(value):
    return (json.dumps(value, sort_keys=True, ensure_ascii=False, indent=2) + "\n").encode()


def verify(root, prepare):
    args = [sys.executable, "-B", "-X", "utf8", str(HERE / "audit.py"),
            "--source-root", str(REPO), "--root", str(root)]
    if prepare:
        args.append("--prepare")
    return subprocess.run(args, capture_output=True, text=True, encoding="utf-8")


def snap(root):
    return manifest(root), identity((root / "evidence.json").read_bytes())


def mutate(root, name, prepare):
    if name == "manifest_changed":
        (root / "evidence.json").write_bytes(b"{}\n")
        return
    if name == "attributes_missing":
        (root / ".gitattributes").unlink()
        return
    if name == "forged_closeout":
        path = root / "closeout.json"
        report = json.loads(path.read_bytes())
        report["b3_4_entry_ready"] = prepare
        path.write_bytes(json_bytes(report))
    else:
        (root / "runs.zip").rename(root / "original.zip")
        with ZipFile(root / "original.zip") as original, ZipFile(root / "runs.zip", "x", ZIP_DEFLATED) as changed:
            label = "platforms/windows.json"
            report = json.loads(original.read(label))
            row = report["artifacts"][0]
            object_name = "objects/" + row["artifact"]["sha256"]
            replacement = None
            if name == "changed_capability":
                row["capabilities"] ^= 1
            elif name == "changed_coff":
                row["verification"] = "unverified-coff-exports"
            elif name == "changed_tutorial":
                row["stdout"] = row["stderr"]
            elif name == "changed_intrinsics":
                report["source_commit"] = BASE
            elif name == "missing_race":
                command = report["commands"]["runtime-and-abi"]
                stdout = original.read("objects/" + command["stdout"]["sha256"])
                fixture = b"filesystem::tests::fs_atomic_no_replace_has_one_winner_and_no_staging_files ... ok"
                assert fixture in stdout
                replacement = stdout.replace(fixture, b"synthetic ... ok")
                command["stdout"] = identity(replacement)
            for key in original.namelist():
                data = original.read(key)
                if key == label:
                    data = json_bytes(report)
                elif key == object_name and name == "changed_artifact":
                    data = data[:-1] + bytes([data[-1] ^ 1])
                changed.writestr(key, data)
            if replacement is not None:
                changed.writestr("objects/" + command["stdout"]["sha256"], replacement)
        (root / "original.zip").unlink()
    (root / "evidence.json").write_bytes(json_bytes({"files": manifest(root)}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--temp-root", type=Path)
    parser.add_argument("--prepare", action="store_true")
    args = parser.parse_args()
    original = snap(HERE)
    baseline = verify(HERE, args.prepare)
    assert baseline.returncode == 0 and not baseline.stderr, baseline.stderr
    rejected = {}
    with tempfile.TemporaryDirectory(dir=args.temp_root, prefix="b3-fs-neg-") as temp:
        for name, reason in NEGATIVE_REASONS.items():
            root = Path(temp) / name
            shutil.copytree(HERE, root)
            mutate(root, name, args.prepare)
            before = snap(root)
            result = verify(root, args.prepare)
            assert result.returncode == 1 and result.stderr == f"B3-3 拒绝：{reason}\n", result.stderr
            assert snap(root) == before, "审计改写测试副本"
            rejected[name] = reason
    assert snap(HERE) == original, "审计改写封存目录"
    print(json_bytes({"baseline_passed": True, "read_only": True, "rejected": rejected}).decode(), end="")


if __name__ == "__main__":
    main()
