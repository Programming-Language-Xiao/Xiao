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


def replace_outputs(members, replacements):
    contents = dict(members)
    index = json.loads(contents["evidence.json"])
    for name, data in replacements.items():
        index[name] = digest(data)
        contents["objects/" + index[name]["sha256"]] = data
    return {"evidence.json": json_bytes(index),
            **{key: contents[key] for key in {"objects/" + item["sha256"] for item in index.values()}}}


def with_stdout(members, stem, transform):
    index = json.loads(members["evidence.json"])
    log, receipt = (f"linux/{stem}.{extension}" for extension in ("stdout", "json"))
    changed = transform(members["objects/" + index[log]["sha256"]])
    row = json.loads(members["objects/" + index[receipt]["sha256"]])
    row["outputs"]["stdout"] = digest(changed)
    return replace_outputs(members, {log: changed, receipt: json_bytes(row)})


def wrong_exports(raw):
    require(raw.count(b"exports=164") == 1, "对账原始标记不唯一")
    return raw.replace(b"exports=164", b"exports=163")


def conflicting_docs_report(raw):
    report = json.loads(raw)
    require(report["passed"] is True, "仓库门禁基线须为通过")
    report["diagnostics"].append({"severity": "error", "code": "A0-DOCS-004"})
    return json_bytes(report)


def replay(archive, status, reason, *, locked_docs=ROOT / "repo-check-locked.zip"):
    inputs = (archive, status, locked_docs, ROOT / "handoff.json")
    before = [digest(path.read_bytes()) for path in inputs]
    command = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"),
               "--archive", str(archive), "--status", str(status), "--locked-docs", str(locked_docs)]
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


def write_archive(path, members):
    with ZipFile(path, "x") as zipped:
        for name, raw in members.items():
            zipped.writestr(name, raw)


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
        numeric = task / "numeric-status.json"
        numeric.write_bytes(json_bytes(json.loads(status.read_bytes()) | {"b7_entry_ready": 1}))
        cases["numeric-status"] = replay(archive, numeric, "封存状态与固定证据派生结果不符")

        repacked = task / "repacked.zip"
        write_archive(repacked, members)
        matching = task / "repacked-status.json"
        summary = json.loads(status.read_bytes())
        summary["archive"].update(digest(repacked.read_bytes()))
        matching.write_bytes(json_bytes(summary))
        cases["repacked-with-matching-status"] = replay(repacked, matching, "归档与固定收口 Git 对象不符")

        # 删除对象，保留原清单。
        removed = dict(members)
        removed.pop(next(name for name in removed if name.startswith("objects/")))
        variants = {
            "missing-object": (removed, "证据包清单不符"),
            "rehashed-exports": (with_stdout(members, "exports", wrong_exports), "Linux 导出对账输出不符"),
            "conflicting-docs-report": (with_stdout(members, "repo-check-final", conflicting_docs_report),
                                        "仓库门禁原始报告不符"),
        }
        for case, (contents, reason) in variants.items():
            temporary = task / f"{case}.zip"
            write_archive(temporary, contents)
            cases[case] = replay(temporary, status, reason)

        with ZipFile(ROOT / "repo-check-locked.zip") as zipped:
            supplement = {name: zipped.read(name) for name in zipped.namelist()}
        index = json.loads(supplement["evidence.json"])
        name = "linux/cargo.log"
        cargo = supplement["objects/" + index[name]["sha256"]]
        require(b'"metadata", "--locked", ' in cargo, "受控 Cargo 基线参数缺失")
        unlocked = cargo.replace(b'"metadata", "--locked", ', b'"metadata", ', 1)
        temporary = task / "unlocked-cargo.zip"
        write_archive(temporary, replace_outputs(supplement, {name: unlocked}))
        cases["unlocked-cargo"] = replay(archive, status, "仓库门禁的 Cargo 子进程未按预期锁定",
                                        locked_docs=temporary)
    print(json.dumps({"passed": True, "state_matrix": verified, "cases": cases}, ensure_ascii=False))


if __name__ == "__main__":
    main()
