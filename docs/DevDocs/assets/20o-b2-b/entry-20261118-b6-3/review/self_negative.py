"""用临时副本复核 B6-3 审计的拒绝原因和只读性。"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from zipfile import ZIP_DEFLATED, ZipFile

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT))

from audit import digest
from prepare import REPO, file_identity, require, utc_now, write_json
from run import frame, frames


BOUNDARIES = ROOT / "boundaries.zip"
REPO_CHECK = ROOT / "repo-check.zip"
AUDIT = ROOT / "audit.py"
NATIVE = "windows/all/o0/bytes-exact"
VM = "windows/all/vm/digits-over"
GUARD = "guards/windows/zero"
CASES = {
    "baseline": None,
    "missing_object": "证据包清单不符",
    "missing_attempt_rehashed": "边界归档与固定收口汇总不符",
    "wrong_build_target": "构建回执不符：windows/runtime-build",
    "wrong_message_id": f"F-12 消息标识不符：{VM}",
    "reused_native_output": f"原生产物或组件核对不符：{NATIVE}",
    "unrelated_guard_failure": "计数守卫拒绝原因不符：windows/zero",
    "missing_repo_check": "仓库门禁归档缺失",
    "missing_repo_check_path": "仓库门禁文件清单不符",
}


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()


def mutate_boundaries(destination, case):
    with ZipFile(BOUNDARIES) as original, ZipFile(destination, "x", ZIP_DEFLATED) as copy:
        entries = json.loads(original.read("evidence.json"))
        replacement = {}

        def read(name):
            member = "objects/" + entries[name]["sha256"]
            return replacement[member] if member in replacement else original.read(member)

        def put(name, data):
            entries[name] = digest(data)
            replacement["objects/" + entries[name]["sha256"]] = data

        def replace_stream(command, stream, data):
            filename = f"{command}.{stream}"
            receipt = json.loads(read(command + ".json"))
            put(filename, data)
            receipt["files"][Path(filename).name] = entries[filename]
            put(command + ".json", json_bytes(receipt))

        missing = None
        if case == "missing_object":
            missing = "objects/" + entries["windows/environment.json"]["sha256"]
        elif case == "missing_attempt_rehashed":
            del entries["attempt-logs/linux-02.log"]
        elif case == "wrong_build_target":
            name = "windows/runtime-build.json"
            build = json.loads(read(name))
            build["argv"][-1] = "xiao-types"
            put(name, json_bytes(build))
        elif case == "wrong_message_id":
            command = f"{VM}/protocol"
            hello, response = frames(read(command + ".stdout"))
            response["report"]["message_id"] = "runtime.invalid_value"
            replace_stream(command, "stdout", frame(hello) + frame(response))
        elif case == "reused_native_output":
            protocol = f"{NATIVE}/protocol"
            stdin_hello, request = frames(read(protocol + ".stdin"))
            stdout_hello, response = frames(read(protocol + ".stdout"))
            program_path = f"{NATIVE}/program.json"
            program = json.loads(read(program_path))
            output = request["output"].replace("bytes-exact", "bytes-over-before-syntax")
            request["output"] = response["artifact"]["executable"] = program["argv"][0] = output
            replace_stream(protocol, "stdin", frame(stdin_hello) + frame(request))
            replace_stream(protocol, "stdout", frame(stdout_hello) + frame(response))
            put(program_path, json_bytes(program))
        elif case == "unrelated_guard_failure":
            replace_stream(f"{GUARD}/guard", "stderr", b"PowerShell input could not be read\n")

        for member in sorted({"objects/" + item["sha256"] for item in entries.values()}):
            if member != missing:
                copy.writestr(member, replacement[member] if member in replacement else original.read(member))
        copy.writestr("evidence.json", json_bytes(entries))


def mutate_repo_check(destination):
    with ZipFile(REPO_CHECK) as original, ZipFile(destination, "x", ZIP_DEFLATED) as copy:
        receipt = json.loads(original.read("receipt.json"))
        for field in ("source_before", "source_after"):
            del receipt[field]["docs/DevDocs/README.md"]
        copy.writestr("receipt.json", json_bytes(receipt))
        for name in ("stdout.json", "stderr.log"):
            copy.writestr(name, original.read(name))


def identities(folder):
    return {path.name: file_identity(path) for path in folder.iterdir() if path.is_file()}


def probe(folder, case):
    folder.mkdir()
    archive = folder / BOUNDARIES.name
    repo_check = folder / REPO_CHECK.name
    if case == "baseline" or case.startswith("missing_repo_check"):
        shutil.copyfile(BOUNDARIES, archive)
    else:
        mutate_boundaries(archive, case)
    if case == "missing_repo_check_path":
        mutate_repo_check(repo_check)
    elif case != "missing_repo_check":
        shutil.copyfile(REPO_CHECK, repo_check)

    before = identities(folder)
    argv = [sys.executable, "-B", "-X", "utf8", str(AUDIT),
            "--archive", str(archive), "--repo-check", str(repo_check)]
    if case == "missing_attempt_rehashed":
        argv.extend(("--report", str(folder / "report.json")))
    started = utc_now()
    with subprocess.Popen(argv, cwd=REPO, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, encoding="utf-8") as child:
        try:
            stdout, stderr = child.communicate(timeout=60)
        except subprocess.TimeoutExpired:
            child.kill()
            child.communicate()
            raise RuntimeError(f"自反例审计超时：{case}") from None
    finished = utc_now()
    require(identities(folder) == before, f"审计改动了证据副本：{case}")
    expected = CASES[case]
    if expected is None:
        summary = json.loads(stdout) if child.returncode == 0 else None
        require(not stderr and summary == json.loads((ROOT / "summary.json").read_bytes()),
                f"原样副本未通过：{stderr}")
        outcome = {"verified": True, "language_runs": summary["language_runs"]}
    else:
        lines = stderr.splitlines()
        actual = lines[-1] if lines else ""
        require(child.returncode == 1 and not stdout and actual == f"ValueError: {expected}",
                f"拒绝原因不符：{case}，实际退出 {child.returncode}，{actual}")
        outcome = {"verified": False, "stderr_last_line": actual}
    return {"expected_reason": expected, "exit_code": child.returncode, "pid": child.pid,
            "started_utc": started, "finished_utc": finished, "inputs": before,
            "read_only": True, "outcome": outcome}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, help="独占创建反例回执")
    args = parser.parse_args()
    original = (BOUNDARIES, REPO_CHECK, AUDIT, ROOT / "summary.json")
    before = {path.name: file_identity(path) for path in original}
    with tempfile.TemporaryDirectory(prefix="xiao-b6-3-review-") as workspace:
        with ThreadPoolExecutor(max_workers=3) as pool:
            futures = {case: pool.submit(probe, Path(workspace) / case, case) for case in CASES}
            results = {case: future.result() for case, future in futures.items()}
    require(before == {path.name: file_identity(path) for path in original}, "审计改动了原始证据")
    report = {"passed": True, "archive_read_only": True, "original": before,
              "script": file_identity(Path(__file__)), "cases": results}
    if args.report:
        write_json(args.report, report)
    print(json.dumps({"passed": True, "baseline": 1, "rejected": len(results) - 1,
                      "archive_read_only": True}, ensure_ascii=False))


if __name__ == "__main__":
    main()
