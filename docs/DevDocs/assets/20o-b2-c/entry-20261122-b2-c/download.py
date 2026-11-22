"""按实际执行尝试下载 B2-C 作业日志与已校验摘要的上传原件。"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import re
import subprocess

API = "repos/Programming-Language-Xiao/Xiao/actions"


def get(path):
    return subprocess.check_output(["gh", "api", f"{API}/{path}"], timeout=120)


def save(path, api_path):
    if not path.exists():
        data = get(api_path)
        with path.open("xb") as stream:
            stream.write(data)
    return path.read_bytes()


def collect(root, run_id, attempt):
    folder = root / str(run_id)
    folder.mkdir(parents=True, exist_ok=True)
    api_path = f"runs/{run_id}"
    run_raw = get(f"{api_path}/attempts/{attempt}")
    run = json.loads(run_raw)
    if run["status"] != "completed":
        print(f"{run_id}/{attempt} {run['status']}", flush=True)
        return False
    run_path = folder / f"run-{attempt}.json"
    if not run_path.exists():
        run_path.write_bytes(run_raw)
    jobs = json.loads(save(folder / f"jobs-{attempt}.json", f"{api_path}/attempts/{attempt}/jobs?per_page=100"))
    artifacts = json.loads(save(folder / f"artifacts-{attempt}.json", f"{api_path}/artifacts?per_page=100"))

    def fetch_job(job):
        # failed-only 重跑会返回未重跑作业的聚合条目，其实际原始日志仍在第一轮。
        if attempt > 1 and job["started_at"] < run["run_started_at"]:
            return
        job_id = job["id"]
        save(folder / f"{job_id}.log", f"jobs/{job_id}/logs")

    def fetch_artifact(artifact):
        if not artifact["name"].startswith("b2-c-") or not artifact["name"].endswith(f"-{run_id}-{attempt}"):
            return
        ident = artifact["id"]
        raw = save(folder / f"{ident}.zip", f"artifacts/{ident}/zip")
        sha = hashlib.sha256(raw).hexdigest()
        if len(raw) != artifact["size_in_bytes"] or artifact["digest"] != f"sha256:{sha}":
            raise ValueError(f"GitHub 上传摘要不符：{ident}")

    with ThreadPoolExecutor(max_workers=4) as pool:
        list(pool.map(fetch_job, jobs["jobs"]))
        list(pool.map(fetch_artifact, artifacts["artifacts"]))
    print(f"{run_id}/{attempt} {run['conclusion']}，{len(jobs['jobs'])} 个作业条目", flush=True)
    return True


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("runs", nargs="+", help="run/attempt")
    args = parser.parse_args()
    if not all(re.fullmatch(r"\d+/\d+", item) for item in args.runs):
        parser.error("需要 run/attempt")
    done = [collect(args.root, *map(int, item.split("/"))) for item in args.runs]
    raise SystemExit(0 if all(done) else 1)
