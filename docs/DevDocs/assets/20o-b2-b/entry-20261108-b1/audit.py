"""B2-B B1私有内核只读审计。"""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]
B0_COMMIT = "e5039bbac7c53cba8ed666e93095796e5fc46fca"
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


def fail(message):
    raise SystemExit(message)


def read(name):
    return json.loads((ROOT / name).read_text(encoding="utf-8"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args):
    return subprocess.check_output(["cmd", "/c", "git", *args], cwd=SOURCE, text=True).splitlines()


def inventory(write=False):
    rows = []
    for path in sorted(ROOT.rglob("*")):
        if path.is_file() and path.name != "evidence.json" and "__pycache__" not in path.parts:
            rows.append({"path": path.relative_to(ROOT).as_posix(), "bytes": path.stat().st_size, "sha256": sha(path)})
    if write:
        (ROOT / "evidence.json").write_text(
            json.dumps(
                {
                    "scope": "仅B1本目录文件；清单自身不计入自身摘要",
                    "file_count": len(rows),
                    "files": rows,
                },
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
            newline="\n",
        )
    else:
        manifest = read("evidence.json")
        if manifest["file_count"] != len(rows) or manifest["files"] != rows:
            fail("B1证据清单或摘要不符")
    return len(rows)


def main():
    bootstrap = read("bootstrap.json")
    current_head = git("rev-parse", "HEAD")[0]
    if bootstrap["code_commit"] != CODE_COMMIT:
        fail("代码提交身份不符")
    if not git("rev-parse", "--verify", CODE_COMMIT):
        fail("代码提交对象不存在")
    product_roots = [
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
    code_paths = set(git("diff", "--name-only", B0_COMMIT, CODE_COMMIT, "--", "core/rust", "tests/benchmarks/Cargo.lock"))
    if code_paths != ALLOWED_CODE:
        fail(f"B1代码范围漂移：{sorted(code_paths ^ ALLOWED_CODE)}")
    between_code_and_head = git("diff", "--name-only", CODE_COMMIT, current_head, "--", *product_roots)
    if between_code_and_head:
        fail(f"代码提交之后产品树仍有变更：{between_code_and_head}")
    for protected in [
        "core/rust/crates/xiao-intrinsics/src",
        "core/rust/crates/xiao-runtime-abi",
        "core/rust/crates/xiao-codegen-llvm",
        "core/rust/crates/xiao-vm",
        "core/rust/crates/xiao-driver",
        "stdlib",
        "docs/UseDocs",
        "docs/module-registry.json",
    ]:
        if git("diff", "--name-only", B0_COMMIT, CODE_COMMIT, "--", protected):
            fail(f"受保护路径发生变化：{protected}")

    source_ids = set()
    logs = {}
    for name, expected in CHECKS.items():
        record = read(name + ".json")
        raw = (ROOT / record["log"]).read_bytes()
        if record["exit_code"] != 0 or sha(ROOT / record["log"]) != record["sha256"]:
            fail("命令失败或日志摘要不符：" + name)
        if not record["source_unchanged"] or record["source_before"] != record["source_after"]:
            fail("命令执行期间源码身份变化：" + name)
        if record["runner_sha256"] != sha(ROOT / "run-step.py"):
            fail("执行器摘要不符：" + name)
        source_ids.add(record["source_before"]["manifest"])
        text = raw.decode("utf-8", errors="replace")
        logs[name] = text
        if expected is not None:
            found = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text, re.M)
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
    lock = (SOURCE / "core/rust/Cargo.lock").read_text(encoding="utf-8")
    runtime_block = re.search(r'name = "xiao-runtime"(?P<body>.*?)(?=\n\n\[\[package\]\]|\Z)', lock, re.S)
    if runtime_block is None or '"libm"' not in runtime_block.group("body"):
        fail("Cargo.lock未记录Runtime直接libm依赖")
    closeout = {
        "passed": True,
        "scope": "B2-B B1：仅libm依赖与私有cmath/显式除法内核",
        "current_head": current_head,
        "code_commit": CODE_COMMIT,
        "b0_commit": B0_COMMIT,
        "source_manifest": next(iter(source_ids)),
        "checks": sorted(CHECKS),
        "runtime_tests": "255 passed / 0 failed / 0 ignored",
        "dependency_versions": {
            "bigdecimal": "0.4.11",
            "libm": "0.2.16",
            "num-bigint": "0.4.8",
            "num-integer": "0.1.47",
            "num-traits": "0.2.19",
            "autocfg": "1.5.1",
        },
        "abi": "1.18",
        "codegen": 20,
        "active_intrinsic_max": 32,
        "cmath": "planned",
        "public_entrypoints": False,
        "public_abi_changed": False,
        "registry_changed": False,
        "stdlib_changed": False,
        "b1_f10": False,
        "b2_a_perf": False,
        "b1_721_executed": False,
        "b2_c_started": False,
        "allowed_code_paths": sorted(ALLOWED_CODE),
    }
    (ROOT / "closeout.json").write_text(
        json.dumps(closeout, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    count = inventory(True)
    print(json.dumps({"passed": True, "files": count, "source_manifest": next(iter(source_ids))}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", action="store_true")
    args = parser.parse_args()
    inventory(True) if args.manifest else main()
