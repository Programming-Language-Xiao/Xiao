"""B4-3 只读证据审计：固定源码对象、隔离运行、O0/O3矩阵与边界。"""
import argparse
from functools import lru_cache
import hashlib
import json
from pathlib import Path, PureWindowsPath
import re
import shutil
import sys
import tempfile

sys.dont_write_bytecode = True
from record import (ROOT, REPOSITORY, PRODUCT_ROOTS, SELF_EVIDENCE_ROOT, canonical,
                    docs_tree_from_commit, fixed_files, git, identity, load, require, sha, write_json)

BASELINE = "0a9e68a7ab25eb672f13e526d85fc9dc4daaf062"
FIRST_FIXED = "b77b0d5cb20b715a1539fc8fe2807e363e334e2d"
SECOND_FIXED = "6056fb94d091368a8684ffa9c835c710e1ad5748"
CODE = "e0d5993e7c8214f2277d1bb81f9b05357d9305e0"
FINAL_VARIANT = "fixed-final2"
TASK_BASE = PureWindowsPath(r"D:\project\Xiao\.b4-3\20261113")
SCRIPT = SELF_EVIDENCE_ROOT + "/record.py"
AUDIT_SCRIPT = SELF_EVIDENCE_ROOT + "/audit.py"
EXPECTED_CODE_PATHS = {
    "core/rust/crates/xiao-codegen-llvm/README.md",
    "core/rust/crates/xiao-codegen-llvm/src/README.md",
    "core/rust/crates/xiao-codegen-llvm/src/artifact.rs",
    "core/rust/crates/xiao-codegen-llvm/src/cmath.rs",
    "core/rust/crates/xiao-codegen-llvm/src/dynamic.rs",
    "core/rust/crates/xiao-codegen-llvm/src/dynamic/cmath.rs",
    "core/rust/crates/xiao-codegen-llvm/src/dynamic/expression.rs",
    "core/rust/crates/xiao-codegen-llvm/src/dynamic/predicate.rs",
    "core/rust/crates/xiao-codegen-llvm/src/ir.rs",
    "core/rust/crates/xiao-codegen-llvm/src/lib.rs",
    "core/rust/crates/xiao-codegen-llvm/src/static_cmath.rs",
    "core/rust/crates/xiao-codegen-llvm/tests/n0_a.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_llvm.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_vm.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_frontend.rs",
    "core/rust/crates/xiao-driver/tests/support/b4_cmath_harness.c",
    "core/rust/crates/xiao-driver/tests/support/b4_cmath_native.rs",
    "core/rust/crates/xiao-runtime-abi/README.md",
    "tests/fixtures/docs/cmath/README.md",
    "tests/fixtures/docs/cmath/pow-legacy-diff.tsv",
}
LOG_DIR = Path("logs/sha256")
HISTORY_AUDITS = {
    "audit-final-b1": "entry-20261108-b1",
    "audit-final-b2": "entry-20261108-b2",
    "audit-final-b3": "entry-20261108-b3",
    "audit-final-b3-cabi": "entry-20261109-b3-cabi",
    "audit-final-b3-wrapper": "entry-20261110-b3-wrapper-final",
    "audit-final-b4-0-1": "entry-20261110-b4-0-1",
    "audit-final-b4-2b": "entry-20261113-b4-2b",
}
PRELIMINARY_AUDITS = {
    "audit-b1": "entry-20261108-b1",
    "audit-b2": "entry-20261108-b2",
    "audit-b3": "entry-20261108-b3",
    "audit-b3-cabi": "entry-20261109-b3-cabi",
    "audit-b3-wrapper": "entry-20261110-b3-wrapper-final",
    "audit-b4-0-1": "entry-20261110-b4-0-1",
    "audit-b4-2b": "entry-20261113-b4-2b",
}
PRELIMINARY_LABELS = {*PRELIMINARY_AUDITS, "collector"}
PRELIMINARY_LABELS.update({"final-docs", "final-docs-final", "final-docs-verified", "final-docs-closeout",
                           "final-docs-signed", "final-docs-finalized", "final-docs-productfix", "final-docs-release"})
PRELIMINARY_REASONS = {label: "parallel temporary log staging changed DevDocs snapshot; retained but not accepted"
                       for label in [*PRELIMINARY_AUDITS, "collector"]}
