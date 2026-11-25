"""保存 B3-2 原始 Runner 回执和本机证据，按 SHA-256 存放日志和产物。"""

import argparse
import json
from pathlib import Path
from shutil import copyfileobj
import subprocess
from tempfile import TemporaryDirectory
from time import sleep
from zipfile import ZIP_DEFLATED, ZipFile

from run import HERE, ROOT, encoded, git, identity, require

REPOSITORY = "Programming-Language-Xiao/Xiao"
FIRST_PRODUCT = "a684689a10e7f0fda697318ab2b6e846e8f78fcf"
HISTORY_RUNS = {
    "first-workspace": (45641972359, FIRST_PRODUCT, "workspace"),
    "first-maintenance": (45641972063, FIRST_PRODUCT, "maintenance"),
    "second-maintenance": (45643907563, "32d2b5b4b4080f92233ce8a2b305de0d5eeff3ce", "maintenance"),
    "third-workspace": (45645230778, "a26cba096b2a4d2f2e8f585df2f2e08eb0caa2c8", "workspace"),
    "third-maintenance": (45645230803, "a26cba096b2a4d2f2e8f585df2f2e08eb0caa2c8", "maintenance"),
    "fourth-maintenance": (45648595833, "879ce625c84965dc4fe4bb11523434635be55826", "maintenance"),
}
WORKFLOW = ".github/workflows/b3-2-args-cli.yml"


def gh(*args):
    for attempt in range(3):
        try:
            return subprocess.check_output(["gh", *args], cwd=ROOT, timeout=60)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired):
            if attempt == 2:
                raise
            sleep(attempt + 1)


def get_ci(archive, name, run_id):
    base = f"repos/{REPOSITORY}/actions/runs/{run_id}"
    raw = gh("api", base)
    run = json.loads(raw)
    require(run["status"] == "completed", f"{name} CI 仍在运行")
    archive.writestr(f"ci/{name}/run.json", raw)
    archive.writestr(f"ci/{name}/jobs.json", gh("api", f"{base}/attempts/{run['run_attempt']}/jobs"))
    return run


def collect_platform(archive, name, root):
    raw = (root / "result.json").read_bytes()
    report = json.loads(raw)
    archive.writestr(f"platforms/{name}.json", raw)
    objects = [(f"logs/{command[stream]['sha256']}.log", command[stream])
               for command in report["commands"].values() for stream in ("stdout", "stderr")]
    objects.extend((obj["path"], obj) for artifact in report.get("artifacts", [])
                   for obj in (artifact["file"], artifact.get("map")) if obj is not None)
    if trace := report.get("clang_commands"):
        objects.append((trace["path"], trace))
    members = set(archive.namelist())
    for path, expected in objects:
        source = (root / path).resolve(strict=True)
        require(source.is_relative_to(root), f"证据路径越出任务根：{path}")
        data = source.read_bytes()
        digest = identity(data)
        require(digest == {field: expected[field] for field in digest}, f"平台对象摘要不符：{name}/{path}")
        member = f"objects/{digest['sha256']}"
        if member not in members:
            archive.writestr(member, data)
            members.add(member)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--windows", required=True, type=Path)
    parser.add_argument("--first-windows", required=True, type=Path)
    for label in ("runner", "workspace", "maintenance"):
        parser.add_argument(f"--{label}-run", required=True, type=int)
    args = parser.parse_args()
    source = json.loads((args.windows / "result.json").read_bytes())["source_commit"]
    with TemporaryDirectory(prefix="xiao-b3-2-collect-") as directory:
        scratch = Path(directory)
        bundle = scratch / "runs.zip"
        with ZipFile(bundle, "w", ZIP_DEFLATED) as archive:
            runner = get_ci(archive, "runner", args.runner_run)
            require(runner["conclusion"] == "success", "Linux/macOS 专项未通过")
            # Runner 提交在独立分支，在封存前读取实际入库的工作流。
            workflow = git("show", f"{runner['head_sha']}:{WORKFLOW}")
            archive.writestr("ci/runner/workflow.yml", workflow)
            for label in ("workspace", "maintenance"):
                run = get_ci(archive, label, getattr(args, f"{label}_run"))
                if label == "workspace":
                    require(run["head_sha"] == source, "工作区门禁未绑定固定产品")
            for label, (run_id, snapshot, _) in HISTORY_RUNS.items():
                run = get_ci(archive, label, run_id)
                require(run["head_sha"] == snapshot and run["conclusion"] in ("failure", "cancelled"),
                        f"{label} 历史运行身份不符")
            collect_platform(archive, "first-windows", args.first_windows.resolve(strict=True))
            collect_platform(archive, "windows", args.windows.resolve(strict=True))
            for platform in ("linux", "macos"):
                root = scratch / platform
                gh("run", "download", str(args.runner_run), "--repo", REPOSITORY,
                   "--name", f"b3-args-cli-{platform}-{runner['run_attempt']}", "--dir", str(root))
                collect_platform(archive, platform, root)
        destination = HERE / "runs.zip"
        require(not destination.exists(), "证据归档已经存在")
        with destination.open("xb") as output, bundle.open("rb") as data:
            copyfileobj(data, output)
    print(encoded({"product_commit": source, "runner_commit": runner["head_sha"]}).decode(), end="")


if __name__ == "__main__":
    main()
