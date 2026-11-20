"""只读复核固定 Git 对象和实跑回执，派生 B6-5 与 B7 入口状态。"""

import argparse
from collections import Counter
from datetime import datetime
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
from zipfile import ZipFile
import sys

sys.dont_write_bytecode = True
from record import (
    BASE, BOUNDARY, COMPONENT_DELTA, DOC_INPUTS, DRIVER, HISTORY, LOCAL_CHECKS, LLVM, PERFORMANCE, PREFIX,
    PRODUCT, ROOT, blob, digest, entries, git, git_blob_id, git_tree, history_identity, json_bytes, product_record,
    recorded_argv, require, text_identity,
)
from evidence import Evidence

TARGET = "x86_64-unknown-linux-gnu"
DOC_GATE = "repo-check-final"
CARGO = ("cargo", "+1.96.0")
MANIFEST = ("--manifest-path", "core/rust/Cargo.toml")
TEST = (*CARGO, "test", *MANIFEST, "--locked")
EXPORTS = "artifact::link_tests::runtime_components_match_release_archive_exports"
NATIVE = {
    "exports": (("-p", "xiao-codegen-llvm", "--lib"), EXPORTS),
    "cmath": (("-p", "xiao-driver", "--test", "20o_b2_b4_cmath_library"), "cmath_library_matches_native_o0_o3"),
    "divide": (("-p", "xiao-driver", "--test", "20o_b2_b2_divide"), "lfloat_divide_matches_native_golden_outputs"),
    "float": (("-p", "xiao-driver", "--test", "20o_static_float_equality"),
              "native_static_dynamic_comparisons_match_vm_o0_o3"),
    "callback": (("-p", "xiao-driver", "--test", "n0_b_dynamic_native"),
                 "optional_table_callback_ir_validates_three_targets"),
    "dynamic-string": (("-p", "xiao-driver", "--test", "n0_b_dynamic_native"),
                       "optional_dynamic_string_native_round_trip"),
}
COMMANDS = {
    "runtime": (*CARGO, "build", *MANIFEST, "--locked", "--release", "-p", "xiao-runtime"),
    "workspace": (*TEST, "--workspace"),
    "clippy": (*CARGO, "clippy", *MANIFEST, "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"),
    "fmt": (*CARGO, "fmt", *MANIFEST, "--all", "--", "--check"),
    "check-lock": ("bun", "run", "check:lock"),
    **{name: (*TEST, *args, "--", "--ignored", "--exact", case, "--nocapture")
       for name, (args, case) in NATIVE.items()},
}
COUNTS = re.compile(r"^test result: (?:ok\.|FAILED\.) (\d+) passed; (\d+) failed; (\d+) ignored;", re.M)
ACCEPTANCE = (
    "b6_1_stage_passed", "b6_2_perf_pass", "b6_2_final_tree_covered", "b6_3_boundary_passed",
    "b6_3_final_tree_covered", "b6_4_passed",
)
CLOSEOUT = (
    "evidence_inventory_complete", "stage_reconciliation_complete", "final_workspace_gates_passed",
    "status_fields_consistent", "open_items_assigned",
)
B6_4_FIELDS = ("b6_4_passed", "original_passed", "followup_passed", "windows_coff_verified", "22d_open")
RELEASE_CASES = ["native-lock-wait-027", "xcon-bounded-string-success", "spawn-join-str-multiargument"]


def derive(flags):
    require(all(type(flags[name]) is bool for name in ACCEPTANCE + CLOSEOUT), "状态输入须使用布尔值")
    accepted = all(flags[name] for name in ACCEPTANCE)
    closed = all(flags[name] for name in CLOSEOUT)
    return {
        "b6_accepted": accepted, "b6_5_passed": closed,
        "b7_entry_ready": closed and flags["open_items_assigned"],
    }


def check_identity(identity):
    require(identity["bytes"] > 0 and re.fullmatch("[a-f0-9]{64}", identity["sha256"]),
            "文件身份无效")


def receipt(bundle, folder, name):
    row = bundle.read(f"{folder}/{name}.json")
    started = datetime.fromisoformat(row["started_utc"])
    finished = datetime.fromisoformat(row["finished_utc"])
    outputs = {stream: bundle.raw(f"{folder}/{name}.{stream}") for stream in ("stdout", "stderr")}
    require(row["pid"] > 0 and started.utcoffset() is not None and started <= finished
            and row["outputs"] == {stream: digest(raw) for stream, raw in outputs.items()},
            f"命令回执或原始输出身份不符：{folder}/{name}")
    return row, *(raw.decode("utf-8") for raw in outputs.values())


