"""采集修复后的原始平台产物、CI 和文档门禁回执。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
from zipfile import ZIP_DEFLATED, ZipFile

from audit import (
    CI_RUNS, FIRST_RUN, FIRST_WINDOWS_JOB, GENERATED, HERE, LINUX_COMPONENTS_JOB, PREFIX, REPO, check_ci,
    checks, derive, document_source, git, identity, load_original, manifest, require,
)

original = load_original("collect")


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()


def store_job_log(archive, label, job):
    endpoint = f"repos/{original.REPOSITORY}/actions/jobs/{job}/logs"
    raw = original.gh("api", endpoint)
    original.keep(archive, f"job-{job}.log", raw)
    archive.writestr(f"ci/{label}-log.json", json_bytes({"job": job, **identity(raw)}))


def capture(args):
    args.scratch.mkdir()
    destination = args.scratch / "runs.zip"
    with ZipFile(destination, "x", ZIP_DEFLATED) as archive:
        for label, run_id in CI_RUNS.items():
            run = original.run_receipt(archive, label, run_id)
            for attempt in range(1, run["run_attempt"]):
                jobs = original.gh("api", f"repos/{original.REPOSITORY}/actions/runs/{run_id}/attempts/{attempt}/jobs")
                archive.writestr(f"ci/{label}/attempt-{attempt}-jobs.json", jobs)
        original.run_receipt(archive, "first-runner", FIRST_RUN)
        first = checks.read_json(archive, "ci/first-runner/jobs.json")["jobs"]
        macos = next(job["id"] for job in first if job["name"] == "B3-3 fs (macos)")
        store_job_log(archive, "first-macos", macos)
        store_job_log(archive, "first-windows-maintenance", FIRST_WINDOWS_JOB)
        current = check_ci(archive, REPO)
        store_job_log(archive, "linux-components", LINUX_COMPONENTS_JOB)
        original.platform(archive, "windows", args.windows.resolve(strict=True))
        for name in ("linux", "macos"):
            path = getattr(args, name)
            if path is None:
                path = args.scratch / name
                original.gh("run", "download", str(current["runner"]["run"]), "--repo", original.REPOSITORY,
                            "--name", f"b3-fs-{name}-{current['runner']['attempt']}", "--dir", str(path))
            original.platform(archive, name, path.resolve(strict=True))
    require(not (HERE / "runs.zip").exists(), "回执归档已存在")
    shutil.copyfile(destination, HERE / "runs.zip")


def check_worktree(commit):
    require(git(REPO, "rev-parse", "HEAD").decode().strip() == commit, "文档提交发生变化")
    changed = (git(REPO, "diff", "--name-only", "-z", commit)
               + git(REPO, "ls-files", "--others", "--exclude-standard", "-z"))
    allowed = {f"{PREFIX}/{name}" for name in GENERATED}
    require(set(filter(None, changed.decode().split("\0"))) <= allowed, "存在未定稿文件")


def seal():
    commit = git(REPO, "rev-parse", "HEAD").decode().strip()
    check_worktree(commit)
    bun = shutil.which("bun")
    require(bun, "找不到 Bun")
    rows, logs = [], {}
    for name in checks.GATES:
        command = [bun, "run", name]
        result = subprocess.run(command, capture_output=True, cwd=REPO, timeout=120)
        row = {"name": name, "argv": command, "exit": result.returncode}
        for stream in ("stdout", "stderr"):
            raw = getattr(result, stream)
            digest = identity(raw)["sha256"]
            row[stream] = digest
            logs.setdefault(digest, raw.decode("utf-8"))
        require(result.returncode == 0, name + " 未通过\n" + logs[row["stdout"]] + logs[row["stderr"]])
        rows.append(row)
    check_worktree(commit)
    doc_gates = {"source": document_source(REPO, commit), "matched_before_after": True,
                 "checks": rows, "logs": logs}
    (HERE / "doc-gates.json").write_bytes(json_bytes(doc_gates))
    status = derive(HERE, REPO)
    (HERE / "status.json").write_bytes(json_bytes(status))
    (HERE / "evidence.json").write_bytes(json_bytes({"files": manifest(HERE)}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    archive = commands.add_parser("capture")
    archive.add_argument("--windows", type=Path, required=True)
    archive.add_argument("--scratch", type=Path, required=True)
    archive.add_argument("--linux", type=Path)
    archive.add_argument("--macos", type=Path)
    commands.add_parser("seal")
    args = parser.parse_args()
    if args.command == "capture":
        capture(args)
    else:
        seal()


if __name__ == "__main__":
    main()
