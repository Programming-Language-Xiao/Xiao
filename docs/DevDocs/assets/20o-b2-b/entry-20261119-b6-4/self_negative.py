"""在临时副本上复核 B6-4 审计的只读性和拒绝原因。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True
from evidence import REPO, ROOT, digest, require, write_json


ARCHIVE = ROOT / "ci.zip"
RUN = "ci/44464422497/run.json"
CASES = {
    "original": None,
    "missing_object": "证据包清单不符",
    "changed_attempt_rehashed": "CI 运行身份不符：44464422497",
}


def make_copy(target, case):
    if case == "original":
        shutil.copyfile(ARCHIVE, target)
        return
    with ZipFile(ARCHIVE) as original, ZipFile(target, "x", compression=ZIP_DEFLATED) as changed:
        entries = json.loads(original.read("evidence.json"))
        member = "objects/" + entries[RUN]["sha256"]
        for name in original.namelist():
            if name not in ("evidence.json", member):
                changed.writestr(name, original.read(name))
        if case == "changed_attempt_rehashed":
            run = json.loads(original.read(member))
            run["run_attempt"] += 1
            raw = json.dumps(run).encode("utf-8")
            entries[RUN] = digest(raw)
            changed.writestr("objects/" + entries[RUN]["sha256"], raw)
        changed.writestr("evidence.json", json.dumps(entries).encode("utf-8"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, help="独占创建反例回执")
    args = parser.parse_args()
    original = digest(ARCHIVE.read_bytes())
    rows = {}
    with TemporaryDirectory(prefix="xiao-b6-4-negative-") as directory:
        for case, expected in CASES.items():
            copy = Path(directory) / f"{case}.zip"
            make_copy(copy, case)
            before = digest(copy.read_bytes())
            argv = [sys.executable, "-B", "-X", "utf8", str(ROOT / "evidence.py"),
                    "--archive", str(copy), "--ci-only"]
            process = subprocess.run(argv, cwd=REPO, capture_output=True, encoding="utf-8", timeout=60)
            require(digest(copy.read_bytes()) == before, f"审计改写了副本：{case}")
            lines = process.stderr.splitlines()
            actual = lines[-1] if lines else None
            if expected:
                require(process.returncode == 1 and not process.stdout and actual == f"ValueError: {expected}",
                        f"审计拒绝原因不符：{case}，退出 {process.returncode}，{actual}")
            else:
                require(process.returncode == 0 and not process.stderr and json.loads(process.stdout)["verified"],
                        f"原样副本未通过：{process.stderr}")
            rows[case] = {"exit": process.returncode, "expected_reason": expected,
                          "stderr_last_line": actual, "read_only": True, "archive": before}
    require(digest(ARCHIVE.read_bytes()) == original, "原始归档发生变化")
    result = {"passed": True, "archive_read_only": True, "original": original, "cases": rows}
    if args.report:
        write_json(args.report, result)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