def verify_history(bundle):
    history = history_identity()
    require(bundle.read("identity/history.json") == history, "历史证据固定 Git 身份不符")
    reports = {}
    for name in HISTORY:
        row, stdout, stderr = receipt(bundle, "stage-audits", name)
        script, args = recorded_argv(name)
        cwd = PureWindowsPath(row["cwd"])
        captured = PureWindowsPath(row["argv"][4])
        require(cwd.is_absolute() and captured == cwd / script
                and row["argv"][1:4] == ["-B", "-X", "utf8"] and row["argv"][5:] == args
                and row["exit_code"] == 0 and not stderr.strip(), f"历史只读入口回执不符：{name}")
        fixed = blob(PRODUCT, script)
        require(row["script"] in (digest(fixed), digest(fixed.replace(b"\n", b"\r\n"))),
                f"历史只读入口字节不符：{name}")
        reports[name] = json.loads(stdout)

    require(reports["b5"]["b5_closed"] and reports["b5"]["codegen_version"] == 22, "B5 收口未对账")
    b5 = json.loads(blob(PRODUCT, f"{history['b5']['directory']}/closeout.json"))["components"]
    require(b5["b5_historical_matrix"]["codegen_version"] == 21, "B5 历史矩阵身份不符")
    rerun, macos = reports["b6-1"], reports["b6-1-macos"]
    b6_1_path = history["b6-1"]["directory"]
    original = json.loads(blob(PRODUCT, f"{b6_1_path}/result.json"))
    clippy = json.loads(blob(PRODUCT, f"{b6_1_path}/linux-tool-availability.json"))
    require(rerun["passed"] and rerun["read_only"] and rerun["rerun_closed"]
            and rerun["b6_1_status"] == "restricted" and rerun["stage_passed"] is original["stage_passed"] is False
            and all(original["platforms"][os]["functional_status"] == "passed" for os in ("windows", "linux"))
            and original["platforms"]["windows"]["artifact_verification"] == "UnverifiedCoffExports"
            and original["platforms"]["linux"]["artifact_verification"] == "Complete"
            and clippy["clippy_available"] is False and macos["passed"] and macos["passing_jobs"] == 2,
            "B6-1 受限及 macOS 专项证据不符")
    perf, reviewed = reports["b6-2"], reports["b6-2-source"]
    require(perf["verified"] and perf["matrix_complete"] and perf["perf_pass"] is True
            and perf["successful_processes"] == 8235
            and len(perf["coverage"]) == 9
            and {(row["platform"], row["backend"]) for row in perf["coverage"].values()}
            == {(platform, backend) for platform in ("windows", "linux", "macos") for backend in ("vm", "o0", "o3")}
            and all(row["counts"] == {"passed": 183} and row["source_commit"] == PERFORMANCE
                    and row["url"]["maximum"]["status"] == "passed" for row in perf["coverage"].values())
            and reviewed["measured_source"] == PERFORMANCE and reviewed["reviewed_source"] == BOUNDARY
            and reviewed["performance_semantics_preserved_at_source_level"] is True
            and reviewed["new_performance_samples"] == 0 and reports["b6-2-review"]["archive_read_only"],
            "B6-2 性能及源码差异证据不符")
    boundary = reports["b6-3"]
    require(boundary["verified"] and boundary["boundary_passed"] is True
            and boundary["source_commit"] == BOUNDARY and boundary["language_cases"] == 85
            and boundary["language_runs"] == 765 and boundary["windows_export_integrity"] == "unverifiable"
            and boundary["guard_replay"] == {"passed": 28, "actual": 4, "derived": 24}
            and reports["b6-3-review"]["archive_read_only"]
            and set(boundary["platforms"]) == {"windows", "linux", "macos"}
            and all(row["c_abi"]["passed"] == 11 for row in boundary["platforms"].values()),
            "B6-3 历史边界证据不符")

    before, followup = reports["b6-4"], reports["b6-4-followup"]
    fixed = json.loads(blob(PRODUCT, f"{history['b6-4-followup']['directory']}/status.json"))
    flags = {name: followup[name] for name in B6_4_FIELDS}
    local = followup["local_checks"]
    require(before["verified"] and followup["verified"] and flags == fixed
            and before["b6_4_passed"] is flags["b6_4_passed"] is False
            and flags == dict(zip(B6_4_FIELDS, (False, False, False, False, True)))
            and followup["fix_runner"]["Windows"]["verification"] == "unverified-coff-exports"
            and local["commit"] == LOCAL_CHECKS and local["linux_abi_exports"]["exports"] == 164
            and local["windows_clippy"] == {"target": "x86_64-pc-windows-msvc", "package": "xiao-codegen-llvm",
                                            "all_targets": True}
            and before["native_maintenance"]["windows"]["case"] == RELEASE_CASES[0]
            and followup["local_windows_full"]["failed_cases"] == RELEASE_CASES[1:]
            and reports["b6-4-review"]["status_matrix"]["checked"] == 128,
            "B6-4 正式状态、平台边界或 22D 失败不符")
    return reports, flags


