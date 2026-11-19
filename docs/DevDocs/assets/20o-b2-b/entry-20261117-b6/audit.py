"""只读复核B6-1实际回执、跨平台阻断、工具Runtime身份和结果边界。"""
import argparse
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
import record
ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
BASELINE = "67319d61c18b2f02a57f974883c57600f70c724d"
CODE = "a067bd90b6ff34b050e2a39efaa40f3b692d75fd"
PREFIX = ROOT.relative_to(REPOSITORY).as_posix()
B6_TEST = "core/rust/crates/xiao-driver/tests/20o_b2_b6_resources.rs"
PRODUCT_ROOTS = [
    ".cargo", ".gitattributes", ".github/workflows", "core/rust", "stdlib",
    "tests/fixtures", "tests/benchmarks", "tools", "cli/ts", "package.json",
    "bun.lock", "docs/module-registry.json", "docs/UseDocs",
]
DOC_FILES = ["docs/DevDocs/" + name for name in (
    "20o-b2-b-implementation-plan.md", "20o-b2-b4-5-closeout-plan.md",
    "20o-b2-b5-version-matrix-plan.md", "20o-b2-b6-resource-performance-plan.md",
    "20o-b2-b7-closeout-plan.md", "20o-b2-c-final-closeout-plan.md",
    "20o-b2-numeric-foundation-plan.md", "20o-official-library-program.md", "README.md",
)]
WINDOWS_PASS = [
    "runtime-release", "runtime-staticlib", "driver-b6-language-r3",
    "windows-vm-regressions-final-r2", "a6-resource-driver", "native-cmath-o0-o3",
    "windows-native-cmath-wrappers-final", "native-a5-errors-o0-o3",
    "native-lfloat-divide-o0-o3", "windows-clippy",
    "windows-b6-language-final-r2", "windows-fmt-final",
]
LINUX_PASS = [
    "linux-toolchain", "linux-runtime-tests",
    "linux-driver-vm-regressions-final", "linux-native-cmath-o0-o3",
    "linux-native-lfloat-divide", "linux-native-a5-errors",
    "linux-native-component-diagnostic-r2",
]
LINUX_FINAL_LABELS = {
    "linux-toolchain": "linux-toolchain-final-r2",
    "linux-runtime-tests": "linux-runtime-tests-final-r2",
    "linux-driver-vm-regressions-final": "linux-driver-vm-regressions-final-r2",
    "linux-native-cmath-o0-o3": "linux-native-cmath-o0-o3-final-r2",
    "linux-native-lfloat-divide": "linux-native-lfloat-divide-final-r2",
    "linux-native-a5-errors": "linux-native-a5-errors-final-r2",
    "linux-native-component-diagnostic-r2": "linux-native-component-diagnostic-final-r2",
}
LINUX_FAIL = "linux-native-cmath-wrappers-final"
RECEIPT_FILES = (
    [name + ".json" for name in WINDOWS_PASS]
    + ["linux-" + name + ".json" for name in LINUX_PASS]
    + ["linux-" + LINUX_FAIL + ".json"]
)
ERROR_REASONS = {
    "missing": "B6清单中的.gitattributes缺失",
    "manifest": "B6清单与实际字节不符",
    "rehash": "B6固定Git清单字节不符",
    "status": "Linux原生包装阻断被改写",
    "stage": "B6阻断状态被冒进升级",
}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def identity(raw):
    return {"bytes": len(raw), "sha256": sha(raw)}


def load(root, name):
    return json.loads((root / name).read_text(encoding="utf-8"))


def inventory(root):
    return {path.relative_to(root).as_posix(): identity(path.read_bytes())
            for path in sorted(root.rglob("*")) if path.is_file()}


def git(repository, *args):
    proc = subprocess.run(["cmd", "/c", "git", "-C", str(repository), *args],
                          capture_output=True)
    require(proc.returncode == 0, "固定Git对象读取失败：" +
            proc.stderr.decode("utf-8", errors="replace"))
    return proc.stdout


def stored(root, reference):
    path = (root / reference["path"]).resolve()
    require(path.is_relative_to(root.resolve()), "B6内容对象路径越界")
    packed = path.read_bytes()
    require(identity(packed) == {key: reference[key] for key in ("bytes", "sha256")},
            "B6内容对象摘要不符")
    require(reference["encoding"] in ("identity", "gzip"), "B6对象编码不符")
    raw = gzip.decompress(packed) if reference["encoding"] == "gzip" else packed
    require(len(raw) == reference["raw_bytes"] and sha(raw) == reference["raw_sha256"],
            "B6内容对象原文摘要不符")
    return raw


