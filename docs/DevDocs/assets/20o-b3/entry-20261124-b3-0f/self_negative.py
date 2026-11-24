"""在临时副本复跑 B3-0F 审计和拒绝用例。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory, gettempdir

sys.dont_write_bytecode = True
from audit import ROOT, data, encoded, identity, inventory, require


def snapshot(root):
    manifest = identity((root / "evidence.json").read_bytes())
    return identity(encoded({"files": inventory(root), "manifest": manifest}))["sha256"]


def tamper(root, case):
    if case == "manifest-tampered":
        manifest = data(root, "evidence.json")
        manifest["files"][".gitattributes"]["bytes"] += 1
        (root / "evidence.json").write_bytes(encoded(manifest))
        return
    if case == "missing-attributes":
        (root / ".gitattributes").unlink()
    elif case == "forged-status":
        status = data(root, "closeout.json")
        status["b3_1_started"] = True
        (root / "closeout.json").write_bytes(encoded(status))
    else:
        table = data(root, "freeze-table.json")
        if case == "forged-coff-observation":
            table["windows_product_coff"]["accepted_path_verified"] = True
        elif case == "direction-as-freeze":
            table["direction_sufficient_for_freeze"] = True
        else:
            raise ValueError(case)
        (root / "freeze-table.json").write_bytes(encoded(table))
    (root / "evidence.json").write_bytes(encoded({"schema": 1, "files": inventory(root)}))


def replay(parent, case, reason, prepare):
    root = parent / case
    shutil.copytree(ROOT, root)
    if reason:
        tamper(root, case)
    before = snapshot(root)
    argv = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"), "--evidence-root", str(root)]
    if prepare:
        argv.append("--prepare")
    result = subprocess.run(argv, capture_output=True, encoding="utf-8", timeout=120)
    after = snapshot(root)
    error = f"B3-0F 拒绝：{reason}\n" if reason else ""
    require(before == after and result.returncode == int(bool(reason)) and result.stderr == error
            and bool(result.stdout) is (not reason), f"{case} 回执不符，退出码 {result.returncode}\n{result.stderr}")
    return {"case": case, "before": before, "after": after, "exit": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--output", type=Path, help="显式写入新回执")
    args = parser.parse_args()
    temp_root = Path(gettempdir()).resolve()
    require(not temp_root.is_relative_to(ROOT), "临时目录须位于封存证据目录之外")
    before = snapshot(ROOT)
    cases = data(ROOT, "negative-cases.json")
    with TemporaryDirectory(prefix="xiao-b3-0f-negative-", dir=temp_root) as directory:
        rows = [replay(Path(directory), case, reason, args.prepare) for case, reason in cases.items()]
    require(snapshot(ROOT) == before, "自反例复跑期间证据目录发生变化")
    if args.output:
        with args.output.open("xb") as report:
            report.write(encoded(rows))
    print(json.dumps({"baseline_passed": True, "read_only": True, "rejected_cases": len(rows) - 1}))


if __name__ == "__main__":
    main()
