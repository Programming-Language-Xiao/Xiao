"""只读核对未修复基线；不执行产品，不把已确认缺陷写成修复通过。"""
import argparse
from datetime import datetime
import gzip
import hashlib
import json
from pathlib import Path, PureWindowsPath
import subprocess
import sys

sys.dont_write_bytecode = True
BASELINE = "5864cbde627f727483ab676c6f4bc1bf0a7819bb"
REVIEW = "816866cda9e66fb0ce6a25c7e45d413cfa6a6627"
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust",
    "tests/fixtures", "tests/benchmarks", "stdlib", "tools", "cli/ts",
    "package.json", "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]
EXPECTED = {
    "float-zero-eq": False, "float-zero-ne": True,
    "float-negative-zero-eq": True, "float-finite-eq": True,
    "float-different-eq": False, "sfloat-zero-eq": False,
    "sfloat-zero-ne": True, "sfloat-negative-zero-eq": True,
    "sfloat-finite-eq": True, "sfloat-different-eq": False,
    "mixed-integer-float": True, "mixed-rounded-integer": True,
    "mixed-sint-sfloat": True, "mixed-sfloat-float": True,
    "integer-control": True, "boolean-control": False,
}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def identity(raw):
    return {"bytes": len(raw), "sha256": sha(raw)}


def matches(raw, expected):
    return identity(raw) == {key: expected[key] for key in ("bytes", "sha256")}


def stored(root, reference):
    path = (root / reference["path"]).resolve()
    require(path.is_relative_to(root.resolve()), "内容对象越过证据目录")
    packed = path.read_bytes()
    require(matches(packed, reference), "内容对象字节摘要不符：" + reference["path"])
    encoding = reference["encoding"]
    require(encoding in ("gzip", "identity"), "内容对象编码未知")
    raw = gzip.decompress(packed) if encoding == "gzip" else packed
    require(len(raw) == reference["raw_bytes"] and sha(raw) == reference["raw_sha256"],
            "内容对象解压摘要不符：" + reference["path"])
    return raw


def git(repository, *arguments):
    result = subprocess.run(
        ["cmd", "/c", "git", "-C", str(repository), *arguments],
        capture_output=True, encoding="utf-8", errors="strict",
    )
    require(result.returncode == 0, "固定Git对象核对失败：" + result.stderr)
    return result.stdout.strip()