PRELIMINARY_REASONS["audit-b4-2b"] = "superseded by the serial final B4-2B historical audit"
PRELIMINARY_REASONS["final-docs"] = "superseded after final evidence auditor and evidence README were committed"
PRELIMINARY_REASONS["final-docs-final"] = "superseded after final audit corrected the B1 VM command identity"
PRELIMINARY_REASONS["final-docs-verified"] = "superseded after preliminary-run classification was corrected"
PRELIMINARY_REASONS["final-docs-closeout"] = "superseded after preliminary B4-2B audit classification was refined"
PRELIMINARY_REASONS["final-docs-signed"] = "superseded after exact B1 target counts were corrected"
PRELIMINARY_REASONS["final-docs-finalized"] = "superseded after baseline product-diff audit was corrected"
PRELIMINARY_REASONS["final-docs-productfix"] = "superseded after the final self-negative and inventory workflow was corrected"
PRELIMINARY_REASONS["final-docs-release"] = "superseded after Runtime identity became an explicit recorded receipt"
PRELIMINARY_REASONS["final-docs-verified"] = "superseded after preliminary-run classification was corrected"
RUN_LABELS = {
    "baseline": {"entry"},
    "fixed": {"runtime-build", "native-matrix", "regressions"},
    "fixed-final": {"runtime-build", "native-matrix"},
    FINAL_VARIANT: {
        "runtime-build", "runtime-identity", "runtime-offline-probe", "native-matrix", "regressions", "core-tests", "b1-vm", "b1-native",
        "clippy", "fmt", "collector-final", "final-docs-runtime", *HISTORY_AUDITS,
    },
}
NEGATIVE_REASONS = {
    "manifest": "B4-3 evidence inventory mismatch",
    "attributes": "B4-3 evidence attributes missing or changed",
    "orphan": "B4-3 unreferenced content log object",
    "product": "B4-3 fixed product Git identity mismatch",
    "docs": "B4-3 DevDocs binding mismatch",
    "environment": "B4-3 fixed task path escaped",
    "state": "B4-3 stage state or run result mismatch",
}


@lru_cache(maxsize=None)
def fixed_product_files(repository, revision):
    return fixed_files(Path(repository), revision)


@lru_cache(maxsize=None)
def fixed_devdocs_tree(repository, revision):
    return docs_tree_from_commit(Path(repository), revision)


def historical_audit_passed(label, result):
    if label in {"audit-final-b1", "audit-b1"}:
        return result.get("passed") == result.get("total") == 18
    return result.get("passed") is True


def negative_reason(name):
    suffix = ": fixed-final2/native-matrix" if name == "docs" else ""
    return NEGATIVE_REASONS[name] + suffix


def git_bytes(repository, *args):
    return git(repository, *args)


def inventory(root):
    files = {}
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(),
                "B4-3 evidence path cannot traverse a link")
        require("__pycache__" not in path.parts, "B4-3 evidence cannot contain Python cache")
        relative = path.relative_to(root).as_posix()
        if path.is_file() and relative not in {"evidence.json", "evidence.json.tmp"}:
            raw = path.read_bytes()
            files[relative] = {"bytes": len(raw), "sha256": sha(raw)}
    return files


def task_child(value, task):
    path = PureWindowsPath(value)
    require(path.is_absolute() and ".." not in path.parts and path != task and path.is_relative_to(task),
            NEGATIVE_REASONS["environment"])


