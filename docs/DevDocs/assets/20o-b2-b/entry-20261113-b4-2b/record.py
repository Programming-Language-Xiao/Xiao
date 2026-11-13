"""B4-2B 受控执行：每个来源独立环境，原始日志与来源回执只追加、不覆盖。"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
from pathlib import PurePosixPath
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
PRODUCT_ROOTS = [
    "core/rust", "stdlib", "tests/fixtures", "tests/benchmarks", ".github/workflows",
    ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json",
    "package.json", "bun.lock", ".gitattributes",
]
DEV_DOCS_ROOT = "docs/DevDocs"
SELF_EVIDENCE_ROOT = "docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-2b"
SELF_EVIDENCE_VARIANTS = {"baseline", "fixed"}
STABLE_VARIANT_JSON = {"environment.json", "source.json", "targets.json", "toolchain.json"}
GENERATED_EVIDENCE_JSON = {
    "closeout.json", "evidence.json", "self-negative-findings.json",
}


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


def require_new_task_root(task_root):
    """每个来源变体必须用此前不存在的任务根。"""
    task = Path(task_root)
    require(task.is_absolute() and not task.exists() and not task.is_symlink(), "拒绝复用已有任务根")
    current = task
    while current != current.parent:
        require(not current.is_symlink() and not getattr(current, "is_junction", lambda: False)(),
                "任务根路径不能经过符号链接或 junction")
        current = current.parent


def fixed_files(repository, revision):
    entries = []
    raw = git(repository, "ls-tree", "-r", "-z", revision, "--", *PRODUCT_ROOTS)
    for entry in raw.split(b"\0"):
        if entry:
            metadata, path = entry.split(b"\t", 1)
            mode, kind, oid = metadata.split()
            require(kind == b"blob" and mode in {b"100644", b"100755"},
                    "受保护路径必须是普通文件 Blob")
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


def is_runtime_devdocs_artifact(path):
    """排除运行期间才生成的文件，保留 README、工具和反例源码。"""
    prefix = SELF_EVIDENCE_ROOT + "/"
    if not path.startswith(prefix):
        return False
    relative = path[len(prefix):]
    if (relative.startswith("logs/sha256/") or relative in GENERATED_EVIDENCE_JSON
            or relative.startswith("self-negative-") and relative.endswith(".json")):
        return True
    parts = PurePosixPath(relative).parts
    return (len(parts) == 2 and parts[0] in SELF_EVIDENCE_VARIANTS
            and parts[1].endswith(".json") and parts[1] not in STABLE_VARIANT_JSON)


def docs_tree_snapshot(repository):
    """分别摘要 DevDocs 工作树原始字节和 UTF-8/LF 规范字节。"""
    root = repository / DEV_DOCS_ROOT
    raw_files, canonical_files = {}, {}
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(repository).as_posix()
        if path.is_symlink():
            raise ValueError("DevDocs 树含符号链接：" + relative)
        if not path.is_file():
            continue
        if is_runtime_devdocs_artifact(relative):
            continue
        data = path.read_bytes()
        normalized = canonical(data)
        raw_files[relative] = {"bytes": len(data), "sha256": sha(data)}
        canonical_files[relative] = {"bytes": len(normalized), "sha256": sha(normalized)}
    return {"raw_sha256": identity(raw_files), "canonical_sha256": identity(canonical_files)}


def docs_tree_from_commit(repository, revision):
    """从固定 Git tree 计算与工作树快照相同的 DevDocs 摘要。"""
    raw = git(repository, "ls-tree", "-r", "-z", revision, "--", DEV_DOCS_ROOT)
    entries = []
    for entry in raw.split(b"\0"):
        if not entry:
            continue
        metadata, raw_path = entry.split(b"\t", 1)
        mode, kind, oid = metadata.split()
        path = raw_path.decode("utf-8")
        if is_runtime_devdocs_artifact(path):
            continue
        require(kind == b"blob" and mode in {b"100644", b"100755"}, "DevDocs Git tree 含非普通文件")
        entries.append((path, oid))
    stream = git(repository, "cat-file", "--batch", data=b"".join(oid + b"\n" for _, oid in entries))
    raw_files, canonical_files, offset = {}, {}, 0
    for path, oid in entries:
        end = stream.index(b"\n", offset)
        header = stream[offset:end].split()
        require(header[:2] == [oid, b"blob"], "DevDocs Git Blob 身份不符：" + path)
        length = int(header[2])
        data = stream[end + 1:end + 1 + length]
        normalized = canonical(data)
        raw_files[path] = {"bytes": length, "sha256": sha(data)}
        canonical_files[path] = {"bytes": len(normalized), "sha256": sha(normalized)}
        offset = end + length + 2
    require(offset == len(stream), "DevDocs Git Blob 流长度不符")
    return {"raw_sha256": identity(raw_files), "canonical_sha256": identity(canonical_files)}


def docs_tree_self_test():
    """摘要须绑定 README/脚本，且忽略运行期间生成的日志和回执。"""
    with tempfile.TemporaryDirectory(prefix="xiao-b4-2b-docs-tree-") as temporary:
        repository = Path(temporary)
        evidence = repository / SELF_EVIDENCE_ROOT
        (repository / DEV_DOCS_ROOT).mkdir(parents=True)
        evidence.mkdir(parents=True)
        (repository / "docs/DevDocs/plan.md").write_bytes(b"plan v1\n")
        (evidence / "README.md").write_bytes(b"readme v1\n")
        (evidence / "audit.py").write_bytes(b"audit v1\n")
        log = evidence / "logs/sha256/00/" / ("0" * 64 + ".log")
        log.parent.mkdir(parents=True)
        log.write_bytes(b"log v1\n")
        receipt = evidence / "fixed/docs_final.json"
        receipt.parent.mkdir()
        receipt.write_bytes(b"receipt v1\n")
        baseline = docs_tree_snapshot(repository)
        log.write_bytes(b"log v2\n")
        receipt.write_bytes(b"receipt v2\n")
        require(docs_tree_snapshot(repository) == baseline,
                "运行时 DevDocs 产物不应改变静态树摘要")
        (evidence / "README.md").write_bytes(b"readme v2\n")
        require(docs_tree_snapshot(repository) != baseline,
                "DevDocs 摘要未绑定本批 README")
    return True


def store_content_log(root, data):
    """按原始字节 SHA-256 独占写入日志对象；同内容返回同一引用。"""
    digest = sha(data)
    relative = Path("logs") / "sha256" / digest[:2] / (digest + ".log")
    destination = root / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        require(destination.read_bytes() == data, "内容寻址日志摘要碰撞或对象损坏")
    else:
        try:
            with destination.open("xb") as output:
                output.write(data)
        except FileExistsError:
            require(destination.read_bytes() == data, "并发写入的日志对象字节不符")
    return relative.as_posix(), digest


def content_store_self_test():
    """验证内容寻址日志存储和 DevDocs 静态摘要边界。"""
    with tempfile.TemporaryDirectory(prefix="xiao-b4-2b-content-store-") as temporary:
        root = Path(temporary)
        first = store_content_log(root, b"same output\n")
        again = store_content_log(root, b"same output\n")
        other = store_content_log(root, b"different output\n")
        objects = sorted(path for path in root.rglob("*.log") if path.is_file())
        require(first == again and first[0] != other[0] and len(objects) == 2,
                "内容寻址日志未按 SHA-256 去重")
        object_path = root / first[0]
        object_path.write_bytes(b"tampered\n")
        try:
            store_content_log(root, b"same output\n")
        except ValueError as error:
            require(str(error) == "内容寻址日志摘要碰撞或对象损坏", "损坏日志对象拒绝原因不符")
        else:
            raise ValueError("内容寻址存储接受了被修改的对象")
    return {"passed": True, "distinct_objects": 2, "identical_outputs_reused": True,
            "tampered_object_rejected": True, "docs_tree_static_bound": docs_tree_self_test()}


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
    sub.add_parser("content-store-self-test", help="对内容寻址日志对象存储执行临时自测")
    args = parser.parse_args()
    if args.action == "content-store-self-test":
        print(json.dumps(content_store_self_test(), sort_keys=True))
        return
    require(Path(args.variant).name == args.variant and args.variant not in {".", ".."}, "变体名非法")
    destination = ROOT / args.variant
    if args.action == "init":
        require_new_task_root(args.task_root)
        source, task = args.source_root.resolve(), args.task_root.resolve()
        require(not destination.exists(), "证据变体已存在")
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
            "runtime_library_override_consumed": False,
            "cargo_builds_runtime_from_source": True,
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
    result = destination / (args.label + ".json")
    require(not result.exists(), "拒绝覆盖运行记录")
    config, source = load(destination / "environment.json"), load(destination / "source.json")
    repository = Path(source["source_root"])
    before = worktree_files(repository)
    require(before == source["files"], "运行前源码/锁变化")
    docs_before = docs_tree_snapshot(repository)
    docs_commit = git(repository, "rev-parse", "HEAD").decode("ascii").strip()
    docs_commit_tree = docs_tree_from_commit(repository, docs_commit)
    require(docs_before["canonical_sha256"] == docs_commit_tree["canonical_sha256"],
            "DevDocs 运行前规范字节不是已提交快照")
    env = {key: value for key, value in os.environ.items() if not key.startswith(tuple(config["removed_environment_prefixes"]))}
    env.update(config["overrides"])
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    with tempfile.TemporaryDirectory(prefix="xiao-log-stage-", dir=ROOT) as staging:
        staged_log = Path(staging) / "output.log"
        with staged_log.open("xb") as output:
            process = subprocess.Popen(argv, cwd=repository, env=env, stdout=output, stderr=subprocess.STDOUT)
            exit_code = process.wait()
        log_bytes = staged_log.read_bytes()
    log_ref, log_digest = store_content_log(ROOT, log_bytes)
    after = worktree_files(repository)
    docs_after = docs_tree_snapshot(repository)
    docs_commit_after = git(repository, "rev-parse", "HEAD").decode("ascii").strip()
    write_json(result, {"label": args.label, "argv": argv, "cwd": str(repository), "pid": process.pid,
               "started_utc": started, "finished_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
               "exit_code": exit_code, "source_before": identity(before), "source_after": identity(after),
               "source_unchanged": before == after,
               "docs_commit": docs_commit, "docs_commit_after": docs_commit_after,
               "docs_tree_before": docs_before["raw_sha256"], "docs_tree_after": docs_after["raw_sha256"],
               "docs_tree_sha256": docs_before["raw_sha256"],
               "docs_tree_canonical_before": docs_before["canonical_sha256"],
               "docs_tree_canonical_after": docs_after["canonical_sha256"],
               "docs_tree_commit_raw_sha256": docs_commit_tree["raw_sha256"],
               "docs_tree_commit_canonical_sha256": docs_commit_tree["canonical_sha256"],
               "docs_tree_unchanged": docs_before == docs_after,
               "log_ref": log_ref, "log_bytes": len(log_bytes),
               "log_sha256": log_digest, "runner_sha256": sha(Path(__file__).read_bytes()),
               "environment_sha256": sha((destination / "environment.json").read_bytes())})
    print(json.dumps({"variant": args.variant, "label": args.label, "exit_code": exit_code, "source_unchanged": before == after}))
    require(before == after, "运行期间源码/锁变化")
    require(docs_commit_after == docs_commit, "运行期间 Git HEAD 变化")
    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
