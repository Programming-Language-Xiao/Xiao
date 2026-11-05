"""A0补充复核：规范文本摘要、引用来源与独立CI查询；不运行产品。"""

import argparse
import datetime as dt
import hashlib
import json
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
ARCHIVE = ROOT.parent / "entry-20261105"
REVIEWED_COMMIT = "d88f68a718d781bd1aeea1ecf7e1b9a1317580d9"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(raw):
    raw.decode("utf-8", errors="strict")
    return raw.replace(b"\r\n", b"\n")


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write(path, value):
    assert not path.exists(), "不覆盖已采集记录：" + str(path)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def collect():
    identity = read(ARCHIVE / "identity.json")
    source = Path(identity["source_root"])
    argv = ["cmd", "/c", "git", "cat-file", "--batch"]
    request = "".join(identity["head"] + ":" + item["path"] + "\n" for item in identity["files"]).encode()
    process = subprocess.Popen(argv, cwd=REPOSITORY, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    output, error = process.communicate(request)
    (ROOT / "git-blobs.stderr.log").write_bytes(error)
    assert process.returncode == 0, "读取固定源码Git blob失败"
    offset = 0
    files = []
    for item in identity["files"]:
        end = output.index(b"\n", offset)
        object_id, kind, length = output[offset:end].split()
        assert kind == b"blob"
        size = int(length)
        blob = output[end + 1:end + 1 + size]
        offset = end + size + 2
        raw = (source / item["path"]).read_bytes()
        current = (REPOSITORY / item["path"]).read_bytes()
        normalized = canonical(raw)
        assert sha(raw) == item["sha256"] and len(raw) == item["bytes"], item["path"]
        assert normalized == canonical(blob) == canonical(current), item["path"]
        files.append({"path": item["path"], "source_raw_bytes": len(raw),
                      "source_raw_sha256": item["sha256"], "utf8_lf_bytes": len(normalized),
                      "utf8_lf_sha256": sha(normalized), "source_crlf_count": raw.count(b"\r\n"),
                      "git_blob_id": object_id.decode("ascii"), "git_blob_raw_sha256": sha(blob),
                      "git_blob_utf8_lf_sha256": sha(canonical(blob)), "main_raw_sha256": sha(current),
                      "main_crlf_count": current.count(b"\r\n"), "main_utf8_lf_sha256": sha(canonical(current)),
                      "canonical_bytes_equal": True})
    assert offset == len(output)
    write(ROOT / "text-identities.json", {
        "source_commit": identity["head"], "reviewed_commit": REVIEWED_COMMIT,
        "collected_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "source_root": str(source), "main_root": str(REPOSITORY),
        "normalization": "严格UTF-8；只将CRLF替换为LF；不删除BOM、不补换行、不改变其他字节",
        "raw_identity_preserved": "../entry-20261105/identity.json",
        "command": {"argv": argv, "cwd": str(REPOSITORY), "pid": process.pid,
                    "exit_code": process.returncode, "stdin_requests": request.decode("utf-8").splitlines(),
                    "stdout_sha256": sha(output), "stderr_sha256": sha(error)},
        "file_count": len(files), "files": files,
    })
    initial = read(ARCHIVE / "result.json")
    closeout = read(ARCHIVE / "closeout.json")
    source_gate = read(ARCHIVE / "repo-all-local-copy.json")
    workspace_gate = read(ARCHIVE / "workspace-doc-gate.json")
    assert not initial["passed"] and initial["windows"] == "failed"
    assert closeout["passed"] and closeout["effective_repo_all"] == "repo-all-local-copy.json"
    assert source_gate["exit_code"] == workspace_gate["exit_code"] == 0
    assert source_gate["cwd"] == str(source) and workspace_gate["cwd"] == str(REPOSITORY)
    write(ROOT / "reference-map.json", {
        "source_commit": identity["head"], "reviewed_commit": REVIEWED_COMMIT,
        "effective_conclusion": {"file": "../entry-20261105/closeout.json", "passed": True,
                                 "scope": "保留初轮失败、环境修复及成功重试后的有效收口"},
        "initial_execution": {"file": "../entry-20261105/result.json", "passed": False,
                              "windows": "failed", "may_be_cited_as_passed": False},
        "source_document_gate": {"file": "../entry-20261105/repo-all-local-copy.json",
                                 "cwd": source_gate["cwd"], "exit_code": 0},
        "workspace_supplement": {"file": "../entry-20261105/workspace-doc-gate.json",
                                 "cwd": workspace_gate["cwd"], "exit_code": 0,
                                 "may_replace_source_document_gate": False},
        "full_a0_rerun_performed": False, "ci_used_as_local_evidence": False,
        "original_files": [{"file": name, "sha256": sha((ARCHIVE / name).read_bytes())} for name in
                           ["identity.json", "result.json", "closeout.json", "repo-all-local-copy.json",
                            "workspace-doc-gate.json", "evidence.json"]],
    })
    print(json.dumps({"canonical_files": len(files), "all_equal": True, "initial_passed": False,
                      "effective_closeout_passed": True}, ensure_ascii=False))


def query_ci():
    argv = [r"D:\Program Files\GitHub CLI\gh.exe", "api",
            "repos/Programming-Language-Xiao/Xiao/actions/runs?head_sha=" + REVIEWED_COMMIT + "&per_page=100"]
    record = {"commit": REVIEWED_COMMIT, "argv": argv, "cwd": str(REPOSITORY),
              "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
              "scope": "只查询d88f68a7外部CI，不代表本次补注文档提交，不代替A0本地基线",
              "used_as_local_evidence": False}
    process = subprocess.Popen(argv, cwd=REPOSITORY, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    record["pid"] = process.pid
    try:
        output, error = process.communicate(timeout=35)
        record["timed_out"] = False
    except subprocess.TimeoutExpired:
        process.kill()
        output, error = process.communicate()
        record["timed_out"] = True
    (ROOT / "ci-response.json").write_bytes(output)
    (ROOT / "ci-stderr.log").write_bytes(error)
    record.update(exit_code=process.returncode, finished_utc=dt.datetime.now(dt.timezone.utc).isoformat(),
                  response_sha256=sha(output), stderr_sha256=sha(error), runs=[])
    if process.returncode == 0:
        record["query_status"] = "queried"
        for run in json.loads(output)["workflow_runs"]:
            assert run["head_sha"] == REVIEWED_COMMIT
            record["runs"].append({key: run[key] for key in
                                   ["id", "name", "event", "status", "conclusion", "head_sha", "run_attempt", "html_url"]})
    else:
        record["query_status"] = "不可验证：查询未成功"
    write(ROOT / "ci-query.json", record)
    print(json.dumps({"query_status": record["query_status"], "exit_code": record["exit_code"],
                      "runs": record["runs"]}, ensure_ascii=False))


def check():
    identities = read(ROOT / "text-identities.json")
    references = read(ROOT / "reference-map.json")
    assert identities["file_count"] == len(identities["files"])
    request = "".join(identities["source_commit"] + ":" + item["path"] + "\n" for item in identities["files"]).encode()
    blobs = subprocess.check_output(["cmd", "/c", "git", "cat-file", "--batch"], input=request, cwd=REPOSITORY)
    offset = 0
    for item in identities["files"]:
        assert item["source_raw_sha256"] == next(row["sha256"] for row in read(ARCHIVE / "identity.json")["files"] if row["path"] == item["path"])
        assert item["utf8_lf_sha256"] == item["git_blob_utf8_lf_sha256"] == item["main_utf8_lf_sha256"]
        end = blobs.index(b"\n", offset)
        object_id, kind, length = blobs[offset:end].split()
        assert object_id.decode("ascii") == item["git_blob_id"] and kind == b"blob"
        blob = blobs[end + 1:end + 1 + int(length)]
        assert sha(canonical(blob)) == item["utf8_lf_sha256"]
        offset = end + int(length) + 2
    assert offset == len(blobs)
    for item in references["original_files"]:
        assert sha((ARCHIVE / item["file"]).read_bytes()) == item["sha256"]
    assert not read(ARCHIVE / "result.json")["passed"] and read(ARCHIVE / "closeout.json")["passed"]
    assert references["source_document_gate"]["cwd"] == read(ARCHIVE / "repo-all-local-copy.json")["cwd"]
    assert references["workspace_supplement"]["cwd"] == read(ARCHIVE / "workspace-doc-gate.json")["cwd"]
    assert references["source_document_gate"]["cwd"] != references["workspace_supplement"]["cwd"]
    ci = read(ROOT / "ci-query.json")
    assert sha((ROOT / "ci-response.json").read_bytes()) == ci["response_sha256"]
    assert sha((ROOT / "ci-stderr.log").read_bytes()) == ci["stderr_sha256"]
    assert ci["commit"] == REVIEWED_COMMIT and not ci["used_as_local_evidence"]
    if ci["query_status"] == "queried":
        assert ci["exit_code"] == 0
        assert all(run["head_sha"] == REVIEWED_COMMIT for run in read(ROOT / "ci-response.json")["workflow_runs"])
    inventory = read(ROOT / "evidence.json")
    actual = {path.relative_to(ROOT).as_posix() for path in ROOT.rglob("*") if path.is_file() and path.name != "evidence.json"}
    assert actual == {item["path"] for item in inventory["files"]} and len(actual) == inventory["file_count"]
    for item in inventory["files"]:
        path = (ROOT / item["path"]).resolve()
        assert path.is_relative_to(ROOT)
        raw = path.read_bytes()
        assert len(raw) == item["bytes"] and sha(raw) == item["sha256"], item["path"]
    print(json.dumps({"passed": True, "canonical_files": identities["file_count"],
                      "effective_conclusion": "closeout.json", "initial_execution_passed": False,
                      "scope": "只读补充证据复核；不重新执行A0或产品"}, ensure_ascii=False))


def manifest():
    files = []
    for path in sorted(ROOT.rglob("*")):
        if path.is_file() and path.name != "evidence.json":
            raw = path.read_bytes()
            files.append({"path": path.relative_to(ROOT).as_posix(), "bytes": len(raw), "sha256": sha(raw)})
    (ROOT / "evidence.json").write_text(json.dumps({"scope": "仅本次补充目录，清单自身不计入自身摘要",
        "reviewed_commit": REVIEWED_COMMIT, "file_count": len(files), "files": files},
        ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(json.dumps({"supplement_files": len(files)}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["collect", "ci", "check", "manifest"])
    command = parser.parse_args().command
    {"collect": collect, "ci": query_ci, "check": check, "manifest": manifest}[command]()
