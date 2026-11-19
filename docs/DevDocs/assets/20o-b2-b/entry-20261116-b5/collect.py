"""显式采集B5-0来源证据；不构建/改写产品或历史产物。"""
import argparse
from datetime import datetime, timezone
from io import BytesIO
import json
from pathlib import Path
import subprocess
import sys
import zipfile

sys.dont_write_bytecode = True
from facts import (ROOT, REPOSITORY, ENTRY, PRODUCT, B1, B1_ROOTS, PRODUCT_ROOTS, RUNS, REQUIRED,
                   candidate, describe, fixed_paths, git, inventory, load, require, sha, source_facts, write_json, zip_inventory)


def store(raw, suffix):
    digest = sha(raw)
    path = ROOT / "objects/sha256" / digest[:2] / (digest + suffix)
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == raw, "B5内容对象损坏")
    else:
        path.write_bytes(raw)
    return {"path": path.relative_to(ROOT).as_posix(), **describe(raw)}


def command(argv, timeout=45):
    started = datetime.now(timezone.utc).isoformat()
    process = subprocess.Popen(argv, cwd=REPOSITORY, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        stdout, stderr = process.communicate(timeout=timeout)
        status = process.returncode
    except subprocess.TimeoutExpired:
        process.kill()
        stdout, stderr = process.communicate()
        status = None
    return {"argv": argv, "cwd": str(REPOSITORY), "pid": process.pid,
            "started_utc": started, "finished_utc": datetime.now(timezone.utc).isoformat(),
            "exit_code": status, "timeout_seconds": timeout,
            "stdout": store(stdout, ".log"), "stderr": store(stderr, ".log")}, stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collect", action="store_true", required=True)
    parser.parse_args()
    require(not (ROOT / "preflight.json").exists() and not (ROOT / "objects").exists(), "B5采集目录不可复用")
    facts = source_facts(REPOSITORY)
    require(not git(REPOSITORY, "diff", "--name-only", ENTRY, "--", *PRODUCT_ROOTS).strip(), "B5开工产品已变化")
    sources = {name: fixed_paths(REPOSITORY, commit, B1_ROOTS)
               for name, commit in [("b1_fixed", B1), ("entry_fixed", ENTRY)]}
    history, history_raw = command(["cmd", "/c", "git", "log", "--all", "--pretty=format:", "--name-only", "--", *B1_ROOTS])
    require(history["exit_code"] == 0, "B5历史路径查询失败")
    history_paths = sorted(set(history_raw.decode("utf-8").splitlines()) - {""})
    sources["all_local_history"] = history_paths
    archives = []
    for relative in fixed_paths(REPOSITORY, ENTRY, B1_ROOTS):
        if relative.endswith(".zip"):
            path = REPOSITORY / relative
            raw = git(REPOSITORY, "show", ENTRY + ":" + relative)
            require(path.read_bytes() == raw, "B1历史ZIP工作树漂移")
            archives.append({"path": relative, **describe(raw), "members": zip_inventory(path)})
    remote = []
    for run in RUNS:
        argv = ["gh", "api", "repos/Programming-Language-Xiao/Xiao/actions/runs/" + run + "/artifacts?per_page=100"]
        receipt, raw = command(argv)
        entry = {"run_id": run, "receipt": receipt, "archives": []}
        if receipt["exit_code"] == 0:
            data = json.loads(raw)
            require(data["total_count"] == len(data["artifacts"]), "B5远端产物列表未完整分页")
            entry["listing"] = data
            print(json.dumps({"run": run, "artifacts": data["total_count"]}), flush=True)
            for artifact in data["artifacts"]:
                require(artifact["size_in_bytes"] <= 1024 * 1024 and not artifact["expired"], "B5远端归档需另行核对大小或有效期")
                download, zipped = command(["gh", "api", "repos/Programming-Language-Xiao/Xiao/actions/artifacts/" + str(artifact["id"]) + "/zip"])
                row = {"artifact_id": artifact["id"], "name": artifact["name"], "receipt": download}
                if download["exit_code"] == 0:
                    row["archive"] = store(zipped, ".zip")
                    with zipfile.ZipFile(BytesIO(zipped)) as archive:
                        row["members"] = [{"path": member.filename, "bytes": member.file_size,
                                           "sha256": sha(archive.read(member))}
                                          for member in archive.infolist() if not member.is_dir()]
                    require(not artifact.get("digest") or artifact["digest"] == "sha256:" + sha(zipped), "B5远端归档摘要不符")
                else:
                    row["unavailable"] = True
                entry["archives"].append(row)
        else:
            entry["unavailable"] = True
        remote.append(entry)
    current = fixed_paths(REPOSITORY, ENTRY, ["docs/DevDocs/assets"])
    b4_artifacts = [path for path in current if candidate(path)]
    parent_audit, audit_raw = command(["python", "-B", "-X", "utf8",
        "docs/DevDocs/assets/20o-b2-b/entry-20261116-b4-5-rerun/audit.py"], timeout=120)
    require(parent_audit["exit_code"] == 0 and json.loads(audit_raw)["b4_5_closed"] is True, "B4-5入口审计未通过")
    binary_paths = {scope: [path for path in paths if candidate(path)] for scope, paths in sources.items()}
    zip_binary = [row["path"] + ":" + item["path"] for row in archives for item in row["members"] if candidate(item["path"])]
    remote_binary = [str(row["artifact_id"]) + ":" + member["path"]
                     for run in remote for row in run["archives"] for member in row.get("members", []) if candidate(member["path"])]
    require(not any(binary_paths.values()) and not zip_binary and not remote_binary, "发现B1原始产物候选，需核对来源后继续")
    report = {"schema_version": 1, "stage": "B5-0", "evidence_valid": True, "stage_passed": False,
              "blocked": True, "blocker_code": "B5-PREFLIGHT-001", "missing_required": REQUIRED,
              "product_started": False, "b5_closed": False, "b6_started": False,
              "basis": "B5 plan section 2 gate 4: missing original B1 artifacts blocks product work",
              "source_facts": facts, "source_paths": sources, "binary_candidates": binary_paths,
              "history_query": history, "local_archives": archives, "archive_binary_candidates": zip_binary,
              "remote_queries": remote, "remote_binary_candidates": remote_binary,
              "b4_artifacts_not_b1": b4_artifacts, "b4_5_audit": parent_audit,
              "scope_limit": "Only listed repository Git history, B1 ZIP archives and two recorded CI runs were searched; external stores are unknown",
              "next_input": "Provide archived B1 original .xiaoc/.xar/native metadata with source/run identity; do not synthesize replacements",
              "not_executed": ["ABI1.19 implementation", "B5 acceptance/rejection matrix", "B5 cache regeneration",
                               "B5 C ABI compatibility", "B5 product tests", "remote CI", "performance", "721 F10"]}
    write_json(ROOT / "preflight.json", report)
    print(json.dumps({"evidence_valid": True, "stage_passed": False, "blocked": True,
                      "missing_required": REQUIRED}, ensure_ascii=False))


if __name__ == "__main__":
    main()