def summaries(raw):
    rows = re.findall(
        rb"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;",
        raw,
    )
    return {
        "targets": len(rows),
        "passed": sum(int(row[0]) for row in rows),
        "failed": sum(int(row[1]) for row in rows),
        "ignored": sum(int(row[2]) for row in rows),
    }


def source_revision(repository, revision, paths):
    archive = git(repository, "archive", "--format=tar", revision, "--", *paths)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        return {
            item.name: tar.extractfile(item).read()
            for item in tar.getmembers() if item.isfile()
        }


def verify_source_snapshot(root, reference, current):
    snapshot = json.loads(stored(root, reference))
    require(set(snapshot) == set(current), "B6受保护源码路径集合改变")
    changed = []
    for name, row in current.items():
        require(snapshot[name]["lf_sha256"] == row["lf_sha256"]
                or name == B6_TEST, "B6实测快照存在超范围产品差异：" + name)
        if snapshot[name]["lf_sha256"] != row["lf_sha256"]:
            changed.append(name)
    require(set(changed) <= {B6_TEST}, "B6历史命令源码差异超出测试文件")


def check_receipt(root, name, platform, expected_exit, current_source):
    receipt = load(root, name + ".json" if platform == "windows" else "linux-" + name + ".json")
    code = receipt["exit_code"] if platform == "windows" else receipt["wsl_exit_code"]
    require(code == expected_exit and receipt["source_unchanged"],
            "B6命令退出或源码状态不符：" + name)
    require(receipt.get("pid") and receipt["elapsed_seconds"] > 0,
            "B6命令PID或时长无效：" + name)
    if platform == "windows":
        env = receipt["environment_overrides"]
        task = Path("D:/project/Xiao/.b6/b6-1-20261117-01").resolve()
        for key in ("CARGO_TARGET_DIR", "TEMP", "TMP", "XIAO_HOME"):
            require(Path(env[key]).resolve().is_relative_to(task),
                    "Windows B6隔离环境路径越界：" + key)
        require(env["RUSTUP_TOOLCHAIN"] == "1.96.0"
                and env["XIAO_STDLIB_ROOT"] == str(REPOSITORY / "stdlib")
                and env["XIAO_B1_F10"] == env["XIAO_B2_A_PERF"] == "0"
                and "RUST_TEST_THREADS" not in env,
                "Windows B6工具链、加载根或测试模式不符")
        require(receipt["recorder"]["sha256"] == sha((root / "record.py").read_bytes())
                and receipt["tools"]["rustc"]["stdout"].startswith("rustc 1.96.0")
                and receipt["tools"]["cargo"]["stdout"].startswith("cargo 1.96.0"),
                "Windows B6采集器或Rust工具链身份不符")
        if receipt.get("runtime_after") is not None:
            require(Path(receipt["runtime_after"]["path"]).resolve().is_relative_to(task),
                    "Windows Runtime不在B6任务根")
            require(receipt["runtime_before"] is None
                    or receipt["runtime_before"]["sha256"] == receipt["runtime_after"]["sha256"],
                    "Windows命令前后Runtime身份变化")
    else:
        require(receipt["wsl_distro"] == "Ubuntu-26.04"
                and receipt["task_root"] == "/home/xiaoczx/.b6/b6-1-20261117-01"
                and receipt["host"] and receipt["kernel"]
                and receipt["rustc"].startswith("rustc 1.96.0")
                and receipt["clang"] and receipt["llvm_as"] and receipt["llc"]
                and receipt["runtime_sha256"] == "08b88569645eb45d7cc6fea793adbe91a459a174392a91d1b57497d22349ad89"
                and receipt["linux_runner"]["sha256"] == sha(
                    (root / "linux_run.sh").read_bytes()
                ), "Linux B6主机、工具、Runtime或采集器身份不符")
        require(receipt["recorder"]["path"].endswith("linux_record.py")
                and len(receipt["recorder"]["sha256"]) == 64
                and receipt["rustc_sha256"] and receipt["cargo_sha256"]
                and receipt["llvm_tools_sha256"]
                and "RUST_TEST_THREADS" not in (receipt["removed_environment_names"] or ""),
                "Linux B6采集器、工具摘要或测试线程覆盖不符")
        env = receipt["environment"]
        for key in ("CARGO_TARGET_DIR", "TEMP", "TMP", "TMPDIR", "XIAO_HOME"):
            require(env[key].startswith(receipt["task_root"] + "/"),
                    "Linux B6隔离路径越界：" + key)
        require(env["XIAO_STDLIB_ROOT"] == "/mnt/d/project/Xiao/Xiao/stdlib"
                and env["XIAO_B1_F10"] == env["XIAO_B2_A_PERF"] == "0"
                and b'RUST_TEST_THREADS = "4"' in git(
                    REPOSITORY, "show", CODE + ":.cargo/config.toml"
                )
                and not any("test-threads" in arg or "RUST_TEST_THREADS" in arg
                            for arg in receipt["argv"]),
                "Linux B6 stdlib、性能模式或默认线程配置不符")
    verify_source_snapshot(root, receipt["source_before"], current_source)
    verify_source_snapshot(root, receipt["source_after"], current_source)
    raw = stored(root, receipt["log"])
    return receipt, raw