def parse_target_counts(text):
    current, result = None, {}
    for line in text.splitlines():
        running = re.match(r"\s*Running (?:unittests .*|tests[\\/].*) \((.*)[\\/]([^\\/]+)-[0-9a-f]+\.exe\)$", line)
        if running:
            current = running.group(2)
        elif line.lstrip().startswith("Doc-tests "):
            current = line.split("Doc-tests ", 1)[1].strip() + ":doc"
        match = re.match(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;.* 0 filtered out;", line)
        if match:
            require(current is not None and current not in result, "B4-3 test target identity missing or repeated")
            result[current] = (int(match.group(1)), int(match.group(2)))
            current = None
    return result


def exact_target_counts(text, target, test_name):
    require(f"Running tests\\{target}.rs " in text and f"test {test_name} ... ok" in text,
            "B4-3 exact test identity mismatch: " + target + "/" + test_name)
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;.* (\d+) filtered out;", text)
    require(len(rows) == 1 and rows[0][0] == "1" and rows[0][1] == "0",
            "B4-3 exact test count mismatch: " + target + "/" + test_name)
    return {target: (1, 0)}


def expected_argv(label):
    cargo = ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml"]
    if label == "runtime-build":
        return ["cargo", "+1.96.0", "build", "--locked", "--manifest-path", "core/rust/Cargo.toml",
                "-p", "xiao-runtime", "--release", "--lib"]
    if label == "runtime-identity":
        return ["python", "-B", "-X", "utf8", AUDIT_SCRIPT.replace("audit.py", "runtime-identity.py"),
                str(TASK_BASE / FINAL_VARIANT / "target" / "release" / "xiao_runtime.lib")]
    if label == "runtime-offline-probe":
        return ["python", "-B", "-X", "utf8", AUDIT_SCRIPT.replace("audit.py", "runtime-offline-probe.py")]
    if label == "native-matrix":
        return cargo + ["-p", "xiao-driver", "--release", "--test", "20o_b2_b4_cmath_llvm", "--",
                        "--ignored", "--exact", "native_cmath_static_dynamic_o0_o3_and_shared_pow", "--nocapture"]
    if label == "regressions":
        return cargo + ["-p", "xiao-driver"] + sum((['--test', name] for name in [
            "20o_b2_b4_cmath_llvm", "20o_b2_b4_cmath_vm", "20o_b2_b4_frontend",
            "20o_b2_b4_cmath_boundary", "20o_b2_b3_wrapper", "20o_b2_b2_divide",
            "20o_b2_a1_identity", "20o_b2_a_contract", "20o_b2_a_resources", "22c_vm_concurrency",
        ]), []) + ["--", "--nocapture"]
    if label == "core-tests":
        return ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml",
                "-p", "xiao-codegen-llvm", "-p", "xiao-runtime", "-p", "xiao-vm", "-p", "xiao-bytecode",
                "-p", "xiao-intrinsics", "--", "--nocapture"]
    if label in {"b1-vm", "b1-native"}:
        args = cargo + ["-p", "xiao-driver"] + (["--release"] if label == "b1-native" else []) + ["--test", "20o_b1_probe", "--"]
        name = "b1_capabilities_have_vm_golden_outputs" if label == "b1-vm" else "b1_capabilities_match_native_golden_outputs"
        flags = ["--exact", name, "--nocapture"]
        return args + (["--ignored"] if label == "b1-native" else []) + flags
    if label == "clippy":
        return ["cargo", "+1.96.0", "clippy", "--manifest-path", "core/rust/Cargo.toml", "--workspace",
                "--all-targets", "--locked", "--", "-D", "warnings"]
    if label == "fmt":
        return ["cargo", "+1.96.0", "fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"]
    if label in {"collector", "collector-final"}:
        return ["python", "-B", "-X", "utf8", SCRIPT, "content-store-self-test"]
    if label in {"final-docs", "final-docs-final", "final-docs-verified", "final-docs-closeout",
                 "final-docs-signed", "final-docs-finalized", "final-docs-productfix", "final-docs-release",
                 "final-docs-runtime"}:
        return ["bun", "tools/repo-check/src/cli.ts", "all", "--format", "json"]
    if label in HISTORY_AUDITS or label in PRELIMINARY_AUDITS:
        directory = (HISTORY_AUDITS | PRELIMINARY_AUDITS)[label]
        return ["python", "-B", "-X", "utf8", f"docs/DevDocs/assets/20o-b2-b/{directory}/audit.py"]
    if label == "entry":
        return ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p",
                "xiao-driver", "--test", "20o_b2_b4_cmath_vm", "--test", "20o_b2_b3_wrapper", "--test",
                "20o_b2_b4_frontend", "--test", "20o_b2_b4_cmath_boundary", "--test", "22c_vm_concurrency",
                "--", "--nocapture"]
    raise ValueError("B4-3 unknown execution label: " + label)


