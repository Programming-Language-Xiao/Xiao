"""只读核验B5四类固定证据、产品身份、最终文档与阶段结论。"""
import argparse
from datetime import datetime
from functools import lru_cache
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
PREFIX = ROOT.relative_to(REPOSITORY).as_posix()
ENTRY = "606bbe6514104068358f9f174d5c6da53ee352b7"
CURRENT = "67319d61c18b2f02a57f974883c57600f70c724d"
FIX = "cfab977105273dcfdfc1831332b3a804ccc217d9"
MATRIX_CODE = "5864cbde627f727483ab676c6f4bc1bf0a7819bb"
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]
SOURCES = {
    "matrix": ("06eee789f9553212de34a70bc2b159b5c807975a",
               "docs/DevDocs/assets/20o-b2-b/entry-20261116-b5-rebuild", "evidence.json"),
    "fix": ("cba346fc6f3039cbc76889f4907ea8539c38ff48",
            "docs/DevDocs/assets/20o-b2-static-float-equality/entry-20261117-fix",
            "evidence-final-r4.json"),
    "url": (ENTRY, "docs/DevDocs/assets/20o-b2-b/entry-20261117-url-optimization",
            "evidence.json"),
}
DOC_FILES = ["docs/DevDocs/" + name for name in (
    "20o-b2-b-implementation-plan.md", "20o-b2-b4-5-closeout-plan.md",
    "20o-b2-b5-version-matrix-plan.md", "20o-b2-b6-resource-performance-plan.md",
    "20o-b2-b7-closeout-plan.md", "20o-b2-c-final-closeout-plan.md",
    "20o-b2-numeric-foundation-plan.md", "20o-official-library-program.md", "README.md",
)]
FIX_PATHS = {
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
URL_PATH = "core/rust/crates/xiao-driver/tests/20o_b1_imp_resources.rs"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False,
                      separators=(",", ":")).encode("utf-8")


def identity(raw):
    return {"bytes": len(raw), "sha256": sha(raw)}


def inventory(root):
    return {p.relative_to(root).as_posix(): identity(p.read_bytes())
            for p in sorted(root.rglob("*")) if p.is_file()}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def git(repository, *args):
    child = subprocess.run(["cmd", "/c", "git", "-C", str(repository), *args],
                           capture_output=True)
    require(child.returncode == 0, "固定Git对象不可读：" +
            child.stderr.decode("utf-8", errors="replace"))
    return child.stdout


@lru_cache(maxsize=None)
def files(repository, commit, paths):
    raw = git(repository, "archive", "--format=tar", commit, "--", *paths)
    with tarfile.open(fileobj=io.BytesIO(raw)) as archive:
        return {item.name: archive.extractfile(item).read()
                for item in archive.getmembers() if item.isfile()}


def stored(root, reference):
    path = (root / reference["path"]).resolve()
    require(path.is_relative_to(root.resolve()), "收口内容对象路径越界")
    packed = path.read_bytes()
    require(identity(packed) == {key: reference[key] for key in ("bytes", "sha256")},
            "收口内容对象字节不符")
    require(reference["encoding"] in ("gzip", "identity"), "收口内容对象编码不符")
    raw = gzip.decompress(packed) if reference["encoding"] == "gzip" else packed
    require(len(raw) == reference["raw_bytes"] and sha(raw) == reference["raw_sha256"],
            "收口内容对象原文不符")
    return raw


def source(repository, key):
    commit, directory, manifest_name = SOURCES[key]
    blobs = files(repository, commit, (directory,))
    relative = {name[len(directory) + 1:]: content for name, content in blobs.items()}
    manifest = json.loads(relative[manifest_name])
    expected = {name: identity(raw) for name, raw in relative.items() if name != manifest_name}
    require(manifest["files"] == expected and ".gitattributes" in expected,
            "历史证据的固定清单不完整：" + key)
    descriptor = {
        "commit": commit, "directory": directory, "manifest": manifest_name,
        "manifest_identity": identity(relative[manifest_name]),
        "git_tree": git(repository, "rev-parse", commit + ":" + directory).decode().strip(),
        "files_including_manifest": len(relative),
        "inventory_sha256": sha(canonical({name: identity(raw) for name, raw in relative.items()})),
    }
    return descriptor, relative