def verify_linux(bundle, source):
    environment = bundle.read("linux/environment.json")
    task = PurePosixPath(environment["task"])
    checkout = str(task / "source")
    runtime_path = str(task / "target/release/libxiao_runtime.a")
    settings = environment["environment"]
    require(task.is_absolute() and environment["platform"].startswith("Linux-")
            and environment["target"] == TARGET and environment["source"] == checkout
            and environment["source_identity"] == source and environment["target_existed_before"] is False
            and settings["RUSTUP_TOOLCHAIN"] == "1.96.0" and settings["XIAO_TARGET_TRIPLE"] == TARGET
            and settings["CARGO_TARGET_DIR"] == str(task / "target")
            and settings["XIAO_RUNTIME_LIBRARY"] == runtime_path and settings["XIAO_HOME"] == str(task / "home")
            and settings["BUN_INSTALL_CACHE_DIR"] == str(task / "cache/bun")
            and settings["XIAO_STDLIB_ROOT"] == str(task / "source/stdlib")
            and not ({"XIAO_B1_F10", "XIAO_B2_A_PERF"} & settings.keys()), "Linux 独立任务身份不符")
    tools = environment["tools"]
    for item in tools.values():
        check_identity(item)
    require(tools["rustc"]["version"].startswith("rustc 1.96.0 ")
            and f"host: {TARGET}" in tools["rustc"]["version"].splitlines()
            and tools["clippy"]["version"].startswith("clippy 0.1.96 ")
            and all("21.1.8" in tools[key]["version"] and settings[key] == tools[key]["path"]
                    for key in ("XIAO_CLANG", "XIAO_LLVM_AS", "XIAO_LLVM_NM", "XIAO_LLC", "XIAO_STRIP"))
            and tools["bun"]["version"] == "1.4.2", "最终工具链身份不符")

    runtime = bundle.read("linux/runtime.json")["runtime_after"]
    check_identity(runtime)
    results, output = {}, {}
    names = (*COMMANDS, "cargo-control", "rust-api", "repo-check", DOC_GATE)
    for name in names:
        row, stdout, stderr = receipt(bundle, "linux", name)
        require(row["cwd"] == checkout and row["temp"] == str(task / "tmp" / name)
                and row["source_before"] == row["source_after"] == source
                and row["runtime_after"] == runtime
                and row["runtime_before"] == (None if name == "runtime" else runtime),
                f"Linux 进程源、Runtime 或临时目录不符：{name}")
        if name in COMMANDS:
            require(tuple(row["argv"]) == COMMANDS[name], f"门禁或补验命令不符：{name}")
        results[name] = row["exit_code"]
        output[name] = (row, stdout, stderr)

    totals = [tuple(map(int, match)) for match in COUNTS.findall(output["workspace"][1])]
    counts = dict(zip(("passed", "failed", "ignored"), map(sum, zip(*totals))))
    require(len(totals) == 192 and counts == {"passed": 1760, "failed": 0, "ignored": 56},
            "默认 workspace 原始测试计数不符")
    require(output["cargo-control"][0]["argv"][1:] == [f"{checkout}/tools/gates/controlled-cargo.py", "--install"]
            and output["check-lock"][2].splitlines().count("F7-CARGO command=check locked=true removed=0") == 2,
            "两个 check:lock Cargo 进程未实际锁定")
    smoke = tuple(NATIVE)
    for name, (_, case) in NATIVE.items():
        stdout = output[name][1]
        require(COUNTS.findall(stdout) == [("1", "0", "0")] and f"test {case} ... ok" in stdout,
                f"定向补验未精确执行一个目标：{name}")
    require(f"RUNTIME-COMPONENTS exports=164 target={TARGET} result=passed" in output["exports"][1],
            "Linux 导出对账输出不符")
    cmath = output["cmath"][1]
    require(set(re.findall(r"B4-4-NATIVE path=(std(?:lib)?\.cmath) optimization=([03]) .*"
                           r"verification=Complete .* stdout=golden errors=009 result=passed", cmath))
            == {(alias, level) for alias in ("std.cmath", "stdlib.cmath") for level in ("0", "3")},
            "Linux cmath 原生矩阵不完整")
    require("backend=native optimization=0,3 intrinsic=33 result=success" in output["divide"][1],
            "Linux 显式除法 O0/O3 结果缺失")
    float_log = output["float"][1]
    require("FLOAT-EQUALITY cases=62 native=248 llvm_as=248 vm=62 result=passed" in float_log
            and set(re.findall(r"FLOAT-EQUALITY-COMPONENTS (original-pow-(?:eq|ne)-static-o[03]) "
                               r'verification=complete declared=\["rc", "value"\] observed=\["rc", "value"\]', float_log))
            == {f"original-pow-{op}-static-o{level}" for op in ("eq", "ne") for level in ("0", "3")},
            "Linux 浮点比较组件矩阵不符")
    verify_rust_api(bundle, task, output["rust-api"])
    docs = verify_docs(bundle, tools, output["repo-check"], output[DOC_GATE])
    require(all(results[name] == 0 for name in ("runtime", "cargo-control", "rust-api", *smoke)),
            "冷构建或定向补验有未通过项")
    return {
        "target": TARGET, "runtime": runtime, "workspace": counts | {"targets": len(totals)},
        "gates": {name: results[name] for name in ("workspace", "clippy", "fmt", "check-lock")}
        | {"repo-check": results[DOC_GATE]},
        "repo_check": docs, "point_checks": {"linux_abi_exports": 164, "rust_api_cases": 8, "native": list(smoke)},
    }