def expected_closeout(root, repository, records, logs, preliminary_records, live_runtime=False):
    source = load(root / FINAL_VARIANT / "source.json")
    environment = load(root / FINAL_VARIANT / "environment.json")
    runtime_path = Path(environment["overrides"]["XIAO_RUNTIME_LIBRARY"])
    runtime = records[FINAL_VARIANT, "runtime-build"]
    runtime_identity_record = records[FINAL_VARIANT, "runtime-identity"]
    require(runtime["exit_code"] == 0 and runtime["source_unchanged"], "B4-3 Runtime冷构建失败")
    require(runtime["finished_utc"] <= runtime_identity_record["started_utc"]
            and runtime_identity_record["exit_code"] == 0, "B4-3 Runtime identity must follow the cold build")
    runtime_identity = json.loads(logs[FINAL_VARIANT, "runtime-identity"])
    require(runtime_identity == {"path": str(runtime_path), "bytes": runtime_identity.get("bytes"),
                                 "sha256": runtime_identity.get("sha256")}
            and runtime_identity["bytes"] > 0 and re.fullmatch(r"[0-9a-f]{64}", runtime_identity["sha256"]),
            "B4-3 Runtime identity receipt is malformed")
    offline_probe = json.loads(logs[FINAL_VARIANT, "runtime-offline-probe"])
    require(offline_probe == {"default_without_live_runtime": True, "live_mode_rejected_missing_runtime": True,
                              "passed": True, "runtime_restored": True}
            and records[FINAL_VARIANT, "runtime-offline-probe"]["exit_code"] == 0,
            "B4-3 default/live Runtime audit boundary probe failed")
    if live_runtime:
        require(runtime_path.is_file() and not runtime_path.is_symlink(), "B4-3 task Runtime artifact missing")
        require(runtime_path.stat().st_size == runtime_identity["bytes"]
                and sha(runtime_path.read_bytes()) == runtime_identity["sha256"],
                "B4-3 live Runtime bytes differ from the captured identity")

    matrix = logs[FINAL_VARIANT, "native-matrix"]
    rows = re.findall(r"B4-3-MATRIX dynamic=(true|false) optimization=([03]).*cases=(\d+) error_cases=(\d+) pow_inputs=(\d+) bit_or_code=exact", matrix)
    require(len(rows) == 4 and {(row[0], row[1]) for row in rows} == {
        ("false", "0"), ("false", "3"), ("true", "0"), ("true", "3")},
        "B4-3 native O0/O3 matrix incomplete")
    require(all(row[2:] == ("18318", "3153", "1075") for row in rows), "B4-3 native matrix counts mismatch")
    require(len(re.findall(r"B4-3-STATUS statuses=0,1,2,3,6,8 semantic=009 structural=fatal result=passed", matrix)) == 4,
            "B4-3 status/error matrix incomplete")
    require("B4-3-STATUS wrong_tag=2 out=preserved pending=untouched fatal=boundary result=passed" in matrix
            and "B4-3-NATIVE language=actual-exit,catch A5=7-fixtures O0,O3 result=passed" in matrix,
            "B4-3 ownership or language error path missing")

    expected_counts = {
        "xiao_bytecode": (33, 0), "xiaoc_check": (0, 0), "b0_a_api": (1, 0), "b4_cmath_tac": (2, 0),
        "fuzz_xiaoc": (2, 0), "r2_tac": (27, 0), "xiao_codegen_llvm": (56, 0), "15e_ci_gated": (0, 5),
        "n0_a": (16, 7), "n0_b_dynamic": (27, 1), "n0_c_errors": (4, 2), "stdlib_bindings": (4, 0),
        "xiao_intrinsics": (8, 0), "xiao_runtime": (272, 0), "b06_runtime": (9, 0),
        "b1_string_bridge": (1, 0), "b2_a1_identity_allocations": (1, 0), "n0_c2_error_boundary": (1, 1),
        "r2a_containers": (17, 0), "xiao_vm": (35, 0), "b0_b_production": (9, 0),
        "b0_e_checkpoint_performance": (1, 0), "r2_iteration_values": (2, 0), "r2_selector_values": (7, 0),
        "r2_set_values": (6, 0), "r2_stack": (55, 0), "r2_table_values": (13, 0), "r2_vectors": (8, 0),
        "xiao_bytecode:doc": (0, 0), "xiao_codegen_llvm:doc": (0, 0), "xiao_intrinsics:doc": (0, 0),
        "xiao_runtime:doc": (4, 0), "xiao_vm:doc": (0, 0),
    }
    require(parse_target_counts(logs[FINAL_VARIANT, "core-tests"]) == expected_counts,
            "B4-3 core workspace test targets changed")
    require("B4-3-POW-DIFF rows=1075 same=1011 error_boundary=40 value_bits=24 reference_inputs=exact result=passed"
            in logs[FINAL_VARIANT, "regressions"], "B4-3 historical pow fixture not checked")
    regression_counts = parse_target_counts(logs[FINAL_VARIANT, "regressions"])
    expected_regressions = {
        "20o_b2_b4_cmath_llvm": (4, 1), "20o_b2_b4_cmath_vm": (7, 0), "20o_b2_b4_frontend": (3, 0),
        "20o_b2_b4_cmath_boundary": (3, 0), "20o_b2_b3_wrapper": (8, 0), "20o_b2_b2_divide": (2, 1),
        "20o_b2_a1_identity": (1, 1), "20o_b2_a_contract": (12, 1), "20o_b2_a_resources": (8, 0),
        "22c_vm_concurrency": (29, 0),
    }
    require(regression_counts == expected_regressions, "B4-3 cross-stage regression targets changed")
    require(len(re.findall(r"B3-WRAPPER-IMPORT .* executable=false result=planned-rejected", logs[FINAL_VARIANT, "regressions"])) == 30,
            "B4-3 planned import boundary count mismatch")
    require(exact_target_counts(logs[FINAL_VARIANT, "b1-vm"], "20o_b1_probe",
                                "b1_capabilities_have_vm_golden_outputs") == {"20o_b1_probe": (1, 0)},
            "B4-3 B1 VM exact entry failed")
    require(exact_target_counts(logs[FINAL_VARIANT, "b1-native"], "20o_b1_probe",
                                "b1_capabilities_match_native_golden_outputs") == {"20o_b1_probe": (1, 0)},
            "B4-3 B1 native exact entry failed")
    require("B2-B2-DIVIDE backend=native optimization=0,3 intrinsic=33 result=success" in logs[FINAL_VARIANT, "b1-native"],
            "B4-3 B2-B native entry missing")
    require(parse_target_counts(logs[FINAL_VARIANT, "collector-final"]) == {} and json.loads(logs[FINAL_VARIANT, "collector-final"]) == {
        "distinct_objects": 2, "docs_tree_static_bound": True, "identical_outputs_reused": True,
        "passed": True, "tampered_object_rejected": True}, "B4-3 content store self-test failed")
    for label in HISTORY_AUDITS:
        result = json.loads(logs[FINAL_VARIANT, label])
        require(historical_audit_passed(label, result), "B4-3 fixed historical audit failed: " + label)
    require(records[FINAL_VARIANT, "clippy"]["exit_code"] == 0 and records[FINAL_VARIANT, "fmt"]["exit_code"] == 0,
            "B4-3 engineering gates failed")
    docs = json.loads(logs[FINAL_VARIANT, "final-docs-runtime"])
    require(docs["passed"] is True and not any(row["severity"] == "error" for row in docs["diagnostics"]),
            "B4-3 final repository document gate failed")

    source_revision = git(repository, "rev-parse", CODE).decode("ascii").strip()
    require(source["revision"] == source_revision, "B4-3 final source revision mismatch")
    abi_text = git(repository, "show", source_revision + ":core/rust/crates/xiao-runtime-abi/src/lib.rs").decode("utf-8")
    codegen_text = git(repository, "show", source_revision + ":core/rust/crates/xiao-codegen-llvm/src/lib.rs").decode("utf-8")
    table = json.loads(git(repository, "show", source_revision + ":core/rust/crates/xiao-intrinsics/intrinsics.json"))["intrinsics"]
    registry = json.loads(git(repository, "show", source_revision + ":docs/module-registry.json"))
    cmath = next(row for row in registry["modules"] if row["id"] == "stdlib.cmath")
    require(re.search(r"ABI_MINOR_VERSION[^=]*=\s*18", abi_text)
            and re.search(r"CODEGEN_VERSION:\s*u32\s*=\s*21", codegen_text)
            and sorted(row["id"] for row in table if row["status"] == "active") == list(range(1, 49))
            and next(row for row in table if row["id"] == 100)["status"] == "tombstone"
            and cmath["status"] == "planned", "B4-3 frozen version or public module boundary changed")
    return {
        "passed": True,
        "stage": "B4-3",
        "b4_3_closed": True,
        "b4_closed": False,
        "language_available": False,
        "cmath": "planned",
        "runtime_abi": "1.18",
        "codegen_version": 21,
        "stdlib_interface": 1,
        "product_commit": source_revision,
        "source_files": len(source["files"]),
        "protected_product_paths_changed": sorted(EXPECTED_CODE_PATHS),
        "runtime": runtime_identity,
        "matrix": {"configurations": 4, "cases_per_configuration": 18318, "error_cases": 3153,
                   "pow_inputs_per_configuration": 1075, "historical_pow": {"same": 1011, "error_boundary": 40, "value_bits": 24}},
        "regressions": {"b1_vm": "passed", "b1_native": "passed", "b2_a_b_vm": "passed",
                        "b2_a_b_native": "passed", "b3_wrapper_hex_isolation": "passed", "historical_audits": len(HISTORY_AUDITS)},
        "platforms_executed": ["Windows x86_64-pc-windows-msvc"],
        "not_executed": ["Linux", "macOS", "remote CI", "B1 721 F10", "performance budget"],
        "runs": {variant + "/" + label: {"pid": row["pid"], "exit_code": row["exit_code"],
                  "log_sha256": row["log_sha256"], "docs_commit": row["docs_commit"]}
                 for (variant, label), row in records.items()},
        "excluded_preliminary_runs": {label: {"pid": row["pid"], "log_sha256": row["log_sha256"],
            "reason": PRELIMINARY_REASONS[label]}
            for label, row in preliminary_records.items()},
    }


