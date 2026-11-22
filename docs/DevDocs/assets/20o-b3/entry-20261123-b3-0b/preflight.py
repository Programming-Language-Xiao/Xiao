"""使用实际只读审计入口复核 B3-0A，并记录 Windows 的 Bun 参数转换。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
PRIOR = ROOT.with_name("entry-20261122-b3-0")
sys.path.insert(0, str(PRIOR))
from audit_entry import archive_tools, base, inventory

PLAN = "7e6754f2e4d467debc9f6b4b79dd9e7b6b780979"
PRIOR_SEAL = "ac6865ea029ad116b0087c556e59b0f5ed6d896e"
AUDIT = PRIOR / "audit_entry.py"
ENTRY_REASON = "入口回执与原始观察不符"
MANIFEST_REASON = "清单字节或身份不符"


def snapshot(directory):
    return inventory(directory) | {"evidence.json": base.identity(directory / "evidence.json")}


def rewrite_manifest(directory):
    (directory / "evidence.json").write_bytes(archive_tools.json_bytes({"schema": 1, "files": inventory(directory)}))


def replay(task, name, *, reason=None, change=None):
    directory = task / name
    shutil.copytree(PRIOR, directory)
    expected = None
    if change:
        change(directory)
        expected = f"B3-0A 拒绝：{reason}\n"
    before = snapshot(directory)
    command = [sys.executable, "-B", "-X", "utf8", str(AUDIT), "--evidence-root", str(directory)]
    result = subprocess.run(command, capture_output=True, encoding="utf-8", timeout=60)
    base.require(before == snapshot(directory), f"B3-0A 复核改写临时副本：{name}")
    base.require(result.returncode == int(bool(change)) and result.stderr == (expected or ""),
                 f"B3-0A 实际退出或拒绝文本不符：{name}\n{result.stderr}")
    base.require(not change or not result.stdout, f"B3-0A 拒绝时出现成功输出：{name}")
    return {"case": name, "argv": command, "exit": result.returncode, "stdout": result.stdout,
            "stderr": result.stderr, "read_only": True}


def forge(directory, field):
    path = directory / "entry.json"
    row = json.loads(path.read_bytes())
    row[field] = True
    path.write_bytes(archive_tools.json_bytes(row))
    rewrite_manifest(directory)


def missing_attributes(directory):
    (directory / ".gitattributes").unlink()
    rewrite_manifest(directory)


def tampered_manifest(directory):
    with (directory / "evidence.json").open("ab") as output:
        output.write(b"\n")


def bun_argv():
    if sys.platform != "win32":
        return {"platform": sys.platform, "executed": False}
    resolved = shutil.which("bun")
    base.require(resolved is not None, "缺少 Bun")
    bun = Path(resolved).resolve(strict=True)
    version = subprocess.check_output([bun, "--version"], timeout=15).decode().strip()
    prefix = [str(bun), "-e", "console.log(JSON.stringify(process.argv.slice(-1)[0]))", "--"]
    rows = {}
    for label, unit, token in (("literal-replacement", "FFFD", "\ufffd"), ("unpaired-utf16", "D800", "\ud800"),
                               ("empty", "", "")):
        result = subprocess.run([*prefix, token], capture_output=True, encoding="utf-8", timeout=15)
        base.require(result.returncode == 0 and not result.stderr, f"Bun 参数观察失败：{label}")
        rows[label] = {"utf16_unit_hex": unit, "exit": result.returncode,
                       "stdout": result.stdout, "stderr": result.stderr}
    return {"platform": "win32", "executed": True, "tool": base.identity(bun) | {"version": version},
            "argv_prefix": prefix, "samples": rows,
            "literal_and_unpaired_same_result": rows["literal-replacement"]["stdout"] == rows["unpaired-utf16"]["stdout"]}


def capture(task):
    task.mkdir(parents=True)
    before = snapshot(PRIOR)
    relative = PRIOR.relative_to(base.REPO).as_posix()
    frozen_manifest = base.git("show", f"{PRIOR_SEAL}:{relative}/evidence.json")
    base.require(frozen_manifest == (PRIOR / "evidence.json").read_bytes(), "B3-0A 历史清单来源不符")
    success = replay(task, "unchanged")
    baseline = json.loads(success["stdout"])
    base.require(baseline["b3_0a_baseline_frozen"] is True
                 and baseline["b3_0_passed"] is baseline["decisions_approved"] is False
                 and baseline["guards"] == {"passed": 28, "actual": 4, "derived": 24}
                 and baseline["platforms"].keys() == {"windows", "linux", "macos"}, "B3-0A 前置结论不符")
    negatives = [
        replay(task, "manifest-tampered", reason=MANIFEST_REASON, change=tampered_manifest),
        replay(task, "missing-attributes", reason=MANIFEST_REASON, change=missing_attributes),
        replay(task, "entry-status-tampered", reason=ENTRY_REASON, change=lambda path: forge(path, "b3_0_passed")),
        replay(task, "false-approval", reason=ENTRY_REASON, change=lambda path: forge(path, "decisions_approved")),
    ]
    base.require(before == snapshot(PRIOR), "B3-0A 原始证据发生变化")
    report = {
        "plan_commit": PLAN, "prior_seal": PRIOR_SEAL, "prior_manifest": archive_tools.identity(frozen_manifest),
        "prior_archive": baseline["archive"], "product": baseline["product"], "guards": baseline["guards"],
        "prior_audit": success, "prior_negatives": negatives, "bun_argv": bun_argv(),
    }
    base.write_json(task / "preflight.json", report)
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("task", type=Path, help="保存回执和临时副本的全新任务根")
    args = parser.parse_args()
    data = capture(args.task.resolve())
    print(archive_tools.json_bytes({
        "baseline_frozen": True, "negative_cases": len(data["prior_negatives"]),
        "bun_argv_collision": data["bun_argv"].get("literal_and_unpaired_same_result"),
    }).decode(), end="")
