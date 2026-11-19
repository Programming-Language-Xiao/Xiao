"""用临时证据包验证 B6-2 审计的拒绝原因和只读行为。"""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from zipfile import ZIP_DEFLATED, ZipFile


ARCHIVE = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ARCHIVE))

from audit import file_identity, require, sha
from verify_boundary import MOVE
from b2_b6_macos import write_json


EVIDENCE = ARCHIVE / "measurements.zip"
SUMMARY = ARCHIVE / "summary.json"
AUDIT = ARCHIVE / "audit.py"
MANIFEST = "evidence.json"
SOURCE_RECEIPT = "windows/environment.json"
BACKEND_RECEIPT = "windows/o3/result.json"
CASES = {
    "missing_object": "证据包清单不符",
    "altered_object": f"证据字节改变：{SOURCE_RECEIPT}",
    "substitute_source_rehashed": "执行源码清单或标准库加载根不符",
    "missing_backend_rehashed": "缺少或重复三平台、VM/O0/O3测量",
}


def mutate(path, case):
    with ZipFile(EVIDENCE) as original, ZipFile(path, "x", compression=ZIP_DEFLATED) as copy:
        entries = json.loads(original.read(MANIFEST))
        target = "objects/" + entries[SOURCE_RECEIPT]["sha256"]
        if case == "substitute_source_rehashed":
            source = json.loads(original.read(target))
            source["source"]["source_commit"] = MOVE
            replacement = json.dumps(source, sort_keys=True).encode("utf-8")
            digest = sha(replacement)
            entries[SOURCE_RECEIPT] = {"sha256": digest, "bytes": len(replacement)}
            copy.writestr("objects/" + digest, replacement)
        if case == "missing_backend_rehashed":
            del entries[BACKEND_RECEIPT]
        required = {"objects/" + entry["sha256"] for entry in entries.values()}
        for member in original.namelist():
            if member not in required or (case == "missing_object" and member == target):
                continue
            data = original.read(member)
            copy.writestr(member, data + b" " if case == "altered_object" and member == target else data)
        copy.writestr(MANIFEST, json.dumps(entries, sort_keys=True) + "\n")


def identity(root):
    return {path.relative_to(root).as_posix(): file_identity(path)
            for path in sorted(root.rglob("*")) if path.is_file()}


def probe(path, expected):
    folder = path.parent
    before = identity(folder)
    argv = [sys.executable, "-B", "-X", "utf8", str(AUDIT), "--archive", str(path)]
    started = datetime.now(timezone.utc).isoformat()
    with subprocess.Popen(argv, cwd=ARCHIVE.parents[4], stdin=subprocess.DEVNULL,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE, encoding="utf-8") as child:
        try:
            stdout, stderr = child.communicate(timeout=60)
        except subprocess.TimeoutExpired:
            child.kill()
            child.communicate()
            raise RuntimeError("B6-2 审计复跑超时") from None
    require(before == identity(folder), "审计改写了临时证据")
    if expected is None:
        observed = json.loads(stdout)
        require(child.returncode == 0 and not stderr and observed == json.loads(SUMMARY.read_bytes()),
                "B6-2 原样副本复核失败")
        outcome = {key: observed[key] for key in ("verified", "matrix_complete", "perf_pass", "successful_processes")}
    else:
        final_line = stderr.splitlines()[-1] if stderr else ""
        require(child.returncode == 1 and not stdout and final_line == f"ValueError: {expected}",
                f"拒绝原因不符：{expected}，实际退出 {child.returncode}，{final_line}")
        # traceback 路径随检出目录变化。保留实际异常文本。
        outcome = {"verified": False, "stderr_last_line": final_line}
    return {
        "argv": argv, "pid": child.pid, "started_utc": started,
        "finished_utc": datetime.now(timezone.utc).isoformat(), "exit_code": child.returncode,
        "archive": before[path.name], "read_only": True, "outcome": outcome,
    }


def run():
    original_files = (EVIDENCE, SUMMARY, AUDIT)
    before = {path.name: file_identity(path) for path in original_files}
    rows = {}
    with tempfile.TemporaryDirectory(prefix="xiao-b6-2-audit-") as temp:
        for case, expected in [("baseline", None), *CASES.items()]:
            folder = Path(temp) / case
            folder.mkdir()
            path = folder / EVIDENCE.name
            if expected is None:
                shutil.copyfile(EVIDENCE, path)
            else:
                mutate(path, case)
            rows[case] = {"expected_reason": expected, **probe(path, expected)}
    require(before == {path.name: file_identity(path) for path in original_files}, "审计改写了原始证据")
    return {"passed": True, "archive_read_only": True, "original": before, "cases": rows,
            "script": file_identity(Path(__file__))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, help="独占创建反例回执")
    options = parser.parse_args()
    report = run()
    if options.report:
        write_json(options.report, report)
    print(json.dumps(report, ensure_ascii=False))


if __name__ == "__main__":
    main()
