"""B2-B B1受控命令执行器：绑定源码身份、环境、日志和退出码。"""

import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]


def snapshot():
    roots = [
        "core/rust",
        "stdlib",
        "tests/fixtures",
        "tests/benchmarks",
        ".github/workflows",
        ".cargo",
        "tools",
        "cli/ts",
        "docs/UseDocs",
        "docs/module-registry.json",
        "package.json",
        "bun.lock",
        ".gitattributes",
    ]
    paths = set()
    for options in ([], ["--others", "--exclude-standard"]):
        raw = subprocess.check_output(
            ["cmd", "/c", "git", "ls-files", "-z", *options, "--", *roots],
            cwd=SOURCE,
        )
        paths.update(name.decode("utf-8") for name in raw.split(b"\0") if name)
    rows = []
    for relative in sorted(paths):
        raw = (SOURCE / relative).read_bytes()
        try:
            raw.decode("utf-8")
            normalized = raw if b"\0" in raw else raw.replace(b"\r\n", b"\n")
        except UnicodeDecodeError:
            normalized = raw
        rows.append(
            {
                "path": relative,
                "bytes": len(raw),
                "raw_sha256": hashlib.sha256(raw).hexdigest(),
                "utf8_lf_or_binary_sha256": hashlib.sha256(normalized).hexdigest(),
            }
        )
    raw_tree = hashlib.sha256(
        json.dumps(rows, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    canonical_tree = hashlib.sha256(
        json.dumps(
            [[row["path"], row["utf8_lf_or_binary_sha256"]] for row in rows],
            ensure_ascii=False,
            separators=(",", ":"),
        ).encode()
    ).hexdigest()
    manifest_dir = ROOT / "source-manifests"
    manifest_dir.mkdir(exist_ok=True)
    manifest = manifest_dir / f"{raw_tree}.json"
    if not manifest.exists():
        manifest.write_text(
            json.dumps(
                {
                    "scope": roots,
                    "source_root": str(SOURCE),
                    "raw_tree_sha256": raw_tree,
                    "utf8_lf_tree_sha256": canonical_tree,
                    "files": rows,
                },
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
            newline="\n",
        )
    return {
        "manifest": manifest.relative_to(ROOT).as_posix(),
        "raw_tree_sha256": raw_tree,
        "utf8_lf_tree_sha256": canonical_tree,
        "files": len(rows),
    }


def main():
    if len(sys.argv) < 3:
        raise SystemExit("用法：run-step.py 名称 命令 参数...")
    name, argv = sys.argv[1], sys.argv[2:]
    log = ROOT / f"{name}.log"
    record_path = ROOT / f"{name}.json"
    if log.exists() or record_path.exists():
        raise SystemExit(f"证据文件已存在：{name}")
    setup = json.loads((ROOT / "environment.json").read_text(encoding="utf-8"))
    removed = [key for key in os.environ if key.startswith("RUST_BIGDECIMAL_")]
    env = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith("RUST_BIGDECIMAL_") and not key.startswith("XIAO_")
    }
    env.update(setup["msvc_whitelist"])
    env.update(setup["overrides"])
    before = snapshot()
    record = {
        "name": name,
        "argv": argv,
        "cwd": str(SOURCE),
        "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "environment": "environment.json",
        "removed_environment_names": removed,
        "log": log.name,
    }
    started = time.monotonic()
    with log.open("wb") as stream:
        process = subprocess.Popen(argv, cwd=SOURCE, env=env, stdout=stream, stderr=subprocess.STDOUT)
        record["pid"] = process.pid
        record["exit_code"] = process.wait()
    after = snapshot()
    record.update(
        {
            "source_before": before,
            "source_after": after,
            "source_unchanged": before == after,
            "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "finished_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "duration_seconds": round(time.monotonic() - started, 3),
            "sha256": hashlib.sha256(log.read_bytes()).hexdigest(),
        }
    )
    record_path.write_text(
        json.dumps(record, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps(record, ensure_ascii=False), flush=True)
    if not record["source_unchanged"]:
        raise SystemExit("执行期间产品源码身份变化")
    raise SystemExit(record["exit_code"])


if __name__ == "__main__":
    main()