def verify_rust_api(bundle, task, result):
    row, stdout, _ = result
    inputs = bundle.read("linux/rust-api-inputs.json")
    recipe = HISTORY["b6-2-source"][0] + "/verify_boundary.py"
    source = f"{BASE}/{recipe}"
    check_identity(inputs["core"])
    require(inputs["recipe"] == digest(blob(PRODUCT, source)) and inputs["home"] == str(task / "home-language")
            and inputs["core"]["path"] == str(task / "target/debug/xiao-core")
            and row["argv"][1:3] == ["-B", "-c"]
            and row["argv"][-5:] == [str(task / "source" / source), inputs["core"]["path"],
                                     str(task / "source"), inputs["home"], TARGET],
            "Rust 宿主 API 语言探针身份不符")
    cases = json.loads(stdout)["cases"]
    expected = {f"{alias}-abs": None for alias in ("std", "stdlib")}
    for name in ("standard_library_module_loader", "with_module_loader"):
        expected[f"bare-{name}"] = "X02-TYPE-001"
        for alias in ("std", "stdlib"):
            expected[f"{alias}-{name}"] = "X05-MODULE-005"
    require(len(cases) == len(expected) and {case["case"] for case in cases} == expected.keys(),
            "Rust 宿主 API 语言探针缺失")
    for case in cases:
        diagnostic = expected[case["case"]]
        require(case["expected_diagnostic"] == diagnostic
                and case["exit_code"] == int(diagnostic is not None)
                and [error["code"] for error in case["errors"]] == ([diagnostic] if diagnostic else [])
                and case["output"] == ("" if diagnostic else "2\n"), "Rust 宿主 API 语言探针输出不符")