def read_run(root, repository, variant, label):
    path = root / variant / (label + ".json")
    record = load(path)
    require(record["label"] == label and record["argv"] == expected_argv(label),
            NEGATIVE_REASONS["state"] + ": wrong command " + variant + "/" + label)
    environment = load(root / variant / "environment.json")
    source = load(root / variant / "source.json")
    require(record["cwd"] == source["source_root"] and record["pid"] > 0
            and record["started_utc"] < record["finished_utc"], "B4-3 run process identity mismatch")
    require(record["environment_sha256"] == sha((root / variant / "environment.json").read_bytes()),
            "B4-3 run environment hash mismatch")
    require(record["source_before"] == record["source_after"] == source["tree_sha256"]
            and record["source_unchanged"] is True, "B4-3 run source changed")
    commit = record["docs_commit"]
    require(record["docs_commit_after"] == commit and commit == git(repository, "rev-parse", commit).decode().strip(),
            "B4-3 DevDocs commit identity mismatch")
    docs = fixed_devdocs_tree(str(repository), commit)
    require(record["docs_tree_unchanged"] is True
            and record["docs_tree_before"] == record["docs_tree_after"] == record["docs_tree_sha256"]
            and record["docs_tree_canonical_before"] == record["docs_tree_canonical_after"] == docs["canonical_sha256"]
            and record["docs_tree_commit_raw_sha256"] == docs["raw_sha256"]
            and record["docs_tree_commit_canonical_sha256"] == docs["canonical_sha256"],
            NEGATIVE_REASONS["docs"] + ": " + variant + "/" + label)
    require(record["runner_sha256"] == sha(git(repository, "show", commit + ":" + SCRIPT)),
            "B4-3 runner Git blob identity mismatch")
    raw = (root / record["log_ref"]).read_bytes()
    require(record["log_bytes"] == len(raw) and record["log_sha256"] == sha(raw), "B4-3 log hash mismatch")
    expected_exit = {("fixed", "regressions"): 101, ("fixed-final", "native-matrix"): 101}.get((variant, label), 0)
    require(record["exit_code"] == expected_exit, NEGATIVE_REASONS["state"] + ": " + variant + "/" + label)
    return record, raw.decode("utf-8")


