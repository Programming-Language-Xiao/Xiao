"""B4-5重开固定来源采集器：隔离工作树/Runtime，双摘要回执，内容寻址日志。"""
import argparse
import datetime
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
PREFIX = ROOT.relative_to(REPOSITORY).as_posix()
BASE_PATH = ROOT.parent / "entry-20261113-b4-3/record.py"
SPEC = importlib.util.spec_from_file_location("fix1_base_record", BASE_PATH)
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
PRODUCT_ROOTS = BASE.PRODUCT_ROOTS
canonical, sha, identity, git = BASE.canonical, BASE.sha, BASE.identity, BASE.git
require, load, write_json = BASE.require, BASE.load, BASE.write_json
fixed_files, worktree_files = BASE.fixed_files, BASE.worktree_files


def docs_snapshot(repository, commit=None):
    args = ["ls-tree", "-r", "--name-only", "-z", commit, "--", "docs/DevDocs"] if commit else [
        "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "docs/DevDocs"]
    names = sorted({p.decode("utf-8") for p in git(repository, *args).split(b"\0") if p})
    names = [p for p in names if "/assets/" not in p or p.startswith(PREFIX + "/") and Path(p).suffix in {".py", ".md", ".rs"}]
    if commit:
        stream = git(repository, "cat-file", "--batch", data=b"".join((commit + ":" + p + "\n").encode("utf-8") for p in names))
        blobs, offset = {}, 0
        for path in names:
            end = stream.index(b"\n", offset)
            header = stream[offset:end].split()
            require(len(header) == 3 and header[1] == b"blob", "文档Git对象缺失")
            size = int(header[2])
            blobs[path] = stream[end + 1:end + 1 + size]
            offset = end + size + 2
        require(offset == len(stream), "文档Git对象长度不符")
    else:
        blobs = {p: (repository / p).read_bytes() for p in names}
    return {"paths": names, "raw_sha256": identity({p: sha(v) for p, v in blobs.items()}),
            "canonical_sha256": identity({p: sha(canonical(v)) for p, v in blobs.items()})}


def environment(task, source):
    tools = Path(r"D:\msys64\ucrt64\bin")
    return {"task_root": str(task), "source_root": str(source), "removed_environment_prefixes": ["XIAO_", "RUST_BIGDECIMAL_"],
            "overrides": {"CARGO_TARGET_DIR": str(task / "target"), "XIAO_HOME": str(task / "home"),
                "TEMP": str(task / "temp"), "TMP": str(task / "temp"), "RUSTUP_TOOLCHAIN": "1.96.0",
                "XIAO_RUNTIME_LIBRARY": str(task / "target/release/xiao_runtime.lib"),
                "XIAO_STDLIB_ROOT": str(source / "stdlib"), "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0",
                "XIAO_TARGET_TRIPLE": "x86_64-pc-windows-msvc", "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8",
                "CARGO_TERM_COLOR": "never", **{key: str(tools / exe) for key, exe in [
                    ("XIAO_CLANG", "clang.exe"), ("XIAO_LLVM_AS", "llvm-as.exe"), ("XIAO_LLC", "llc.exe")]}}}


def runtime(config):
    path = Path(config["overrides"]["XIAO_RUNTIME_LIBRARY"])
    return {"path": str(path), "exists": path.is_file(), **({"bytes": path.stat().st_size,
             "sha256": sha(path.read_bytes())} if path.is_file() else {})}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    init = sub.add_parser("init")
    init.add_argument("variant")
    init.add_argument("revision")
    init.add_argument("--task-root", type=Path, required=True)
    init.add_argument("--worktree", action="store_true")
    run = sub.add_parser("run")
    run.add_argument("variant")
    run.add_argument("label")
    run.add_argument("argv", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    require(Path(args.variant).name == args.variant and args.variant not in {".", ".."}, "变体名非法")
    variant = ROOT / args.variant
    if args.action == "init":
        task = args.task_root
        BASE.require_new_task_root(task)
        require(not variant.exists(), "证据变体不得复用")
        revision = git(REPOSITORY, "rev-parse", args.revision).decode().strip()
        for name in ["target", "home", "temp"]:
            (task / name).mkdir(parents=True)
        source = task / "source" if args.worktree else REPOSITORY
        if args.worktree:
            git(REPOSITORY, "worktree", "add", "--detach", str(source), revision)
        files = worktree_files(source)
        require({p: r["canonical_sha256"] for p, r in files.items()} == fixed_files(REPOSITORY, revision), "来源不是固定产品快照")
        variant.mkdir()
        write_json(variant / "source.json", {"revision": revision, "source_root": str(source),
                   "files": files, "sha256": identity(files)})
        write_json(variant / "environment.json", environment(task, source))
        print(json.dumps({"initialized": args.variant, "revision": revision, "source": str(source), "files": len(files)}))
        return
    require(Path(args.label).name == args.label and args.label not in {".", ".."}, "标签非法")
    destination = variant / (args.label + ".json")
    require(not destination.exists(), "禁止覆盖运行回执")
    argv = args.argv[1:] if args.argv[:1] == ["--"] else args.argv
    require(argv, "命令不能为空")
    source, config = load(variant / "source.json"), load(variant / "environment.json")
    repository = Path(source["source_root"])
    before, docs_before = worktree_files(repository), docs_snapshot(repository)
    require(before == source["files"], "运行前产品偏离固定快照")
    commit = git(repository, "rev-parse", "HEAD").decode().strip()
    docs_commit = docs_snapshot(repository, commit)
    require(docs_before["canonical_sha256"] == docs_commit["canonical_sha256"], "文档必须先定稿提交")
    tools_commit = git(REPOSITORY, "rev-parse", "HEAD").decode().strip()
    runner = sha(Path(__file__).read_bytes())
    require(runner == sha(git(REPOSITORY, "show", tools_commit + ":" + PREFIX + "/record.py")), "采集器必须已提交")
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith(tuple(config["removed_environment_prefixes"]))}
    env.update(config["overrides"])
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    runtime_before = runtime(config)
    with tempfile.TemporaryDirectory(prefix="b4-3-fix1-stage-", dir=config["overrides"]["TEMP"]) as temporary:
        path = Path(temporary) / "output.log"
        with path.open("xb") as output:
            process = subprocess.Popen(argv, cwd=repository, env=env, stdout=output, stderr=subprocess.STDOUT)
            code = process.wait()
        raw = path.read_bytes()
    log_ref, digest = BASE.store_content_log(ROOT, raw)
    after, docs_after = worktree_files(repository), docs_snapshot(repository)
    finished = datetime.datetime.now(datetime.timezone.utc).isoformat()
    write_json(destination, {"variant": args.variant, "label": args.label, "argv": argv, "cwd": str(repository),
        "pid": process.pid, "started_utc": started, "finished_utc": finished, "exit_code": code,
        "source_before": identity(before), "source_after": identity(after), "source_unchanged": before == after,
        "docs_commit": commit, "docs_commit_tree": docs_commit, "docs_before": docs_before, "docs_after": docs_after,
        "docs_unchanged": docs_before == docs_after, "tools_commit": tools_commit, "runner_sha256": runner,
        "base_runner_sha256": sha(BASE_PATH.read_bytes()), "environment_sha256": sha((variant / "environment.json").read_bytes()),
        "runtime_before": runtime_before, "runtime_after": runtime(config), "log_ref": log_ref,
        "log_bytes": len(raw), "log_sha256": digest})
    print(json.dumps({"variant": args.variant, "label": args.label, "exit_code": code,
                      "source_unchanged": before == after, "docs_unchanged": docs_before == docs_after}))
    require(before == after and docs_before == docs_after, "运行期间产品或文档变化")
    raise SystemExit(code)


if __name__ == "__main__":
    main()
