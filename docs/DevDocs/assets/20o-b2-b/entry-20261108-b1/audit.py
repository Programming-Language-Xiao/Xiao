"""B2-B B1只读审计；只读取固定证据和固定 Git 对象，不改写任何文件。"""

import hashlib
import json
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]
BASELINE_COMMIT = "d08b8460"
CODE_COMMIT = "35872782"
ALLOWED_CODE = {
    "core/rust/Cargo.toml",
    "core/rust/Cargo.lock",
    "core/rust/crates/xiao-runtime/Cargo.toml",
    "core/rust/crates/xiao-runtime/src/value/arithmetic/mod.rs",
    "core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs",
    "core/rust/crates/xiao-runtime/src/value/arithmetic/explicit_division.rs",
    "tests/benchmarks/Cargo.lock",
}
CHECKS = {
    "b1-runtime-tests": (255, 0),
    "b1-runtime-clippy": None,
    "b1-runtime-fmt": None,
    "b1-dependency-tree": None,
    "b1-benchmark-check": None,
    "b1-docs-all": None,
}
PASS_NAMES = [
    "evidence-manifest",
    "required-final-files",
    "evidence-bytes",
    "closeout-contract",
    "guards-28",
    "negative-manifest-rejection",
    "records-six",
    "same-source-manifest",
    "runtime-count",
    "dependency-closure",
    "core-lock-git-object",
    "benchmark-lock-git-object",
    "baseline-git-object",
    "code-diff-allowlist",
    "protected-paths",
    "source-manifest-git-objects",
    "version-boundaries",
    "no-public-surface",
]


def fail(message):
    raise SystemExit(message)


