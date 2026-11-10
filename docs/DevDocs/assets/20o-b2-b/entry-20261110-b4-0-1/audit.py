"""只读审计固定 B4-0/B4-1 产品提交、向量迁移与类型/IR证据。"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import runpy
import subprocess

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
BASELINE = "7d8b85e4d8c588b40ec76d641edc865481295032"
CODE = "c7a45e10b76b26915499717563761ed20fda6e6c"
B3_CABI = "89d782e13f00776b90193f1719b4454c54367db7"
REFERENCE = "docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/reference-vectors.tsv"
FIXTURE = "tests/fixtures/docs/cmath/reference-vectors.tsv"
REFERENCE_SHA256 = "0e61a2da361448477b15f5508df6d612a3f01e29a06e50699d6ed48a917d1ed3"
PRODUCT_ROOTS = [
    "core/rust", "stdlib", "tests/fixtures", "tests/benchmarks", ".github/workflows",
    ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json",
    "package.json", "bun.lock", ".gitattributes",
]
ALLOWED_PATHS = {
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_frontend.rs",
    "core/rust/crates/xiao-runtime/src/abi_cmath_tests.rs",
    "core/rust/crates/xiao-runtime/src/value/arithmetic/cmath_tests.rs",
    "core/rust/crates/xiao-types/src/checker/expression.rs",
    "tests/fixtures/docs/cmath/.gitattributes",
    "tests/fixtures/docs/cmath/README.md",
    FIXTURE,
}
RUNS = {
    "runtime_cmath": ("b4-0", ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-runtime", "cmath", "--", "--nocapture"]),
    "runtime_all": ("b4-0", ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-runtime"]),
    "types_ir": ("b4-1", ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-types", "-p", "xiao-ir"]),
    "b4_frontend": ("b4-1", ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-driver", "--test", "20o_b2_b4_frontend", "--", "--nocapture"]),
    "b3_wrapper": ("b4-1", ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-driver", "--test", "20o_b2_b3_wrapper", "--", "--nocapture"]),
    "b1_history": ("history", ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/entry-20261108-b1/audit.py"]),
    "b2_history": ("history", ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/entry-20261108-b2/audit.py"]),
    "b3_cabi_history": ("history", ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/entry-20261109-b3-cabi/audit.py"]),
    "b3_wrapper_history": ("history", ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/entry-20261110-b3-wrapper-final/audit.py"]),
    "clippy": ("gate", ["cargo", "+1.96.0", "clippy", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-types", "-p", "xiao-ir", "-p", "xiao-runtime", "-p", "xiao-driver", "--all-targets", "--", "-D", "warnings"]),
    "fmt": ("gate", ["cargo", "+1.96.0", "fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"]),
    "repo_all": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_all_closeout": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_all_final": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_after_audit_fixes": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_final_status": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
}
SUMMARY = re.compile(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", re.M)
CMATH = [
    ("sqrt", ["value"]), ("floor", ["value"]), ("ceil", ["value"]), ("round", ["value"]),
    ("trunc", ["value"]), ("abs", ["value"]), ("min", ["left", "right"]),
    ("max", ["left", "right"]), ("pow", ["base", "exponent"]), ("sin", ["value"]),
    ("cos", ["value"]), ("tan", ["value"]), ("exp", ["value"]), ("ln", ["value"]),
    ("log10", ["value"]),
]


def fail(message):
    raise SystemExit(message)


def require(condition, message):
    if not condition:
        fail(message)


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


def inventory(root):
    rows = []
    for path in sorted(item for item in root.rglob("*") if item.is_file()):
        if path.name == "evidence.json" or path.name.startswith("self-negative-result") or "__pycache__" in path.parts:
            continue
        data = path.read_bytes()
        rows.append({"path": path.relative_to(root).as_posix(), "bytes": len(data), "sha256": sha(data)})
    return rows


def current_product_snapshot(repository):
    names = set()
    for flags in [[], ["--others", "--exclude-standard"]]:
        raw = git(repository, "ls-files", "-z", *flags, "--", *PRODUCT_ROOTS)
        names.update(value.decode("utf-8") for value in raw.split(b"\0") if value)
    files = []
    for name in sorted(names):
        path = repository / name
        data = path.read_bytes() if path.is_file() else None
        files.append({"path": name, "bytes": None if data is None else len(data),
                      "raw_sha256": None if data is None else sha(data),
                      "utf8_lf_or_binary_sha256": None if data is None else sha(canonical(data))})
    raw_tree = sha(json.dumps(files, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode())
    lf_tree = sha(json.dumps([[row["path"], row["utf8_lf_or_binary_sha256"]] for row in files],
                             ensure_ascii=False, separators=(",", ":")).encode())
    locks = {}
    for name in ["core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock"]:
        data = (repository / name).read_bytes()
        locks[name] = {"bytes": len(data), "sha256": sha(data)}
    return {"raw_tree_sha256": raw_tree, "utf8_lf_tree_sha256": lf_tree,
            "files": len(files), "product_files": files, "locks": locks}


def read_run(root, repository, key):
    phase, argv = RUNS[key]
    record = json.loads((root / (key + ".json")).read_text(encoding="utf-8"))
    data = (root / record["log"]).read_bytes()
    log = data.decode("utf-8", errors="strict")
    require(record["phase"] == phase and record["label"] == key and record["argv"] == argv,
            "B4执行点命令身份不符：" + key)
    require(record["cwd"] == str(repository) and record["pid"] > 0 and record["task_root"]
            == json.loads((root / "bootstrap.json").read_text(encoding="utf-8"))["task_root"],
            "B4执行点cwd/PID/task身份不符：" + key)
    require(record["exit_code"] == 0 and record["source_unchanged"]
            and record["source_before"] == record["source_after"], "B4执行失败或期间源码变化：" + key)
    require(record["log"] == key + ".log" and record["log_bytes"] == len(data)
            and record["log_sha256"] == sha(data), "B4执行日志摘要不符：" + key)
    require(record["runner_sha256"] == sha((root / "record-step.py").read_bytes())
            and record["environment_sha256"] == sha((root / "environment.json").read_bytes()),
            "B4执行器/环境摘要不符：" + key)
    return record, log


def validate_source_manifest(snapshotter, manifest, current, product_commit, product_files):
    require(manifest["product_commit"] == product_commit, "B4源清单产品提交SHA不符")
    require(snapshotter["summary"](manifest) == current, "B4源清单与当前产品树/锁不符")
    require(manifest["product_files"] == product_files,
            "B4源清单文件集合/字节摘要与产品快照不符")


def check_static_contract(repository):
    entry = git(repository, "rev-parse", BASELINE).decode("ascii").strip()
    code = git(repository, "rev-parse", CODE).decode("ascii").strip()
    head = git(repository, "rev-parse", "HEAD").decode("ascii").strip()
    for ancestor, descendant, reason in [
        (entry, code, "B4产品代码不在B3-4基线历史之后"),
        (code, head, "固定B4产品代码不是当前HEAD祖先"),
    ]:
        result = subprocess.run(["cmd", "/c", "git", "merge-base", "--is-ancestor", ancestor, descendant], cwd=repository)
        require(result.returncode == 0, reason)
    changed = set(git(repository, "diff", "--name-only", entry, code, "--", *PRODUCT_ROOTS).decode("utf-8").splitlines())
    require(changed == ALLOWED_PATHS, "B4-0/B4-1相对入口产品路径集合偏离登记")
    post = subprocess.run(["cmd", "/c", "git", "diff", "--quiet", code, head, "--", *PRODUCT_ROOTS], cwd=repository)
    require(post.returncode == 0 and not git(repository, "ls-files", "--others", "--exclude-standard", "--", *PRODUCT_ROOTS).strip(),
            "固定B4代码提交后的受保护产品树发生变化")
    tracked = {name.decode("utf-8") for name in git(repository, "ls-tree", "-r", "--name-only", "-z", code, "--", *PRODUCT_ROOTS).split(b"\0") if name}
    require(ALLOWED_PATHS <= tracked, "B4产品提交缺少登记路径")
    requests = "".join(code + ":" + name + "\n" for name in sorted(ALLOWED_PATHS)).encode()
    blobs = subprocess.check_output(["cmd", "/c", "git", "cat-file", "--batch"], cwd=repository, input=requests)
    offset = 0
    for name in sorted(ALLOWED_PATHS):
        end = blobs.index(b"\n", offset)
        header = blobs[offset:end].split()
        require(len(header) == 3 and header[1] == b"blob", "B4代码对象缺Blob：" + name)
        length = int(header[2])
        content = blobs[end + 1:end + 1 + length]
        offset = end + length + 2
        require((repository / name).is_file()
                and canonical(content) == canonical((repository / name).read_bytes()),
                "B4工作树偏离固定产品Blob：" + name)
    require(offset == len(blobs), "B4产品Blob流解析长度不符")

    fixture = (repository / FIXTURE).read_bytes()
    original = (repository / REFERENCE).read_bytes()
    historical = git(repository, "show", B3_CABI + ":" + REFERENCE)
    require(fixture == original == historical and sha(fixture) == REFERENCE_SHA256
            and len(fixture) == 1755726, "B4向量迁移字节、历史Git对象、大小或摘要不符")
    for name in ["core/rust/crates/xiao-runtime/src/value/arithmetic/cmath_tests.rs",
                 "core/rust/crates/xiao-runtime/src/abi_cmath_tests.rs"]:
        text = (repository / name).read_text(encoding="utf-8").replace("\r\n", "\n")
        require(FIXTURE in text and "entry-20261108-b3" not in text,
                "B4 Runtime测试仍引用DevDocs向量：" + name)

    intrinsic_table = json.loads((repository / "core/rust/crates/xiao-intrinsics/intrinsics.json").read_text(encoding="utf-8"))["intrinsics"]
    require(sorted(row["id"] for row in intrinsic_table if row["status"] == "active") == list(range(1, 49))
            and next(row for row in intrinsic_table if row["id"] == 100)["status"] == "tombstone",
            "B4-0/B4-1修改intrinsic活动范围或ID100")
    abi_text = (repository / "core/rust/crates/xiao-runtime-abi/src/lib.rs").read_text(encoding="utf-8")
    codegen_text = (repository / "core/rust/crates/xiao-codegen-llvm/src/lib.rs").read_text(encoding="utf-8")
    require(re.search(r"ABI_MINOR_VERSION[^=]*=\s*18", abi_text)
            and re.search(r"pub const CODEGEN_VERSION:\s*u32\s*=\s*20;", codegen_text),
            "B4-0/B4-1改动ABI或生成器版本")
    registry = json.loads((repository / "docs/module-registry.json").read_text(encoding="utf-8"))
    cmath = next(row for row in registry["modules"] if row["id"] == "stdlib.cmath")
    require(cmath["status"] == "planned" and cmath["code"] == [] and cmath["tests"] == [],
            "B4-0/B4-1把cmath提前提升为产品")
    return {"entry_commit": entry, "code_commit": code, "head_commit": head,
            "source": current_product_snapshot(repository),
            "fixture": {"path": FIXTURE, "source": REFERENCE, "bytes": len(fixture), "sha256": sha(fixture)},
            "runtime_abi": "1.18", "codegen_version": 20, "stdlib_interface": 1,
            "active_intrinsics": [1, 48], "cmath_status": "planned"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--prepare", action="store_true")
    args = parser.parse_args()
    root, repository = args.root.resolve(), args.repository_root.resolve()
    attributes = (root / ".gitattributes").read_text(encoding="utf-8")
    require(attributes == "* -text\n*.log -diff\n", "B4证据目录属性不符")
    source = check_static_contract(repository)
    snapshotter = runpy.run_path(str(root / "record-step.py"))
    snapshotter["REPOSITORY"] = repository
    current = snapshotter["summary"](snapshotter["manifest_value"]())
    manifest_path = root / "source-manifest.json"
    source_manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    validate_source_manifest(snapshotter, source_manifest, current, source["code_commit"],
                             source["source"]["product_files"])
    invalid_manifest = json.loads((root / "source-manifest-invalid-product-sha.json")
                                  .read_text(encoding="utf-8"))
    try:
        validate_source_manifest(snapshotter, invalid_manifest, current, source["code_commit"],
                                 source["source"]["product_files"])
    except SystemExit as error:
        require(str(error) == "B4源清单产品提交SHA不符",
                "B4产品SHA反例未按声明原因拒绝")
    else:
        fail("B4产品SHA反例意外通过")
    records, logs = {}, {}
    for key in RUNS:
        records[key], logs[key] = read_run(root, repository, key)
        require(records[key]["source_before"] == current, "B4有效执行未绑定相同源码与锁：" + key)
    b4_closed = False
    require("test result: ok. 15 passed; 0 failed; 0 ignored;" in logs["runtime_cmath"]
            and "all_abi_results_match_formal_reference_errors_signs_and_ulp ... ok" in logs["runtime_cmath"]
            and "mpfr_pow_vectors_and_public_float_share_kernel ... ok" in logs["runtime_cmath"]
            and len(re.findall(r"B3-CMATH-ABI function=.* cases=\d+ result=success", logs["runtime_cmath"])) == 15,
            "迁移后的Runtime内核/C ABI参考回归不完整")
    require("test result: ok. 269 passed; 0 failed; 0 ignored;" in logs["runtime_all"]
            and "all_abi_results_match_formal_reference_errors_signs_and_ulp ... ok" in logs["runtime_all"],
            "Runtime全量测试/正式参考未通过")
    frontend = logs["b4_frontend"]
    require("test result: ok. 3 passed; 0 failed; 0 ignored;" in frontend
            and "B4-1-CMATH count=15 positional=named=exact ids=34..48 return=float result=success" in frontend
            and "B4-1-CMATH-WRAPPER module=stdlib.cmath ids=34..48 import=retained result=success" in frontend
            and "B4-1-CMATH-NEGATIVE cases=10 error=X04-TYPE-002 span=present result=success" in frontend,
            "B4-1类型/IR/负例输出缺失")
    wrapper = logs["b3_wrapper"]
    require("test result: ok. 8 passed; 0 failed; 0 ignored;" in wrapper
            and wrapper.count("executable=false result=planned-rejected") == 30
            and "B3-CMATH-REACHABILITY root=stdlib.hex unreachable=stdlib.path dependency=stdlib.cmath result=success" in wrapper,
            "B3 wrapper/reachability planned边界变化")
    require("test result: ok. 269 passed; 0 failed; 0 ignored;" in logs["runtime_all"],
            "B4 Runtime全量计数不符")
    require('"passed": 18' in logs["b1_history"] and '"total": 18' in logs["b1_history"],
            "B1固定历史审计未过18/18")
    require('"checks": 14' in logs["b2_history"] and '"exact_checks": 4' in logs["b2_history"],
            "B2固定历史审计未过14项/4精确入口")
    require('"checks": 22' in logs["b3_cabi_history"] and '"code_commit": "89d782e1"' in logs["b3_cabi_history"],
            "B3-CABI固定历史审计未过22项")
    require('"b3_4_closed": true' in logs["b3_wrapper_history"]
            and '"code_commit": "7d8b85e4d8c588b40ec76d641edc865481295032"' in logs["b3_wrapper_history"]
            and '"identity_source": "fixed Git commit blobs; canonical UTF-8/LF hashes"' in logs["b3_wrapper_history"],
            "B3-4固定Git对象历史审计未通过")
    require("Finished" in logs["clippy"] and "error:" not in logs["clippy"], "B4影响crates Clippy失败")
    require(records["fmt"]["exit_code"] == 0 and records["repo_all"]["exit_code"] == 0
            and "[error]" not in logs["repo_all"], "B4-0/B4-1 fmt或docs/usedocs/layout门禁失败")
    if not args.prepare:
        closeout = json.loads((root / "closeout.json").read_text(encoding="utf-8"))
        evidence = json.loads((root / "evidence.json").read_text(encoding="utf-8"))
        require(closeout["passed"] is True and closeout["b4_0_closed"] is True
                and closeout["b4_1_closed"] is True and closeout["b4_closed"] is False
                and closeout["language_available"] is False and closeout["cmath_status"] == "planned"
                and closeout["vm_llvm_cmath_binding"] is False and closeout["old_721_executed"] is False,
                "B4 closeout状态越界")
        require(closeout["entry_commit"] == source["entry_commit"]
                and closeout["product_commit"] == source["code_commit"]
                and closeout["runtime_abi"] == "1.18" and closeout["codegen_version"] == 20,
                "B4 closeout源码或版本身份不符")
        expected_runs = {
            key: {"pid": records[key]["pid"], "log": records[key]["log"],
                  "log_sha256": records[key]["log_sha256"]}
            for key in RUNS
        }
        require(closeout["runs"] == expected_runs, "B4 closeout运行记录映射不符")
        require(evidence["files"] == inventory(root), "B4证据字节清单不符")
        require(evidence["scope"] == "B4-0/B4-1 internal vector/type/IR closeout; cmath language unavailable",
                "B4 evidence scope不符")
        require(evidence["baseline_commit"] == source["entry_commit"]
                and evidence["product_commit"] == source["code_commit"]
                and evidence["product_files"] == source["source"]["files"]
                and evidence["raw_tree_sha256"] == source["source"]["raw_tree_sha256"]
                and evidence["utf8_lf_tree_sha256"] == source["source"]["utf8_lf_tree_sha256"]
                and evidence["runtime_abi"] == "1.18" and evidence["codegen_version"] == 20
                and evidence["runs"] == list(RUNS),
                "B4 evidence源码、版本或运行范围不符")
        b4_closed = closeout["b4_closed"]
    print(json.dumps({"passed": True, "product_commit": source["code_commit"],
                      "product_files": source["source"]["files"], "fixture_sha256": source["fixture"]["sha256"],
                      "b4_0_closed": True, "b4_1_closed": True,
                      "b4_0_1_closed": not args.prepare, "b4_closed": b4_closed,
                      "language_available": False, "cmath": "planned"},
                     ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
