"""在临时副本运行真实FIX.2审计，核对清单、缺失文件和伪造状态的实际拒绝原因。"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
ARCHIVE = ROOT.with_name("entry-20261118-b4-component-fix")
REPOSITORY = ROOT.parents[4]
AUDIT = ARCHIVE / "audit.py"
PREFIX = ARCHIVE.relative_to(REPOSITORY).as_posix()


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def inventory(root):
    result = {}
    for path in sorted(root.rglob("*")):
        if path.is_file():
            raw = path.read_bytes()
            result[path.relative_to(root).as_posix()] = {
                "bytes": len(raw), "sha256": sha(raw),
            }
    return result


def git(*args):
    result = subprocess.run(
        ["cmd", "/c", "git", "-C", str(REPOSITORY), *args],
        capture_output=True, encoding="utf-8", errors="strict", check=True,
    )
    return result.stdout.strip()


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write_new(path, value):
    with path.open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))


def rewrite(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n",
                    encoding="utf-8", newline="\n")


def rehash_manifest(root):
    path = root / "evidence.json"
    value = read(path)
    files = inventory(root)
    files.pop("evidence.json")
    value["file_count_excluding_manifest"] = len(files)
    value["files"] = files
    rewrite(path, value)


def run_audit(root):
    before = inventory(root)
    argv = [sys.executable, "-B", "-X", "utf8", str(AUDIT),
            "--root", str(root), "--repository-root", str(REPOSITORY)]
    env = os.environ.copy()
    env.update(PYTHONUTF8="1", PYTHONIOENCODING="utf-8")
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    child = subprocess.Popen(
        argv, cwd=REPOSITORY, env=env, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        encoding="utf-8", errors="strict",
    )
    try:
        stdout, stderr = child.communicate(timeout=60)
    except subprocess.TimeoutExpired:
        child.kill()
        child.communicate()
        raise RuntimeError("FIX.2反例审计超过60秒；不计作预期拒绝") from None
    elapsed = time.monotonic() - clock
    finished = datetime.now(timezone.utc).isoformat()
    unchanged = before == inventory(root)
    require(not stderr and unchanged, "FIX.2反例审计抛出异常或修改了证据")
    outcome = json.loads(stdout)
    require(outcome["read_only"] is True, "FIX.2反例审计未报告只读")
    return {
        "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished,
        "elapsed_seconds": elapsed, "exit_code": child.returncode,
        "stdout": stdout, "stderr": stderr, "read_only": unchanged,
        "outcome": outcome,
    }


def mutate(root, case):
    if case == "missing_attributes":
        (root / ".gitattributes").unlink()
    elif case == "missing_registered_file":
        (root / "README.md").unlink()
    elif case in {"file_count", "manifest_digest"}:
        path = root / "evidence.json"
        value = read(path)
        if case == "file_count":
            value["file_count_excluding_manifest"] += 1
        else:
            value["files"]["README.md"]["sha256"] = "0" * 64
        rewrite(path, value)
    elif case in {"forged_b6_state", "forged_windows_complete"}:
        path = root / "result.json"
        value = read(path)
        if case == "forged_b6_state":
            value["b6_handoff"]["b6_2_started"] = True
        else:
            for row in value["native_wrapper_rows"]["windows"]:
                row["verification"] = "Complete"
        rewrite(path, value)
        rehash_manifest(root)
    elif case == "rewritten_file_and_manifest":
        path = root / "README.md"
        path.write_bytes(path.read_bytes() + b"\nFIX2-NEGATIVE-TAMPER\n")
        rehash_manifest(root)
    else:
        raise ValueError("未知反例：" + case)


def collect():
    manifest_commit = git("log", "-1", "--format=%H", "--", PREFIX + "/evidence.json")
    manifest = read(ARCHIVE / "evidence.json")
    baseline_manifest_sha = sha((ARCHIVE / "evidence.json").read_bytes())
    cases = [
        ("missing_attributes", "FIX.2证据缺少.gitattributes", False),
        ("missing_registered_file", "FIX.2清单文件缺失：README.md", False),
        ("file_count", "FIX.2清单文件数不符", False),
        ("manifest_digest", "FIX.2清单与实际字节不符", False),
        ("forged_b6_state", "FIX.2交接越过B6-1自有重跑", True),
        ("forged_windows_complete", "FIX.2 Windows组件完整性被冒进标记", True),
        ("rewritten_file_and_manifest", "FIX.2固定Git清单字节不符", True),
    ]
    original = inventory(ARCHIVE)
    rows = []
    with tempfile.TemporaryDirectory(prefix="xiao-fix2-audit-negative-") as temporary:
        base = Path(temporary) / "baseline"
        shutil.copytree(ARCHIVE, base)
        baseline = run_audit(base)
        require(baseline["exit_code"] == 0 and baseline["outcome"]["passed"] is True
                and baseline["outcome"]["fix2_closed"] is True,
                "FIX.2真实基线审计未通过：" + baseline["stdout"])
        rows.append({"case": "baseline", "expected_reason": None,
                     "manifest_rebuilt": False, **baseline})
        for case, expected_reason, rebuilt in cases:
            copy = Path(temporary) / case
            shutil.copytree(ARCHIVE, copy)
            mutate(copy, case)
            outcome = run_audit(copy)
            require(outcome["exit_code"] == 1
                    and outcome["outcome"]["passed"] is False
                    and outcome["outcome"].get("reason") == expected_reason,
                    "FIX.2反例未按预期原因拒绝：" + case + "：" + outcome["stdout"])
            rows.append({"case": case, "expected_reason": expected_reason,
                         "manifest_rebuilt": rebuilt, **outcome})
    require(original == inventory(ARCHIVE), "FIX.2原始归档在反例执行中发生变化")
    return {
        "passed": True, "archive_read_only": True,
        "source_manifest_commit": manifest_commit,
        "source_manifest_sha256": baseline_manifest_sha,
        "docs_receipt": manifest["docs_receipt"],
        "code_commit": manifest["code_commit"],
        "audit_sha256": sha(AUDIT.read_bytes()),
        "script_sha256": sha(Path(__file__).read_bytes()),
        "case_count": len(rows), "cases": rows,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", action="store_true", help="新建报告与补充证据清单")
    args = parser.parse_args()
    if args.record:
        require(not (ROOT / "negative-cases.json").exists()
                and not (ROOT / "evidence.json").exists(), "FIX.2补充反例报告禁止覆盖")
    report = collect()
    if args.record:
        write_new(ROOT / "negative-cases.json", report)
        files = inventory(ROOT)
        require(".gitattributes" in files, "FIX.2补充清单缺少.gitattributes")
        write_new(ROOT / "evidence.json", {
            "generated_from_actual_bytes": True,
            "generator": "self_negative.py --record",
            "source_manifest_commit": report["source_manifest_commit"],
            "file_count_excluding_manifest": len(files), "files": files,
        })
    print(json.dumps(report, ensure_ascii=False))


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    main()
