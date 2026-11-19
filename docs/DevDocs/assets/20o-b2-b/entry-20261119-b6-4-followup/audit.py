"""只读复核 B4 组件修复与 Windows 22D 复跑。"""

import argparse
from io import BytesIO
import json
from pathlib import Path
import re
import sys
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT.with_name("entry-20261119-b6-4")))

from evidence import CLOSEOUT, COMPONENT_ERROR, LOCKS, MAINTENANCE, PRODUCT, PROTECTED, RERUN, WORKSPACE
from evidence import Evidence, blob, clean_log, digest, git, observed_identity, require


FIX = "9d75ab1a73e96a37ba4f9675064817b690eaa117"
RUNNER = "e11914455273676998c006d2ef610aefc722c790"
COMPONENT_FILE = "core/rust/crates/xiao-codegen-llvm/src/artifact.rs"
FLOAT_TEST = "core/rust/crates/xiao-driver/tests/20o_static_float_equality.rs"
TRACE_CASE = "native-lock-wait-027"
TRACE_PASS = f"22D-CASE {TRACE_CASE}: ok"
TRACE_FAILURE = f"{TRACE_CASE}: 稳定共享对象释放顺序"
FLOAT_PASS = "FLOAT-EQUALITY cases=62 native=248 llvm_as=248 vm=62 result=passed"
ORIGINAL_EVIDENCE = "docs/DevDocs/assets/20o-b2-b/entry-20261119-b6-4"


def verify_source(bundle):
    changed = git("diff", "--name-only", PRODUCT, FIX, "--", *PROTECTED).decode().splitlines()
    require(set(changed) == {COMPONENT_FILE, FLOAT_TEST}, "产品变更超出组件登记与浮点回归")
    git("diff", "--exit-code", FIX, RUNNER, "--", *PROTECTED)
    git("diff", "--exit-code", RERUN, RUNNER, "--", MAINTENANCE, WORKSPACE)
    environment = bundle.read("linux/fixed/environment.json")
    after = bundle.read("linux/fixed/results.json")
    require(environment["baseline_commit"] == PRODUCT and environment["runtime_before_build"] is None,
            "Linux 修复任务入口不符")
    inputs = {name: digest(blob(FIX, name)) for name in (COMPONENT_FILE, FLOAT_TEST, *LOCKS)}
    require(environment["overlay"] == after["source_unchanged"] == inputs, "Linux 修复源码身份不符")


def linux_process(bundle, stage, name, expected):
    prefix = f"linux/{stage}/{name}"
    receipt = bundle.read(prefix + ".json")
    require(receipt["exit"] == expected and receipt["pid"] > 0 and "--locked" in receipt["argv"]
            and receipt["log"] == bundle.entries[prefix + ".log"], f"Linux 进程回执不符：{name}")
    build = bundle.read(f"linux/{stage}/build-runtime.json")
    require(build["runtime_before"] is None and build["runtime_after"]["bytes"] > 0
            and receipt["runtime_after"] == build["runtime_after"]
            and receipt["runtime_before"] == (None if name == "build-runtime" else build["runtime_after"]),
            f"Linux Runtime 身份不符：{name}")
    return bundle.raw(prefix + ".log").decode("utf-8")


def verify_linux(bundle):
    linux_process(bundle, "baseline", "build-runtime", 0)
    baseline = linux_process(bundle, "baseline", "baseline-float-equality", 101)
    require(COMPONENT_ERROR in baseline, "基线缺少 rc 产物拒绝")
    name = "original-pow-eq-static-o0.ll"
    ir = bundle.raw(f"linux/baseline/{name}")
    relink = bundle.read("linux/diagnostic/report.json")
    baseline_process = bundle.read("linux/baseline/baseline-float-equality.json")
    require(ir == bundle.raw(f"linux/fixed/{name}") and digest(ir)["sha256"] == relink["ir_sha256"]
            and relink["runtime_sha256"] == baseline_process["runtime_after"]["sha256"]
            and b"@xiao_runtime_cmath_pow(" in ir and b"@xiao_runtime_release(" not in ir
            and b" T xiao_runtime_release\n" in bundle.raw("linux/diagnostic/runtime-symbols.log"),
            "静态 IR 或诊断重链符号链不符")
    dependencies = bundle.read("linux/diagnostic/elf-symbols.json")
    cmath = [symbols for name, symbols in dependencies.items() if name.startswith("xiao_runtime_cmath_")]
    require(len(dependencies) == 121 and len(cmath) == 15
            and all("xiao_runtime_release" in symbols for symbols in cmath)
            and "xiao_runtime_release" in dependencies["xiao_runtime_lfloat_divide"],
            "ELF ABI 依赖样本不符")
    log = linux_process(bundle, "fixed", "float-native", 0)
    require(FLOAT_PASS in log and log.count("verification=complete declared=[\"rc\", \"value\"]") == 4,
            "Linux 浮点与组件矩阵不符")
    local_wrapper = linux_process(bundle, "fixed", "wrapper-native-retry", 0)
    require(all(f"B4-4-NATIVE path={prefix}.cmath optimization={level}" in local_wrapper
                for prefix in ("std", "stdlib") for level in (0, 3)), "Linux 包装矩阵缺项")
    for name in ("build-runtime", "codegen-tests-retry", "float-default", "direct-cmath-native",
                 "divide-native", "b1-native", "component-native"):
        linux_process(bundle, "fixed", name, 0)