def recompute(root, repository):
    current = record_source(root)
    fixed = source_revision(repository, CODE, tuple(PRODUCT_ROOTS))
    fixed_hashes = {
        name: sha(content.replace(b"\r\n", b"\n")) for name, content in fixed.items()
    }
    require({name: row["lf_sha256"] for name, row in current.items()} == fixed_hashes,
            "当前受保护源码偏离B6固定代码对象")
    changed = git(repository, "diff", "--name-only", BASELINE, CODE,
                   "--", *PRODUCT_ROOTS).decode().splitlines()
    require(changed == [B6_TEST], "B6产品差异越出测试文件")

    b5 = json.loads(git(repository, "show",
        "88a14834f1dd6389b6d37374d5a89d9176642339:docs/DevDocs/assets/"
        "20o-b2-b/entry-20261117-b5-closeout/closeout.json"))
    require(b5["b5_closed"] is True and b5["b6_entry_open"] is True
            and b5["b6_started"] is False, "B6固定B5开工前置不符")
    require(b"pub const CODEGEN_VERSION: u32 = 22;" in
            fixed["core/rust/crates/xiao-codegen-llvm/src/lib.rs"]
            and b"pub const ABI_MINOR_VERSION: u32 = 19;" in
            fixed["core/rust/crates/xiao-runtime-abi/src/lib.rs"],
            "B6 ABI或生成器版本越界")

    receipts, raw_logs = {}, {}
    for label in WINDOWS_PASS:
        receipts[label], raw_logs[label] = check_receipt(
            root, label, "windows", 0, current)
    for label in LINUX_PASS:
        final_label = LINUX_FINAL_LABELS[label]
        receipts["linux-" + label], raw_logs["linux-" + label] = check_receipt(
            root, final_label, "linux", 0, current)
    linux_blocker_label = LINUX_FAIL + "-r2"
    receipts["linux-" + LINUX_FAIL], raw_logs["linux-" + LINUX_FAIL] = check_receipt(
        root, linux_blocker_label, "linux", 101, current)

    windows_runtime = summaries(raw_logs["runtime-release"])
    linux_runtime = summaries(raw_logs["linux-linux-runtime-tests"])
    windows_vm = summaries(raw_logs["windows-vm-regressions-final-r2"])
    linux_vm = summaries(raw_logs["linux-linux-driver-vm-regressions-final"])
    a6 = summaries(raw_logs["a6-resource-driver"])
    b6_win = summaries(raw_logs["windows-b6-language-final-r2"])
    require(windows_runtime == {"targets": 7, "passed": 305, "failed": 0, "ignored": 1}
            and linux_runtime == windows_runtime, "Runtime正式参考/ABI测试摘要不符")
    require(windows_vm == {"targets": 7, "passed": 41, "failed": 0, "ignored": 3}
            and linux_vm == {"targets": 8, "passed": 43, "failed": 0, "ignored": 3},
            "B6 VM跨平台回归摘要不符")
    require(a6 == {"targets": 1, "passed": 8, "failed": 0, "ignored": 0}
            and b6_win == {"targets": 1, "passed": 1, "failed": 0, "ignored": 0},
            "数值资源或B6语言测试摘要不符")
    for platform_log in ("runtime-release", "linux-linux-runtime-tests"):
        abi_rows = re.findall(
            rb"B3-CMATH-ABI function=([^ ]+) cases=(\d+) result=success",
            raw_logs[platform_log],
        )
        require(len(abi_rows) == 15 and sum(int(row[1]) for row in abi_rows) == 17243
                and b"reference_inputs=17243 facades=15 kernel_identity=exact"
                in raw_logs[platform_log],
                "B6正式Cmath向量或C ABI摘要不完整：" + platform_log)
    require(b"B6-1-LANGUAGE module=std.cmath" in raw_logs["windows-b6-language-final-r2"]
            and b"B6-1-LANGUAGE module=stdlib.cmath" in raw_logs["windows-b6-language-final-r2"]
            and b"B6-1-LANGUAGE module=std.cmath" in raw_logs["linux-linux-driver-vm-regressions-final"]
            and b"B6-1-LANGUAGE module=stdlib.cmath" in raw_logs["linux-linux-driver-vm-regressions-final"],
            "B6语言两种官方导入路径未实际通过")
    require(b"B4-3-NATIVE" in raw_logs["native-cmath-o0-o3"]
            and b"B4-3-NATIVE" in raw_logs["linux-linux-native-cmath-o0-o3"],
            "正式Cmath C ABI原生O0/O3向量未实际运行")
    require(b"B4-4-NATIVE path=std.cmath optimization=0" in raw_logs["windows-native-cmath-wrappers-final"]
            and b"B4-4-NATIVE path=std.cmath optimization=3" in raw_logs["windows-native-cmath-wrappers-final"]
            and b"B4-4-NATIVE path=stdlib.cmath optimization=0" in raw_logs["windows-native-cmath-wrappers-final"]
            and b"B4-4-NATIVE path=stdlib.cmath optimization=3" in raw_logs["windows-native-cmath-wrappers-final"],
            "Windows Cmath包装原生矩阵不完整")
    linux_failure = raw_logs["linux-" + LINUX_FAIL]
    require(b"ArtifactVerification" in linux_failure and b"containers" in linux_failure,
            "Linux原生包装预期阻断身份不符")
    diagnostic = raw_logs["linux-linux-native-component-diagnostic-r2"]
    diagnostic_line = next((line for line in diagnostic.splitlines()
                            if b"B6-NATIVE-DIAGNOSTIC" in line), None)
    require(diagnostic_line is not None and
            b'declared=["rc", "value", "weak"]' in diagnostic_line and
            b"call i32 @xiao_runtime_array_new" in diagnostic_line and
            b'array_symbols=["xiao_runtime_array_new"]' in diagnostic_line,
            "Linux Runtime组件缺项诊断不符")

    result = load(root, "result-final-r5.json")
    expected = {
        "stage": "B6-1", "evidence_valid": True, "stage_passed": False,
        "status": "blocked", "b6_started": True, "b6_closed": False,
        "b6_2_started": False, "macos_status": "unverifiable",
        "linux_native_wrapper_exit": 101, "linux_error": "containers",
        "code_commit": CODE, "runtime_abi": "1.19", "codegen_version": 22,
    }
    for key, value in expected.items():
        require(result.get(key) == value, "B6阶段状态或平台阻断被改写：" + key)
    require(result["runtime_summaries"] == {"windows": windows_runtime, "linux": linux_runtime},
            "B6 Runtime原始摘要与收口结果不符")
    require(result["driver_vm_summaries"] == {
        "windows": windows_vm, "linux": linux_vm,
    }, "B6驱动回归摘要与收口结果不符")
    require(result["performance"]["status"] == "not_started"
            and result["performance"]["url_decode_unicode_full_escaped_maximum"]["measured"] is False,
            "B6性能工作被冒进登记")
    require(result["artifact_identities"] == load(root, "artifacts-final-r5.json"),
            "B6二进制和Runtime摘要与旁表不一致")
    require(result["artifact_identities"]["windows"]
            and any("libxiao_runtime.a" in name
                    for name in result["artifact_identities"]["linux"]),
            "Windows/Linux Runtime与实测产物身份清单不完整")
    require(result["macos"]["status"] == "unverifiable"
            and result["linux"]["native_cmath_wrappers_o0_o3"]["passed"] is False,
            "未验证或失败平台被冒进升级")
    return result


