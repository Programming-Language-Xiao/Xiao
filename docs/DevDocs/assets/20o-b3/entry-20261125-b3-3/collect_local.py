"""采集固定产品的本机 Runtime、C ABI 与 VM 实测回执。"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
MANIFEST = "core/rust/Cargo.toml"
SYMBOLS = {
    "xiao_runtime_fs_write_text",
    "xiao_runtime_fs_list_dir",
    "xiao_runtime_fs_remove",
    "xiao_runtime_fs_rename",
    "xiao_runtime_fs_metadata",
}


def git(*args):
    return subprocess.check_output(["cmd", "/d", "/c", "git", *args], cwd=REPO)


def tracked_source_matches(commit):
    paths = git("status", "--porcelain", "--untracked-files=all").decode().splitlines()
    scope = HERE.relative_to(REPO).as_posix() + "/"
    return (
        git("rev-parse", "HEAD").decode().strip() == commit
        and all(row[3:].startswith(scope) for row in paths)
    )


def collect(args):
    task_root = args.task_root.rstrip("/\\")
    env = os.environ.copy()
    if args.wsl:
        if not (args.wsl_source and task_root.startswith("/")):
            raise SystemExit("WSL 采集需要源码目录和 Linux 绝对任务路径")
        target, name, nm = "x86_64-unknown-linux-gnu", "libxiao_runtime.a", "nm"
        prefix = [
            "wsl", "-d", args.wsl, "--cd", args.wsl_source, "--exec",
            "sh", "-lc", 'exec "$@"', "xiao-b3-3", "env",
            f"TMPDIR={task_root}/temp", f"XIAO_HOME={task_root}/xiao-home",
            f"CARGO_TARGET_DIR={task_root}/target",
        ]
        subprocess.run(
            prefix + ["mkdir", "-p", "--", f"{task_root}/temp", f"{task_root}/xiao-home"],
            cwd=REPO, check=True,
        )
    else:
        target, name, nm = "x86_64-pc-windows-msvc", "xiao_runtime.lib", "llvm-nm"
        prefix = []
        task = Path(task_root)
        for folder in ("temp", "xiao-home", "target"):
            (task / folder).mkdir(parents=True, exist_ok=True)
        env.update(
            TEMP=str(task / "temp"), TMP=str(task / "temp"),
            XIAO_HOME=str(task / "xiao-home"), CARGO_TARGET_DIR=str(task / "target"),
        )
    cargo = ["cargo", "+1.96.0"]
    archive_path = f"{task_root}/target/{target}/release/{name}"
    commands = [
        ("rustc", ["rustc", "+1.96.0", "-Vv"]),
        ("nm-version", [nm, "--version"]),
        ("runtime", cargo + ["test", "--locked", "--manifest-path", MANIFEST,
                            "-p", "xiao-runtime", "-p", "xiao-intrinsics", "--lib", "--", "--quiet"]),
        ("fs", cargo + ["test", "--locked", "--manifest-path", MANIFEST,
                       "-p", "xiao-runtime", "--lib", "fs_", "--", "--nocapture"]),
        ("vm", cargo + ["test", "--locked", "--manifest-path", MANIFEST, "-p", "xiao-driver",
                       "--test", "20o_b3_fs_tutorial", "--test", "20o_b3_time_tutorial",
                       "--test", "20o_b3_args_cli", "--", "--nocapture"]),
        ("release", cargo + ["build", "--locked", "--manifest-path", MANIFEST,
                            "-p", "xiao-runtime", "--release", "--target", target]),
        ("exports", [nm, "--defined-only", "--extern-only", "-j", archive_path]),
    ]
    receipts, objects = {}, {}
    for label, command in commands:
        command = prefix + command
        result = subprocess.run(command, cwd=REPO, env=env, capture_output=True)
        row = {"command": command, "exit": result.returncode}
        for stream in ("stdout", "stderr"):
            data = getattr(result, stream)
            digest = hashlib.sha256(data).hexdigest()
            objects[digest] = data
            row[stream] = digest
        receipts[label] = row
        print(f"{target} {label}: {result.returncode}", flush=True)
        if result.returncode != 0:
            print(result.stderr[-8000:].decode("utf-8", errors="replace"), flush=True)
            break
    runtime = None
    matched = []
    if "exports" in receipts and receipts["exports"]["exit"] == 0:
        export_output = objects[receipts["exports"]["stdout"]].decode("utf-8", errors="replace")
        matched = sorted({line.strip() for line in export_output.splitlines()} & SYMBOLS)
        if args.wsl:
            digest = subprocess.check_output(prefix + ["sha256sum", archive_path], cwd=REPO).decode()
            digest = digest.split()[0]
        else:
            with Path(archive_path).open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
        runtime = {"path": archive_path, "sha256": digest, "exports": matched}
    assert tracked_source_matches(args.product_commit), "采集期间产品源码发生变化"
    record = {
        "product_commit": args.product_commit, "target": target, "task_root": task_root,
        "runtime": runtime, "source_root": args.wsl_source if args.wsl else str(REPO),
        "scope": "local-vm-and-runtime", "commands": receipts,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.output, "x", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("receipt.json", json.dumps(record, indent=2, sort_keys=True) + "\n")
        for digest, content in sorted(objects.items()):
            archive.writestr("objects/" + digest, content)
    return set(matched) == SYMBOLS


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--product-commit", required=True)
    parser.add_argument("--task-root", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--wsl")
    parser.add_argument("--wsl-source")
    args = parser.parse_args()
    if not re.fullmatch("[0-9a-f]{40}", args.product_commit):
        raise SystemExit("需要完整产品提交号")
    if not tracked_source_matches(args.product_commit):
        raise SystemExit("产品提交或工作树与采集输入不符")
    if not collect(args):
        raise SystemExit("本机采集有缺项，已保留原始回执")


if __name__ == "__main__":
    main()