def read_preliminary(root, repository, source, environment, label):
    """保留并核验并行暂存污染的早期审计尝试，但不采纳其运行结论。"""
    record = load(root / FINAL_VARIANT / (label + ".json"))
    require(record["label"] == label and record["argv"] == expected_argv(label)
            and record["exit_code"] == 0 and record["source_unchanged"] is True
            and record["source_before"] == record["source_after"] == source["tree_sha256"],
            "B4-3 preliminary run identity mismatch: " + label)
    require(record["environment_sha256"] == sha((root / FINAL_VARIANT / "environment.json").read_bytes())
            and record["pid"] > 0 and record["started_utc"] < record["finished_utc"],
            "B4-3 preliminary environment/process mismatch: " + label)
    commit = record["docs_commit"]
    require(record["docs_commit_after"] == commit and commit == git(repository, "rev-parse", commit).decode().strip()
            and record["runner_sha256"] == sha(git(repository, "show", commit + ":" + SCRIPT)),
            "B4-3 preliminary runner identity mismatch: " + label)
    docs = fixed_devdocs_tree(str(repository), commit)
    stable_labels = {"audit-b4-2b", "final-docs", "final-docs-final", "final-docs-verified",
                     "final-docs-closeout", "final-docs-signed", "final-docs-finalized",
                     "final-docs-productfix", "final-docs-release", "final-docs-runtime"}
    stable = record["docs_tree_unchanged"] is True if label in stable_labels else record["docs_tree_unchanged"] is False
    require(stable and record["docs_tree_canonical_before"] == docs["canonical_sha256"]
            and record["docs_tree_commit_raw_sha256"] == docs["raw_sha256"]
            and record["docs_tree_commit_canonical_sha256"] == docs["canonical_sha256"]
            and (record["docs_tree_before"] == record["docs_tree_sha256"] and record["docs_tree_after"] != record["docs_tree_before"]
                 if label not in stable_labels else record["docs_tree_before"] == record["docs_tree_after"] == record["docs_tree_sha256"]),
            "B4-3 preliminary instability classification mismatch: " + label)
    raw = (root / record["log_ref"]).read_bytes()
    require(record["log_bytes"] == len(raw) and record["log_sha256"] == sha(raw),
            "B4-3 preliminary log identity mismatch: " + label)
    output = raw.decode("utf-8")
    result = json.loads(output)
    final_label = "audit-final-" + label.removeprefix("audit-") if label in PRELIMINARY_AUDITS else label
    accepted = historical_audit_passed(final_label, result) if label in PRELIMINARY_AUDITS else result.get("passed") is True
    require(accepted, "B4-3 preliminary command itself failed: " + label)
    return record, output


