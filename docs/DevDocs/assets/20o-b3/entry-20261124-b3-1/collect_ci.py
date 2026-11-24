"""封装固定产品的 time Runner 与 workspace-gates 原始 CI 回执。"""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import time
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True
from audit import GENERATED, PREFIX, REPO, ROOT, encoded, git, require


def gh(*arguments):
    for attempt in range(3):
        result = subprocess.run(["gh", *arguments], cwd=REPO, capture_output=True)
        if result.returncode == 0:
            return result.stdout
        if attempt < 2:
            time.sleep(2 ** (attempt + 1))
    raise RuntimeError(result.stderr.decode("utf-8", errors="replace"))


def ci_run(run_id, directory, name):
    run = json.loads(gh("api", f"repos/Programming-Language-Xiao/Xiao/actions/runs/{run_id}"))
    jobs = json.loads(gh("api", f"repos/Programming-Language-Xiao/Xiao/actions/runs/{run_id}/jobs"))
    directory.mkdir(parents=True)
    (directory / "run.json").write_bytes(encoded(run))
    (directory / "jobs.json").write_bytes(encoded(jobs))
    if name:
        gh("run", "download", str(run_id), "--repo", "Programming-Language-Xiao/Xiao",
           "--name", name, "--dir", str(directory / "artifact"))
    return run, jobs


def write_tree(bundle, folder, prefix):
    if not folder.exists():
        return
    for path in sorted(folder.rglob("*")):
        if path.is_file():
            bundle.write(path, prefix + path.relative_to(folder).as_posix())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--runner-commit", required=True)
    parser.add_argument("--document-commit", required=True)
    parser.add_argument("--time-run", required=True, type=int)
    parser.add_argument("--workspace-run", required=True, type=int)
    args = parser.parse_args()
    changed = set(filter(None, git("diff", "--name-only", "-z").decode().split("\0")))
    untracked = set(filter(None, git("ls-files", "--others", "--exclude-standard", "-z").decode().split("\0")))
    allowed_outputs = {f"{PREFIX}/{name}" for name in GENERATED}
    require(not changed and untracked <= allowed_outputs,
            "CI 采集要求固定源文件；只允许存在证据审计生成物")
    workflow = git("show", f"{args.runner_commit}:.github/workflows/b3-1-time.yml")
    expected = git("show", f"{args.source_commit}:{PREFIX}/time-runner.yml").replace(
        b"B3_TIME_SOURCE_COMMIT", args.source_commit.encode())
    require(workflow == expected, "Runner 工作流与产品工作流模板不符")
    with TemporaryDirectory(prefix=".xiao-b3-1-ci-", dir=ROOT) as temporary:
        work = Path(temporary)
        time, _ = ci_run(args.time_run, work / "time", None)
        workspace, _ = ci_run(args.workspace_run, work / "workspace", None)
        run_jobs = json.loads((work / "time/jobs.json").read_bytes())["jobs"]
        require(time["head_sha"] == args.runner_commit and time["conclusion"] == "success"
                and workspace["head_sha"] == args.source_commit and workspace["conclusion"] == "success",
                "CI 运行未绑定固定提交或未通过")
        ci_archive = work / "ci.zip"
        with ZipFile(ci_archive, "w", ZIP_DEFLATED) as bundle:
            bundle.writestr("time/run.json", (work / "time/run.json").read_bytes())
            bundle.writestr("time/jobs.json", (work / "time/jobs.json").read_bytes())
            bundle.writestr("time/workflow.yml", workflow)
            for job in run_jobs:
                runner_os = "Linux" if "Linux" in job["name"] else "macOS"
                require(job["conclusion"] == "success", f"time {runner_os} 作业未通过")
                name = f"b3-time-{runner_os}-1"
                gh("run", "download", str(args.time_run), "--repo", "Programming-Language-Xiao/Xiao",
                   "--name", name, "--dir", str(work / runner_os))
                write_tree(bundle, work / runner_os, f"{runner_os.lower()}/")
            bundle.writestr("workspace/run.json", (work / "workspace/run.json").read_bytes())
            bundle.writestr("workspace/jobs.json", (work / "workspace/jobs.json").read_bytes())
        ci_archive.replace(ROOT / "ci.zip")
    matrix = {
        "source_commit": args.source_commit,
        "runner_commit": args.runner_commit,
        "document_commit": args.document_commit,
        "time_run": args.time_run,
        "workspace_run": args.workspace_run,
    }
    (ROOT / "matrix.json").write_bytes(encoded(matrix))
    print(encoded(matrix).decode("utf-8"), end="")


if __name__ == "__main__":
    main()
