"""只增不覆盖地保存本轮命令、退出码、日志及相对固定基线的源码字节身份。"""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
BASELINE = "0e33e6bf"
PRODUCT_ROOTS = [
    "core/rust", "stdlib", "tests/fixtures", "tests/benchmarks", ".github/workflows",
    ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json",
    "package.json", "bun.lock", ".gitattributes",
]


def git(*args):
    return subprocess.check_output(["cmd", "/c", "git", *args], cwd=REPOSITORY)


def source_identity():
    paths = set()
    for args in [
        ["diff", "--name-only", "-z", BASELINE, "--", *PRODUCT_ROOTS],
        ["ls-files", "--others", "--exclude-standard", "-z", "--", *PRODUCT_ROOTS],
    ]:
        paths.update(value.decode("utf-8") for value in git(*args).split(b"\0") if value)
    files = []
    for name in sorted(paths):
        path = REPOSITORY / name
        data = path.read_bytes() if path.is_file() else None
        files.append({"path": name, "bytes": None if data is None else len(data),
                      "sha256": None if data is None else hashlib.sha256(data).hexdigest(),
                      "utf8_lf_sha256": None if data is None else hashlib.sha256(data.replace(b"\r\n", b"\n")).hexdigest()})
    return {"baseline_commit": git("rev-parse", BASELINE).decode("ascii").strip(),
            "baseline_tree": git("show", "--no-patch", "--format=%T", BASELINE).decode("ascii").strip(),
            "changed_product_files": files}


def main():
    if sys.argv[1:] == ["--manifest"]:
        collect_evidence()
        return
    label, *argv = sys.argv[1:]
    if not label or Path(label).name != label or not argv:
        raise SystemExit("用法：record-check.py 标签 命令 参数...")
    log = ROOT / (label + ".log")
    record = ROOT / (label + ".json")
    if log.exists() or record.exists():
        raise SystemExit("拒绝覆盖已有执行记录")
    before = source_identity()
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    with log.open("xb") as stream:
        process = subprocess.Popen(argv, cwd=REPOSITORY, stdout=stream, stderr=subprocess.STDOUT)
        exit_code = process.wait()
    after = source_identity()
    data = log.read_bytes()
    result = {"argv": argv, "cwd": str(REPOSITORY), "pid": process.pid,
              "started_utc": started, "finished_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "exit_code": exit_code, "source_before": before, "source_after": after,
              "source_unchanged": before == after, "log": log.name, "log_bytes": len(data),
              "log_sha256": hashlib.sha256(data).hexdigest(),
              "recorder_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    with record.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"record": record.name, "exit_code": exit_code,
                      "source_unchanged": result["source_unchanged"]}, ensure_ascii=False))
    raise SystemExit(exit_code if before == after else "执行期间源码变化")


def collect_evidence():
    identity = source_identity()
    runs = ["codegen-accepted", "driver-accepted", "clippy-accepted", "fmt-accepted", "final-docs"]
    for label in runs:
        record = json.loads((ROOT / (label + ".json")).read_text(encoding="utf-8"))
        if record["exit_code"] != 0 or record["source_before"] != identity or record["source_after"] != identity:
            raise SystemExit(f"最终执行失败或源码身份不一致：{label}")
    files = []
    for path in sorted(path for path in ROOT.rglob("*") if path.is_file()):
        if path.name == "evidence.json" or "__pycache__" in path.parts:
            continue
        data = path.read_bytes()
        files.append({"path": path.relative_to(ROOT).as_posix(), "bytes": len(data),
                      "sha256": hashlib.sha256(data).hexdigest()})
    result = {"scope": "仅B3可达性与planned拒绝回归；不是B3-4独立收口",
              "source": identity, "source_scope": PRODUCT_ROOTS, "final_runs": runs,
              "b3_4_closed": False, "cmath_status": "planned", "files": files}
    with (ROOT / "evidence.json").open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"files": len(files), "final_runs": runs, "b3_4_closed": False}))


if __name__ == "__main__":
    main()