def verify(root, repository, live_runtime=False, preparing=False, check_inventory=True):
    before = all_entries(root)
    attributes = root / ".gitattributes"
    require(attributes.is_file() and attributes.read_bytes() == b"* -text -whitespace\n*.log -diff\n",
            NEGATIVE_REASONS["attributes"])
    if check_inventory:
        evidence = load(root / "evidence.json")
        require(evidence == {"files": inventory(root)}, NEGATIVE_REASONS["manifest"])
    sources, environments, records, logs, referenced = {}, {}, {}, {}, set()
    for variant, labels in RUN_LABELS.items():
        source = load(root / variant / "source.json")
        environment = load(root / variant / "environment.json")
        revision = source["revision"]
        require(revision == git(repository, "rev-parse", revision).decode().strip()
                and {path: row["canonical_sha256"] for path, row in source["files"].items()} == fixed_product_files(str(repository), revision)
                and source["tree_sha256"] == identity(source["files"]), NEGATIVE_REASONS["product"])
        task = PureWindowsPath(environment["task_root"])
        require(task == TASK_BASE / variant, NEGATIVE_REASONS["environment"])
        for key in ["CARGO_TARGET_DIR", "XIAO_RUNTIME_LIBRARY", "XIAO_HOME", "TEMP", "TMP"]:
            task_child(environment["overrides"][key], task)
        require(environment["source_root"] == source["source_root"] == r"D:\project\Xiao\Xiao"
                and PureWindowsPath(environment["overrides"]["XIAO_STDLIB_ROOT"]) == PureWindowsPath(source["source_root"]) / "stdlib"
                and environment["overrides"]["XIAO_B1_F10"] == environment["overrides"]["XIAO_B2_A_PERF"] == "0"
                and environment["removed_environment_prefixes"] == ["XIAO_", "RUST_BIGDECIMAL_"],
                NEGATIVE_REASONS["environment"])
        sources[variant], environments[variant] = source, environment
        require({path: row["canonical_sha256"] for path, row in source["files"].items()} ==
                fixed_product_files(str(repository), source["revision"]), NEGATIVE_REASONS["product"])
        expected_files = labels | (PRELIMINARY_LABELS if variant == FINAL_VARIANT else set())
        require({label for path in (root / variant).glob("*.json")
                 if (label := path.stem) not in {"source", "environment"}} == expected_files,
                NEGATIVE_REASONS["state"] + ": execution set changed " + variant)
        for label in sorted(labels):
            record, text = read_run(root, repository, variant, label)
            key = (variant, label)
            records[key], logs[key] = record, text
            referenced.add(record["log_sha256"])

    preliminary_records = {}
    for label in sorted(PRELIMINARY_LABELS):
        record, _ = read_preliminary(root, repository, sources[FINAL_VARIANT], environments[FINAL_VARIANT], label)
        preliminary_records[label] = record
        referenced.add(record["log_sha256"])

    changed = set(git(repository, "diff", "--name-only", sources["baseline"]["revision"], CODE,
                      "--", *PRODUCT_ROOTS).decode().splitlines())
    require(changed == EXPECTED_CODE_PATHS, "B4-3 product path set changed")
    require(sources["baseline"]["revision"] == git(repository, "rev-parse", BASELINE).decode().strip()
            and sources["fixed"]["revision"] == git(repository, "rev-parse", FIRST_FIXED).decode().strip()
            and sources["fixed-final"]["revision"] == git(repository, "rev-parse", SECOND_FIXED).decode().strip()
            and sources[FINAL_VARIANT]["revision"] == git(repository, "rev-parse", CODE).decode().strip(),
            NEGATIVE_REASONS["product"])

    objects = {}
    for path in sorted((root / LOG_DIR).rglob("*.log")):
        digest = sha(path.read_bytes())
        relative = path.relative_to(root).as_posix()
        require(relative == f"logs/sha256/{digest[:2]}/{digest}.log" and digest not in objects,
                "B4-3 content object path or uniqueness mismatch")
        objects[digest] = relative
    require(referenced == set(objects), NEGATIVE_REASONS["orphan"])
    require("FAILED" in logs["fixed", "regressions"] and "existing_hex_import_keeps_native_lowering" in logs["fixed", "regressions"]
            and "unimported_module_cannot_pull_cmath_into_hex" in logs["fixed", "regressions"],
            "B4-3 retained isolation failure record missing")
    require("undefined value '@xiao_runtime_cmath_abs'" in logs["fixed-final", "native-matrix"],
            "B4-3 retained missing-declaration failure record missing")
    require("passed" in logs["fixed", "native-matrix"] and "cases=18318" in logs["fixed", "native-matrix"],
            "B4-3 first matrix record missing")
    result = expected_closeout(root, repository, records, logs, preliminary_records, live_runtime)
    closeout_path = root / "closeout.json"
    require(preparing or closeout_path.is_file(), "B4-3 closeout missing")
    if closeout_path.exists():
        require(load(closeout_path) == result, NEGATIVE_REASONS["state"])
    negative_path = root / "self-negative-findings.json"
    require(preparing or negative_path.is_file(), "B4-3 self-negative report missing")
    if negative_path.exists():
        expected = [{"case": "baseline", "exit_code": 0, "reason": None, "read_only": True}]
        expected.extend({"case": key, "exit_code": 1, "reason": negative_reason(key), "read_only": True}
                        for key, reason in NEGATIVE_REASONS.items())
        require(load(negative_path) == {"passed": True, "cases": expected}, "B4-3 self-negative report mismatch")
    require(before == all_entries(root), "B4-3 audit wrote to evidence tree")
    return result


def all_entries(root):
    return {path.relative_to(root).as_posix() + ("/" if path.is_dir() else ""):
            None if path.is_dir() else sha(path.read_bytes()) for path in root.rglob("*")}