def verify_docs(bundle, tools, initial, result):
    failed_row, failed_stdout, _ = initial
    failed_docs = bundle.read("linux/docs-source.json")
    failed = json.loads(failed_stdout)
    errors = {item["code"]: item["message"] for item in failed["diagnostics"] if item["severity"] == "error"}
    require(failed_row["exit_code"] == 1 and failed["passed"] is False
            and failed_docs["before"] == failed_docs["after"]
            and failed_docs["bun"] == tools["bun"]
            and errors.keys() == {"A0-WORKSPACE-003", "A0-PARSER-001"}
            and all('Executable not found in $PATH: "cargo"' in reason for reason in errors.values()),
            "仓库门禁环境失败回执不符")
    row, stdout, stderr = result
    capture = bundle.read("linux/docs-source-final.json")
    selected_tools = {name: tools[name] for name in ("bun", "cargo")}
    require(capture["before"] == capture["after"] and set(capture["before"]["files"]) == set(DOC_INPUTS)
            and capture["tools"] == selected_tools
            and capture["path_prefix"] == [str(PurePosixPath(tool["path"]).parent)
                                          for tool in selected_tools.values()]
            and row["argv"] == failed_row["argv"]
            == [tools["bun"]["path"], "tools/repo-check/src/cli.ts", "all", "--format", "json"],
            "定稿 DevDocs 输入、前后摘要或命令不符")
    markdown = {path: object_id for path, object_id in entries(git_tree(PRODUCT, "docs/DevDocs")).items()
                if path.endswith(".md")}
    for path, identity in capture["before"]["files"].items():
        object_id = identity["blob"]
        require(re.fullmatch("[0-9a-f]{40}", object_id), "定稿 DevDocs Git blob 无效")
        fixed = git("cat-file", "blob", object_id)
        # Windows 导出的定稿文件可带 CRLF。整树摘要仍使用门禁实读字节。
        materialized = fixed if digest(fixed) == identity["raw"] else fixed.replace(b"\n", b"\r\n")
        require(identity == text_identity(materialized), f"定稿 DevDocs 字节未绑定 Git 对象：{path}")
        if path.endswith(".md"):
            markdown[path] = git_blob_id(materialized)
    require(capture["before"]["markdown"] == {"files": len(markdown), **digest(json_bytes(markdown))},
            "定稿 DevDocs Markdown 树清单不符")
    report = json.loads(stdout)
    warnings = Counter(item["code"] for item in report["diagnostics"] if item["severity"] == "warning")
    require(report["checker"] == "xiao-repo-check" and report["passed"] == (row["exit_code"] == 0)
            and not stderr and not (warnings.keys() - {"A0-COVERAGE-002"}), "仓库门禁原始报告不符")
    return {"markdown": capture["before"]["markdown"], "diagnostics": dict(warnings),
            "initial_environment_failure": failed_row["exit_code"]}


def compat():
    crates = "core/rust/crates"
    fields = {
        "ABI_MAJOR_VERSION": f"{crates}/xiao-runtime-abi/src/lib.rs",
        "ABI_MINOR_VERSION": f"{crates}/xiao-runtime-abi/src/lib.rs",
        "CODEGEN_VERSION": f"{LLVM}/src/lib.rs",
        "STDLIB_INTERFACE_VERSION": f"{crates}/xiao-standard-library/src/lib.rs",
    }
    values = [int(re.search(rf"const {label}: u32 = (\d+);", blob(PRODUCT, path).decode()).group(1))
              for label, path in fields.items()]
    cmath = next(item for item in json.loads(blob(PRODUCT, "docs/module-registry.json"))["modules"]
                 if item["id"] == "stdlib.cmath")
    table = json.loads(blob(PRODUCT, f"{crates}/xiao-intrinsics/intrinsics.json"))["intrinsics"]
    ids = sorted((item["id"], item["status"]) for item in table)
    require(values == [1, 19, 22, 1] and cmath["status"] == "planned"
            and ids == [(ident, "active") for ident in range(1, 49)] + [(100, "tombstone")],
            "版本、intrinsic 或 cmath planned 边界不符")
    return {"abi": "1.19", "generator": 22, "stdlib_interface": 1, "cmath": cmath["status"],
            "active_intrinsics": [1, 48], "tombstone": 100}


