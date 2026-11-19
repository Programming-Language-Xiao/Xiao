"""只读核验本机修复回执、LLVM对象、版本/缓存矩阵及最终清单。"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path, PureWindowsPath
import subprocess
import sys
from datetime import datetime

sys.dont_write_bytecode = True
BASELINE_COMMIT = "5864cbde627f727483ab676c6f4bc1bf0a7819bb"
ENTRY_COMMIT = "aead9c5501177f17976bf8a54f324ac6091bbff7"
PRODUCT_COMMIT = "cfab977105273dcfdfc1831332b3a804ccc217d9"
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]
ALLOWED_CHANGED_PRODUCTS = {
    "core/rust/crates/xiao-codegen-llvm/README.md",
    "core/rust/crates/xiao-codegen-llvm/src/ir.rs",
    "core/rust/crates/xiao-codegen-llvm/src/lib.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_llvm.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_vm.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_frontend.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b5_versions.rs",
    "core/rust/crates/xiao-driver/tests/20o_static_float_equality.rs",
    "core/rust/crates/xiao-driver/tests/support/float_equality.rs",
}
RECEIPT_STATUS = {
    "runtime-build.json": 0, "comparisons-default.json": 0,
    "comparisons-default-r2.json": 101, "comparisons-default-r3.json": 0,
    "comparisons-native.json": 0, "existing-regressions.json": 0,
    "clippy.json": 0, "version-probes.json": 1,
    "version-probes-r2.json": 0, "version-probes-r3.json": 0,
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def identity(raw):
    return {"bytes": len(raw), "sha256": sha(raw)}


def file_identity(path):
    raw = path.read_bytes()
    return identity(raw)


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def stored(root, ref):
    path = (root / ref["path"]).resolve()
    require(path.is_relative_to(root.resolve()), "内容对象越出本证据目录")
    packed = path.read_bytes()
    require(identity(packed) == {"bytes": ref["bytes"], "sha256": ref["sha256"]},
            "内容对象压缩字节摘要错误：" + ref["path"])
    raw = gzip.decompress(packed) if ref["encoding"] == "gzip" else packed
    require(ref["encoding"] in ("gzip", "identity")
            and len(raw) == ref["raw_bytes"] and sha(raw) == ref["raw_sha256"],
            "内容对象原始字节摘要错误：" + ref["path"])
    return raw


def git(repository, *arguments):
    result = subprocess.run(["cmd", "/c", "git", "-C", str(repository), *arguments],
                            capture_output=True, encoding="utf-8", errors="strict")
    require(result.returncode == 0, "固定Git对象复核失败：" + result.stderr)
    return result.stdout.strip()


def git_bytes(repository, *arguments):
    result = subprocess.run(["cmd", "/c", "git", "-C", str(repository), *arguments],
                            capture_output=True)
    require(result.returncode == 0,
            "固定Git字节对象复核失败：" + result.stderr.decode("utf-8", errors="replace"))
    return result.stdout


def scan(root):
    return {
        path.relative_to(root).as_posix(): file_identity(path)
        for path in sorted(root.rglob("*")) if path.is_file()
    }


def audit(root, repository, receipts_only):
    before = scan(root)
    environment = load(root / "environment.json")
    task = Path(environment["task_root"]).resolve()
    require(task == (repository.parent / ".b5/float-equality-fix-20261117-01").resolve()
            and environment["fresh_target"] is True,
            "任务根或全新构建身份不符")
    require(git(repository, "rev-parse", ENTRY_COMMIT) == ENTRY_COMMIT
            and git(repository, "rev-parse", BASELINE_COMMIT) == BASELINE_COMMIT,
            "入口/基线Git对象不可复核")
    require(environment["entry_commit"] == ENTRY_COMMIT, "记录入口提交不符")
    require(git(repository, "rev-parse", PRODUCT_COMMIT) == PRODUCT_COMMIT
            and git(repository, "merge-base", "--is-ancestor", PRODUCT_COMMIT, "HEAD") == "",
            "产品提交不是本批证据的祖先")
    changed = set(git(repository, "diff", "--name-only", ENTRY_COMMIT, PRODUCT_COMMIT,
                      "--", *PRODUCT_ROOTS).splitlines())
    require(changed == ALLOWED_CHANGED_PRODUCTS, "产品差异越出本批精确允许路径")
    for key in ("CARGO_TARGET_DIR", "TEMP", "TMP", "XIAO_HOME", "XIAO_RUNTIME_LIBRARY",
                "XIAO_FLOAT_EQUALITY_OUTPUT"):
        path = Path(environment["environment_overrides"][key]).resolve()
        require(path.is_relative_to(task), "环境路径未隔离于任务根：" + key)

    counts = {"commands": 0, "commands_passed": 0, "commands_failed_preserved": 0}
    for name, expected in RECEIPT_STATUS.items():
        receipt = load(root / name)
        require(receipt["exit_code"] == expected and receipt["source_unchanged"],
                "命令回执状态或源码前后摘要不符：" + name)
        require(receipt["pid"] > 0
                and datetime.fromisoformat(receipt["finished_utc"])
                >= datetime.fromisoformat(receipt["started_utc"]),
                "命令PID或时间无效：" + name)
        require(stored(root, receipt["log"]), "命令原始日志为空：" + name)
        counts["commands"] += 1
        if expected == 0:
            counts["commands_passed"] += 1
        else:
            counts["commands_failed_preserved"] += 1
        source_before = json.loads(stored(root, receipt["source_before"]))
        source_after = json.loads(stored(root, receipt["source_after"]))
        require(source_before == source_after, "源码身份前后不一致：" + name)
        if receipt.get("runtime_before") is not None:
            require(receipt["runtime_before"] == receipt["runtime_after"],
                    "单项命令前后Runtime身份变化：" + name)
        if name == "runtime-build.json":
            runtime = receipt["runtime_after"]
            require(runtime is not None and Path(runtime["path"]).resolve().is_relative_to(task),
                    "Runtime构建产物未落在全新任务根")

    if not receipts_only:
        docs = load(root / "docs-gate-final-r13.json")
        require(docs["exit_code"] == 0 and docs["source_unchanged"]
                and docs["source_before"] == docs["source_after"]
                and docs["pid"] > 0,
                "最终文档/源码门禁未通过")
        require(datetime.fromisoformat(docs["finished_utc"])
                >= datetime.fromisoformat(docs["started_utc"]),
                "最终文档门禁时间无效")
        stored(root, docs["log"])
        inputs = docs["inputs"]
        plan_name = "docs/DevDocs/20o-b2-static-float-equality-fix-plan.md"
        readme_name = (
            "docs/DevDocs/assets/20o-b2-static-float-equality/"
            "entry-20261117-fix/README.md"
        )
        require(plan_name in inputs and readme_name in inputs
                and any(name.startswith("docs/UseDocs/") for name in inputs)
                and "core/rust/crates/xiao-codegen-llvm/src/ir.rs" in inputs
                and "core/rust/crates/xiao-driver/tests/support/float_equality.rs" in inputs,
                "最终门禁输入未覆盖规划、产品、回归或DevDocs")
        negative = load(root / "audit-negative.json")
        require(negative["passed"] is True and negative["source_unchanged"] is True
                and {row["case"] for row in negative["cases"]}
                == {"baseline", "tampered-manifest", "missing-gitattributes"}
                and all(row["read_only"] and row["reason_matches"]
                        for row in negative["cases"]),
                "审计自反例未按预期完成")
        tampered = next(row for row in negative["cases"]
                        if row["case"] == "tampered-manifest")
        missing = next(row for row in negative["cases"]
                       if row["case"] == "missing-gitattributes")
        require(tampered["expected_reason"] == "工作区证据清单与最终Git对象字节不一致"
                and missing["expected_reason"] == "证据清单与实际字节不一致",
                "自反例预期拒绝原因不符")

    default_test = stored(root, load(root / "comparisons-default-r3.json")["log"])
    require(b"2 passed; 0 failed; 1 ignored" in default_test,
            "默认VM/静态回归最终摘要不符")
    native = load(root / "native-results.json")
    require(native["passed"] is True and native["case_count"] == 62
            and native["native_count"] == 248
            and len(native["rows"]) == 248,
            "显式原生矩阵尺寸不符")
    llvm_objects, bitcode_objects, native_count = set(), set(), 0
    for row in native["rows"]:
        require(row["exit_code"] == (0 if row["mode"] == "dynamic"
                                     else int(row["expected"]))
                and row["stderr"] == "",
                "原生结果状态不符：" + row["case"])
        require(row["stdout"] == (
            str(row["expected"]).lower() + chr(10) if row["mode"] == "dynamic" else ""
        ), "原生输出不符：" + row["case"])
        llvm = stored(root, row["llvm"]["object"])
        bitcode = stored(root, row["bitcode"]["object"])
        require(identity(llvm) == {key: row["llvm"][key] for key in ("bytes", "sha256")}
                and identity(bitcode) == {
                    key: row["bitcode"][key] for key in ("bytes", "sha256")
                }, "LLVM对象摘要不符：" + row["case"])
        require(row["executable"]["bytes"] > 0 and row["executable"]["sha256"],
                "原生执行文件身份缺失：" + row["case"])
        llvm_objects.add(row["llvm"]["object"]["sha256"])
        bitcode_objects.add(row["bitcode"]["object"]["sha256"])
        native_count += 1
    require(len(llvm_objects) == native["llvm_unique_content_objects"]
            and len(bitcode_objects) == native["bitcode_unique_content_objects"]
            and native_count == 248
            and len(native["executables_recorded_not_archived"]) == 248,
            "原生对象计数或内容寻址清单不符")
    require(b"FLOAT-EQUALITY cases=62 native=248 llvm_as=248 vm=62 result=passed"
            in stored(root, load(root / "comparisons-native.json")["log"]),
            "显式原生测试宿主日志缺少完整矩阵摘要")

    matrix = load(root / "version-matrix-r3.json")
    require(matrix["passed"] is True and len(matrix["validation"]) == 4
            and len(matrix["cache"]) == 12,
            "21/22真实版本矩阵缺项")
    accepted = {(row["actual_validator"], row["artifact_generator"]): row["accepted"]
                for row in matrix["validation"]}
    require(accepted == {(21, 21): True, (21, 22): False,
                         (22, 21): True, (22, 22): True},
            "元数据接受/拒绝方向错误")
    rejected = next(row for row in matrix["validation"]
                    if row["actual_validator"] == 21 and row["artifact_generator"] == 22)
    require(rejected["code"] == "X17-XAR-007" and rejected["entry_executed"] is False,
            "21拒绝22的错误身份或执行边界错误")
    for mode in ("static", "dynamic"):
        for level in (0, 3):
            rows = [row for row in matrix["cache"]
                    if row["lowering"] == mode and row["optimization"] == level]
            require(len(rows) == 3, "缓存分组行数不符")
            old, new, hit = rows
            require(old["actual_codegen"] == 21 and new["actual_codegen"] == 22
                    and old["cache_hit"] is False and old["regenerated"] is True
                    and new["cache_hit"] is False and new["regenerated"] is True
                    and old["fingerprint"] != new["fingerprint"]
                    and old["request_key"] != new["request_key"]
                    and old["previous_codegen"] == []
                    and new["previous_codegen"] == [21]
                    and hit["cache_hit"] is True and hit["regenerated"] is False
                    and hit["sha256"] == new["sha256"],
                    "真实缓存失配/重建/控制命中不符：" + mode + str(level))

    if not receipts_only:
        manifest = load(root / "evidence-final-r4.json")
        require(manifest["file_count"] == len(manifest["files"])
                and manifest["files"] == {
                    name: value for name, value in before.items() if name != "evidence-final-r4.json"
                }, "证据清单与实际字节不一致")
        committed_manifest = git_bytes(
            repository,
            "show",
            "HEAD:docs/DevDocs/assets/20o-b2-static-float-equality/entry-20261117-fix/evidence-final-r4.json",
        )
        require((root / "evidence-final-r4.json").read_bytes() == committed_manifest,
                "工作区证据清单与最终Git对象字节不一致")
        require(".gitattributes" in manifest["files"], "清单缺少.gitattributes")
        plan = (repository / plan_name).read_bytes()
        require(manifest["plan"] == identity(plan)
                and inputs[plan_name]["sha256"] == sha(plan)
                and inputs[readme_name]["sha256"] == sha((root / "README.md").read_bytes()),
                "门禁、证据清单和最终规划/README字节不一致")
        require(stored(root, manifest["plan_object"]) == plan,
                "归档规划对象与当前已验收规划不同")
    after = scan(root)
    require(before == after, "审计过程改写了本目录")
    return {
        "scope": "只读Windows本机静态浮点相等修复证据核验",
        "read_only": True, "repair_acceptance_passed": not receipts_only,
        "receipt_only": receipts_only, "manifest_checked": not receipts_only,
        "counts": counts, "native": 248, "cases": 62,
        "generator": 22, "runtime_abi": 65555,
        "baseline_commit": BASELINE_COMMIT, "entry_commit": ENTRY_COMMIT,
        "product_commit": PRODUCT_COMMIT,
        "multi_platform_or_ci_passed": False, "external_blockers_closed": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument("--repository-root", type=Path)
    parser.add_argument("--receipts-only", action="store_true",
                        help="只核对真实回执，不检查最终清单")
    args = parser.parse_args()
    root = args.root.resolve()
    repository = (args.repository_root or root.parents[4]).resolve()
    try:
        print(json.dumps(audit(root, repository, args.receipts_only),
                         ensure_ascii=False, allow_nan=False))
    except (ValueError, KeyError, OSError, TypeError) as error:
        print(json.dumps({"scope": "只读本机修复证据核验", "failed": str(error)},
                         ensure_ascii=False), file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
