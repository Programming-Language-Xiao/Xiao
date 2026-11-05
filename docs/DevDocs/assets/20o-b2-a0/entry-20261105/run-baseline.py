"""A0 Windows 基线执行器；只运行既有目标，使用已创建的独立 worktree。"""

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import threading
import time


def now():
    return dt.datetime.now(dt.timezone.utc).isoformat()


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def git(root, *args):
    return subprocess.check_output(
        ["cmd", "/c", "git", *args], cwd=root, encoding="utf-8"
    ).strip()


def file_identity(root, relative):
    path = root / relative
    return {"path": relative, "bytes": path.stat().st_size, "sha256": digest(path)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task-root", type=Path, required=True)
    parser.add_argument("--evidence-root", type=Path, required=True)
    args = parser.parse_args()
    task = args.task_root.resolve()
    evidence = args.evidence_root.resolve()
    bootstrap = json.loads((task / "bootstrap.json").read_text(encoding="utf-8"))
    source = Path(bootstrap["source_root"])
    repository = Path(bootstrap["repository_root"])
    assert git(source, "rev-parse", "HEAD") == bootstrap["head"]
    assert not git(source, "status", "--porcelain=v1")
    assert not any((task / "cargo-target").iterdir()), "必须使用全新空 target"
    assert not any((task / "xiao-home").iterdir()), "必须使用全新 Xiao 缓存"
    assert not (evidence / "commands.json").exists(), "不覆盖旧运行记录"
    evidence.mkdir(parents=True, exist_ok=True)
    (evidence / "logs").mkdir(exist_ok=True)
    shutil.copy2(task / "bootstrap.json", evidence / "bootstrap.json")
    shutil.copy2(task / "worktree-add.log", evidence / "logs/worktree-add.log")

    vcvars = Path(r"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat")
    env_batch = task / "msvc-environment.cmd"
    env_batch.write_text(
        '@echo off\ncall "' + str(vcvars) + '" >nul\nif errorlevel 1 exit /b %errorlevel%\nset\n',
        encoding="utf-8",
    )
    # set 的完整输出仅在内存中解析，绝不落盘或输出，以免记录无关凭据。
    captured = subprocess.check_output(["cmd", "/d", "/c", str(env_batch)], text=True)
    msvc_names = {
        "PATH", "LIB", "LIBPATH", "INCLUDE", "VCTOOLSINSTALLDIR", "VCTOOLSVERSION",
        "VSINSTALLDIR", "VCINSTALLDIR", "WINDOWSSDKDIR", "WINDOWSSDKVERSION",
        "UNIVERSALCRTSDKDIR", "UCRTVERSION", "VSCMD_ARG_HOST_ARCH", "VSCMD_ARG_TGT_ARCH",
    }
    env = {k: v for k, v in os.environ.items() if not k.startswith("XIAO_")}
    selected = {}
    for line in captured.splitlines():
        name, separator, value = line.partition("=")
        if separator and name.upper() in msvc_names:
            selected[name.upper()] = value
            env[name.upper()] = value
    assert selected.get("LIB") and selected.get("INCLUDE"), "MSVC 环境导入失败"
    cargo = Path(r"D:\Program Files\.cargo\bin\cargo.exe")
    rustup = cargo.with_name("rustup.exe")
    rustc = subprocess.check_output(
        [str(rustup), "which", "--toolchain", "1.96.0", "rustc"], text=True, env=env
    ).strip()
    bun = Path(r"C:\Users\xiaoc\node_modules\bun\bin\bun.exe")
    pwsh = Path(r"D:\Program Files\PowerShell\7\pwsh.exe")
    llvm = Path(r"D:\msys64\ucrt64\bin")
    bash = Path(r"D:\msys64\usr\bin\bash.exe")
    overrides = {
        "CARGO_TARGET_DIR": str(task / "cargo-target"),
        "RUSTUP_TOOLCHAIN": "1.96.0", "RUSTC": rustc,
        "XIAO_HOME": str(task / "xiao-home"),
        "TEMP": str(task / "run-temp"), "TMP": str(task / "run-temp"),
        "XIAO_STDLIB_ROOT": str(source / "stdlib"),
        "XIAO_TARGET_TRIPLE": "x86_64-pc-windows-msvc",
        "XIAO_CLANG": str(llvm / "clang.exe"),
        "XIAO_LLVM_AS": str(llvm / "llvm-as.exe"),
        "XIAO_LLC": str(llvm / "llc.exe"),
        "XIAO_RUNTIME_LIBRARY": str(task / "cargo-target/release/xiao_runtime.lib"),
        "XIAO_DIAGNOSTICS_PATH": str(task / "cargo-target/debug/xiao-diagnostics.exe"),
        "XIAO_DIAGNOSTICS_HOLD_MS": "0",
        "XIAO_RUST_DOC_ADAPTER": str(task / "cargo-target/debug/xiao-doc-coverage-rust.exe"),
    }
    env.update(overrides)
    env["PATH"] = str(llvm) + os.pathsep + env["PATH"]
    selected["PATH"] = env["PATH"]
    write_json(evidence / "environment.json", {
        "scope": "仅记录本次显式覆盖及 MSVC 白名单，其他继承环境不落盘",
        "overrides": overrides, "msvc": selected, "vcvars": str(vcvars),
        "cargo_config": ".cargo/config.toml: jobs=4, incremental=false, RUST_TEST_THREADS=4",
        "fresh_target_before_first_build": True, "fresh_xiao_home": True,
        "cargo_home_policy": "可复用 Cargo 下载缓存；不复用任何 Rust 构建产物或 Xiao 原生缓存",
    })

    node_modules = source / "tools/doc-coverage/node_modules"
    node_source = repository / "tools/doc-coverage/node_modules"
    # 实体复制同一 JS 依赖；仓库目录门禁禁止 codeRoot 中的链接越出源码根。
    node_modules.mkdir(parents=True, exist_ok=False)
    shutil.copytree((node_source / "typescript").resolve(), node_modules / "typescript")
    assert json.loads((node_modules / "typescript/package.json").read_text(encoding="utf-8"))["version"] == "5.9.3"
    write_json(evidence / "javascript-dependency.json", {
        "kind": "physical_copy_existing_javascript_dependency", "copy_root": str(node_modules),
        "source": str(node_source), "typescript_version": "5.9.3",
        "files": [file_identity(node_source, name) for name in
                  ["typescript/package.json", "typescript/lib/typescript.js"]],
    })

    signed_audit = json.loads((source / "docs/DevDocs/assets/20o-b2-a/entry-20261105/signature-and-scope-audit.json").read_text(encoding="utf-8"))
    signed_checks = []
    for item in signed_audit["signed_objects"]:
        raw = (source / item["file"]).read_bytes()
        normalized = hashlib.sha256(raw.replace(b"\r\n", b"\n")).hexdigest()
        assert normalized == item["sha256"], item["file"]
        signed_checks.append({**item, "raw_checkout_sha256": hashlib.sha256(raw).hexdigest(),
                              "utf8_lf_sha256": normalized, "passed": True})
    draft = (source / "docs/DevDocs/20o-b2-freeze-draft.md").read_text(encoding="utf-8").replace("\r\n", "\n")
    lines = draft.splitlines(keepends=True)
    assert lines[2].startswith("状态：")
    body_hash = hashlib.sha256("".join(lines[:2] + lines[3:]).encode()).hexdigest()
    assert body_hash == signed_audit["signed_body_sha256"]
    ledger = json.loads((source / signed_audit["signed_objects"][1]["file"]).read_text(encoding="utf-8"))
    items_hash = hashlib.sha256(json.dumps(ledger["items"], ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    assert items_hash == signed_audit["signed_ledger_items_sha256"]
    abi = (source / "core/rust/crates/xiao-runtime-abi/src/lib.rs").read_text(encoding="utf-8")
    codegen = (source / "core/rust/crates/xiao-codegen-llvm/src/lib.rs").read_text(encoding="utf-8")
    intrinsic_table = json.loads((source / "core/rust/crates/xiao-intrinsics/intrinsics.json").read_text(encoding="utf-8"))["intrinsics"]
    registry = json.loads((source / "docs/module-registry.json").read_text(encoding="utf-8"))["modules"]
    assert "ABI_MAJOR_VERSION: u32 = 1;" in abi and "ABI_MINOR_VERSION: u32 = 18;" in abi
    assert "CODEGEN_VERSION: u32 = 20;" in codegen
    assert max(row["id"] for row in intrinsic_table if row["status"] == "active") == 32
    assert [row["status"] for row in intrinsic_table if row["id"] == 100] == ["tombstone"]
    assert [row["status"] for row in registry if row["id"] == "stdlib.cmath"] == ["planned"]
    tracked_files = [
        "core/rust/Cargo.toml", "core/rust/Cargo.lock", "core/rust/rust-toolchain.toml",
        "tests/benchmarks/Cargo.toml", "tests/benchmarks/Cargo.lock", ".cargo/config.toml",
        "core/rust/crates/xiao-runtime-abi/src/lib.rs", "core/rust/crates/xiao-codegen-llvm/src/lib.rs",
        "core/rust/crates/xiao-intrinsics/intrinsics.json", "docs/module-registry.json",
        "docs/DevDocs/assets/20o-b2-a/entry-20261105/decision-receipts.json",
        "docs/DevDocs/assets/20o-b2-a/review-fix-20261105/approval-conversation-source.json",
    ] + [item["file"] for item in signed_audit["signed_objects"]] + [
        "stdlib/" + name + ".xiao" for name in ["math", "hex", "base64", "url", "csv", "fmt", "statistics"]
    ]
    protected = [".cargo", "core/rust", "stdlib", "tests/fixtures", "tests/benchmarks",
                 ".github/workflows", "tools", "docs/UseDocs", "docs/module-registry.json"]
    identity = {
        **bootstrap, "worktree_status_before": git(source, "status", "--porcelain=v1"),
        "signed_body_sha256": body_hash, "ledger_items_sha256": items_hash,
        "signed_object_checks": signed_checks,
        "version_boundary": {"abi": "1.18", "codegen": 20, "active_intrinsic_max": 32,
                             "intrinsic_100": "tombstone", "cmath": "planned"},
        "files": [file_identity(source, name) for name in tracked_files],
        "protected_git_objects": {name: git(source, "rev-parse", "HEAD:" + name) for name in protected},
        "platform": "Windows x86_64-pc-windows-msvc", "linux": "未验证", "macos": "未验证",
    }
    write_json(evidence / "identity.json", identity)

    records = []

    def run(name, argv, expected_tests=None):
        log_path = evidence / "logs" / (name + ".log")
        entry = {"name": name, "argv": [str(a) for a in argv], "cwd": str(source),
                 "environment": "environment.json", "started_utc": now(),
                 "log": "logs/" + log_path.name}
        print("开始 " + name, flush=True)
        started = time.monotonic()
        with log_path.open("wb") as output:
            process = subprocess.Popen(entry["argv"], cwd=source, env=env, stdout=output, stderr=subprocess.STDOUT)
            entry["pid"] = process.pid
            records.append(entry)
            write_json(evidence / "commands.json", records)
            entry["exit_code"] = process.wait()
        entry.update(finished_utc=now(), duration_seconds=round(time.monotonic() - started, 3), sha256=digest(log_path))
        entry["passed"] = entry["exit_code"] == 0
        if expected_tests is not None:
            summaries = re.findall(r"(?m)^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\r\n]*", log_path.read_text(encoding="utf-8", errors="replace"))
            entry["test_summaries"] = summaries
            entry["expected_single_summary_passed_count"] = expected_tests
            entry["passed"] &= len(summaries) == 1 and summaries[0][:4] == ("ok", str(expected_tests), "0", "0")
        write_json(evidence / "commands.json", records)
        print(f"结束 {name}: exit={entry['exit_code']} passed={entry['passed']} seconds={entry['duration_seconds']}", flush=True)
        return entry["passed"]

    native_artifacts = {}
    stop_observer = threading.Event()

    def observe_native():
        # 原测试在 Drop 中删除临时目录；只旁观并记录稳定文件摘要，不改清理语义。
        while not stop_observer.wait(0.2):
            for root in (task / "run-temp").glob("xiao-20o-b0-*"):
                for path in root.rglob("*"):
                    if path.suffix.lower() not in {".exe", ".ll", ".obj", ".json"}:
                        continue
                    key = str(path)
                    try:
                        stat = path.stat()
                        if key in native_artifacts and native_artifacts[key]["stamp"] == [stat.st_size, stat.st_mtime_ns]:
                            continue
                        sha = digest(path)
                        after = path.stat()
                        if stat.st_size == after.st_size and stat.st_mtime_ns == after.st_mtime_ns:
                            native_artifacts[key] = {"path": key, "bytes": stat.st_size, "sha256": sha,
                                                     "stamp": [stat.st_size, stat.st_mtime_ns], "observed_utc": now()}
                    except OSError:
                        continue

    cargo_args = [str(cargo), "+1.96.0"]
    manifest = ["--locked", "--manifest-path", "core/rust/Cargo.toml"]
    linker = shutil.which("link.exe", path=env["PATH"])
    assert linker is not None
    for name, argv in [
        ("rustc-version", [rustc, "-vV"]), ("cargo-version", cargo_args + ["--version"]),
        ("bun-version", [str(bun), "--version"]),
        ("powershell-version", [str(pwsh), "-NoProfile", "-Command", "$PSVersionTable | ConvertTo-Json"]),
        ("clang-version", [str(llvm / "clang.exe"), "--version"]),
        ("llvm-as-version", [str(llvm / "llvm-as.exe"), "--version"]),
        ("llc-version", [str(llvm / "llc.exe"), "--version"]),
        ("link-version", [str(pwsh), "-NoProfile", "-Command",
                          "(Get-Item -LiteralPath '" + linker.replace("'", "''") + "').VersionInfo | Select-Object FileVersion,ProductVersion,FileName | ConvertTo-Json"]),
    ]:
        if not run(name, argv):
            raise RuntimeError("工具链发现失败，见 " + name)
    prerequisites = [
        ("build-runtime", cargo_args + ["build", *manifest, "--release", "-p", "xiao-runtime"]),
        ("build-diagnostics", cargo_args + ["build", *manifest, "-p", "xiao-diagnostics"]),
        ("build-doc-adapter", cargo_args + ["build", *manifest, "-p", "xiao-doc-coverage-rust", "--bin", "xiao-doc-coverage-rust"]),
    ]
    for name, argv in prerequisites:
        if not run(name, argv):
            raise RuntimeError("既有构建失败，停止产品测试，见 " + name)
    write_json(evidence / "built-tools.json", [
        {"role": key, "path": overrides[key], "bytes": Path(overrides[key]).stat().st_size, "sha256": digest(overrides[key])}
        for key in ["XIAO_RUNTIME_LIBRARY", "XIAO_DIAGNOSTICS_PATH", "XIAO_RUST_DOC_ADAPTER"]
    ])
    for target, count in [("20o_b1_imp_contract", 9), ("20o_b1_imp_resources", 17)]:
        run(target, cargo_args + ["test", *manifest, "-p", "xiao-driver", "--test", target], count)
    probe = cargo_args + ["test", *manifest, "-p", "xiao-driver", "--test", "20o_b1_probe", "--"]
    run("vm-exact", probe + ["b1_capabilities_have_vm_golden_outputs", "--exact", "--nocapture"], 1)
    observer = threading.Thread(target=observe_native, daemon=True)
    observer.start()
    try:
        run("windows-native-exact", probe + ["--ignored", "b1_capabilities_match_native_golden_outputs", "--exact", "--nocapture"], 1)
    finally:
        stop_observer.set()
        observer.join()
        write_json(evidence / "native-artifacts.json", {
            "policy": "测试运行期间稳定文件的旁观摘要；临时产物由原测试清理，未归档二进制，非逐进程执行追踪",
            "temporary_root": overrides["TEMP"], "files": sorted(native_artifacts.values(), key=lambda item: item["path"]),
        })
    run("repo-all", [str(bun), "tools/repo-check/src/cli.ts", "all"])
    run("workspace-clippy", cargo_args + ["clippy", *manifest, "--workspace", "--all-targets", "--", "-D", "warnings"])
    run("workspace-check", cargo_args + ["check", *manifest, "--workspace"])
    run("benchmarks-check", cargo_args + ["check", "--locked", "--manifest-path", "tests/benchmarks/Cargo.toml"])
    run("lock-diff", ["cmd", "/c", "git", "diff", "--exit-code", "--", "core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock"])
    run("b1-guards", [str(pwsh), "-NoProfile", "-File", "tools/gates/b1-probe-guards.ps1", "-BashPath", str(bash), "-ReportPath", str(evidence / "b1-guards.json")])
    run("source-diff-check", ["cmd", "/c", "git", "diff", "--check"])
    status = git(source, "status", "--porcelain=v1")
    unchanged = all(digest(source / item["path"]) == item["sha256"] for item in identity["files"])
    guards = json.loads((evidence / "b1-guards.json").read_text(encoding="utf-8"))
    passed = all(item["passed"] for item in records) and not status and unchanged and guards["passed"] == guards["total"] == 28
    write_json(evidence / "result.json", {
        "source_commit": bootstrap["head"], "finished_utc": now(), "passed": passed,
        "commands": len(records), "source_status_after": status, "bound_files_unchanged": unchanged,
        "guard_passed": guards["passed"], "guard_total": guards["total"],
        "windows": "passed" if passed else "failed", "linux": "未验证", "macos": "未验证",
        "b1_721_executed": False, "b2_a_product_started": False, "a1_started": False,
        "bun_check_equivalence": "repo-all + workspace-clippy + workspace-check + benchmarks-check + cmd /c git lock-diff；全程 Cargo --locked",
    })
    print("A0 完成 passed=" + str(passed), flush=True)
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
