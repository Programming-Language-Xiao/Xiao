"""在临时副本复跑 B3-1 证据审计，核对精确拒绝原因和只读性。"""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True
from audit import ROOT, encoded, identity, inventory, read, require


def snapshot(root):
    manifest = identity((root / "evidence.json").read_bytes())
    return identity(encoded({"files": inventory(root), "manifest": manifest}))["sha256"]


def edit_windows(root, case):
    archive = root / "windows.zip"
    temporary = root / "windows-mutated.zip"
    with ZipFile(archive) as original, ZipFile(temporary, "x", compression=ZIP_DEFLATED) as changed:
        receipt = read_receipt(original)
        if case == "forged-coff-grade":
            receipt["artifacts"][0]["verification"] = "unverified-coff-exports"
        else:
            label, old, new = {
                "missing-clock-check": ("native-exact", b"B3-1-BOUNDARY case=denied ", b"B3-1-BOUNDARY case=omitted "),
                "replaced-error-identity": ("vm-boundaries", b"time_vm_errors_and_child_clock_follow_the_frozen_contract ... ok",
                                            b"time_vm_errors_and_child_clock_follow_the_frozen_contract ... ignored"),
                "forged-tutorial-execution": ("vm-exact", b"test result: ok. 1 passed;", b"test result: ok. 0 passed;"),
                "forged-coff-map-path": ("native-exact", b"/denied.exe.map map_sha256=", b"/wrong.exe.map map_sha256="),
            }[case]
            command = receipt["commands"][label]
            raw = original.read(f"logs/{command['stdout']['sha256']}.log")
            if case == "forged-coff-map-path":
                # 先走完正斜杠的教程和边界路径核对，再拒绝明确错绑的 map。
                raw = raw.replace(b"\\", b"/")
            require(old in raw, f"反例缺少可改动的原始记录：{case}")
            raw = raw.replace(old, new, 1)
            command["stdout"] = identity(raw)
            changed.writestr(f"logs/{command['stdout']['sha256']}.log", raw)
        for name in original.namelist():
            if name != "result.json":
                changed.writestr(name, original.read(name))
        changed.writestr("result.json", encoded(receipt))
    temporary.replace(archive)


def read_receipt(archive):
    import json
    return json.loads(archive.read("result.json"))


def tamper(root, case):
    manifest = root / "evidence.json"
    if case == "manifest-tampered":
        value = read(root, "evidence.json")
        value["files"][".gitattributes"]["bytes"] += 1
        manifest.write_bytes(encoded(value))
        return
    if case == "missing-attributes":
        (root / ".gitattributes").unlink()
    elif case == "forged-status":
        status = read(root, "closeout.json")
        status["b3_1_passed"] = not status["b3_1_passed"]
        (root / "closeout.json").write_bytes(encoded(status))
    else:
        edit_windows(root, case)
    manifest.write_bytes(encoded({"schema": 1, "files": inventory(root)}))


def replay(parent, case, reason, prepare):
    target = parent / case
    shutil.copytree(ROOT, target)
    if reason:
        tamper(target, case)
    before = snapshot(target)
    argv = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"), "--evidence-root", str(target)]
    if prepare:
        argv.append("--prepare")
    run = subprocess.run(argv, capture_output=True, encoding="utf-8", timeout=120)
    after = snapshot(target)
    expected = f"B3-1 拒绝：{reason}\n" if reason else ""
    require(before == after and run.returncode == int(bool(reason)) and run.stderr == expected
            and bool(run.stdout) is (not reason), f"{case} 回执不符，退出 {run.returncode}\n{run.stderr}")
    return {"case": case, "before": before, "after": after, "exit": run.returncode, "error": reason}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    before = snapshot(ROOT)
    cases = read(ROOT, "negative-cases.json")
    with TemporaryDirectory(prefix="xiao-b3-1-negative-") as temporary:
        directory = Path(temporary).resolve()
        require(not directory.is_relative_to(ROOT), "临时目录越入封存根")
        rows = [replay(directory, case, reason, args.prepare) for case, reason in cases.items()]
    require(snapshot(ROOT) == before, "反例复跑改写了封存证据")
    if args.output:
        with args.output.open("xb") as file:
            file.write(encoded(rows))
    print(encoded({"baseline_passed": True, "rejected_cases": len(rows) - 1, "read_only": True}).decode(), end="")


if __name__ == "__main__":
    main()