def audit(root, repository, receipts_only):
    before_files = {path.relative_to(root).as_posix(): identity(path.read_bytes())
                    for path in sorted(root.rglob("*")) if path.is_file()}
    build = load(root / "probe-build.json")
    run = load(root / "probe-run.json")
    observed = load(root / "observations.json")
    require(build["baseline_code_commit"] == BASELINE
            and build["expected_codegen"] == 21
            and build["expected_runtime_abi"] == "1.19"
            and build["exit_code"] == 0 and run["exit_code"] == 0,
            "不是已成功采集的未修复基线")
    require(not build["repair_applied"] and not run["repair_applied"],
            "基线回执被误标为产品修复")
    require(run["build_receipt_sha256"] == sha((root / "probe-build.json").read_bytes()),
            "执行回执没有绑定实际构建回执")
    for filename, reference in (
        ("build-probe.py", build["build_script"]),
        ("run-probe.py", run["run_script"]),
        ("probe.rs", build["probe_source"]),
        ("probe.rs", run["probe_source"]),
    ):
        require(matches((root / filename).read_bytes(), reference),
                "采集源码已偏离实际执行字节：" + filename)
    require(build["libraries_before"] == build["libraries_after"]
            == run["libraries_before"] == run["libraries_after"]
            and build["libraries_unchanged"] and run["libraries_unchanged"],
            "借用调试库身份不一致")
    require(build["runtime_static_library"] == run["runtime_static_library"]
            and build["compiler"] == run["compiler"]
            and build["executable"] == run["executable_before"] == run["executable_after"],
            "Runtime、编译器或探针身份发生变化")
    require(run["product_worktree_clean_before"] and run["product_worktree_clean_after"],
            "采集时产品工作区并非干净")
    require(all(reference["binary"] == run["tools_after"][name]
                for name, reference in run["tools_before"].items()),
            "采集前后LLVM工具身份发生变化")
    require(run["json_parse_error"] is None and run["matrix_shape_valid"],
            "探针输出解析或矩阵形状失败")
    require(json.loads(stored(root, run["stdout"])) == observed,
            "观察文件与归档原始stdout不一致")
    require(stored(root, run["stderr"]) == b"", "探针宿主存在未说明stderr")
    require(stored(root, build["log"]) == b"", "探针构建日志出现额外内容")
    for receipt in (build, run):
        require(datetime.fromisoformat(receipt["finished_utc"])
                >= datetime.fromisoformat(receipt["started_utc"])
                and receipt["pid"] > 0, "执行时间或PID无效")
    timing = repository / "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-url-timing-20261116-01"
    for filename, key in (("sources/B.json", "cached_source_receipt_sha256"),
                          ("builds/B.json", "cached_build_receipt_sha256")):
        require(sha((timing / filename).read_bytes()) == build[key],
                "借用构建的历史来源回执变化：" + filename)
    require(git(repository, "rev-parse", BASELINE) == BASELINE
            and git(repository, "rev-parse", REVIEW) == REVIEW, "固定提交身份不符")
    require(not git(repository, "diff", "--name-only", BASELINE, REVIEW, "--", *PRODUCT_ROOTS),
            "借用构建与复核入口的受保护产品树不同")
    require(observed["runtime_abi"] == 65555 and observed["codegen_version"] == 21,
            "实际探针版本不符")
    require(observed["reference"] == {
        "runtime_language_float_zero_eq": False,
        "runtime_language_float_zero_ne": True,
        "runtime_language_sfloat_zero_eq": False,
        "runtime_language_sfloat_zero_ne": True,
        "rust_numeric_float_zero_equality": True,
    }, "实际语言入口的异号零参照不符")
    cases = observed["cases"]
    require(len(cases) == len(EXPECTED)
            and {row["case"] for row in cases} == set(EXPECTED), "16组基线缺失或重复")
    counts = {"vm_success": 0, "dynamic_matches_vm": 0,
              "static_fcmp_eq_rejected": 0, "static_zero_ne_mismatch": 0,
              "static_controls_match_vm": 0}
    archived_names = set()
    executable_names = set()
    for row in cases:
        label = row["case"]
        expected = EXPECTED[label]
        stdout = str(expected).lower() + chr(10)
        require(row["vm"]["success"] and row["vm"]["code"] is None
                and row["vm"]["stdout"] == stdout, "VM基线不符：" + label)
        counts["vm_success"] += 1
        for mode in ("static", "dynamic"):
            observations = row[mode].get("observations", [])
            require(len(observations) == 2
                    and {item["optimization"] for item in observations} == {0, 3},
                    "O0/O3观察缺失：" + label + "/" + mode)
            for item in observations:
                stem = label + "-" + mode + "-o" + str(item["optimization"])
                llvm_name = stem + ".ll"
                archived_names.add(llvm_name)
                llvm = stored(root, run["archived_llvm_files"][llvm_name]).decode("utf-8")
                require(("fcmp eq " in llvm) == item["contains_invalid_fcmp_eq"]
                        and ("fcmp une " in llvm) == item["contains_numeric_fcmp_une"],
                        "比较指令标记与真实LLVM不符：" + stem)
                assembly = item["assembly"]
                rejected_eq = (mode == "static" and row["operator"] == "=="
                               and label not in ("integer-control", "boolean-control"))
                if rejected_eq:
                    require(assembly["exit_code"] == 1
                            and "error: expected fcmp predicate" in assembly["stderr"]
                            and "fcmp eq " in assembly["stderr"]
                            and item["contains_invalid_fcmp_eq"]
                            and not item["native_executed"] and "execution" not in item,
                            "浮点相等未按声明原因拒绝：" + stem)
                    counts["static_fcmp_eq_rejected"] += 1
                    continue
                require(assembly["exit_code"] == 0 and item["native_executed"]
                        and item["build_text_matches_recorded"] and "build_error" not in item,
                        "原生基线构建或执行失败：" + stem)
                bitcode_name = stem + ".bc"
                archived_names.add(bitcode_name)
                stored(root, run["archived_llvm_files"][bitcode_name])
                executable_name = PureWindowsPath(item["executable"]).name
                executable_names.add(executable_name)
                require(executable_name in run["native_executables"]
                        and run["native_executables"][executable_name]["bytes"] > 0,
                        "原生执行文件缺少登记：" + stem)
                execution = item["execution"]
                require(execution["stderr"] == "", "原生观察有额外stderr：" + stem)
                if mode == "dynamic":
                    require(item["actual_uses_runtime"] and execution["exit_code"] == 0
                            and execution["stdout"] == stdout, "动态输出与VM不同：" + stem)
                    counts["dynamic_matches_vm"] += 1
                elif label in ("float-zero-ne", "sfloat-zero-ne"):
                    require(item["contains_numeric_fcmp_une"] and expected
                            and execution["exit_code"] == 0 and execution["stdout"] == "",
                            "异号零不等差异未按基线复现：" + stem)
                    counts["static_zero_ne_mismatch"] += 1
                else:
                    require(execution["exit_code"] == int(expected)
                            and execution["stdout"] == "", "静态对照输出不符：" + stem)
                    counts["static_controls_match_vm"] += 1
    require(counts == {"vm_success": 16, "dynamic_matches_vm": 32,
                       "static_fcmp_eq_rejected": 24, "static_zero_ne_mismatch": 4,
                       "static_controls_match_vm": 4}, "基线统计不符")
    require(archived_names == set(run["archived_llvm_files"]) and len(archived_names) == 104
            and executable_names == set(run["native_executables"])
            and len(executable_names) == 40, "LLVM或原生产物清单不符")
    if not receipts_only:
        manifest = load(root / "evidence.json")
        require(manifest["files"] == {
            name: value for name, value in before_files.items() if name != "evidence.json"
        }, "证据清单与实际字节不一致")
        require(manifest["file_count"] == len(manifest["files"]), "证据清单文件数不一致")
        require(manifest["plan"] == identity(stored(root, manifest["plan_object"])),
                "归档修订规划字节与证据清单不一致")
        docs = load(root / "docs-gate-final-verified.json")
        docs_identity = sha(json.dumps(
            docs["inputs"], ensure_ascii=False, sort_keys=True, separators=(",", ":")
        ).encode("utf-8"))
        require(docs["exit_code"] == 0 and docs["inputs_unchanged"]
                and docs["source_before"] == docs["source_after"] == docs_identity,
                "最终文档门禁或前后源码身份不符")
        require(docs["inputs"]["docs/DevDocs/20o-b2-static-float-equality-fix-plan.md"]
                == manifest["plan"], "文档门禁未绑定归档规划字节")
        readme_name = (
            "docs/DevDocs/assets/20o-b2-static-float-equality/"
            "entry-20261117-review/README.md"
        )
        require(docs["inputs"][readme_name] == identity((root / "README.md").read_bytes()),
                "文档门禁未绑定最终证据说明字节")
        stored(root, docs["log"])
    after_files = {path.relative_to(root).as_posix(): identity(path.read_bytes())
                   for path in sorted(root.rglob("*")) if path.is_file()}
    require(before_files == after_files, "审计改写了证据目录")
    return {
        "scope": "未修复基线回执核对", "baseline_findings_confirmed": True,
        "repair_applied": False, "product_acceptance_passed": False, "read_only": True,
        "manifest_checked": not receipts_only, "counts": counts, "llvm_files": 104,
        "native_executables_recorded_only": 40, "baseline_commit": BASELINE,
        "review_commit": REVIEW, "external_runtime_rehashed": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument("--repository-root", type=Path)
    parser.add_argument("--receipts-only", action="store_true",
                        help="只核对采集回执，明确不检查最终证据清单")
    args = parser.parse_args()
    root = args.root.resolve()
    repository = (args.repository_root or root.parents[4]).resolve()
    try:
        print(json.dumps(audit(root, repository, args.receipts_only),
                         ensure_ascii=False, allow_nan=False))
    except (ValueError, KeyError, OSError, TypeError) as error:
        print(json.dumps({"scope": "未修复基线回执核对", "failed": str(error)},
                         ensure_ascii=False), file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
