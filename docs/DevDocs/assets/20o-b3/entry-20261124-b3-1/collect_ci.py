"""封装固定产品的 time Runner 与 workspace-gates 原始 CI 回执。"""

import argparse
import json
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True
from audit import PREFIX, REPO, ROOT, encoded, git, require


def gh(*arguments):
    result = subprocess.run(["gh", *arguments], cwd=REPO, capture_output=True)
    require(result.returncode == 0, result.stderr.decode("utf-8", errors="replace"))
    return result.stdout


def ci_run(run_id, directory, label, name):
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
    require(git("status", "--porcelain=v1", "-z") == b"", "CI 采集要求干净工作树")
    workflow = git("show", f"{args.runner_commit}:.github/workflows/b3-1-time.yml")
    expected = git("show", f"{args.source_commit}:{PREFIX}/time-runner.yml").replace(
        b"B3_TIME_SOURCE_COMMIT", args.source_commit.encode())
    require(workflow == expected, "Runner 工作流与产品工作流模板不符")
    with TemporaryDirectory(prefix="xiao-b3-1-ci-") as temporary:
        work = Path(temporary)
        time, _ = ci_run(args.time_run, work / "time", "time", None)
        workspace, _ = ci_run(args.workspace_run, work / "workspace", "workspace", None)
        run_jobs = json.loads((work / "time/jobs.json").read_bytes())["jobs"]
        require(time["head_sha"] == args.runner_commit and time["conclusion"] == "success"
                and workspace["head_sha"] == args.source_commit and workspace["conclusion"] == "success",
                "CI 运行未绑定固定提交或未通过")
        with ZipFile(ROOT / "ci.zip", "x", ZIP_DEFLATED) as bundle:
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
