"""显式采集 B4-2A 自审记录；不覆盖已有运行，不生成通过结论。"""
import argparse
import datetime
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[5]
ENTRY = "4042aba5"
SOURCE = "09fa912b"
PRODUCT_ROOTS = [
    "core/rust", "stdlib", "tests/fixtures", "tests/benchmarks", ".github/workflows",
    ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json",
    "package.json", "bun.lock", ".gitattributes",
]


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(data):
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        return data
    return data if b"\0" in data else data.replace(b"\r\n", b"\n")


def git(repository, *args):
    return subprocess.check_output(["cmd", "/c", "git", *args], cwd=repository)


def tree_sha(files):
    return sha(json.dumps(files, sort_keys=True, separators=(",", ":")).encode("utf-8"))


def fixed_files(repository, commit):
    archive = git(repository, "archive", "--format=tar", commit, "--", *PRODUCT_ROOTS)
    with tarfile.open(fileobj=io.BytesIO(archive)) as stream:
        return {member.name: sha(canonical(stream.extractfile(member).read()))
                for member in stream if member.isfile()}


def worktree_files(repository):
    names = git(repository, "ls-files", "-z", "--cached", "--others", "--exclude-standard",
                "--", *PRODUCT_ROOTS)
    files = {}
    for raw in set(names.split(b"\0")) - {b""}:
        name = raw.decode("utf-8")
        files[name] = sha(canonical((repository / name).read_bytes()))
    return files


def write_json(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    initialize = sub.add_parser("init")
    initialize.add_argument("--task-root", type=Path, required=True)
    run = sub.add_parser("run")
    run.add_argument("label")
    run.add_argument("argv", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.action == "init":
        task = args.task_root.resolve()
        require(not task.exists(), "拒绝复用已有任务根")
        require(not (ROOT / "source.json").exists(), "拒绝覆盖来源记录")
        files = worktree_files(REPOSITORY)
        require(files == fixed_files(REPOSITORY, SOURCE), "工作树与固定测试提交不一致")
        for name in ["target", "home", "temp", "unbuilt-runtime"]:
            (task / name).mkdir(parents=True)
        write_json(ROOT / "source.json", {
            "source_commit": git(REPOSITORY, "rev-parse", SOURCE).decode().strip(),
            "entry_commit": git(REPOSITORY, "rev-parse", ENTRY).decode().strip(),
            "canonical_tree_sha256": tree_sha(files), "files": files,
        })
        write_json(ROOT / "environment.json", {
            "task_root": str(task), "repository": str(REPOSITORY),
            "overrides": {
                "CARGO_TARGET_DIR": str(task / "target"),
                "XIAO_HOME": str(task / "home"), "TEMP": str(task / "temp"),
                "TMP": str(task / "temp"),
                "XIAO_RUNTIME_LIBRARY": str(task / "unbuilt-runtime" / "xiao_runtime.lib"),
                "XIAO_STDLIB_ROOT": str(REPOSITORY / "stdlib"),
                "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0", "RUSTUP_TOOLCHAIN": "1.96.0",
                "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8", "CARGO_TERM_COLOR": "never",
            },
            "runtime_staticlib_built_or_used": False,
            "removed_environment_prefixes": ["XIAO_", "RUST_BIGDECIMAL_"],
        })
        print(json.dumps({"initialized": True, "files": len(files), "task_root": str(task)}))
        return
    require(Path(args.label).name == args.label and args.label not in {".", ".."}, "非法标签")
    argv = args.argv[1:] if args.argv[:1] == ["--"] else args.argv
    require(bool(argv), "执行命令不能为空")
    log, record = ROOT / (args.label + ".log"), ROOT / (args.label + ".json")
    require(not log.exists() and not record.exists(), "拒绝覆盖已有运行")
    config = load(ROOT / "environment.json")
    source = load(ROOT / "source.json")
    before = worktree_files(REPOSITORY)
    require(before == source["files"], "运行前源码身份不符")
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(tuple(config["removed_environment_prefixes"]))}
    env.update(config["overrides"])
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    with log.open("xb") as output:
        process = subprocess.Popen(argv, cwd=REPOSITORY, env=env, stdout=output,
                                   stderr=subprocess.STDOUT)
        exit_code = process.wait()
    after = worktree_files(REPOSITORY)
    write_json(record, {
        "label": args.label, "argv": argv, "cwd": str(REPOSITORY), "pid": process.pid,
        "started_utc": started, "finished_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "exit_code": exit_code, "source_before": tree_sha(before), "source_after": tree_sha(after),
        "source_unchanged": before == after, "log": log.name, "log_bytes": log.stat().st_size,
        "log_sha256": sha(log.read_bytes()), "environment_sha256": sha((ROOT / "environment.json").read_bytes()),
        "runner_sha256": sha(Path(__file__).read_bytes()),
    })
    print(json.dumps({"label": args.label, "exit_code": exit_code, "source_unchanged": before == after}))
    require(before == after, "执行期间源码或锁文件变化")
    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
