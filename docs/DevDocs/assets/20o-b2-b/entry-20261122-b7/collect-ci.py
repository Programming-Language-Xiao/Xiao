"""下载 B7 已登记 CI 的 API 回执、原始作业日志和专项归档。"""

import argparse
from concurrent.futures import ThreadPoolExecutor
from io import BytesIO
import json
from pathlib import Path
import subprocess
from zipfile import ZipFile


API = "repos/Programming-Language-Xiao/Xiao/actions"
RUNS = (44996207048, 44996207236, 44998495908, 44998495938, 44998495850)


def api(path):
    return subprocess.check_output(["gh", "api", f"{API}/{path}"], timeout=90)


def fetch(folder, name, request):
    target = folder / name
    if not target.exists():
        data = api(request)
        with target.open("xb") as stream:
            stream.write(data)
    return target.read_bytes()


def run_data(root, run_id):
    folder = root / str(run_id)
    folder.mkdir(exist_ok=True)
    run_request = f"runs/{run_id}"
    observed = json.loads(api(run_request))
    if observed["status"] != "completed":
        print(f"{run_id} {observed['status']}", flush=True)
        return False
    attempt = observed["run_attempt"]
    run = json.loads(fetch(folder, "run.json", f"{run_request}/attempts/{attempt}"))
    jobs = json.loads(fetch(folder, "jobs.json", f"{run_request}/attempts/{attempt}/jobs?per_page=100"))
    for job in jobs["jobs"]:
        fetch(folder, f"{job['id']}.log", f"jobs/{job['id']}/logs")
    artifacts = json.loads(fetch(folder, "artifacts.json", f"{run_request}/artifacts?per_page=100"))
    for row in artifacts["artifacts"]:
        if not row["name"].startswith("b7-macos-"):
            continue
        raw = fetch(folder, f"{row['id']}.zip", f"artifacts/{row['id']}/zip")
        destination = root.parent / "macos" / row["name"]
        destination.mkdir(parents=True, exist_ok=True)
        with ZipFile(BytesIO(raw)) as archive:
            for item in archive.infolist():
                if item.is_dir():
                    continue
                name = Path(item.filename)
                if (name.is_absolute() or ".." in name.parts or len(name.parts) > 2
                        or name.suffix not in (".json", ".log", ".stdout", ".xiaoc", ".xar")
                        or (len(name.parts) == 2 and name.parts[0] not in ("native", "portable"))):
                    raise ValueError(f"macOS 专项归档路径不符：{item.filename}")
                target = destination / name
                target.parent.mkdir(parents=True, exist_ok=True)
                data = archive.read(item)
                if target.exists():
                    if target.read_bytes() != data:
                        raise ValueError(f"已提取的日志字节不符：{target}")
                else:
                    target.write_bytes(data)
    print(f"{run_id} attempt={attempt} {run['conclusion']}，作业 {jobs['total_count']}", flush=True)
    return True


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("run", nargs="*", type=int)
    args = parser.parse_args()
    args.root.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=3) as executor:
        done = list(executor.map(lambda run: run_data(args.root, run), args.run or RUNS))
    raise SystemExit(0 if all(done) else 1)