def verify_local_windows(bundle):
    build = bundle.read("windows/build-runtime/receipt.json")
    environment = bundle.read("windows/environment.json")
    require(environment["product"] == PRODUCT and build["runtime_before"] is None
            and build["exit_code"] == 0 and "--locked" in build["argv"]
            and digest(bundle.raw("windows/build-runtime/cargo.log"))["sha256"] == build["log"]["sha256"],
            "Windows 基线 Runtime 构建回执不符")
    for name, identity in environment["inputs"].items():
        require(digest(blob(PRODUCT, name))["sha256"] == identity["sha256"], f"Windows 源码身份不符：{name}")
    expected = {2: "xcon-bounded-string-success", 4: "spawn-join-str-multiargument"}
    for round_number in range(1, 6):
        prefix = f"windows/22d-{round_number:02d}"
        receipt = bundle.read(f"{prefix}/receipt.json")
        log = bundle.raw(f"{prefix}/cargo.log").decode("utf-8")
        require(receipt["log"]["sha256"] == bundle.entries[f"{prefix}/cargo.log"]["sha256"]
                and receipt["runtime_before"] == receipt["runtime_after"] == build["runtime_after"]
                and receipt["pid"] > 0 and "--locked" in receipt["argv"]
                and "22d_llvm_concurrency" in receipt["argv"],
                f"Windows 完整入口回执不符：{round_number}")
        if round_number in expected:
            case = expected[round_number]
            require(receipt["exit_code"] == 101 and TRACE_PASS not in log
                    and f"{case}: VM/native 完整释放对象顺序" in log, f"Windows 失败位置不符：{round_number}")
            for backend in ("vm", "native"):
                bundle.raw(f"{prefix}/{case}.{backend}-release-trace.tsv")
        else:
            require(receipt["exit_code"] == 0 and TRACE_PASS in log, f"Windows 成功回执不符：{round_number}")
    for round_number in range(1, 26):
        prefix = f"windows/single/{round_number:02d}"
        receipt = bundle.read(f"{prefix}/receipt.json")
        require(receipt["exit"] == 0 and receipt["pid"] > 0 and receipt["log"] == bundle.entries[f"{prefix}/cargo.log"]
                and TRACE_PASS in bundle.raw(f"{prefix}/cargo.log").decode("utf-8"),
                f"Windows 单夹具回执不符：{round_number}")
        for backend in ("vm", "native"):
            name = f"{TRACE_CASE}.{backend}-release-trace.tsv"
            trace = bundle.raw(f"{prefix}/{name}")
            actual = [line.split(b"\t")[-1] for line in trace.splitlines()]
            actions = [action for action in actual if action in (b"shared_destroy", b"weak_release")]
            identity = {key: receipt["produced_files"][name][key] for key in ("bytes", "sha256")}
            require(actions == [b"shared_destroy", b"weak_release"] * 2 and identity == digest(trace),
                    f"Windows 单夹具原始轨迹不符：{round_number}/{backend}")
    return {"passed": 3, "failed_before_fixture": len(expected), "failed_cases": list(expected.values())}


def completed_job(bundle, run_id, attempt, job_id, head):
    prefix = f"ci/{run_id}-attempt-{attempt}"
    run = bundle.read(f"{prefix}/run.json")
    jobs = bundle.read(f"{prefix}/jobs.json")["jobs"]
    require((run["id"], run["head_sha"], run["run_attempt"], run["status"]) ==
            (run_id, head, attempt, "completed"), f"CI 运行身份不符：{run_id}/{attempt}")
    matches = [job for job in jobs if job["id"] == job_id]
    require(len(matches) == 1, f"CI 作业缺失：{job_id}")
    job = matches[0]
    require((job["run_id"], job["head_sha"], job["run_attempt"], job["status"]) ==
            (run_id, head, attempt, "completed"), f"CI 作业身份不符：{job_id}")
    log = clean_log(bundle.raw(f"{prefix}/{job_id}.log"))
    require(f"Complete job name: {job['name']}" in log and f"\n{head}\n" in log,
            f"CI 作业或检出日志不符：{job_id}")
    return run, job, log


