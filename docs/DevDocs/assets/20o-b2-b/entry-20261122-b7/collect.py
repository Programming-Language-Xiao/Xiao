"""独立采集 B7 定稿门禁、内容寻址归档与封存清单。"""

import argparse
import hashlib
from importlib.util import module_from_spec, spec_from_file_location
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
spec = spec_from_file_location("b7_audit", ROOT / "audit.py")
audit = module_from_spec(spec)
spec.loader.exec_module(audit)
run = audit.run
DOC_ROOTS = ("docs/DevDocs", "docs/UseDocs")
SUFFIXES = {".json", ".log", ".stdin", ".stdout", ".stderr", ".sh", ".ps1", ".zip", ".xiaoc", ".xar"}
CONTROLLED_CARGO = """import json
import os
from pathlib import Path
import runpy
import subprocess
import sys

recipe = Path.cwd() / "tools/gates/controlled-cargo.py"
args, env, _, _, _ = runpy.run_path(str(recipe))["prepare"](sys.argv[1:], os.environ)
argv = [env["XIAO_B7_CARGO"], *args]
with subprocess.Popen(argv, env=env) as child:
    row = {"argv": argv, "pid": child.pid, "exit": child.wait(), "cwd": os.getcwd(),
           "parent_pid": os.getppid(), "step": env["XIAO_B7_DOC_STEP"]}
with open(env["XIAO_B7_CARGO_LOG"], "a", encoding="utf-8") as output:
    output.write(json.dumps(row) + "\\n")
raise SystemExit(row["exit"])
"""


def record(argv, cwd, env, output, label):
    stdout, stderr = (output / f"{label}.{stream}" for stream in ("stdout", "stderr"))
    start = run.utc()
    with stdout.open("xb") as out, stderr.open("xb") as err, subprocess.Popen(
        argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=out, stderr=err,
    ) as process:
        code = process.wait()
    return {"argv": list(map(str, argv)), "pid": process.pid, "cwd": str(cwd), "exit": code,
            "started_utc": start, "finished_utc": run.utc(),
            "outputs": {path.suffix[1:]: run.identity(path) for path in (stdout, stderr)}}


def documents(repo, commit):
    entries = run.tracked(run.git("ls-tree", "-rz", commit, "--", *DOC_ROOTS, repo=repo))
    expected = {name: oid for name, oid in entries.items() if name.endswith(".md")}
    paths = {path.relative_to(repo).as_posix() for folder in DOC_ROOTS
             for path in (repo / folder).rglob("*.md") if path.is_file()}
    run.require(paths == expected.keys(), "定稿工作区 Markdown 文件集合不符")
    observed = {}
    for name in expected:
        raw = (repo / name).read_bytes()
        observed[name] = hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()
    run.require(expected == observed, "定稿工作区 Markdown 字节不符")
    return observed


