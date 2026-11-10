"""B3-4 独立执行器：新 target/Home/temp、固定工具链、源码/锁与原始日志身份。"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
PRODUCT_ROOTS = [
    "core/rust", "stdlib", "tests/fixtures", "tests/benchmarks", ".github/workflows",
    ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json",
    "package.json", "bun.lock", ".gitattributes",
]


def git(*args):
    return subprocess.check_output(["cmd", "/c", "git", *args], cwd=REPOSITORY)


def canonical(data):
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        return data
    return data if b"\0" in data else data.replace(b"\r\n", b"\n")


def snapshot():
    names = set()
    for flags in [[], ["--others", "--exclude-standard"]]:
        raw = git("ls-files", "-z", *flags, "--", *PRODUCT_ROOTS)
        names.update(value.decode("utf-8") for value in raw.split(b"\0") if value)
    files = []
    for name in sorted(names):
        path = REPOSITORY / name
        data = path.read_bytes() if path.is_file() else None
        files.append({"path": name, "bytes": None if data is None else len(data),
                      "raw_sha256": None if data is None else hashlib.sha256(data).hexdigest(),
                      "utf8_lf_or_binary_sha256": None if data is None else hashlib.sha256(canonical(data)).hexdigest()})
    raw_tree = hashlib.sha256(json.dumps(files, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    lf_tree = hashlib.sha256(json.dumps([[row["path"], row["utf8_lf_or_binary_sha256"]] for row in files],
                                        ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
    locks = {}
    for name in json.loads((ROOT / "environment.json").read_text(encoding="utf-8"))["lock_files"]:
        data = (REPOSITORY / name).read_bytes()
        locks[name] = {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    return {"raw_tree_sha256": raw_tree, "utf8_lf_tree_sha256": lf_tree,
            "files": len(files), "product_files": files, "locks": locks}


def main():
    if len(sys.argv) < 4:
        raise SystemExit("用法：run-step.py 阶段 标签 命令 参数...")
    phase, label, *argv = sys.argv[1:]
    if phase not in {"toolchain", "historical", "b3-4", "workspace", "gate"} or Path(label).name != label:
        raise SystemExit("执行阶段或标签无效")
    log, record_path = ROOT / (label + ".log"), ROOT / (label + ".json")
    if log.exists() or record_path.exists():
        raise SystemExit("拒绝覆盖已有执行证据：" + label)
    config = json.loads((ROOT / "environment.json").read_text(encoding="utf-8"))
    env = {key: value for key, value in os.environ.items()
           if not key.startswith(tuple(config["removed_environment_prefixes"]))}
    env.update(config["overrides"])
    removed_names = sorted(key for key in os.environ if key.startswith(tuple(config["removed_environment_prefixes"])))
    before = snapshot()
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    with log.open("xb") as stream:
        process = subprocess.Popen(argv, cwd=REPOSITORY, env=env, stdout=stream, stderr=subprocess.STDOUT)
        exit_code = process.wait()
    after = snapshot()
    data = log.read_bytes()
    record = {"phase": phase, "label": label, "argv": argv, "cwd": str(REPOSITORY),
              "task_root": json.loads((ROOT / "bootstrap.json").read_text(encoding="utf-8"))["task_root"],
              "pid": process.pid, "started_utc": started,
              "finished_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "exit_code": exit_code, "removed_environment_names": removed_names,
              "source_before": before, "source_after": after, "source_unchanged": before == after,
              "runtime_library": None, "log": log.name, "log_bytes": len(data),
              "log_sha256": hashlib.sha256(data).hexdigest(),
              "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "environment_sha256": hashlib.sha256((ROOT / "environment.json").read_bytes()).hexdigest()}
    with record_path.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(record, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"record": record_path.name, "exit_code": exit_code,
                      "source_unchanged": record["source_unchanged"]}, ensure_ascii=False))
    raise SystemExit(exit_code if record["source_unchanged"] else "执行期间产品源码发生变化")


if __name__ == "__main__":
    main()