def windows_tool_identity(log):
    marker = re.search(r"^B6-4-IDENTITY native (\{.+\})$", log, re.M)
    require(marker is not None, "Windows Runner 缺少 Runtime 与工具身份")
    native = json.loads(marker[1])
    require(native["target"] == "x86_64-pc-windows-msvc", "Windows Runner 目标不符")
    return {
        "image": native["context"]["ImageVersion"],
        "runtime_sha256": native["runtime"]["sha256"],
        "clang_sha256": native["llvm"]["XIAO_CLANG"]["sha256"],
    }


def verify_runner_windows(bundle):
    recorded = blob(CLOSEOUT, f"{ORIGINAL_EVIDENCE}/ci.zip")
    with ZipFile(BytesIO(recorded)) as archive:
        original = clean_log(Evidence(archive).raw("ci/44464422497/135358661620.log"))
    require(TRACE_FAILURE in original and 'left: [(0, "shared_destroy"), (0, "weak_release")]' in original
            and '(1, "shared_destroy"), (1, "weak_release")]' in original, "原 Windows 失败投影不符")
    results = [dict(job=135358661620, case="failed", **windows_tool_identity(original))]
    for attempt, job_id in ((2, 135613903433), (3, 135623035222)):
        _, job, log = completed_job(bundle, 44464422497, attempt, job_id, RERUN)
        require(job["labels"] == ["windows-2025"], f"Windows Runner 标签不符：{job_id}")
        require(job["conclusion"] == "success" and TRACE_PASS in log and TRACE_FAILURE not in log,
                f"原 Windows 入口缺少实际成功回执：{job_id}")
        results.append(dict(job=job_id, case="passed", **windows_tool_identity(log)))
    for field in ("runtime_sha256", "clang_sha256"):
        require(len({row[field] for row in results}) == 1, f"Windows 三次作业的 {field} 不一致")
    return results


def verify_fix_runner(bundle):
    results = {}
    for platform, job_id, verification in (
        ("Linux", 135621969980, "complete"),
        ("macOS", 135621970069, "complete"),
        ("Windows", 135621969868, "unverified-coff-exports"),
    ):
        run, job, log = completed_job(bundle, 44541850714, 1, job_id, RUNNER)
        observed_identity(log, run, job)
        require(run["conclusion"] == job["conclusion"] == "success" and FLOAT_PASS in log
                and log.count(f"verification={verification} declared=[\"rc\", \"value\"] observed=[\"rc\", \"value\"]") == 4,
                f"Runner 浮点矩阵不符：{platform}")
        require(all(f"B4-4-NATIVE path={prefix}.cmath optimization={level}" in log
                    for prefix in ("std", "stdlib") for level in (0, 3)), f"Runner 包装矩阵缺项：{platform}")
        results[platform] = {"job": job_id, "conclusion": job["conclusion"], "verification": verification}
    return results


def verify_engineering(bundle):
    for run_id, job_id, markers in (
        (44541850714, 135621969590, ("--skip b1_capabilities_have_vm_golden_outputs",
                                   "test b1_capabilities_have_vm_golden_outputs ... ok")),
        (44543362261, 135627307737, ("F7-CARGO command=clippy locked=true", "bun run check:lock",
                                   "tools/repo-check/src/cli.ts all", "cargo fmt")),
    ):
        run, job, log = completed_job(bundle, run_id, 1, job_id, RUNNER)
        observed_identity(log, run, job)
        require(run["conclusion"] == job["conclusion"] == "success" and all(marker in log for marker in markers),
                f"工程门禁作业不符：{job_id}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, default=ROOT / "followup.zip")
    args = parser.parse_args()
    with ZipFile(args.archive) as archive:
        bundle = Evidence(archive)
        verify_source(bundle)
        verify_linux(bundle)
        local_windows = verify_local_windows(bundle)
        verify_engineering(bundle)
        fix_runner = verify_fix_runner(bundle)
        original_windows = verify_runner_windows(bundle)
        original_passed = json.loads(blob(CLOSEOUT, f"{ORIGINAL_EVIDENCE}/summary.json"))["b6_4_passed"]
        status = {"b6_4_passed": original_passed, "original_passed": original_passed,
                  "followup_passed": all(job["conclusion"] == "success" for job in fix_runner.values()),
                  "22d_open": True}
        require(json.loads((ROOT / "status.json").read_text(encoding="utf-8")) == status, "B6-4 交接状态不符")
        print(json.dumps({"verified": True, "files": len(bundle.entries), "objects": bundle.unique,
                          **status, "fix_runner": fix_runner, "original_windows": original_windows,
                          "local_windows_full": local_windows,
                          "local_windows_single_passed": 25, "engineering_passed": True}, ensure_ascii=False))


if __name__ == "__main__":
    main()
