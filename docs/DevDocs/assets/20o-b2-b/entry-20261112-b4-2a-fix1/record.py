"""FIX1 受控执行：每个来源独立环境，原始日志与来源回执只追加、不覆盖。"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
PRODUCT_ROOTS = [
    "core/rust", "stdlib", "tests/fixtures", "tests/benchmarks", ".github/workflows",
    ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json",
    "package.json", "bun.lock", ".gitattributes",
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(data):
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        return data
    return data if b"\0" in data else data.replace(b"\r\n", b"\n")


def git(repository, *args, data=None):
    return subprocess.check_output(["cmd", "/c", "git", *args], cwd=repository, input=data)


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as output:
        output.write(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def fixed_files(repository, revision):
    entries = []
    raw = git(repository, "ls-tree", "-r", "-z", revision, "--", *PRODUCT_ROOTS)
    for entry in raw.split(b"\0"):
        if entry:
            metadata, path = entry.split(b"\t", 1)
            _, kind, oid = metadata.split()
            require(kind == b"blob", "受保护路径必须是 Blob")
            entries.append((path.decode("utf-8"), oid))
    raw = git(repository, "cat-file", "--batch", data=b"".join(oid + b"\n" for _, oid in entries))
    files, offset = {}, 0
    for path, oid in entries:
        end = raw.index(b"\n", offset)
        header = raw[offset:end].split()
        require(header[:2] == [oid, b"blob"], "Git Blob 身份不符：" + path)
        length = int(header[2])
        files[path] = sha(canonical(raw[end + 1:end + 1 + length]))
        offset = end + 2 + length
    require(offset == len(raw), "Git Blob 流长度不符")
    return files


def worktree_files(repository):
    raw = git(repository, "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", *PRODUCT_ROOTS)
    files = {}
    for item in sorted(set(raw.split(b"\0")) - {b""}):
        path = item.decode("utf-8")
        data = (repository / path).read_bytes()
        files[path] = {"bytes": len(data), "raw_sha256": sha(data), "canonical_sha256": sha(canonical(data))}
    return files


def identity(files):
    return sha(json.dumps(files, sort_keys=True, separators=(",", ":")).encode("utf-8"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    initialize = sub.add_parser("init")
    initialize.add_argument("variant")
    initialize.add_argument("--source-root", type=Path, required=True)
    initialize.add_argument("--task-root", type=Path, required=True)
    initialize.add_argument("--revision", required=True)
    run = sub.add_parser("run")
    run.add_argument("variant")
    run.add_argument("label")
    run.add_argument("argv", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    require(Path(args.variant).name == args.variant and args.variant not in {".", ".."}, "变体名非法")
    destination = ROOT / args.variant
    if args.action == "init":
        source, task = args.source_root.resolve(), args.task_root.resolve()
        require(not destination.exists(), "证据变体已存在")
        require(not any((task / name).exists() for name in ["target", "home", "temp", "runtime"]), "拒绝复用执行目录")
        revision = git(REPOSITORY, "rev-parse", args.revision).decode().strip()
        files = worktree_files(source)
        require({path: row["canonical_sha256"] for path, row in files.items()} == fixed_files(REPOSITORY, revision), "工作树偏离固定提交")
        destination.mkdir()
        for name in ["target", "home", "temp", "runtime"]:
            (task / name).mkdir(parents=True)
        write_json(destination / "source.json", {"revision": revision, "source_root": str(source),
                    "files": files, "tree_sha256": identity(files)})
        write_json(destination / "environment.json", {
            "task_root": str(task), "source_root": str(source),
            "runtime_staticlib_built_or_used": False,
            "removed_environment_prefixes": ["XIAO_", "RUST_BIGDECIMAL_"],
            "overrides": {
                "CARGO_TARGET_DIR": str(task / "target"), "XIAO_HOME": str(task / "home"),
                "TEMP": str(task / "temp"), "TMP": str(task / "temp"),
                "XIAO_RUNTIME_LIBRARY": str(task / "runtime" / "not-built.lib"),
                "XIAO_STDLIB_ROOT": str(source / "stdlib"), "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0",
                "RUSTUP_TOOLCHAIN": "1.96.0", "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8", "CARGO_TERM_COLOR": "never",
            },
        })
        print(json.dumps({"initialized": args.variant, "revision": revision, "files": len(files)}))
        return
    require(Path(args.label).name == args.label and args.label not in {".", ".."}, "标签非法")
    argv = args.argv[1:] if args.argv[:1] == ["--"] else args.argv
    require(bool(argv), "命令不能为空")
    log, result = destination / (args.label + ".log"), destination / (args.label + ".json")
    require(not log.exists() and not result.exists(), "拒绝覆盖运行记录")
    config, source = load(destination / "environment.json"), load(destination / "source.json")
    repository = Path(source["source_root"])
    before = worktree_files(repository)
    require(before == source["files"], "运行前源码/锁变化")
    env = {key: value for key, value in os.environ.items() if not key.startswith(tuple(config["removed_environment_prefixes"]))}
    env.update(config["overrides"])
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    with log.open("xb") as output:
        process = subprocess.Popen(argv, cwd=repository, env=env, stdout=output, stderr=subprocess.STDOUT)
        exit_code = process.wait()
    after = worktree_files(repository)
    write_json(result, {"label": args.label, "argv": argv, "cwd": str(repository), "pid": process.pid,
               "started_utc": started, "finished_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
               "exit_code": exit_code, "source_before": identity(before), "source_after": identity(after),
               "source_unchanged": before == after, "log": log.name, "log_bytes": log.stat().st_size,
               "log_sha256": sha(log.read_bytes()), "runner_sha256": sha(Path(__file__).read_bytes()),
               "environment_sha256": sha((destination / "environment.json").read_bytes())})
    print(json.dumps({"variant": args.variant, "label": args.label, "exit_code": exit_code, "source_unchanged": before == after}))
    require(before == after, "运行期间源码/锁变化")
    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
