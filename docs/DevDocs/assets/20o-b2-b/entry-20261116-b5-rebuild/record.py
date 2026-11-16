"""B5固定来源采集：独立任务根与单摘要对象存储，不覆盖回执。"""
import argparse
from datetime import datetime, timezone
import hashlib
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
_spec = importlib.util.spec_from_file_location("b5_base_record", ROOT.parent / "entry-20261113-b4-3/record.py")
_base = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_base)
PRODUCT_ROOTS = _base.PRODUCT_ROOTS
canonical, sha, git, identity = _base.canonical, _base.sha, _base.git, _base.identity
fixed_files, worktree_files = _base.fixed_files, _base.worktree_files
require, load, write_json = _base.require, _base.load, _base.write_json


def store(raw, media_type="application/octet-stream"):
    digest = sha(raw)
    relative = f"objects/sha256/{digest[:2]}/{digest}.bin"
    path = ROOT / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        require(path.read_bytes() == raw, "B5内容对象损坏")
    else:
        try:
            with path.open("xb") as output:
                output.write(raw)
        except FileExistsError:
            require(path.read_bytes() == raw, "B5并发内容对象字节不符")
    return {"path": relative, "bytes": len(raw), "sha256": digest, "media_type": media_type}


def docs_snapshot():
    paths = sorted(set(p.decode("utf-8") for p in git(REPOSITORY, "ls-files", "-z", "--cached", "--others",
                   "--exclude-standard", "--", "docs/DevDocs").split(b"\0") if p))
    paths = [path for path in paths if "/assets/" not in path or path.startswith(PREFIX + "/")
             and Path(path).suffix in {".py", ".md", ".rs", ".c", ".xiao"}]
    raw = {path: sha((REPOSITORY / path).read_bytes()) for path in paths}
    normalized = {path: sha(canonical((REPOSITORY / path).read_bytes())) for path in paths}
    return {"raw_sha256": identity(raw), "canonical_sha256": identity(normalized),
            "files": store((json.dumps(normalized, ensure_ascii=False, sort_keys=True) + "\n").encode("utf-8"), "application/json")}


def environment(task, source):
    llvm = Path(r"D:\msys64\ucrt64\bin")
    return {"task_root": str(task), "source_root": str(source), "removed_prefixes": ["XIAO_", "RUST_BIGDECIMAL_"],
            "overrides": {"CARGO_TARGET_DIR": str(task / "target"), "XIAO_HOME": str(task / "home"),
              "TEMP": str(task / "temp"), "TMP": str(task / "temp"), "RUSTUP_TOOLCHAIN": "1.96.0",
              "XIAO_RUNTIME_LIBRARY": str(task / "target/release/xiao_runtime.lib"),
              "XIAO_STDLIB_ROOT": str(source / "stdlib"), "XIAO_TARGET_TRIPLE": "x86_64-pc-windows-msvc",
              "XIAO_B1_F10": "0", "XIAO_B2_A_PERF": "0", "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8",
              "CARGO_TERM_COLOR": "never", **{key: str(llvm / name) for key, name in [
                ("XIAO_CLANG", "clang.exe"), ("XIAO_LLVM_AS", "llvm-as.exe"), ("XIAO_LLC", "llc.exe")]}}}


def controlled(config):
    result = {key: value for key, value in os.environ.items() if not key.upper().startswith(tuple(config["removed_prefixes"]))}
    result.update(config["overrides"])
    return result


def runtime(config):
    path = Path(config["overrides"]["XIAO_RUNTIME_LIBRARY"])
    return {"path": str(path), "exists": path.is_file(),
            **({"bytes": path.stat().st_size, "sha256": sha(path.read_bytes())} if path.is_file() else {})}