def object_from(blobs, reference):
    packed = blobs[reference["path"]]
    require(identity(packed) == {key: reference[key] for key in ("bytes", "sha256")},
            "历史日志对象摘要不符")
    if reference.get("encoding") == "gzip":
        return gzip.decompress(packed)
    return packed


def expected(repository, root):
    product_files = files(repository, CURRENT, tuple(PRODUCT_ROOTS))
    product_sha256 = sha(canonical({
        name: sha(content.replace(b"\r\n", b"\n")) for name, content in product_files.items()
    }))
    dependencies = files(repository, ENTRY, (
        "docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/record.py",
        "docs/DevDocs/20o-b2-static-float-equality-fix-plan.md",
    ))
    dependencies = {name: identity(raw.replace(b"\r\n", b"\n"))
                    for name, raw in dependencies.items()}
    descriptors, blobs = {}, {}
    for key in SOURCES:
        descriptors[key], blobs[key] = source(repository, key)
        receipt = load(root / (key + "-audit.json"))
        require(receipt["source"] == descriptors[key] and receipt["exit_code"] == 0
                and receipt["source_before"] == receipt["source_after"]
                == descriptors[key]["inventory_sha256"] and receipt["read_only"],
                "历史审计回执或来源不符：" + key)
        require(receipt["pid"] > 0 and datetime.fromisoformat(receipt["finished_utc"])
                >= datetime.fromisoformat(receipt["started_utc"]), "历史审计PID或时间不符")
        require(receipt["argv"][1:] == [
            "-B", "-X", "utf8", descriptors[key]["directory"] + "/audit.py"]
            and receipt["product_before"] == receipt["product_after"] == product_sha256
            and receipt["dependency_inputs"] == dependencies
            and receipt["collector_sha256"] == sha((root / "collect.py").read_bytes()),
            "历史审计的命令、采集器或依赖身份不符：" + key)
        output = json.loads(stored(root, receipt["stdout"]))
        require(not stored(root, receipt["stderr"]), "历史审计存在异常输出：" + key)
        if key == "matrix":
            require(output["evidence_valid"] is True and output["blocked"] is True
                    and output["stage_passed"] is False and output["b5_closed"] is False
                    and output["codegen_version"] == 21, "B5历史失败被改写为通过")
        elif key == "fix":
            require(output["repair_acceptance_passed"] and output["read_only"]
                    and output["generator"] == 22 and output["native"] == 248,
                    "21→22增量审计未通过")
        else:
            require(output["passed"] and output["sealed"] and output["read_only"]
                    and output["workspace_passed"] == 1754 and output["resource_tests"] == 18,
                    "URL优化与完整workspace审计未通过")
    old = json.loads(blobs["matrix"]["closeout.json"])
    require(old["evidence_valid"] and not old["passed"] and not old["b5_closed"]
            and not old["default_workspace"]["passed"]
            and old["baseline_kind"] == "reconstructed_from_B1_source"
            and old["historical_originals_available"] is False
            and [old[key] for key in ("version_matrix_cases", "new_entry_matrix_cases",
                                     "cache_matrix_groups", "c_preserved_signatures",
                                     "c_added_signatures")] == [36, 12, 2, 136, 16],
            "历史版本/缓存/C ABI矩阵结论不符")
    require(old["guards"]["actual_output_cases"] == 4
            and old["guards"]["derived_negatives"] == 24, "守卫来源口径不符")
    matrix = json.loads(blobs["fix"]["version-matrix-r3.json"])
    require(matrix["passed"] and len(matrix["cache"]) == 12
            and {(r["actual_validator"], r["artifact_generator"]): r["accepted"]
                 for r in matrix["validation"]}
            == {(21, 21): True, (21, 22): False, (22, 21): True, (22, 22): True},
            "21→22拒绝/缓存增量不符")
    result = json.loads(blobs["url"]["result.json"])
    workspace = json.loads(blobs["url"]["workspace-final.json"])
    log = object_from(blobs["url"], workspace["log"])
    rows = re.findall(rb"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", log)
    require(len(rows) == 191 and tuple(sum(int(r[i]) for r in rows) for i in range(3))
            == (1754, 0, 55), "当前完整workspace原始日志不符")
    require(b"test url_decoding_reaches_the_unicode_result_budget ... ok" in log
            and b"test url_decoding_resets_utf8_state_between_escaped_codepoints ... ok" in log,
            "当前默认workspace缺少URL两项成功记录")
    resource_log = object_from(blobs["url"], result["resource_target"]["log"])
    require(b"18 passed; 0 failed; 0 ignored;" in resource_log,
            "URL独立资源目标不是18/18")
    for previous, current, paths in (
        (MATRIX_CODE, FIX, FIX_PATHS), (FIX, CURRENT, {URL_PATH}), (CURRENT, ENTRY, set()),
    ):
        changed = set(git(repository, "diff", "--name-only", previous, current,
                          "--", *PRODUCT_ROOTS).decode().splitlines())
        require(changed == paths, "四类证据之间有未验收的产品变化")
    current = product_files
    require(b"pub const CODEGEN_VERSION: u32 = 22;" in
            current["core/rust/crates/xiao-codegen-llvm/src/lib.rs"]
            and b"pub const ABI_MINOR_VERSION: u32 = 19;" in
            current["core/rust/crates/xiao-runtime-abi/src/lib.rs"], "当前1.19/22身份不符")
    return {
        "stage": "B5", "passed": True, "evidence_valid": True, "stage_passed": True,
        "blocked": False, "b5_closed": True, "b6_entry_open": True,
        "b6_started": False, "b7_started": False, "b2_c_started": False,
        "entry_commit": ENTRY, "current_code_commit": CURRENT,
        "runtime_abi": "1.19", "codegen_version": 22, "stdlib_interface_version": 1,
        "cmath": "planned", "public_release": False,
        "components": {
            "b5_historical_matrix": {"source": "matrix", "code_commit": MATRIX_CODE,
                                    "runtime_abi": "1.19", "codegen_version": 21},
            "generator_21_to_22": {"source": "fix", "code_commit": FIX,
                                   "metadata_validation_cases": 4, "cache_rows": 12},
            "url_optimization": {"source": "url", "code_commit": CURRENT,
                                  "resource_passed": 18, "resource_failed": 0,
                                  "unicode_test_seconds": 5.65},
            "current_default_workspace": {"source": "url", "receipt": "workspace-final.json",
                                           "source_commit": CURRENT},
        },
        "sources": descriptors,
        "audit_receipts": {key: identity((root / (key + "-audit.json")).read_bytes())
                           for key in SOURCES},
        "historical_boundary": {
            "b5_matrix_closed": False, "original_default_workspace_passed": False,
            "baseline_kind": "reconstructed_from_B1_source",
            "historical_originals_available": False,
            "version_cases": 36, "new_entry_cases": 12, "cache_groups": 2,
            "c_preserved_signatures": 136, "c_added_signatures": 16,
            "guards_real_output": 4, "guards_derived_negatives": 24,
            "historical_direct_timing_samples": 8, "causal_attribution_proven": False,
        },
        "current_workspace": result["workspace"],
        "b6_required_url_load": {
            "case": "url.decode_unicode", "phase": "maximum",
            "escaped_codepoints": 16384, "input_utf8_bytes": 196608,
            "expected_utf8_bytes": 65536, "batches": 5,
            "vm_release_max_seconds": 10, "native_o0_o3_max_seconds": 2,
            "resident_delta_max_mib": 64,
            "platforms": ["Windows", "Linux", "macOS"],
            "replacement_by_default_256_or_mixed_case_allowed": False,
        },
        "platforms_executed": ["Windows x86_64-pc-windows-msvc"],
        "not_executed": ["Linux", "macOS", "remote CI", "B6 performance", "721 F10"],
        "retained_debts": ["B1 historical originals missing", "22C", "22D", "19/11/22"],
    }


