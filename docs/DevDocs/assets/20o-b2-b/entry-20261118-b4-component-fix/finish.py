"""汇总FIX.2回执、运行唯一最终docs门禁并按实字节封存清单。"""
from datetime import datetime, timezone
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import io
import time

sys.dont_write_bytecode = True
import record

ROOT, REPOSITORY, TASK = record.ROOT, record.REPOSITORY, record.TASK
ARCHIVE_PREFIX = ROOT.relative_to(REPOSITORY).as_posix()
REVIEW_PREFIX = ROOT.with_name("entry-20261118-b4-component-review").relative_to(REPOSITORY).as_posix()
BASELINE = "a067bd90b6ff34b050e2a39efaa40f3b692d75fd"
CODE = "43f18274175ba09ac84e759999c40cde72eae1c5"
PROTECTED_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]
DOC_FILES = [
    "docs/DevDocs/20o-b2-b4-3-fix2-runtime-components-plan.md",
    "docs/DevDocs/20o-b2-b4-3-llvm-plan.md",
    "docs/DevDocs/20o-b2-b4-3-fix1-numeric-range-routing-plan.md",
    "docs/DevDocs/20o-b2-b6-resource-performance-plan.md",
    "docs/DevDocs/20o-b2-b7-closeout-plan.md",
    "docs/DevDocs/20o-official-library-program.md",
    "docs/DevDocs/README.md",
    REVIEW_PREFIX + "/.gitattributes",
    REVIEW_PREFIX + "/README.md",
    REVIEW_PREFIX + "/self_negative.py",
]
FINAL_WINDOWS = [
    "fix4-win-codegen-test-r2", "fix4-win-driver-regressions",
    "fix4-win-test-build-r2", "fix4-win-test-run-r2",
]
FINAL_LINUX = [
    "fix4-linux-codegen-test-r2", "fix4-linux-driver-regressions",
    "fix4-linux-test-build-r2", "fix4-linux-test-run-r2",
]
DOCS_RECEIPT = "docs-final-r7.json"
DOCS_LOG = "docs-final-r7.log"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read(root, name):
    return json.loads((root / name).read_text(encoding="utf-8"))


def git(*args):
    return subprocess.run(
        ["cmd", "/c", "git", "-C", str(REPOSITORY), *args],
        capture_output=True, check=True,
    ).stdout


def receipt(label, linux=False):
    return read(ROOT, ("linux-" if linux else "") + label + ".json")


def log_bytes(value):
    return unpack(value["log"])


def unpack(reference):
    packed = (ROOT / reference["path"]).read_bytes()
    record.require(record.sha(packed) == reference["sha256"], "FIX.2内容对象摘要不符")
    raw = gzip.decompress(packed) if reference["encoding"] == "gzip" else packed
    record.require(len(raw) == reference["raw_bytes"]
                   and record.sha(raw) == reference["raw_sha256"],
                   "FIX.2内容对象原文摘要不符")
    return raw


def test_summary(raw):
    rows = re.findall(
        rb"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",
        raw,
    )
    return {
        "targets": len(rows), "passed": sum(int(row[0]) for row in rows),
        "failed": sum(int(row[1]) for row in rows),
        "ignored": sum(int(row[2]) for row in rows),
    }


def native_rows(raw):
    rows = []
    for line in raw.decode("utf-8", errors="replace").splitlines():
        if not line.startswith("B4-4-NATIVE "):
            continue
        match = re.fullmatch(
            r"B4-4-NATIVE path=([^ ]+) optimization=(\d+) executable=(.+?) "
            r"verification=([^ ]+) declared=(\[.*?\]) observed=(\[.*?\]) "
            r"stdout=([^ ]+) errors=([^ ]+) result=([^ ]+)",
            line,
        )
        require(match is not None, "FIX.2原生回执行格式无效")
        keys = ("path", "optimization", "executable", "verification", "declared",
                "observed", "stdout", "errors", "result")
        rows.append(dict(zip(keys, match.groups())))
    return rows