def run(variant, label, argv):
    directory = ROOT / variant
    destination = directory / (label + ".json")
    require(not destination.exists(), "B5禁止覆盖运行回执")
    source = load(directory / "source.json")
    config = load(directory / "environment.json")
    source_root = Path(config["source_root"])
    before = worktree_files(source_root)
    require(before == source["files"], "B5运行前产品变化")
    doc_before = docs_snapshot()
    tool_commit = git(REPOSITORY, "rev-parse", "HEAD").decode().strip()
    for path in ROOT.iterdir():
        if path.suffix in {".py", ".rs", ".c", ".md", ".xiao"}:
            require(canonical(path.read_bytes()) == canonical(git(REPOSITORY, "show", tool_commit + ":" + PREFIX + "/" + path.name)),
                    "B5静态工具须先提交：" + path.name)
    runtime_before = runtime(config)
    started = datetime.now(timezone.utc).isoformat()
    with tempfile.TemporaryDirectory(prefix="b5-stage-", dir=config["overrides"]["TEMP"]) as temporary:
        with (Path(temporary) / "output.log").open("xb") as output:
            process = subprocess.Popen(argv, cwd=source_root, env=controlled(config), stdout=output, stderr=subprocess.STDOUT)
            status = process.wait()
        log = store((Path(temporary) / "output.log").read_bytes(), "text/plain; charset=utf-8")
    after = worktree_files(source_root)
    doc_after = docs_snapshot()
    receipt = {"variant": variant, "label": label, "argv": argv, "cwd": str(source_root), "pid": process.pid,
               "started_utc": started, "finished_utc": datetime.now(timezone.utc).isoformat(), "exit_code": status,
               "source_before": identity(before), "source_after": identity(after), "source_unchanged": before == after,
               "docs_before": doc_before, "docs_after": doc_after, "docs_unchanged": doc_before == doc_after,
               "tools_commit": tool_commit, "runner_sha256": sha(Path(__file__).read_bytes()),
               "environment_sha256": sha((directory / "environment.json").read_bytes()),
               "runtime_before": runtime_before, "runtime_after": runtime(config), "log": log}
    write_json(destination, receipt)
    print(json.dumps({"variant": variant, "label": label, "exit_code": status,
                      "source_unchanged": before == after, "docs_unchanged": doc_before == doc_after}), flush=True)
    require(before == after and doc_before == doc_after, "B5运行期间源码或文档变化")
    return status


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    init = sub.add_parser("init")
    init.add_argument("variant")
    init.add_argument("revision")
    init.add_argument("--task-root", type=Path, required=True)
    init.add_argument("--worktree", action="store_true")
    init.add_argument("--inherit-test-task", help="仅测试断言修订时复用同批冷构建Runtime；登记父来源")
    execute = sub.add_parser("run")
    execute.add_argument("variant")
    execute.add_argument("label")
    execute.add_argument("argv", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    require(Path(args.variant).name == args.variant and args.variant not in {".", ".."}, "B5非法变体")
    if args.action == "init":
        if not args.inherit_test_task:
            _base.require_new_task_root(args.task_root)
        directory = ROOT / args.variant
        require(not directory.exists(), "B5变体不可复用")
        revision = git(REPOSITORY, "rev-parse", args.revision).decode().strip()
        task = args.task_root
        inherited = None
        if args.inherit_test_task:
            parent = load(ROOT / args.inherit_test_task / "source.json")
            config = load(ROOT / args.inherit_test_task / "environment.json")
            changed = git(REPOSITORY,"diff","--name-only",parent["revision"],revision,"--",*PRODUCT_ROOTS).decode().splitlines()
            require(changed and all("/tests/" in path for path in changed), "B5只有测试修订可继承任务Runtime")
            require(task == Path(config["task_root"]) and not args.worktree, "B5测试继承任务不符")
            inherited = {"variant":args.inherit_test_task,"source_commit":parent["revision"],"test_only_paths":changed,
                         "runtime":runtime(config),"reason":"same production source; only version assertion changed"}
        else:
            for name in ["target", "home", "temp", "artifacts"]:
                (task / name).mkdir(parents=True)
        source_root = task / "source" if args.worktree else REPOSITORY
        if args.worktree:
            git(REPOSITORY, "worktree", "add", "--detach", str(source_root), revision)
        files = worktree_files(source_root)
        require({path: row["canonical_sha256"] for path, row in files.items()} == fixed_files(REPOSITORY, revision),
                "B5来源与固定提交不符")
        directory.mkdir()
        write_json(directory / "source.json", {"revision": revision, "source_root": str(source_root),
                   "files": files, "sha256": identity(files),
                   "baseline_kind": "reconstructed_from_B1_source" if args.variant == "b1" else "internal_source_build",
                   "historical_originals_available": False})
        config = environment(task, source_root)
        if inherited:
            config["inherited_test_task"] = inherited
        write_json(directory / "environment.json", config)
        print(json.dumps({"initialized": args.variant, "revision": revision, "files": len(files)}))
    else:
        require(Path(args.label).name == args.label and args.label not in {".", ".."}, "B5非法标签")
        argv = args.argv[1:] if args.argv[:1] == ["--"] else args.argv
        require(argv, "B5命令不能为空")
        raise SystemExit(run(args.variant, args.label, argv))


if __name__ == "__main__":
    main()
