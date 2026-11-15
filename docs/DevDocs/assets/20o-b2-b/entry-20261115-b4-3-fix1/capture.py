"""B4-5当前源码的工具、守卫、阶段与1075项pow对账；不改历史样本。"""
from collections import Counter
import json
import math
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys

sys.dont_write_bytecode = True
from record import ROOT, REPOSITORY, PREFIX, PRODUCT_ROOTS, fixed_files, git, identity, load, require, sha, write_json
HISTORY = {"b0": "entry-20261107", "b1": "entry-20261108-b1", "b2": "entry-20261108-b2",
           "b3": "entry-20261108-b3", "b3-cabi": "entry-20261109-b3-cabi",
           "b3-wrapper": "entry-20261110-b3-wrapper-final", "b4-0-1": "entry-20261110-b4-0-1",
           "fix1": "entry-20261112-b4-2a-fix1", "b4-2b": "entry-20261113-b4-2b",
           "b4-3": "entry-20261113-b4-3", "b4-4": "entry-20261115-b4-4"}

MANIFEST = "docs/DevDocs/assets/20o-b2-b/entry-20261109-b3-cabi/baseline/pow-switch-manifest.json"
MANIFEST_SHA = "53b751666f42814f94993ed606e3dfe650941b022392d7d04ea57d9f3d16443d"
VARIANT = Path(os.environ["CARGO_TARGET_DIR"]).parent.name
STAGES = {
    "B4-1": ("b4-0-1", "c7a45e10b76b26915499717563761ed20fda6e6c"),
    "B4-2A/FIX1": ("fix1", "c46102115c63956466361d83243dea8ba3e4e74f"),
    "B4-2B": ("b4-2b", "246bea46105ead280aed91e5e085c8f253a8913d"),
    "B4-3": ("b4-3", "e0d5993e7c8214f2277d1bb81f9b05357d9305e0"),
    "B4-4": ("b4-4", "bb0b8196479e90d28d676f1bdefd04403e4ba6eb"),
}
FIX_COMMIT = "cfa79381bbae027cec11136ca380e82f66720c34"
FIX_PATHS = {
    "core/rust/crates/xiao-codegen-llvm/src/dynamic/predicate.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_numeric_range_routing.rs",
}


def toolchain():
    commands = {"rustc": ["rustc", "+1.96.0", "-Vv"], "cargo": ["cargo", "+1.96.0", "-V"],
                "targets": ["rustup", "target", "list", "--installed", "--toolchain", "1.96.0"],
                **{name: [os.environ[key], "--version"] for name, key in [
                    ("clang", "XIAO_CLANG"), ("llvm-as", "XIAO_LLVM_AS"), ("llc", "XIAO_LLC")]}}
    records = {}
    for name, argv in commands.items():
        run = subprocess.run(argv, check=True, capture_output=True, encoding="utf-8")
        path = Path(shutil.which(argv[0]) or argv[0])
        records[name] = {"argv": argv, "stdout": run.stdout, "stderr": run.stderr,
                         "path": str(path), "sha256": sha(path.read_bytes())}
    print(json.dumps(records, ensure_ascii=False))


def guards():
    report_path = ROOT / VARIANT / "guards-report.json"
    require(not report_path.exists(), "禁止覆盖守卫报告")
    argv = ["pwsh", "-NoProfile", "-File", "tools/gates/b1-probe-guards.ps1", "-BashPath",
            r"D:\msys64\usr\bin\bash.exe", "-ReportPath", str(report_path)]
    run = subprocess.run(argv, check=True, capture_output=True, encoding="utf-8")
    report = load(report_path)
    require(report["passed"] == report["total"] == len(report["cases"]) == 28
            and all(row["Passed"] for row in report["cases"]), "守卫没有完整通过")
    source = load(ROOT / VARIANT / "source.json")
    print(json.dumps({"passed": True, "scope": "simulated-cargo-output", "source_commit": source["revision"],
        "argv": argv, "stdout": run.stdout, "stderr": run.stderr, "report_sha256": sha(report_path.read_bytes()),
        "workflow_sha256": sha((REPOSITORY / report["workflow"]).read_bytes()),
        "workflow_canonical_sha256": source["files"][report["workflow"]]["canonical_sha256"]}, ensure_ascii=False))