def negative_cases(root, repository):
    with tempfile.TemporaryDirectory(prefix="xiao-b4-3-negative-baseline-") as temporary:
        copied = Path(temporary) / "entry"
        shutil.copytree(root, copied)
        before = all_entries(copied)
        baseline = verify(copied, repository, preparing=True)
        require(before == all_entries(copied) and baseline["passed"] is True,
                "B4-3 self-negative baseline failed or wrote evidence")
    cases = [{"case": "baseline", "exit_code": 0, "reason": None, "read_only": True}]
    for name, reason in NEGATIVE_REASONS.items():
        with tempfile.TemporaryDirectory(prefix="xiao-b4-3-negative-") as temporary:
            copied = Path(temporary) / "entry"
            shutil.copytree(root, copied)
            if name == "manifest":
                (copied / "evidence.json").write_text('{"files":{}}', encoding="utf-8")
            elif name == "attributes":
                (copied / ".gitattributes").unlink()
            elif name == "orphan":
                raw = b"unreferenced B4-3 object\n"
                digest = sha(raw)
                path = copied / f"logs/sha256/{digest[:2]}/{digest}.log"
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)
                (copied / "evidence.json").write_text(json.dumps({"files": inventory(copied)}), encoding="utf-8")
            elif name == "product":
                source = load(copied / f"{FINAL_VARIANT}/source.json")
                source["files"]["core/rust/Cargo.lock"]["canonical_sha256"] = "0" * 64
                (copied / f"{FINAL_VARIANT}/source.json").write_text(json.dumps(source), encoding="utf-8")
                (copied / "evidence.json").write_text(json.dumps({"files": inventory(copied)}), encoding="utf-8")
            elif name == "docs":
                receipt = load(copied / f"{FINAL_VARIANT}/native-matrix.json")
                receipt["docs_tree_sha256"] = "0" * 64
                (copied / f"{FINAL_VARIANT}/native-matrix.json").write_text(json.dumps(receipt), encoding="utf-8")
                (copied / "evidence.json").write_text(json.dumps({"files": inventory(copied)}), encoding="utf-8")
            elif name == "environment":
                environment = load(copied / f"{FINAL_VARIANT}/environment.json")
                environment["overrides"]["CARGO_TARGET_DIR"] = str(TASK_BASE / "escaped")
                (copied / f"{FINAL_VARIANT}/environment.json").write_text(json.dumps(environment), encoding="utf-8")
                (copied / "evidence.json").write_text(json.dumps({"files": inventory(copied)}), encoding="utf-8")
            elif name == "state":
                closeout = load(copied / "closeout.json")
                closeout["language_available"] = True
                (copied / "closeout.json").write_text(json.dumps(closeout), encoding="utf-8")
                (copied / "evidence.json").write_text(json.dumps({"files": inventory(copied)}), encoding="utf-8")
            before = all_entries(copied)
            try:
                verify(copied, repository, preparing=True)
            except ValueError as error:
                actual_reason = str(error)
            else:
                raise ValueError("B4-3 negative case unexpectedly passed: " + name)
            require(before == all_entries(copied), "B4-3 negative audit wrote to copied evidence")
            require(actual_reason == negative_reason(name), "B4-3 negative case rejected for wrong reason: " + name + ": " + actual_reason)
            cases.append({"case": name, "exit_code": 1, "reason": negative_reason(name), "read_only": True})
    return {"passed": True, "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--collect", action="store_true")
    parser.add_argument("--refresh-inventory", action="store_true")
    parser.add_argument("--live-runtime", action="store_true")
    parser.add_argument("--self-negative", action="store_true")
    parser.add_argument("--prepare", action="store_true", help="仅供自反例基线核验时允许尚无报告")
    parser.add_argument("--write-negative", type=Path)
    args = parser.parse_args()
    require(sum([args.collect, args.refresh_inventory, args.self_negative]) <= 1,
            "B4-3 generation and read-only modes cannot be mixed")
    require(not args.write_negative or args.self_negative, "B4-3 negative report requires self-negative mode")
    if args.collect:
        require(not (args.root / "closeout.json").exists() and not (args.root / "evidence.json").exists(),
                "B4-3 collect refuses to overwrite existing closeout")
        rows = {}
        for variant, labels in RUN_LABELS.items():
            for label in labels:
                record, _ = read_run(args.root, args.repository_root, variant, label)
                rows[variant, label] = record
        logs = {(variant, label): read_run(args.root, args.repository_root, variant, label)[1]
                for variant, labels in RUN_LABELS.items() for label in labels}
        final_source = load(args.root / f"{FINAL_VARIANT}/source.json")
        final_environment = load(args.root / f"{FINAL_VARIANT}/environment.json")
        preliminary = {label: read_preliminary(args.root, args.repository_root, final_source,
            final_environment, label)[0] for label in PRELIMINARY_LABELS}
        write_json(args.root / "closeout.json",
                   expected_closeout(args.root, args.repository_root, rows, logs, preliminary, True))
        write_json(args.root / "evidence.json", {"files": inventory(args.root)})
    elif args.refresh_inventory:
        require(load(args.root / "closeout.json") == verify(args.root, args.repository_root, args.live_runtime,
                                                            preparing=True, check_inventory=False),
                "B4-3 inventory refresh cannot change closeout")
        temporary = args.root / "evidence.json.tmp"
        write_json(temporary, {"files": inventory(args.root)})
        temporary.replace(args.root / "evidence.json")
    if args.self_negative:
        result = negative_cases(args.root, args.repository_root)
        if args.write_negative:
            write_json(args.write_negative, result)
    else:
        verify(args.root, args.repository_root, args.live_runtime,
               preparing=args.collect or args.prepare or args.refresh_inventory)
        result = {"passed": True, "files": len(load(args.root / "evidence.json")["files"]),
                  "runs": sum(map(len, RUN_LABELS.values())), "b4_3_closed": True,
                  "b4_closed": False, "language_available": False,
                  "self_negative_checked": (args.root / "self-negative-findings.json").exists(),
                  "live_runtime_checked": args.live_runtime}
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error))
        raise SystemExit(1)
