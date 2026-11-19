"""在临时副本上复核 B6-4 审计的只读性和拒绝原因。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

sys.dont_write_bytecode = True
from evidence import DOCS_SEAL, REPO, ROOT, digest, require, write_json


CI = ROOT / "ci.zip"
DOCS = ROOT / "docs-check.zip"
SUMMARY = ROOT / "summary.json"
RUN = "ci/44464422497/run.json"
NATIVE_LOG = "ci/44464422497/135358661593.log"
DOCS_SOURCE = "docs/docs-source.json"
DOCS_PAGE = "docs/DevDocs/README.md"


def change_attempt(raw):
    run = json.loads(raw)
    run["run_attempt"] += 1
    return json.dumps(run).encode()


def change_log(raw):
    return raw + "复核标记\n".encode()


def reuse_runtime(raw):
    lines = raw.splitlines(keepends=True)
    before_index, = (index for index, line in enumerate(lines) if b" B6-4-IDENTITY before {" in line)
    native_line, = (line for line in lines if b" B6-4-IDENTITY native {" in line)
    prefix, marker, payload = lines[before_index].partition(b" B6-4-IDENTITY before ")
    before = json.loads(payload)
    before["runtime"] = json.loads(native_line.split(b" B6-4-IDENTITY native ", 1)[1])["runtime"]
    lines[before_index] = prefix + marker + json.dumps(before).encode() + b"\n"
    return b"".join(lines)


def change_docs_raw_digest(raw):
    sources = json.loads(raw)
    for phase in ("before", "after"):
        sources[phase][DOCS_PAGE]["raw"]["sha256"] = "0" * 64
    return json.dumps(sources).encode()


def change_docs_commit(raw):
    sources = json.loads(raw)
    sources["commit"] = DOCS_SEAL
    return json.dumps(sources).encode()


def change_docs_pid(raw):
    receipt = json.loads(raw)
    receipt["pid"] += 1
    return json.dumps(receipt).encode()


CASES = {
    "missing_object": (CI, RUN, None, "证据包清单不符"),
    "changed_attempt_rehashed": (CI, RUN, change_attempt, "CI 运行身份不符：44464422497"),
    "changed_job_log_rehashed": (CI, NATIVE_LOG, change_log, "归档与封存提交不符：ci.zip"),
    "runtime_present_before_build": (CI, NATIVE_LOG, reuse_runtime,
                                     "Runtime 不在本次构建目标目录，或构建前已存在：135358661593"),
    "changed_docs_raw_digest": (DOCS, DOCS_SOURCE, change_docs_raw_digest,
                                f"定稿文档原始字节摘要不符：{DOCS_PAGE}"),
    "changed_docs_commit": (DOCS, DOCS_SOURCE, change_docs_commit, "定稿门禁执行提交不符"),
    "changed_docs_pid": (DOCS, "docs/repo-check.json", change_docs_pid, "归档与封存提交不符：docs-check.zip"),
}


def write_member(archive, name, data):
    archive.writestr(ZipInfo(name), data, compress_type=ZIP_DEFLATED)


def make_copy(source, target, object_path, edit):
    with ZipFile(source) as original, ZipFile(target, "x") as changed:
        entries = json.loads(original.read("evidence.json"))
        member = "objects/" + entries[object_path]["sha256"]
        for name in original.namelist():
            if name not in ("evidence.json", member):
                write_member(changed, name, original.read(name))
        if edit is not None:
            raw = edit(original.read(member))
            entries[object_path] = digest(raw)
            write_member(changed, "objects/" + entries[object_path]["sha256"], raw)
        write_member(changed, "evidence.json", json.dumps(entries).encode())


def identities(paths):
    return {path.name: digest(path.read_bytes()) for path in paths}


def audit_case(ci, docs, expected):
    inputs = (ci, docs, SUMMARY)
    before = identities(inputs)
    argv = [sys.executable, "-B", "-X", "utf8", str(ROOT / "evidence.py"),
            "--archive", str(ci), "--docs", str(docs)]
    process = subprocess.run(argv, cwd=REPO, capture_output=True, encoding="utf-8", timeout=60)
    require(identities(inputs) == before, f"审计改写了输入：{expected}")
    actual = next(iter(reversed(process.stderr.splitlines())), None)
    if expected is None:
        require(process.returncode == 0 and not process.stderr, f"原样副本未通过：{process.stderr}")
        outcome = json.loads(process.stdout)
        require(outcome["verified"] is True and outcome["b6_4_passed"] is False
                and outcome["final_docs"]["exit"] == 0, "原样副本结论不符")
    else:
        require(process.returncode == 1 and not process.stdout and actual == f"ValueError: {expected}",
                f"审计拒绝原因不符，预期 {expected}，实际退出 {process.returncode}，{actual}")
    return {"exit": process.returncode, "expected_reason": expected, "stderr_last_line": actual,
            "read_only": True, "inputs": before}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, help="独占创建反例回执")
    args = parser.parse_args()
    original = identities((CI, DOCS, SUMMARY))
    with TemporaryDirectory(prefix="xiao-b6-4-negative-") as directory:
        temporary = Path(directory)
        ci_copy, docs_copy = (temporary / path.name for path in (CI, DOCS))
        shutil.copyfile(CI, ci_copy)
        shutil.copyfile(DOCS, docs_copy)
        rows = {"original": audit_case(ci_copy, docs_copy, None)}
        for case, (source, object_path, edit, expected) in CASES.items():
            modified = temporary / f"{case}.zip"
            make_copy(source, modified, object_path, edit)
            ci = modified if source == CI else ci_copy
            docs = modified if source == DOCS else docs_copy
            rows[case] = audit_case(ci, docs, expected)
    require(identities((CI, DOCS, SUMMARY)) == original, "原始归档或汇总发生变化")
    result = {"passed": True, "archive_read_only": True, "original": original, "cases": rows}
    if args.report:
        write_json(args.report, result)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