def verify_handoff(bundle, handoff):
    observed = json.loads(handoff.read_bytes())
    captured = bundle.read("linux/docs-source-final.json")["before"]["files"][f"{PREFIX}/handoff.json"]
    require(text_identity(handoff.read_bytes())["utf8_lf"] == captured["utf8_lf"], "交接清单与定稿门禁输入不符")
    require(observed["b7_started"] is observed["b2_c_started"] is False
            and observed["b7"] == {"b1_entries": 20, "b1_resources": 18, "b1_guards": 28,
                                   "stage_reconciliation": ["A0-A7", "B0-B6"], "cmath": "planned"}
            and observed["b2_c"] == {"new_snapshot_f10": 721, "public_artifacts": ["windows", "linux", "macos"],
                                     "final_tutorial_and_registry": "pending", "b6_numeric_workloads": "pending"}
            and observed["open_items"] == {
                "22d": {"owner": "B7", "cases": RELEASE_CASES},
                "windows_coff": {"owner": "B2-C", "verification": "UnverifiedCoffExports"},
                "b6_1_linux_clippy": {"owner": "B7-4", "historical": "not_run"},
                "final_tree_artifact_verification": {"owner": "B2-C", "pending": ["windows", "macos"]},
            }, "交接范围或未完成项归属不符")
    return observed


def inspect(archive, handoff):
    product = product_record()
    with ZipFile(archive) as zipped:
        bundle = Evidence(zipped)
        expected = {f"identity/{name}.json" for name in ("history", "product", "recipes")}
        expected.update(f"linux/{name}.json"
                        for name in ("environment", "rust-api-inputs", "docs-source", "docs-source-final"))
        for folder, names in (("stage-audits", HISTORY),
                              ("linux", (*COMMANDS, "cargo-control", "rust-api", "repo-check", DOC_GATE))):
            expected.update(f"{folder}/{name}.{ext}" for name in names for ext in ("json", "stdout", "stderr"))
        require(bundle.entries.keys() == expected, "归档逻辑路径清单不符")
        require(bundle.read("identity/product.json") == product, "最终产品固定 Git 身份不符")
        reports, flags = verify_history(bundle)
        linux = verify_linux(bundle, product["source"])
        handoff_data = verify_handoff(bundle, handoff)
        archive_info = digest(archive.read_bytes()) | {"files": len(bundle.entries), "objects": bundle.unique}
    measured, boundary = (product["changes"][rev] for rev in (PERFORMANCE, BOUNDARY))
    require(set(boundary) == COMPONENT_DELTA and set(measured) >= set(boundary),
            "Runtime 组件受影响差异与交接范围不符")
    native = f"{DRIVER}/examples/numeric_perf/native.rs"
    lowering = blob(PRODUCT, native)
    perf_covered = (
        reports["b6-2-source"]["performance_semantics_preserved_at_source_level"] is True
        and set(measured) - set(boundary) == set(reports["b6-2-source"]["changed_sources"])
        | {"tools/gates/b2-b6-performance-baseline.json"}
        and lowering == blob(PERFORMANCE, native) and b".lower(&artifact.ir" in lowering
        and b"module.text" in lowering and b"runtime_components" not in lowering
    )
    pending_platforms = handoff_data["open_items"]["final_tree_artifact_verification"]["pending"]
    state = flags | {
        "b6_1_stage_passed": reports["b6-1"]["stage_passed"],
        "b6_2_perf_pass": reports["b6-2"]["perf_pass"],
        "b6_2_final_tree_covered": perf_covered,
        "b6_3_boundary_passed": reports["b6-3"]["boundary_passed"],
        "b6_3_final_tree_covered": not pending_platforms,
        "evidence_inventory_complete": True,
        "stage_reconciliation_complete": True,
        "status_fields_consistent": True,
        "final_workspace_gates_passed": all(exit_code == 0 for exit_code in linux["gates"].values()),
        "open_items_assigned": bool(handoff_data["open_items"]),
    }
    return state | derive(state) | {
        "product": product["source"]["commit"], "tree": product["tree"], "compatibility": compat(),
        "b6_2_measured_source": PERFORMANCE, "b6_3_boundary_source": BOUNDARY, "new_performance_samples": 0,
        "historical_replays": len(HISTORY), "linux": linux, "archive": archive_info,
        "final_tree_artifact_verification_pending": pending_platforms,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, default=ROOT / "closeout.zip")
    parser.add_argument("--status", type=Path, default=ROOT / "status.json")
    parser.add_argument("--derive", action="store_true", help="输出派生状态，供封存时落盘")
    args = parser.parse_args()
    try:
        result = inspect(args.archive, ROOT / "handoff.json")
        if not args.derive:
            require(result == json.loads(args.status.read_bytes()), "封存状态与固定证据派生结果不符")
    except (SystemExit, KeyError, ValueError, OSError) as error:
        parser.exit(1, f"{error}\n")
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