def stages():
    source = load(ROOT / VARIANT / "source.json")
    rows = []
    for stage, (name, revision) in STAGES.items():
        directory = "docs/DevDocs/assets/20o-b2-b/" + HISTORY[name]
        closeout_path = REPOSITORY / directory / "closeout.json"
        closeout = load(closeout_path)
        require(closeout["passed"] is True and closeout["product_commit"] == revision, "阶段提交与收口不符")
        files = fixed_files(REPOSITORY, revision)
        evidence = REPOSITORY / directory / "evidence.json"
        rows.append({"stage": stage, "product_commit": revision, "closeout_path": directory + "/closeout.json",
                     "closeout_sha256": sha(closeout_path.read_bytes()), "evidence_sha256": sha(evidence.read_bytes()),
                     "product_tree_sha256": identity(files), "product_files": len(files),
                     "runtime_abi": closeout["runtime_abi"], "codegen_version": closeout["codegen_version"],
                     "scope": "fixed historical snapshot; current execution recorded separately"})
    changed = set(git(REPOSITORY, "diff", "--name-only", STAGES["B4-4"][1], source["revision"], "--", *PRODUCT_ROOTS).decode().splitlines())
    require(fixed_files(REPOSITORY, source["revision"]) == fixed_files(REPOSITORY, FIX_COMMIT)
            and changed == FIX_PATHS, "FIX.1产品允许路径或提交身份不符")
    rows.append({"stage": "B4-3-FIX1", "product_commit": FIX_COMMIT, "source_snapshot": source["revision"],
                 "allowed_product_paths": sorted(changed),
                 "product_tree_sha256": identity(fixed_files(REPOSITORY, FIX_COMMIT)),
                 "runtime_abi": "1.18", "codegen_version": 21, "scope": "current FIX.1 source; execution receipts recorded separately"})
    write_json(ROOT / VARIANT / "stage-reconciliation.json", {"passed": True, "source_commit": source["revision"], "stages": rows})
    print(json.dumps({"passed": True, "stages": len(rows), "source_commit": source["revision"],
        "report_sha256": sha((ROOT / VARIANT / "stage-reconciliation.json").read_bytes())}))


def value(bits):
    return struct.unpack(">d", bytes.fromhex(bits))[0]


def distance(left, right):
    if left.startswith("X") or right.startswith("X"):
        return None
    x, y = value(left), value(right)
    require((x == 0) == (y == 0) and math.copysign(1, x) == math.copysign(1, y), "pow零/符号不符")
    return abs(int(left, 16) - int(right, 16))


def frozen_rule(a, b, actual):
    if not math.isfinite(a) or not math.isfinite(b):
        require(actual == "X06-RUNTIME-009", "非有限输入优先级不符")
        return "F-2:nonfinite-input-first"
    if b == 0:
        require(actual == "3ff0000000000000", "零指数不符")
        return "F-2:zero-exponent"
    if a == 0 and b < 0:
        require(actual == "X06-RUNTIME-009", "零负幂不符")
        return "F-2:zero-negative-exponent-domain"
    if a < 0 and not b.is_integer():
        require(actual == "X06-RUNTIME-009", "负底非整数幂不符")
        return "F-2:negative-base-noninteger-domain"
    if a == 0:
        negative = math.copysign(1, a) < 0 and b.is_integer() and b % 2
        require(actual == ("8000000000000000" if negative else "0000000000000000"), "零幂符号不符")
        return "F-2:zero-base-sign-and-parity"
    if actual == "X06-RUNTIME-009":
        return "F-2:nonfinite-result-overflow"
    return "F-2/F-9:ieee-underflow-and-signed-zero" if value(actual) == 0 else "F-9:shared-libm-and-four-ulp"


