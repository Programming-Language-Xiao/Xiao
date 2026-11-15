"""B4-4固定任务采集：源码/文档双摘要，内容寻址日志，回执独占写入。"""
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
SPEC = importlib.util.spec_from_file_location("b4_3_record", ROOT.parent / "entry-20261113-b4-3/record.py")
BASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BASE)
PRODUCT_ROOTS = BASE.PRODUCT_ROOTS
canonical, sha, identity, git = BASE.canonical, BASE.sha, BASE.identity, BASE.git
require, load, write_json = BASE.require, BASE.load, BASE.write_json
fixed_files, worktree_files = BASE.fixed_files, BASE.worktree_files


def docs_paths(repository, commit=None):
    """绑定全部DevDocs正文与本批静态工具，历史证据不反复扫描。"""
    args = ["ls-tree", "-r", "--name-only", "-z", commit, "--", "docs/DevDocs"] if commit else [
        "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "docs/DevDocs"]
    names = {name.decode("utf-8") for name in git(repository, *args).split(b"\0") if name}
    return sorted(name for name in names if "/assets/" not in name or
                  name.startswith(PREFIX + "/") and Path(name).suffix in {".py", ".md", ".rs"})


def docs_snapshot(repository, commit=None):
    paths = docs_paths(repository, commit)
    if commit:
        inputs = b"".join((commit + ":" + path + "\n").encode("utf-8") for path in paths)
        stream = git(repository, "cat-file", "--batch", data=inputs)
        blobs, offset = {}, 0
        for path in paths:
            end = stream.index(b"\n", offset)
            header = stream[offset:end].split()
            require(len(header) == 3 and header[1] == b"blob", "文档Git对象缺失")
            size = int(header[2])
            blobs[path] = stream[end + 1:end + 1 + size]
            offset = end + size + 2
        require(offset == len(stream), "文档Git对象流长度不符")
    else:
        blobs = {path: (repository / path).read_bytes() for path in paths}
    return {"paths": paths, "raw_sha256": identity({p: sha(v) for p, v in blobs.items()}),
            "canonical_sha256": identity({p: sha(canonical(v)) for p, v in blobs.items()})}


def runtime_snapshot(environment):
    runtime = Path(environment["overrides"]["XIAO_RUNTIME_LIBRARY"])
    return {"path": str(runtime), "exists": runtime.is_file(),
            **({"bytes": runtime.stat().st_size, "sha256": sha(runtime.read_bytes())} if runtime.is_file() else {})}


def environment(variant, task):
    return {"task_root": str(task), "source_root": str(REPOSITORY),
            "removed_environment_prefixes": ["XIAO_", "RUST_BIGDECIMAL_"], "overrides": {
                "CARGO_TARGET_DIR": str(task / "target"), "XIAO_HOME": str(task / "home"),
                "TEMP": str(task / "temp"), "TMP": str(task / "temp"),
                "XIAO_RUNTIME_LIBRARY": str(task / "target/release/xiao_runtime.lib"),
                "XIAO_STDLIB_ROOT": str(REPOSITORY / "stdlib"),
                "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0", "RUSTUP_TOOLCHAIN": "1.96.0",
                "XIAO_TARGET_TRIPLE": "x86_64-pc-windows-msvc",
                **{name: str(Path(r"D:\msys64\ucrt64\bin") / tool) for name, tool in [
                    ("XIAO_CLANG", "clang.exe"), ("XIAO_LLVM_AS", "llvm-as.exe"), ("XIAO_LLC", "llc.exe")]},
                "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8", "CARGO_TERM_COLOR": "never"}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    init = sub.add_parser("init")
    init.add_argument("variant")
    init.add_argument("revision")
    init.add_argument("--task-root", type=Path, required=True)
    run = sub.add_parser("run")
    run.add_argument("variant")
    run.add_argument("label")
    run.add_argument("argv", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    require(Path(args.variant).name == args.variant and args.variant not in {".", ".."}, "非法变体")
    variant = ROOT / args.variant
    if args.action == "init":
        BASE.require_new_task_root(args.task_root)
        require(not variant.exists(), "禁止复用证据变体")
        source = worktree_files(REPOSITORY)
        revision = git(REPOSITORY, "rev-parse", args.revision).decode().strip()
        require({p: r["canonical_sha256"] for p, r in source.items()} == fixed_files(REPOSITORY, revision),
                "产品树不是固定Git快照")
        variant.mkdir()
        for name in ["target", "home", "temp"]:
            (args.task_root / name).mkdir(parents=True)
        write_json(variant / "source.json", {"revision": revision, "files": source, "sha256": identity(source)})
        write_json(variant / "environment.json", environment(args.variant, args.task_root))
        print(json.dumps({"initialized": args.variant, "revision": revision, "product_files": len(source)}))
        return
    require(Path(args.label).name == args.label and args.label not in {".", ".."}, "非法运行标签")
    target = variant / (args.label + ".json")
    require(not target.exists(), "禁止覆盖运行回执")
    argv = args.argv[1:] if args.argv[:1] == ["--"] else args.argv
    require(bool(argv), "命令不能为空")
    source, config = load(variant / "source.json"), load(variant / "environment.json")
    before, docs_before = worktree_files(REPOSITORY), docs_snapshot(REPOSITORY)
    require(before == source["files"], "运行前产品变化")
    commit = git(REPOSITORY, "rev-parse", "HEAD").decode().strip()
    docs_commit = docs_snapshot(REPOSITORY, commit)
    require(docs_before["canonical_sha256"] == docs_commit["canonical_sha256"], "文档须先提交固定")
    env = {k: v for k, v in os.environ.items() if not k.upper().startswith(tuple(config["removed_environment_prefixes"]))}
    env.update(config["overrides"])
    runtime_before = runtime_snapshot(config)
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    with tempfile.TemporaryDirectory(prefix="b4-4-stage-", dir=config["overrides"]["TEMP"]) as stage:
        raw_log = Path(stage) / "output.log"
        with raw_log.open("xb") as output:
            process = subprocess.Popen(argv, cwd=REPOSITORY, env=env, stdout=output, stderr=subprocess.STDOUT)
            code = process.wait()
        raw = raw_log.read_bytes()
    log_ref, digest = BASE.store_content_log(ROOT, raw)
    after, docs_after = worktree_files(REPOSITORY), docs_snapshot(REPOSITORY)
    finished = datetime.datetime.now(datetime.timezone.utc).isoformat()
    receipt = {"variant": args.variant, "label": args.label, "argv": argv, "cwd": str(REPOSITORY),
               "pid": process.pid, "started_utc": started, "finished_utc": finished, "exit_code": code,
               "source_before": identity(before), "source_after": identity(after), "source_unchanged": before == after,
               "docs_commit": commit, "docs_commit_tree": docs_commit, "docs_before": docs_before, "docs_after": docs_after,
               "docs_unchanged": docs_before == docs_after,
               "runtime_before": runtime_before, "runtime_after": runtime_snapshot(config),
               "environment_sha256": sha((variant / "environment.json").read_bytes()),
               "runner_sha256": sha(Path(__file__).read_bytes()), "base_runner_sha256": sha(Path(BASE.__file__).read_bytes()),
               "log_ref": log_ref, "log_bytes": len(raw), "log_sha256": digest}
    write_json(target, receipt)
    print(json.dumps({"variant": args.variant, "label": args.label, "exit_code": code,
                      "source_unchanged": before == after, "docs_unchanged": docs_before == docs_after}))
    require(before == after and docs_before == docs_after, "运行期间产品或文档变化")
    raise SystemExit(code)


if __name__ == "__main__":
    main()