def check_command(row, exit_code, platform):
    code = row["exit_code"]
    require(code == exit_code and row["source_unchanged"],
            "FIX.2命令退出码或源码身份不符：" + row["label"])
    require(row.get("pid") and row["elapsed_seconds"] > 0 and row.get("log"),
            "FIX.2命令回执缺PID、时长或日志：" + row["label"])
    before, after = row["runtime_before"], row["runtime_after"]
    if row["label"].endswith("runtime-build"):
        require(before is None and after is not None,
                "FIX.2冷建Runtime前后身份不符：" + row["label"])
    else:
        require(before and after and before["sha256"] == after["sha256"],
                "FIX.2命令前后Runtime身份变化：" + row["label"])
    return log_bytes(row)


def verify_snapshot(row, expected, label):
    snapshot = row["source_after"]
    require(set(snapshot) == set(expected), "FIX.2源码路径清单不符：" + label)
    require(all(snapshot[name]["lf_sha256"] == digest
                for name, digest in expected.items()),
            "FIX.2源码摘要不符：" + label)


def collect():
    code_commit = git("rev-parse", CODE).decode().strip()
    require(code_commit == CODE, "FIX.2代码提交对象摘要不符")
    changed = git(
        "diff", "--name-only", BASELINE, CODE, "--", *PROTECTED_ROOTS
    ).decode().splitlines()
    expected_changes = [
        "core/rust/crates/xiao-codegen-llvm/src/artifact.rs",
        "core/rust/crates/xiao-codegen-llvm/src/dynamic/runtime_abi.rs",
        "core/rust/crates/xiao-codegen-llvm/src/ir.rs",
        "core/rust/crates/xiao-codegen-llvm/tests/n0_a.rs",
        "core/rust/crates/xiao-driver/tests/support/b4_cmath_library_native.rs",
    ]
    require(changed == expected_changes, "FIX.2受保护产品差异越界")
    abi = (REPOSITORY / "core/rust/crates/xiao-runtime-abi/src/lib.rs").read_text(encoding="utf-8")
    codegen = (REPOSITORY / "core/rust/crates/xiao-codegen-llvm/src/lib.rs").read_text(encoding="utf-8")
    require("pub const ABI_MINOR_VERSION: u32 = 19;" in abi
            and "pub const CODEGEN_VERSION: u32 = 22;" in codegen,
            "FIX.2 ABI或生成器版本越界")

    baseline_source = git_source_snapshot(BASELINE, record.PRODUCT_ROOTS)
    code_source = git_source_snapshot(CODE, record.PRODUCT_ROOTS)
    baseline_rows = baseline_source
    code_rows = code_source
    for label in ("fix0-win-runtime-build", "fix0-win-test-build", "fix0-win-test-run"):
        value = receipt(label)
        check_command(value, 0, "windows")
        verify_snapshot(value, baseline_rows, label)
    for label in ("fix0-linux-runtime-build", "fix0-linux-test-build", "fix0-linux-test-run"):
        value = receipt(label, linux=True)
        expected_exit = 101 if label.endswith("test-run") else 0
        check_command(value, expected_exit, "linux")
        verify_snapshot(value, baseline_rows, label)

    baseline_win = receipt("fix0-win-test-run")
    baseline_linux = receipt("fix0-linux-test-run", linux=True)
    baseline_win_log = check_command(baseline_win, 0, "windows")
    baseline_linux_log = check_command(baseline_linux, 101, "linux")
    win_baseline_rows = [
        line for line in baseline_win_log.decode("utf-8", errors="replace").splitlines()
        if line.startswith("B4-4-NATIVE ")
    ]
    require(len(win_baseline_rows) == 4
            and all("result=passed" in line and "stdout=golden" in line
                    for line in win_baseline_rows),
            "FIX-0 Windows基线包装矩阵不是四格通过")
    require(b"ArtifactVerification" in baseline_linux_log
            and b"containers" in baseline_linux_log,
            "FIX-0 Linux基线阻断原因不是containers完整性拒绝")

    codegen = {}
    for label in (FINAL_WINDOWS[0], FINAL_LINUX[0]):
        linux = label.startswith("fix4-linux-")
        value = receipt(label, linux=linux)
        raw = check_command(value, 0, "linux" if linux else "windows")
        verify_snapshot(value, code_rows, label)
        codegen["linux" if linux else "windows"] = test_summary(raw)
    require(codegen["windows"] == codegen["linux"]
            and codegen["windows"] == {
                "targets": 7, "passed": 109, "failed": 0, "ignored": 15,
            }, "FIX-1 codegen跨平台测试摘要不符")

    driver = {}
    for label in (FINAL_WINDOWS[1], FINAL_LINUX[1]):
        linux = label.startswith("fix4-linux-")
        value = receipt(label, linux=linux)
        raw = check_command(value, 0, "linux" if linux else "windows")
        verify_snapshot(value, code_rows, label)
        driver["linux" if linux else "windows"] = test_summary(raw)
    require(driver["windows"] == driver["linux"]
            and driver["windows"] == {
                "targets": 4, "passed": 29, "failed": 0, "ignored": 4,
            }, "FIX-2 driver回归摘要不符")

    native = {}
    for label in (FINAL_WINDOWS[3], FINAL_LINUX[3]):
        linux = label.startswith("fix4-linux-")
        value = receipt(label, linux=linux)
        raw = check_command(value, 0, "linux" if linux else "windows")
        verify_snapshot(value, code_rows, label)
        rows = native_rows(raw)
        require(len(rows) == 4 and all(row.get("result") == "passed" for row in rows),
                "FIX-2 Cmath原生矩阵缺项或失败：" + label)
        expected_verification = "Complete" if linux else "UnverifiedCoffExports"
        expected_components = ["containers", "rc", "value", "weak"]
        for row in rows:
            require(row["verification"] == expected_verification,
                    "FIX-2平台验证等级不符：" + label)
            declared = json.loads(row["declared"])
            observed = json.loads(row["observed"])
            require(declared == expected_components,
                    "FIX-2声明组件集合不符：" + label)
            if linux:
                require(observed == declared, "Linux ELF声明/观察组件不一致")
            require(row["stdout"] == "golden" and row["errors"] == "009",
                    "FIX-2包装黄金或错误回归不完整")
        native["linux" if linux else "windows"] = rows

    clippy_win = receipt("fix3-win-codegen-clippy")
    clippy_linux = receipt("fix3-linux-codegen-clippy", linux=True)
    clippy_win_log = check_command(clippy_win, 0, "windows")
    clippy_linux_log = check_command(clippy_linux, 1, "linux")
    require(clippy_win["argv"][2] == "clippy"
            and b"cargo-clippy' is not installed" in clippy_linux_log,
            "FIX.3 Clippy平台结果身份不符")

    linux_runtime = read(ROOT, "linux-fix0-linux-runtime-build.json")
    windows_runtime = read(ROOT, "fix0-win-runtime-build.json")
    result = {
        "stage": "B4-3-FIX.2", "fix2_closed": True,
        "baseline_commit": BASELINE, "code_commit": code_commit,
        "runtime_abi": "1.19", "codegen_version": 22,
        "stdlib_interface_version": 1,
        "protected_product_changes": changed,
        "llvm_text_emission_changed": False,
        "fix0": {
            "windows_native_matrix": "passed",
            "windows_verification": "UnverifiedCoffExports",
            "linux_native_matrix_exit": 101,
            "linux_error": "ArtifactVerification: containers",
            "linux_verification": "Complete",
            "platform_difference": "PE导出观察未验证；ELF符号表完整并拒绝缺少containers的声明",
        },
        "runtimes": {
            "windows": windows_runtime["runtime_after"],
            "linux": linux_runtime["runtime_after"],
        },
        "codegen_test_summaries": codegen,
        "driver_regression_summaries": driver,
        "native_wrapper_rows": native,
        "clippy": {
            "windows": "passed",
            "linux": "unverifiable: cargo-clippy未安装于Rust 1.96.0工具链",
        },
        "llvm_text": {
            "emission_path_changed": False,
            "explanation": "组件集合在LLVM文本生成完成后推导；helper只读文本，不重写文本或指纹",
        },
        "b6_handoff": {
            "b6_1_must_rerun_in_own_task_root": True,
            "b6_1_historical_status_rewritten": False,
            "b6_2_started": False,
        },
        "macos": "unverifiable",
        "remote_ci": "not_run",
        "legacy_721": "not_run",
    }
    return result


