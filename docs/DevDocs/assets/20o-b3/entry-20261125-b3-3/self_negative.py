"""复跑原样证据及本机收据的三种篡改，封存目录只读。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

from audit import HERE, REPO, identity, inventory


def audit(root):
    return subprocess.run(
        [sys.executable, "-B", "-X", "utf8", str(HERE / "audit.py"),
         "--root", str(root), "--source-root", str(REPO)],
        capture_output=True, text=True, encoding="utf-8",
    )


def snapshot(root):
    return inventory(root), identity((root / "evidence.json").read_bytes())


def manifest_mutated(root):
    (root / "evidence.json").write_text("{}\n", encoding="utf-8")


def attributes_missing(root):
    (root / ".gitattributes").unlink()


def forged_stage(root):
    status = json.loads((root / "status.json").read_bytes())
    status["b3_3_passed"] = True
    status["b3_4_entry_ready"] = True
    (root / "status.json").write_text(json.dumps(status), encoding="utf-8")
    (root / "evidence.json").write_text(json.dumps({"files": inventory(root)}), encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--temp-root", type=Path)
    args = parser.parse_args()
    original = snapshot(HERE)
    baseline = audit(HERE)
    if baseline.returncode != 0 or baseline.stderr:
        raise SystemExit(baseline.stderr or baseline.stdout)
    rejected = {}
    with tempfile.TemporaryDirectory(dir=args.temp_root, prefix="b3-3-replay-") as path:
        for mutation, reason in (
            (manifest_mutated, "封存清单与当前文件字节不一致"),
            (attributes_missing, "缺少 .gitattributes"),
            (forged_stage, "阶段状态与实测回执不一致"),
        ):
            root = Path(path) / mutation.__name__
            shutil.copytree(HERE, root)
            before_manifest = snapshot(root)
            assert before_manifest == original
            mutation(root)
            mutated = snapshot(root)
            result = audit(root)
            assert result.returncode == 1 and result.stderr == f"B3-3 拒绝：{reason}\n", result.stderr
            assert snapshot(root) == mutated, "审计改写临时副本"
            rejected[mutation.__name__] = result.stderr.strip()
    assert snapshot(HERE) == original, "审计改写封存目录"
    print(json.dumps({"baseline_passed": True, "read_only": True, "rejected": rejected},
                     ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