def docs(repo, output, target):
    run.require(not output.exists() and target.is_dir(), "文档回执须使用新目录和本批 Linux target")
    output.mkdir()
    commit = run.git("rev-parse", "HEAD", repo=repo).decode().strip()
    run.require(not run.git("status", "--porcelain=v1", repo=repo), "定稿门禁需要干净的固定检出")
    before = documents(repo, commit)
    before_locks = {name: run.identity(repo / name) for name in run.LOCKS}
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(("XIAO_", "RUST_BIGDECIMAL_", "CARGO_PROFILE_"))}
    rust = Path(subprocess.check_output(["rustup", "which", "--toolchain", "1.96.0", "cargo"],
                                       env=env).decode().strip()).parent
    bun = Path(shutil.which("bun")).resolve(strict=True)
    env.update(CARGO_TARGET_DIR=str(target), RUSTUP_TOOLCHAIN="1.96.0", RUSTC=str(rust / "rustc"),
               RUSTDOC=str(rust / "rustdoc"), RUST_TEST_THREADS="4", CARGO_TERM_COLOR="never",
               PATH=os.pathsep.join((str(rust), str(bun.parent), env["PATH"])),
               BUN_INSTALL_CACHE_DIR=str(output / "bun-cache"))
    directory = output / "bin"
    directory.mkdir()
    wrapper = directory / "cargo"
    wrapper.write_text(f"#!{sys.executable}\n{CONTROLLED_CARGO}", encoding="utf-8")
    wrapper.chmod(0o755)
    env.update(XIAO_B7_CARGO=str(rust / "cargo"), XIAO_B7_CARGO_LOG=str(output / "cargo.log"),
               PATH=str(directory) + os.pathsep + env["PATH"])
    for label, argv in (
        ("install", [bun, "install", "--frozen-lockfile"]),
        ("check-lock", [bun, "run", "check:lock"]),
    ):
        result = record(argv, repo, env | {"XIAO_B7_DOC_STEP": label}, output, label)
        run.write_json(output / f"{label}.json", result)
        run.require(result["exit"] == 0, f"定稿门禁前置失败：{label}")
    gate = record([bun, "tools/repo-check/src/cli.ts", "all", "--format", "json"], repo,
                  env | {"XIAO_B7_DOC_STEP": "repo-check"}, output, "repo-check")
    after = documents(repo, commit)
    after_locks = {name: run.identity(repo / name) for name in run.LOCKS}
    run.write_json(output / "receipt.json", gate | {"commit": commit, "markdown_before": before,
                   "markdown_after": after, "locks_before": before_locks, "locks_after": after_locks,
                   "bun": run.identity(bun), "cargo": {"path": str(rust / "cargo"), **run.identity(rust / "cargo")},
                   "cargo_calls": run.identity(output / "cargo.log"),
                   "cargo_wrapper": run.identity(wrapper), "target": str(target)})
    run.require(gate["exit"] == 0 and before == after and before_locks == after_locks
                and not run.git("status", "--porcelain=v1", repo=repo), "定稿完整仓库门禁失败")


def archive(out, sources):
    files = {}
    objects = set()
    with ZipFile(out, "x", compression=ZIP_DEFLATED, compresslevel=6) as bundle:
        for group, source in sources:
            run.require(source.is_dir(), f"采集来源目录缺失：{source}")
            for path in sorted(source.rglob("*")):
                if not path.is_file() or path.suffix not in SUFFIXES:
                    continue
                relative = path.relative_to(source)
                if any(part.startswith(("bun-cache", "xiao-controlled-cargo-")) for part in relative.parts):
                    continue
                key = f"{group}/{relative.as_posix()}"
                run.require(key not in files, f"重复证据逻辑路径：{key}")
                raw = path.read_bytes()
                identity = audit.digest(raw)
                files[key] = identity
                name = "objects/" + identity["sha256"]
                if name not in objects:
                    bundle.writestr(name, raw)
                    objects.add(name)
        bundle.writestr("evidence.json", audit.json_bytes(files))


def manifest(root):
    return {path.relative_to(root).as_posix(): audit.digest(path.read_bytes())
            for path in root.rglob("*") if path.is_file() and path.name != "evidence.json"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    docs_args = commands.add_parser("docs")
    for name in ("repo", "output", "target"):
        docs_args.add_argument(name, type=Path)
    archive_args = commands.add_parser("archive")
    archive_args.add_argument("output", type=Path)
    archive_args.add_argument("source", nargs="+", metavar="LABEL=PATH")
    seal_args = commands.add_parser("seal")
    seal_args.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    if args.command == "docs":
        docs(args.repo.resolve(strict=True), args.output.resolve(), args.target.resolve(strict=True))
    elif args.command == "archive":
        folders = []
        for spec in args.source:
            name, value = spec.split("=", 1)
            run.require(re.fullmatch(r"[a-z][a-z0-9-]*", name), f"证据分组无效：{name}")
            folders.append((name, Path(value).resolve(strict=True)))
        archive(args.output, folders)
    else:
        state = audit.inspect(args.root, prepare=True)
        run.write_json(args.root / "closeout.json", state)
        run.write_json(args.root / "evidence.json", manifest(args.root))


if __name__ == "__main__":
    main()