def record_source(root):
    # 采集器按受保护路径逐文件登记，LF摘要用于跨平台对照Git Blob。
    raw = record.git("ls-files", "-z", "--cached", "--others", "--exclude-standard",
                     "--", *PRODUCT_ROOTS)
    names = sorted(set(raw.decode("utf-8").split("\0")) - {""})
    output = {}
    for name in names:
        data = (REPOSITORY / name).read_bytes()
        output[name] = {"bytes": len(data), "sha256": sha(data),
                        "lf_sha256": sha(data.replace(b"\r\n", b"\n"))}
    return output


def audit(root, repository, prepare):
    before = inventory(root)
    for name in (".gitattributes", "README.md", "result-final-r5.json", "artifacts-final-r5.json"):
        require(name in before, "B6证据必需文件缺失：" + name)
    if not prepare:
        manifest = load(root, "evidence.json")
        expected_files = {name: row for name, row in before.items()
                          if name != "evidence.json"}
        require(manifest["file_count_excluding_manifest"] == len(expected_files),
                "B6清单文件数与实际字节不符")
        require(manifest["files"] == expected_files, "B6清单与实际字节不符")
        committed = git(repository, "log", "-1", "--format=%H", "--",
                        PREFIX + "/evidence.json").decode().strip()
        require(committed and git(repository, "show", committed + ":" + PREFIX +
                                  "/evidence.json") == (root / "evidence.json").read_bytes(),
                "B6固定Git清单字节不符")
    result = recompute(root, repository)
    if not prepare:
        docs = load(root, "docs-final-r9.json")
        require(docs["exit_code"] == 0 and docs["source_unchanged"]
                and docs["argv"][1:] ==
                ["tools/repo-check/src/cli.ts", "all", "--format", "json"],
                "B6定稿后文档门禁未通过")
        report = json.loads(stored(root, docs["log"]))
        require(report["passed"] and not any(
            row["severity"] == "error" for row in report["diagnostics"]),
            "B6文档门禁实际报告未通过")
        docs_inputs = json.loads(stored(root, docs["docs_before"]))
        require(docs_inputs == json.loads(stored(root, docs["docs_after"])),
                "B6文档门禁前后字节变化")
        required = set(DOC_FILES) | {
            PREFIX + "/" + path.relative_to(root).as_posix()
            for path in root.rglob("*")
            if path.is_file()
            and path.relative_to(root).parts[0] != "objects"
            and not path.name.startswith("docs-final")
            and path.name != "evidence.json"
        }
        require(set(docs_inputs) == required, "B6文档门禁输入集合不全")
        commit = git(repository, "log", "-1", "--format=%H", "--",
                     PREFIX + "/evidence.json").decode().strip()
        blobs = source_revision(repository, commit, tuple(docs_inputs))
        for name, content in blobs.items():
            binary = docs_inputs[name].get("binary", False)
            canonical_content = content if binary else content.replace(b"\r\n", b"\n")
            require(docs_inputs[name]["lf_sha256"] == sha(canonical_content),
                    "B6门禁输入偏离固定Git对象：" + name)
        review = load(root, "review-final-r8.json")
        require(review["passed"] and
                [row["case"] for row in review["cases"]]
                == ["baseline", "missing", "status", "linux"]
                and all(row["read_only"] for row in review["cases"]),
                "B6封存前自反例集合不完整")
    return {
        "passed": True, "read_only": True, "sealed": not prepare,
        "stage": "B6-1", "stage_passed": False, "b6_status": "blocked",
        "b6_2_started": False, "windows_functional_passed": True,
        "linux_native_wrapper_passed": False, "macos": "unverifiable",
        "code_commit": CODE,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--prepare", action="store_true")
    args = parser.parse_args()
    root, repository = args.root.resolve(), args.repository_root.resolve()
    before = inventory(root)
    try:
        outcome, code = audit(root, repository, args.prepare), 0
    except Exception as error:
        outcome, code = {"passed": False, "reason": str(error)}, 1
    outcome["read_only"] = inventory(root) == before
    if not outcome["read_only"]:
        outcome, code = {"passed": False, "reason": "B6审计修改了证据", "read_only": False}, 1
    sys.stdout.reconfigure(encoding="utf-8")
    print(json.dumps(outcome, ensure_ascii=False))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
