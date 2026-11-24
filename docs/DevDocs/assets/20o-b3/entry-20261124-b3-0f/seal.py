"""采集前置审计、文档门禁和 B3-0F 收口回执。"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True
from audit import (ENTRY, GATES, PREFIX, RECEIPTS, REPO, ROOT, STAGES, derive, doc_source,
                   documents_only, encoded, expected_upstream, freeze_table, git, identity, inventory, path_at, require)


def diff_worktree(commit, *paths):
    changed = git("diff", "--name-only", "-z", commit, "--", *paths)
    untracked = git("ls-files", "--others", "--exclude-standard", "-z", "--", *paths)
    return set(filter(None, (changed + untracked).decode("utf-8").split("\0")))


def collect():
    stages = [path_at(stage, "").rstrip("/") for stage in STAGES]
    require(not diff_worktree(ENTRY, *stages), "前置证据工作树偏离固定入口")
    statuses, expected = expected_upstream()

    def run(script):
        argv = [sys.executable, "-B", "-X", "utf8", script]
        result = subprocess.run(argv, cwd=REPO, capture_output=True, encoding="utf-8", timeout=120)
        require(result.returncode == 0 and not result.stderr, f"前置脚本失败：{script}\n{result.stdout}{result.stderr}")
        return {"script": script, "exit": result.returncode, "stderr": result.stderr, "stdout": result.stdout}

    with ThreadPoolExecutor(max_workers=len(stages) + 1) as executor:
        runs = dict(zip(STAGES, executor.map(run, (row["script"] for row in expected.values())), strict=True))
        negative = executor.submit(run, path_at("E", "self_negative.py")).result()
    require(not diff_worktree(ENTRY, *stages), "复跑期间前置证据工作树发生变化")
    receipts = {}
    for stage, report in runs.items():
        stdout = json.loads(report["stdout"])
        receipts[stage] = report | {"stdout": identity(encoded(stdout)), "files": stdout["audit_files"]}
    require(receipts == expected, "前置审计输出偏离固定收口回执")
    negative["stdout"] = json.loads(negative["stdout"])
    (ROOT / "preflight.json").write_bytes(encoded({
        "source_commit": ENTRY, "stages": receipts, "b3_0e_self_negative": negative,
    }))
    (ROOT / "freeze-table.json").write_bytes(encoded(freeze_table(statuses, receipts)))
    print(json.dumps({"prior_audits": len(receipts), "prior_evidence_files": sum(row["files"] for row in receipts.values())}))


def check_worktree(commit):
    generated = {f"{PREFIX}/{name}" for name in RECEIPTS}
    changes = diff_worktree(commit) - generated
    require(not changes, f"工作树偏离受检快照：{', '.join(sorted(changes))}")


def run_gates(commit):
    check_worktree(commit)
    bun = shutil.which("bun")
    require(bun, "找不到 bun")
    checks, logs = [], {}
    for name in GATES:
        argv = [bun, "run", name]
        result = subprocess.run(argv, cwd=REPO, capture_output=True, encoding="utf-8", timeout=120)
        row = {"name": name, "argv": argv, "exit": result.returncode}
        for stream, output in (("stdout", result.stdout), ("stderr", result.stderr)):
            digest = identity(output.encode())["sha256"]
            logs.setdefault(digest, output)
            row[f"{stream}_sha256"] = digest
        checks.append(row)
        if result.returncode:
            print(result.stdout + result.stderr, file=sys.stderr, end="")
    check_worktree(commit)
    return {"source": doc_source(commit), "checks": checks, "logs": logs, "worktree_matched_before_after": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--collect", action="store_true", help="复跑 A～E 并生成冻结表")
    mode.add_argument("--document-commit", help="采集指定文档提交的门禁")
    args = parser.parse_args()
    if args.collect:
        collect()
        return
    if args.document_commit:
        commit = git("rev-parse", "--verify", args.document_commit).decode().strip()
        require(documents_only(git("diff", "--name-only", "-z", ENTRY, commit)), "受检提交包含产品改动")
        (ROOT / "doc-gates.json").write_bytes(encoded(run_gates(commit)))
    prepare = not (ROOT / "self-negative-result.json").is_file()
    result = derive(ROOT, prepare)
    (ROOT / "closeout.json").write_bytes(encoded(result))
    (ROOT / "evidence.json").write_bytes(encoded({"schema": 1, "files": inventory(ROOT)}))
    print(json.dumps({"b3_0_passed": result["b3_0_passed"], "b3_1_entry_ready": result["b3_1_entry_ready"]}))


if __name__ == "__main__":
    main()
