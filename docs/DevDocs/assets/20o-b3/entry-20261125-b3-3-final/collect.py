"""归档三平台专项和 CI 的原始回执，按摘要去重。"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from zipfile import ZIP_DEFLATED, ZipFile

from audit import HERE, REPO
REPOSITORY = "Programming-Language-Xiao/Xiao"


def gh(*args):
    return subprocess.check_output(["gh", *args], cwd=REPO, timeout=60)


def keep(archive, name, data):
    member = f"objects/{hashlib.sha256(data).hexdigest()}"
    if member not in archive.namelist():
        archive.writestr(member, data)
    return {"object": member, "bytes": len(data), "source": name}


def run_receipt(archive, label, run_id):
    endpoint = f"repos/{REPOSITORY}/actions/runs/{run_id}"
    data = gh("api", endpoint)
    run = json.loads(data)
    if run["status"] != "completed":
        raise SystemExit(f"CI {label} 仍在运行")
    archive.writestr(f"ci/{label}/run.json", data)
    jobs = gh("api", f"{endpoint}/attempts/{run['run_attempt']}/jobs")
    archive.writestr(f"ci/{label}/jobs.json", jobs)
    return run


def platform(archive, name, root):
    raw = (root / "result.json").read_bytes()
    report = json.loads(raw)
    archive.writestr(f"platforms/{name}.json", raw)
    files = [(f"logs/{command[stream]['sha256']}.log", command[stream])
             for command in report["commands"].values() for stream in ("stdout", "stderr")]
    files.extend((obj["path"], obj) for artifact in report.get("artifacts", [])
                 for key in ("artifact", "map", "stdout", "stderr") if (obj := artifact.get(key)))
    if trace := report.get("clang_commands"):
        files.append((trace["path"], trace))
    for path, expected in files:
        source = (root / path).resolve(strict=True)
        if not source.is_relative_to(root):
            raise SystemExit("原件路径超出任务目录")
        ref = keep(archive, path, source.read_bytes())
        if ref["object"] != "objects/" + expected["sha256"] or ref["bytes"] != expected["bytes"]:
            raise SystemExit(f"{name} 原件摘要不符：{path}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--windows", type=Path, required=True)
    parser.add_argument("--linux", type=Path)
    parser.add_argument("--macos", type=Path)
    parser.add_argument("--previous-windows", type=Path, action="append", default=[])
    parser.add_argument("--runner", type=int, required=True)
    parser.add_argument("--maintenance", type=int, required=True)
    parser.add_argument("--workspace", type=int, required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=HERE / "runs.zip")
    args = parser.parse_args()
    args.scratch.mkdir()
    destination = args.output
    with ZipFile(destination, "x", ZIP_DEFLATED) as archive:
        runner = run_receipt(archive, "runner", args.runner)
        run_receipt(archive, "maintenance", args.maintenance)
        run_receipt(archive, "workspace", args.workspace)
        for label, run_id in {
            "first-runner": 45763139530, "first-maintenance": 45763139462,
            "first-workspace": 45763139471, "second-runner": 45764365801,
            "second-maintenance": 45764365932, "second-workspace": 45764515849,
        }.items():
            if run_id != args.workspace:
                run_receipt(archive, label, run_id)
        logs = {}
        for job_id in (139717611487, 139717611437, 139717611881, 139721595232, 139724477677):
            data = gh("api", f"repos/{REPOSITORY}/actions/jobs/{job_id}/logs")
            logs[str(job_id)] = keep(archive, f"job-{job_id}.log", data)
        archive.writestr("ci/job-logs.json", json.dumps(logs, indent=2) + "\n")
        for name in ("linux", "macos"):
            path = getattr(args, name)
            if path is None:
                path = args.scratch / name
                gh("run", "download", str(args.runner), "--repo", REPOSITORY,
                   "--name", f"b3-fs-{name}-{runner['run_attempt']}", "--dir", str(path))
            platform(archive, name, path.resolve())
        platform(archive, "windows", args.windows.resolve(strict=True))
        for path in args.previous_windows:
            platform(archive, path.name, path.resolve(strict=True))
    print(destination)


if __name__ == "__main__":
    main()