def audit(root, repository, prepare):
    actual = inventory(root)
    for name in (".gitattributes", "closeout.json", "matrix-audit.json", "fix-audit.json",
                 "url-audit.json"):
        require(name in actual, "收口必需文件缺失：" + name)
    if not prepare:
        manifest = load(root / "evidence.json")
        require(manifest["files"] == {name: row for name, row in actual.items()
                                      if name != "evidence.json"}, "收口清单与实际字节不符")
        commit = git(repository, "log", "-1", "--format=%H", "--",
                     PREFIX + "/evidence.json").decode().strip()
        require(commit and git(repository, "show", commit + ":" + PREFIX + "/evidence.json")
                == (root / "evidence.json").read_bytes(), "收口清单偏离固定Git对象")
    value, computed = load(root / "closeout.json"), expected(repository, root)
    require(value.get("sources") == computed["sources"], "收口引用偏离固定Git来源")
    require(value.get("components") == computed["components"], "四类收口证据缺失或被替换")
    require(value.get("historical_boundary") == computed["historical_boundary"],
            "历史失败/重建来源边界被改写")
    require(value.get("current_workspace") == computed["current_workspace"], "当前默认workspace统计不符")
    require(value == computed, "收口状态或发布边界冒进")
    if not prepare:
        docs = load(root / "docs-final.json")
        require(docs["exit_code"] == 0 and docs["read_only"] and docs["pid"] > 0
                and docs["argv"][1:] == ["tools/repo-check/src/cli.ts", "all", "--format", "json"]
                and docs["product_before"] == docs["product_after"]
                == sha(canonical({name: sha(content.replace(b"\r\n", b"\n"))
                                  for name, content in files(repository, CURRENT,
                                      tuple(PRODUCT_ROOTS)).items()})),
                "收口文档门禁未通过")
        inputs = json.loads(stored(root, docs["inputs"]))
        require(inputs == json.loads(stored(root, docs["inputs_after"])),
                "收口文档门禁前后字节不同")
        frozen = files(repository, commit, tuple(inputs))
        required_inputs = set(DOC_FILES) | {
            PREFIX + "/" + path.name for path in root.iterdir()
            if path.is_file() and path.name not in ("docs-final.json", "evidence.json")
        }
        require(set(inputs) == set(frozen) == required_inputs,
                "收口文档门禁未覆盖全部修改文档")
        for name, content in frozen.items():
            require(inputs[name]["lf_sha256"] == sha(content.replace(b"\r\n", b"\n")),
                    "收口文档定稿身份不符：" + name)
        report = json.loads(stored(root, docs["stdout"]))
        require(report["passed"] and not any(r["severity"] == "error"
                                           for r in report["diagnostics"]),
                "收口文档门禁实际报告未通过")
        review = load(root / "review.json")
        reasons = {
            "missing": "收口必需文件缺失：.gitattributes",
            "source": "收口引用偏离固定Git来源",
            "component": "四类收口证据缺失或被替换",
            "historical": "历史失败/重建来源边界被改写",
            "statistics": "当前默认workspace统计不符",
            "state": "收口状态或发布边界冒进",
        }
        require(review["passed"] and [row["case"] for row in review["cases"]]
                == ["baseline", *reasons], "封存前语义反例集合不完整")
        for row in review["cases"]:
            reason = reasons.get(row["case"])
            require(row["read_only"] and row["expected_reason"] == reason
                    and row["exit_code"] == (0 if reason is None else 1)
                    and row.get("reason") == reason and row["passed"] is (reason is None),
                    "封存前语义反例没有按实际原因拒绝")
    return {"passed": True, "sealed": not prepare, "b5_closed": True, "b6_entry_open": True,
            "b6_started": False, "code_commit": CURRENT, "runtime_abi": "1.19",
            "codegen_version": 22, "workspace": [1754, 0, 55], "resource_tests": 18,
            "files_including_manifest": len(actual), "multi_platform_or_ci_passed": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--prepare", action="store_true", help="封存前仅核验四类证据语义")
    args = parser.parse_args()
    root, repository = args.root.resolve(), args.repository_root.resolve()
    before = inventory(root)
    try:
        result, code = audit(root, repository, args.prepare), 0
    except Exception as error:
        result, code = {"passed": False, "reason": str(error)}, 1
    result["read_only"] = inventory(root) == before
    if not result["read_only"]:
        result, code = {"passed": False, "reason": "审计改写了收口目录", "read_only": False}, 1
    sys.stdout.reconfigure(encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