def pow_review():
    task = Path(os.environ["CARGO_TARGET_DIR"]).parent
    dependencies = task / "target/release/deps"
    executable = task / "pow-probe.exe"
    argv = ["rustc", "+1.96.0", "--edition=2024", "--crate-name", "b4_5_pow_probe", str(ROOT / "pow_probe.rs"),
            "-L", "dependency=" + str(dependencies), "-o", str(executable)]
    for output in sorted((task / "target/release/build").glob("*/output")):
        for line in output.read_text(encoding="utf-8").splitlines():
            if line.startswith("cargo:rustc-link-search="):
                argv.extend(["-L", line.split("=", 1)[1]])
    drivers = list(dependencies.glob("libxiao_driver-*.rlib"))
    require(len(drivers) == 1, "pow宿主必须使用唯一驱动器依赖")
    fingerprints = task / "target/release/.fingerprint"
    driver_id = drivers[0].stem.removeprefix("libxiao_driver-")
    driver = load(fingerprints / ("xiao-driver-" + driver_id) / "lib-xiao_driver.json")
    dep_identity = {row[1]: row[3] for row in driver["deps"]}
    libraries = {"xiao_driver": drivers[0]}
    for name in ["xiao_bytecode", "xiao_runtime", "xiao_vm"]:
        matched = []
        for candidate in dependencies.glob("lib" + name + "-*.rlib"):
            suffix = candidate.stem.removeprefix("lib" + name + "-")
            metadata = fingerprints / (name.replace("_", "-") + "-" + suffix) / ("lib-" + name)
            if metadata.is_file() and int.from_bytes(bytes.fromhex(metadata.read_text().strip()), "little") == dep_identity[name]:
                matched.append(candidate)
        require(len(matched) == 1, "pow探针无法唯一匹配Cargo依赖指纹：" + name)
        libraries[name] = matched[0]
    for name, library in libraries.items():
        argv.extend(["--extern", name + "=" + str(library)])
    subprocess.run(argv, check=True)
    path = REPOSITORY / "tests/fixtures/docs/cmath/pow-legacy-diff.tsv"
    result = subprocess.run([str(executable), str(path)], check=True, capture_output=True, encoding="utf-8")
    # 逐行原始输出进入外层内容寻址日志；不只保存成功摘要。
    print(result.stdout, end="")
    raw = (REPOSITORY / MANIFEST).read_bytes()
    require(sha(raw) == MANIFEST_SHA, "固定pow清单摘要不符")
    manifest = json.loads(raw)
    actual = {}
    for line in result.stdout.splitlines():
        if line.startswith("B4-5-POW\t"):
            columns = line.split("\t")
            require(len(columns) == 12 and columns[1] not in actual, "pow输出重复或字段不符")
            actual[columns[1]] = columns[2:]
    require(len(actual) == len(manifest["rows"]) == 1075, "pow输出不足1075项")
    rows, summary, rules = [], Counter(), Counter()
    for old in manifest["rows"]:
        current = actual[old["id"]]
        require(current[:2] == [old["a_bits"], old["b_bits"]] and all(v == old["new_runtime_float_power"] for v in current[2:]),
                "pow当前VM/Runtime与冻结新行为不符")
        public = current[2]
        category = "same" if public == old["legacy_float_power"] else "value_or_error_boundary" if public.startswith("X") != old["legacy_float_power"].startswith("X") else "value_bits"
        basis = frozen_rule(value(old["a_bits"]), value(old["b_bits"]), public)
        ulp, reference_ulp = distance(old["legacy_float_power"], public), distance(public, old["reference"])
        require(category == old["difference"] and basis == old["frozen_rule"] and ulp == old["old_new_ulp"]
                and reference_ulp == old["new_reference_ulp"] and (reference_ulp is None or reference_ulp <= 4), "pow分类/规则/ULP不符")
        require(category != "value_bits" or ulp == 1, "24项差异必须各为1ULP")
        require(category != "value_or_error_boundary" or basis == "F-2:nonfinite-input-first", "40项边界归属不符")
        summary[category] += 1
        rules[basis] += 1
        rows.append({"id": old["id"], "a_bits": old["a_bits"], "b_bits": old["b_bits"], "legacy": old["legacy_float_power"],
                     "runtime_power": current[2], "cmath_pow": current[3], "vm_operator_and_primitive_3_carriers": current[4:],
                     "reference": old["reference"], "old_new_ulp": ulp, "new_reference_ulp": reference_ulp,
                     "difference": category, "frozen_rule": basis})
    require(dict(summary) == manifest["summary"] == {"same": 1011, "value_or_error_boundary": 40, "value_bits": 24}
            and dict(rules) == manifest["rules"], "pow完整分类不符")
    report = {"passed": True, "source_commit": load(ROOT / VARIANT / "source.json")["revision"], "manifest_sha256": MANIFEST_SHA,
              "fixture_sha256": sha(path.read_bytes()), "probe_sha256": sha((ROOT / "pow_probe.rs").read_bytes()),
              "build_argv": argv, "executable_sha256": sha(executable.read_bytes()), "cases": 1075,
              "summary": dict(summary), "rules": dict(rules), "vm_cases": 6450, "rows": rows,
              "scope": "internal bit injection for nonfinite and exact binary64; source/native gates separately recorded"}
    write_json(ROOT / VARIANT / "pow-review.json", report)
    print(json.dumps({"passed": True, "rows": 1075, "summary": dict(summary), "vm_cases": 6450,
                      "report_sha256": sha((ROOT / VARIANT / "pow-review.json").read_bytes())}))


if __name__ == "__main__":
    {"toolchain": toolchain, "guards": guards, "stages": stages, "pow": pow_review}[sys.argv[1]]()