def read(name):
    return json.loads((ROOT / name).read_text(encoding="utf-8"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args):
    return subprocess.check_output(["cmd", "/c", "git", *args], cwd=SOURCE, text=True).splitlines()


def git_blob(commit, path):
    return subprocess.check_output(["cmd", "/c", "git", "show", f"{commit}:{path}"], cwd=SOURCE)


def normalized(raw):
    try:
        raw.decode("utf-8")
    except UnicodeDecodeError:
        return raw
    return raw if b"\0" in raw else raw.replace(b"\r\n", b"\n")


def inventory_rows():
    return [
        {
            "path": path.relative_to(ROOT).as_posix(),
            "bytes": path.stat().st_size,
            "sha256": sha(path),
        }
        for path in sorted(ROOT.rglob("*"))
        if path.is_file() and path.name != "evidence.json" and "__pycache__" not in path.parts
    ]


def source_tree_blobs(commit):
    raw = subprocess.check_output(["cmd", "/c", "git", "ls-tree", "-r", "-z", commit], cwd=SOURCE)
    result = {}
    for record in raw.split(b"\0"):
        if not record:
            continue
        header, path = record.split(b"\t", 1)
        parts = header.split()
        if len(parts) == 3 and parts[1] == b"blob":
            result[path.decode("utf-8")] = parts[2].decode("ascii")
    return result


def blob_bytes(blob_ids):
    payload = "".join(blob_id + "\n" for blob_id in blob_ids).encode("ascii")
    result = subprocess.run(
        ["cmd", "/c", "git", "cat-file", "--batch"],
        cwd=SOURCE,
        input=payload,
        stdout=subprocess.PIPE,
        check=True,
    ).stdout
    offset = 0
    values = {}
    for blob_id in blob_ids:
        end = result.index(b"\n", offset)
        header = result[offset:end].split()
        if len(header) != 3 or header[0].decode("ascii") != blob_id or header[1] != b"blob":
            fail("Git blob 流格式不符")
        length = int(header[2])
        start = end + 1
        values[blob_id] = result[start : start + length]
        offset = start + length + 1
    if offset != len(result):
        fail("Git blob 流长度不符")
    return values


def main():
    evidence = read("evidence.json")
    rows = inventory_rows()
    by_path = {row["path"]: row for row in rows}
    if evidence["file_count"] != len(rows) or evidence["files"] != rows:
        fail("evidence.json 与实际证据文件不一致")
    if len(PASS_NAMES) != 18:
        fail("审计检查数量不是18")

    required = {".gitattributes", "b1-guards-report.json", "closeout.json", "negative-cases.json"}
    if not required.issubset(by_path):
        fail("最终证据缺少定稿文件")
    for row in evidence["files"]:
        path = ROOT / row["path"]
        if path.stat().st_size != row["bytes"] or sha(path) != row["sha256"]:
            fail("证据字节或摘要不一致：" + row["path"])

    closeout = read("closeout.json")
    if (
        not closeout.get("passed")
        or closeout.get("baseline_commit") != BASELINE_COMMIT
        or closeout.get("code_commit") != CODE_COMMIT
        or "current_head" in closeout
        or closeout.get("checks") != sorted(CHECKS)
        or closeout.get("passed_checks") != 18
    ):
        fail("closeout.json 契约不符")

    guards = read("b1-guards-report.json")
    if (
        guards.get("passed") != 28
        or guards.get("total") != 28
        or len(guards.get("cases", [])) != 28
        or not all(case.get("Passed") for case in guards["cases"])
    ):
        fail("b1-guards-report.json 不是28/28")

    negative = read("negative-cases.json")
    fixture = read(negative["fixture"])
    if not negative.get("expected_rejected") or fixture == evidence:
        fail("清单反例没有保持为可拒绝的实际变体")
    actual_by_path = {item["path"]: item for item in evidence["files"]}
    mutation_path = negative["mutation"]["path"]
    if fixture.get("files", [{}])[0].get("path") != mutation_path:
        fail("清单反例突变位置不符")
    if actual_by_path[mutation_path]["sha256"] == fixture["files"][0]["sha256"]:
        fail("清单反例摘要没有改变")

    source_ids = set()
    logs = {}
    for name, expected in CHECKS.items():
        record = read(name + ".json")
        log_path = ROOT / record["log"]
        raw = log_path.read_bytes()
        if record["exit_code"] != 0 or sha(log_path) != record["sha256"]:
            fail("命令失败或日志摘要不符：" + name)
        if not record["source_unchanged"] or record["source_before"] != record["source_after"]:
            fail("命令执行期间源码身份变化：" + name)
        if record["runner_sha256"] != sha(ROOT / "run-step.py"):
            fail("执行器摘要不符：" + name)
        source_ids.add(record["source_before"]["manifest"])
        logs[name] = raw.decode("utf-8", errors="replace")
        if expected is not None:
            found = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", logs[name], re.M)
            if found != [(str(expected[0]), str(expected[1]))]:
                fail("Runtime测试计数不符")
    if len(source_ids) != 1:
        fail("B1验收不是同一份源码输入")

    tree = logs["b1-dependency-tree"]
    for package, version in {
        "bigdecimal": "0.4.11",
        "libm": "0.2.16",
        "num-bigint": "0.4.8",
        "num-integer": "0.1.47",
        "num-traits": "0.2.19",
        "autocfg": "1.5.1",
    }.items():
        if f"{package} v{version}" not in tree:
            fail(f"依赖版本缺失：{package} {version}")
    if 'libm feature "unstable"' in tree or "rand" in tree:
        fail("libm特性或可选依赖越界")

    baseline = git("rev-parse", "--verify", BASELINE_COMMIT)[0]
    code = git("rev-parse", "--verify", CODE_COMMIT)[0]
    if not baseline or not code:
        fail("固定 Git 对象不存在")
    code_paths = set(
        git("diff", "--name-only", BASELINE_COMMIT, CODE_COMMIT, "--", "core/rust", "tests/benchmarks/Cargo.lock")
    )
    if code_paths != ALLOWED_CODE:
        fail(f"固定 Git 对象的代码范围漂移：{sorted(code_paths ^ ALLOWED_CODE)}")
    protected = [
        "core/rust/crates/xiao-intrinsics/src",
        "core/rust/crates/xiao-runtime-abi",
        "core/rust/crates/xiao-codegen-llvm",
        "core/rust/crates/xiao-vm",
        "core/rust/crates/xiao-driver",
        "stdlib",
        "docs/UseDocs",
        "docs/module-registry.json",
    ]
    if any(git("diff", "--name-only", BASELINE_COMMIT, CODE_COMMIT, "--", path) for path in protected):
        fail("固定 Git 对象的受保护路径发生变化")

    baseline_lock = git_blob(BASELINE_COMMIT, "core/rust/Cargo.lock").decode("utf-8")
    code_lock = git_blob(CODE_COMMIT, "core/rust/Cargo.lock").decode("utf-8")
    benchmark_lock = git_blob(CODE_COMMIT, "tests/benchmarks/Cargo.lock").decode("utf-8")
    if 'name = "xiao-runtime"' not in baseline_lock or 'name = "xiao-runtime"' not in code_lock:
        fail("固定 Git 对象缺少 Runtime 锁记录")
    if '"libm"' not in code_lock or '"libm"' not in benchmark_lock:
        fail("固定 Git 对象的锁文件未记录libm")

    manifest = read(next(iter(source_ids)))
    blobs = source_tree_blobs(CODE_COMMIT)
    ids = []
    for item in manifest["files"]:
        if item["path"] not in blobs:
            fail("源码清单路径不在固定 Git 树中：" + item["path"])
        ids.append(blobs[item["path"]])
    blob_values = blob_bytes(ids)
    for item, blob_id in zip(manifest["files"], ids):
        if hashlib.sha256(normalized(blob_values[blob_id])).hexdigest() != item["utf8_lf_or_binary_sha256"]:
            fail("源码清单与固定 Git blob 不符：" + item["path"])

    if closeout.get("abi") != "1.18" or closeout.get("codegen") != 20 or closeout.get("active_intrinsic_max") != 32:
        fail("版本边界漂移")
    if closeout.get("cmath") != "planned" or closeout.get("public_entrypoints") is not False:
        fail("公开状态越界")

    print(
        json.dumps(
            {
                "passed": 18,
                "total": 18,
                "files": len(rows),
                "baseline_commit": BASELINE_COMMIT,
                "code_commit": CODE_COMMIT,
                "source_manifest": next(iter(source_ids)),
            },
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()
