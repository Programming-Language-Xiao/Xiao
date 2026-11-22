"""在隔离副本中复跑 B2-C 审计并核对每项拒绝原因与只读性。"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True
from audit import CLOSEOUT, MANIFEST, REJECTION_REASONS, ROOT, base, inventory, json_bytes
from performance_audit import identity


def modify(path, operation):
    data = json.loads(path.read_bytes())
    operation(data)
    path.write_bytes(json_bytes(data))


def modify_archive(root, mutations):
    original = root / "execution.zip"
    modified = root / "modified.zip"
    with ZipFile(original) as source, ZipFile(modified, "x", compression=ZIP_DEFLATED, compresslevel=1) as target:
        entries = json.loads(source.read("evidence.json"))
        retained = {}
        for name, info in entries.items():
            if name in mutations:
                operation = mutations[name]
                if operation is None:
                    continue
                data = json.loads(source.read("objects/" + info["sha256"]))
                operation(data)
                raw = json_bytes(data)
                entries[name] = identity(raw)
                retained[entries[name]["sha256"]] = raw
        entries = {name: info for name, info in entries.items()
                   if name not in mutations or mutations[name] is not None}
        for sha in sorted({info["sha256"] for info in entries.values()}):
            key = "objects/" + sha
            target.writestr(key, retained[sha] if sha in retained else source.read(key))
        target.writestr("evidence.json", json_bytes(entries))
    modified.replace(original)


def alter(root, name):
    if name == "manifest-tampered":
        def tamper(manifest):
            manifest["files"]["README.md"]["sha256"] = "0" * 64
        modify(root / MANIFEST, tamper)
        return
    if name == "missing-gitattributes":
        (root / ".gitattributes").unlink()
    elif name == "forged-status":
        def forge_status(status):
            status["b2c_passed"] = not status["b2c_passed"]
        modify(root / CLOSEOUT, forge_status)
    else:
        f10 = "linux-f10-vm"
        candidate = "windows-candidates/candidates-tutorial-final/cmath-tutorial/result.json"
        def forge_ci(result):
            job, = [job for job in result["jobs"] if job["id"] == 137179416348]
            job["head_sha"] = "0" * 40
        mutations = {
            "f10-missing": {f"{f10}/report.json": lambda row: row["records"].pop(),
                            f"{f10}/measurement.json": lambda row: row.update(record_count=102)},
            "f10-duplicate": {f"{f10}/report.json": lambda row: row["records"].__setitem__(-1, row["records"][0])},
            "candidate-missing-o3": {candidate: lambda row: row["native"].pop()},
            "candidate-missing-platform": {"windows-candidates/environment.json": None},
            "forged-source": {f"{f10}/environment.json": lambda row: row["source"].update(commit="0" * 40)},
            "forged-runtime": {f"{f10}/runtime.json": lambda row: row["runtime_after"].update(sha256="0" * 64)},
            "forged-ci-job": {"ci/45013071077/jobs-1.json": forge_ci},
        }
        modify_archive(root, mutations[name])
    (root / MANIFEST).write_bytes(json_bytes({"schema": 1, "files": inventory(root)}))


def fingerprint(root):
    return base.sha(json_bytes(inventory(root) | {MANIFEST: base.identity(root / MANIFEST)}))


def execute(parent, name, preflight):
    root = parent / name
    shutil.copytree(ROOT, root)
    if name != "unchanged":
        alter(root, name)
    before = fingerprint(root)
    argv = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"), "--evidence-root", str(root)]
    if preflight:
        argv.append("--preflight")
    result = subprocess.run(argv, cwd=ROOT, capture_output=True, encoding="utf-8", timeout=120)
    after = fingerprint(root)
    expected = REJECTION_REASONS.get(name)
    stderr = f"B2-C 拒绝：{expected}\n" if expected else ""
    base.require(before == after and result.returncode == (1 if expected else 0) and result.stderr == stderr
                 and (expected is None or result.stdout == ""),
                 f"{name} 的实际拒绝或只读性不符：{result.returncode} {result.stderr}")
    if not expected:
        base.require(json.loads(result.stdout)["b2c_passed"] is not preflight, "正向对照的阶段状态不符")
    return {"case": name, "argv": argv, "exit": result.returncode, "rejection": expected, "read_only": True,
            "before": before, "after": after}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", action="store_true")
    parser.add_argument("--output", type=Path, help="写入独立执行回执")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="xiao-b2-c-audit-") as directory, ThreadPoolExecutor(max_workers=3) as pool:
        cases = list(pool.map(lambda name: execute(Path(directory), name, args.preflight),
                              ("unchanged", *REJECTION_REASONS)))
    result = json_bytes({"mode": "preflight" if args.preflight else "sealed", "cases": cases})
    if args.output:
        with args.output.open("xb") as handle:
            handle.write(result)
    print(result.decode(), end="")
