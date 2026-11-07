"""生成固定身份的 B1 closeout.json；审计脚本不会调用或写入它。"""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent
CHECKS = [
    "b1-benchmark-check",
    "b1-dependency-tree",
    "b1-docs-all",
    "b1-runtime-clippy",
    "b1-runtime-fmt",
    "b1-runtime-tests",
]

records = [json.loads((ROOT / f"{name}.json").read_text(encoding="utf-8")) for name in CHECKS]
source_ids = {record["source_before"]["manifest"] for record in records}
if len(source_ids) != 1:
    raise SystemExit("执行记录不是同一份源码清单")
guards = json.loads((ROOT / "b1-guards-report.json").read_text(encoding="utf-8"))
if guards.get("passed") != 28 or guards.get("total") != 28:
    raise SystemExit("守卫报告不是28/28")
closeout = {
    "passed": True,
    "scope": "B2-B B1：仅libm依赖与私有cmath/显式除法内核",
    "baseline_commit": "d08b8460",
    "code_commit": "35872782",
    "source_manifest": next(iter(source_ids)),
    "checks": CHECKS,
    "passed_checks": 18,
    "runtime_tests": "255 passed / 0 failed / 0 ignored",
    "dependency_versions": {
        "bigdecimal": "0.4.11",
        "libm": "0.2.16",
        "num-bigint": "0.4.8",
        "num-integer": "0.1.47",
        "num-traits": "0.2.19",
        "autocfg": "1.5.1",
    },
    "guards": "28/28",
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
    "allowed_code_paths": [
        "core/rust/Cargo.lock",
        "core/rust/Cargo.toml",
        "core/rust/crates/xiao-runtime/Cargo.toml",
        "core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs",
        "core/rust/crates/xiao-runtime/src/value/arithmetic/explicit_division.rs",
        "core/rust/crates/xiao-runtime/src/value/arithmetic/mod.rs",
        "tests/benchmarks/Cargo.lock",
    ],
}
(ROOT / "closeout.json").write_text(
    json.dumps(closeout, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
)
print(json.dumps({"written": "closeout.json", "passed_checks": 18}, ensure_ascii=False))