def write_new(path, value):
    with path.open("xb") as output:
        output.write((json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def capture_failed_docs_attempt():
    old_log = TASK / "logs/docs-final.log"
    receipt_path = ROOT / "docs-final-attempt1.json"
    record.require(old_log.is_file() and not receipt_path.exists(),
                   "FIX.2首次文档门禁原始日志缺失或回执已存在")
    raw = old_log.read_bytes()
    report = json.loads(raw.decode("utf-8"))
    record.require(report.get("passed") is True,
                   "FIX.2首次docs命令报告并非通过，须保留其实际状态")
    write_new(receipt_path, {
        "attempt": 1, "command": [
            "bun", "tools/repo-check/src/cli.ts", "all", "--format", "json",
        ],
        "gate_report_passed": True,
        "collector_status": "failed_after_child_exit",
        "collector_error": "AttributeError: 回执封装引用了不存在的record.canonical辅助函数",
        "child_pid_recorded": False, "elapsed_seconds_recorded": False,
        "log": record.store(raw),
    })
    print(json.dumps({"captured_report_passed": True,
                      "collector_error_preserved": True}, ensure_ascii=False))


def docs_snapshot():
    paths = [REPOSITORY / name for name in DOC_FILES]
    paths.extend(
        path for path in ROOT.rglob("*")
        if path.is_file()
        and path.relative_to(ROOT).parts[0] != "objects"
        and path.name not in {DOCS_RECEIPT, "evidence.json"}
    )
    result = {}
    for path in sorted(paths):
        raw = path.read_bytes()
        binary = path.suffix == ".bin" or b"\0" in raw
        result[path.relative_to(REPOSITORY).as_posix()] = {
            "bytes": len(raw), "sha256": record.sha(raw),
            "lf_sha256": record.sha(raw if binary else raw.replace(b"\r\n", b"\n")),
            "binary": binary,
        }
    return result


def git_source_snapshot(revision, roots):
    archive = git("archive", "--format=tar", revision, "--", *roots)
    with tarfile.open(fileobj=io.BytesIO(archive)) as bundle:
        return {
            member.name: record.sha(bundle.extractfile(member).read().replace(b"\r\n", b"\n"))
            for member in bundle.getmembers() if member.isfile()
        }


def docs_gate():
    receipt_path = ROOT / DOCS_RECEIPT
    task_log = TASK / "logs" / DOCS_LOG
    record.require(not receipt_path.exists() and not task_log.exists(),
                   "FIX.2最终文档门禁回执或日志已存在，拒绝覆盖")
    before_docs = docs_snapshot()
    before_product = record.source_snapshot()
    env, removed = record.environment(Path(DOCS_RECEIPT).stem)
    bun = shutil.which("bun", path=env.get("PATH"))
    record.require(bun is not None, "FIX.2最终docs门禁找不到Bun")
    bun_version = subprocess.run([bun, "--version"], capture_output=True,
                                 encoding="utf-8", check=True, env=env).stdout.strip()
    argv = [bun, "tools/repo-check/src/cli.ts", "all", "--format", "json"]
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    with task_log.open("xb") as output:
        child = subprocess.Popen(argv, cwd=REPOSITORY, env=env,
                                 stdin=subprocess.DEVNULL, stdout=output,
                                 stderr=subprocess.STDOUT)
        exit_code = child.wait()
    elapsed = time.monotonic() - clock
    finished = datetime.now(timezone.utc).isoformat()
    after_docs, after_product = docs_snapshot(), record.source_snapshot()
    raw_log = task_log.read_bytes()
    try:
        report = json.loads(raw_log.decode("utf-8"))
    except json.JSONDecodeError as error:
        report = {"passed": False, "parse_error": str(error)}
    receipt_value = {
        "argv": argv, "cwd": str(REPOSITORY), "pid": child.pid,
        "started_utc": started, "finished_utc": finished,
        "elapsed_seconds": elapsed, "exit_code": exit_code,
        "environment_overrides": {
            key: env[key] for key in env
            if key.startswith("XIAO_") or key in {"RUSTUP_TOOLCHAIN", "CARGO_TARGET_DIR", "TEMP", "TMP"}
        },
        "removed_environment_names": sorted(removed),
        "bun": {"binary": record.identity(bun), "version": bun_version},
        "docs_before": record.store(canonical(before_docs)),
        "docs_after": record.store(canonical(after_docs)),
        "product_before": record.store(canonical(before_product)),
        "product_after": record.store(canonical(after_product)),
        "source_unchanged": before_docs == after_docs and before_product == after_product,
        "report_passed": report.get("passed") is True,
        "report_summary": {
            "diagnostics": len(report.get("diagnostics", [])),
            "errors": sum(row.get("severity") == "error"
                           for row in report.get("diagnostics", [])),
        },
        "result_sha256": record.sha((ROOT / "result.json").read_bytes()),
        "log": record.store(raw_log),
    }
    write_new(receipt_path, receipt_value)
    record.require(exit_code == 0 and report.get("passed") is True
                   and receipt_value["source_unchanged"],
                   "FIX.2最终docs门禁未通过或其间源码变化")
    print(json.dumps({"docs_passed": True, "pid": child.pid,
                      "elapsed_seconds": elapsed}, ensure_ascii=False))


def seal():
    value = read(ROOT, DOCS_RECEIPT)
    stored_docs = unpack(value["docs_after"])
    record.require(value["exit_code"] == 0 and value["report_passed"]
                   and value["source_unchanged"]
                   and stored_docs == canonical(docs_snapshot()),
                   "FIX.2文档门禁后仍有改动")
    files = {}
    for path in sorted(ROOT.rglob("*")):
        if path.is_file() and path.name != "evidence.json":
            raw = path.read_bytes()
            files[path.relative_to(ROOT).as_posix()] = {
                "bytes": len(raw), "sha256": record.sha(raw),
            }
    record.require(".gitattributes" in files, "FIX.2证据清单遗漏.gitattributes")
    manifest = {
        "file_count_excluding_manifest": len(files), "files": files,
        "generated_from_actual_bytes": True,
        "generator": "finish.py seal", "docs_receipt": DOCS_RECEIPT,
        "result": "result.json", "code_commit": CODE,
    }
    manifest_path = ROOT / "evidence.json"
    temporary_path = ROOT / ".evidence.json.tmp"
    record.require(not temporary_path.exists(), "FIX.2临时清单路径已存在")
    write_new(temporary_path, manifest)
    os.replace(temporary_path, manifest_path)
    print(json.dumps({"sealed_files_excluding_manifest": len(files)}))


def main():
    record.require(len(sys.argv) == 2 and sys.argv[1] in {"prepare", "capture", "docs", "seal"},
                   "用法：finish.py prepare | capture | docs | seal")
    if sys.argv[1] == "prepare":
        require(not (ROOT / "result.json").exists(), "FIX.2 result.json禁止覆盖")
        write_new(ROOT / "result.json", collect())
        print(json.dumps({"result": "passed", "code_commit": CODE}, ensure_ascii=False))
    elif sys.argv[1] == "docs":
        docs_gate()
    elif sys.argv[1] == "capture":
        capture_failed_docs_attempt()
    else:
        seal()


if __name__ == "__main__":
    main()
