"""枚举 B6-5 派生状态，并在临时副本回放只读审计反例。"""

import argparse
from itertools import product
import json
from pathlib import Path
import runpy
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZipFile

sys.dont_write_bytecode = True
from record import REPO, ROOT, digest, json_bytes, require


def matrix():
    audit = runpy.run_path(str(ROOT / "audit.py"))
    acceptance, closeout = (audit[key] for key in ("ACCEPTANCE", "CLOSEOUT"))
    summary = {"checked": 0, "b6_accepted": 0, "b6_5_passed": 0, "b7_entry_ready": 0}
    for values in product((False, True), repeat=len(acceptance + closeout)):
        inputs = dict(zip(acceptance + closeout, values))
        states = audit["derive"](inputs)
        require(states == {"b6_accepted": False not in values[:len(acceptance)],
                           "b6_5_passed": False not in values[len(acceptance):],
                           "b7_entry_ready": False not in values[len(acceptance):]}, "状态组合派生不符")
        summary["checked"] += 1
        for name, value in states.items():
            summary[name] += int(value)
    return summary


def rewritten_exports(members):
    contents = dict(members)
    index = json.loads(contents["evidence.json"])
    names = ("linux/exports.stdout", "linux/exports.json")
    log, receipt = (contents["objects/" + index[name]["sha256"]] for name in names)
    require(log.count(b"exports=164") == 1, "对账原始标记不唯一")
    changed = log.replace(b"exports=164", b"exports=163")
    row = json.loads(receipt)
    row["outputs"]["stdout"] = digest(changed)
    for name, data in zip(names, (changed, json_bytes(row))):
        index[name] = digest(data)
        contents["objects/" + index[name]["sha256"]] = data
    return {"evidence.json": json_bytes(index),
            **{key: contents[key] for key in {"objects/" + item["sha256"] for item in index.values()}}}


def replay(archive, status, reason):
    inputs = (archive, status)
    before = [digest(path.read_bytes()) for path in inputs]
    command = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"),
               "--archive", str(archive), "--status", str(status)]
    process = subprocess.run(command, cwd=REPO, capture_output=True, encoding="utf-8")
    after = [digest(path.read_bytes()) for path in inputs]
    require(before == after, "审计修改了输入")
    if reason is None:
        require(process.returncode == 0 and not process.stderr and json.loads(process.stdout)["b6_5_passed"],
                "未改动基线未通过")
    else:
        require(process.returncode == 1 and not process.stdout and process.stderr.strip() == reason,
                f"反例拒绝原因不符，预期 {reason}，实际 {process.stderr.strip()}")
    return {"expected_reason": reason, "observed_reason": process.stderr.strip() or None,
            "exit": process.returncode, "read_only": before == after}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    archive, status = (ROOT / name for name in ("closeout.zip", "status.json"))
    with ZipFile(archive) as zipped:
        members = {name: zipped.read(name) for name in zipped.namelist()}
    verified = matrix()
    cases = {"unchanged": replay(archive, status, None)}
    with TemporaryDirectory(prefix="xiao-b6-5-negative-") as task:
        task = Path(task)
        forged = task / "status.json"
        fields = json.loads(status.read_bytes()) | {"b6_accepted": True, "b6_4_passed": True, "22d_open": False}
        forged.write_bytes(json_bytes(fields))
        cases["forged-stage-pass"] = replay(archive, forged, "封存状态与固定证据派生结果不符")

        # 删除对象，保留原清单。
        removed = dict(members)
        removed.pop(next(name for name in removed if name.startswith("objects/")))
        variants = {
            "missing-object": (removed, "证据包清单不符"),
            "rehashed-exports": (rewritten_exports(members), "Linux 导出对账输出不符"),
        }
        for case, (contents, reason) in variants.items():
            temporary = task / f"{case}.zip"
            with ZipFile(temporary, "x") as zipped:
                for name, raw in contents.items():
                    zipped.writestr(name, raw)
            cases[case] = replay(temporary, status, reason)
    print(json.dumps({"passed": True, "state_matrix": verified, "cases": cases}, ensure_ascii=False))


if __name__ == "__main__":
    main()
