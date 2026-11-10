"""只读审计固定 B3-4 planned 源码包装收口；不会写清单或 closeout。"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
ENTRY = "89d782e13f00776b90193f1719b4454c54367db7"
CODE = "7d8b85e4d8c588b40ec76d641edc865481295032"
PRODUCT_ROOTS = [
    "core/rust", "stdlib", "tests/fixtures", "tests/benchmarks", ".github/workflows",
    ".cargo", "tools", "cli/ts", "docs/UseDocs", "docs/module-registry.json",
    "package.json", "bun.lock", ".gitattributes",
]
ALLOWED_PATHS = {
    "core/rust/crates/xiao-codegen-llvm/src/dynamic.rs",
    "core/rust/crates/xiao-codegen-llvm/src/dynamic/README.md",
    "core/rust/crates/xiao-codegen-llvm/src/dynamic/imports.rs",
    "core/rust/crates/xiao-codegen-llvm/src/dynamic_architecture_tests.rs",
    "core/rust/crates/xiao-codegen-llvm/tests/stdlib_bindings.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b2_divide.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b3_wrapper.rs",
    "core/rust/crates/xiao-driver/tests/22c_vm_concurrency.rs",
    "core/rust/crates/xiao-vm/src/semantics/concurrent.rs",
    "docs/UseDocs/library/README.md",
    "docs/UseDocs/library/cmath.md",
    "stdlib/README.md",
    "stdlib/cmath.md",
    "stdlib/cmath.xiao",
}
RUNS = {
    "toolchain": ("toolchain", ["cargo", "+1.96.0", "--version"]),
    "wrapper": ("b3-4", ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-driver", "--test", "20o_b2_b3_wrapper", "--", "--nocapture"]),
    "concurrency": ("b3-4", ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-driver", "--test", "22c_vm_concurrency", "--", "--nocapture"]),
    "codegen": ("b3-4", ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-codegen-llvm"]),
    "archive_wiring": ("b3-4", ["cargo", "+1.96.0", "test", "--locked", "--manifest-path", "core/rust/Cargo.toml", "-p", "xiao-driver", "--test", "20e_archive_wiring", "--", "--nocapture"]),
    "b1_history": ("historical", ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/entry-20261108-b1/audit.py"]),
    "b2_history": ("historical", ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/entry-20261108-b2/audit.py"]),
    "b3_cabi_history": ("historical", ["python", "-B", "-X", "utf8", "docs/DevDocs/assets/20o-b2-b/entry-20261109-b3-cabi/audit.py"]),
    "workspace_tests_final": ("workspace", ["cargo", "+1.96.0", "test", "--manifest-path", "core/rust/Cargo.toml", "--workspace", "--locked"]),
    "clippy": ("gate", ["cargo", "+1.96.0", "clippy", "--manifest-path", "core/rust/Cargo.toml", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"]),
    "fmt": ("gate", ["cargo", "+1.96.0", "fmt", "--manifest-path", "core/rust/Cargo.toml", "--all", "--", "--check"]),
    "lock_check": ("gate", ["bun", "run", "check:lock"]),
    "repo_all": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_all_closeout": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_all_final": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_all_authoritative": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_all_final_final": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_all_authoritative_v2": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
    "repo_all_review_final": ("gate", ["bun", "tools/repo-check/src/cli.ts", "all"]),
}
SUMMARY = re.compile(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", re.M)
FROZEN = {
    "sqrt": ["value"], "floor": ["value"], "ceil": ["value"], "round": ["value"],
    "trunc": ["value"], "abs": ["value"], "min": ["left", "right"],
    "max": ["left", "right"], "pow": ["base", "exponent"], "sin": ["value"],
    "cos": ["value"], "tan": ["value"], "exp": ["value"], "ln": ["value"],
    "log10": ["value"],
}


def fail(message):
    raise SystemExit(message)


def require(condition, message):
    if not condition:
        fail(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(data):
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        return data
    return data if b"\0" in data else data.replace(b"\r\n", b"\n")


def git(repository, *args):
    return subprocess.check_output(["cmd", "/c", "git", *args], cwd=repository)


def evidence_inventory(root):
    rows = []
    for path in sorted(item for item in root.rglob("*") if item.is_file()):
        if path.name == "evidence.json" or "__pycache__" in path.parts:
            continue
        data = path.read_bytes()
        rows.append({"path": path.relative_to(root).as_posix(), "bytes": len(data), "sha256": sha(data)})
    return rows


def product_blobs(repository, revision):
    raw = git(repository, "ls-tree", "-r", "-z", "--full-tree", revision)
    entries = []
    roots = [item.encode("utf-8") for item in PRODUCT_ROOTS]
    for item in raw.split(b"\0"):
        if not item:
            continue
        metadata, name = item.split(b"\t", 1)
        mode, kind, object_id = metadata.split(b" ", 2)
        if kind != b"blob" or not any(name == root or name.startswith(root + b"/") for root in roots):
            continue
        entries.append((name.decode("utf-8"), object_id))
    request = b"".join(object_id + b"\n" for _, object_id in entries)
    output = subprocess.check_output(["cmd", "/c", "git", "cat-file", "--batch"],
                                     cwd=repository, input=request)
    blobs, offset = {}, 0
    for name, object_id in entries:
        end = output.index(b"\n", offset)
        header = output[offset:end].split()
        require(len(header) == 3 and header[0] == object_id and header[1] == b"blob",
                "固定Git树Blob响应不符：" + name)
        size = int(header[2])
        start = end + 1
        blobs[name] = output[start:start + size]
        offset = start + size + 1
    require(offset == len(output), "固定Git树Blob流长度不符")
    return blobs


def product_snapshot(repository, revision):
    blobs = product_blobs(repository, revision)
    files = []
    for name, data in sorted(blobs.items()):
        files.append({"path": name, "bytes": len(data), "raw_sha256": sha(data),
                      "utf8_lf_or_binary_sha256": sha(canonical(data))})
    raw_tree = sha(json.dumps(files, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode())
    lf_tree = sha(json.dumps([[row["path"], row["utf8_lf_or_binary_sha256"]] for row in files],
                             ensure_ascii=False, separators=(",", ":")).encode())
    locks = {}
    for name in ["core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock"]:
        data = blobs[name]
        locks[name] = {"bytes": len(data), "sha256": sha(data)}
    return {"raw_tree_sha256": raw_tree, "utf8_lf_tree_sha256": lf_tree,
            "files": len(files), "product_files": files, "locks": locks}


def validate_captured_source(captured, committed):
    files = captured["product_files"]
    raw_tree = sha(json.dumps(files, ensure_ascii=False, sort_keys=True,
                              separators=(",", ":")).encode())
    normalized_tree = sha(json.dumps(
        [[row["path"], row["utf8_lf_or_binary_sha256"]] for row in files],
        ensure_ascii=False, separators=(",", ":")).encode())
    require(captured["files"] == committed["files"]
            and normalized_tree == captured["utf8_lf_tree_sha256"]
            and normalized_tree == committed["utf8_lf_tree_sha256"],
            "B3-4执行身份与固定Git对象的规范化产品树不符")
    committed_files = {row["path"]: row["utf8_lf_or_binary_sha256"]
                       for row in committed["product_files"]}
    require(set(row["path"] for row in files) == set(committed_files)
            and all(row["utf8_lf_or_binary_sha256"] == committed_files[row["path"]]
                    for row in files),
            "B3-4执行身份与固定Git Blob逐路径摘要不符")
    require(raw_tree == captured["raw_tree_sha256"], "B3-4执行记录的原始树摘要自洽性失败")
    captured_files = {row["path"]: row for row in files}
    require(set(captured["locks"]) == {"core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock"},
            "B3-4执行记录锁文件集合不符")
    for name, lock in captured["locks"].items():
        row = captured_files.get(name)
        require(row is not None and row["bytes"] == lock["bytes"]
                and row["raw_sha256"] == lock["sha256"],
                "B3-4执行记录的原始Cargo.lock摘要不自洽：" + name)


def read_run(root, repository, key):
    phase, argv = RUNS[key]
    record_path, log_path = root / (key + ".json"), root / (key + ".log")
    record = json.loads(record_path.read_text(encoding="utf-8"))
    data = log_path.read_bytes()
    log = data.decode("utf-8", errors="strict")
    require(record["phase"] == phase and record["label"] == key and record["argv"] == argv,
            "B3-4命令角色或argv不符：" + key)
    bootstrap = json.loads((root / "bootstrap.json").read_text(encoding="utf-8"))
    require(record["cwd"] == bootstrap["repository_root"] and record["log"] == log_path.name and record["pid"] > 0,
            "B3-4工作目录/PID/日志路径不符：" + key)
    require(record["exit_code"] == 0 and record["source_unchanged"] is True
            and record["source_before"] == record["source_after"], "B3-4执行失败或源码变化：" + key)
    require(record["log_bytes"] == len(data) and record["log_sha256"] == sha(data), "B3-4原始日志摘要不符：" + key)
    require(record["runner_sha256"] == sha((root / "run-step.py").read_bytes()), "B3-4执行器身份不符：" + key)
    require(record["environment_sha256"] == sha((root / "environment.json").read_bytes()), "B3-4环境身份不符：" + key)
    require(record["task_root"] == bootstrap["task_root"] and record["runtime_library"] is None,
            "B3-4复用或声明了外部Runtime：" + key)
    return record, log


def check_static_contract(repository):
    entry = git(repository, "rev-parse", ENTRY).decode("ascii").strip()
    code = git(repository, "rev-parse", CODE).decode("ascii").strip()
    check = subprocess.run(["cmd", "/c", "git", "merge-base", "--is-ancestor", entry, code], cwd=repository)
    require(check.returncode == 0, "B3-CABI入口不在B3-4历史前")
    changed = set(git(repository, "diff", "--name-only", entry, code, "--", *PRODUCT_ROOTS).decode("utf-8").splitlines())
    require(changed == ALLOWED_PATHS, "B3-4相对B3-CABI基线的产品路径集合偏离冻结范围")
    entry_blobs = product_blobs(repository, entry)
    code_blobs = product_blobs(repository, code)
    require(set(ALLOWED_PATHS) <= set(code_blobs), "B3-4固定代码树缺少登记产品路径")
    for name in ["core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock"]:
        require(entry_blobs[name] == code_blobs[name],
                "B3-4不得改动锁文件：" + name)
    for forbidden in ["core/rust/crates/xiao-types", "core/rust/crates/xiao-ir",
                      "core/rust/crates/xiao-bytecode", "core/rust/crates/xiao-runtime-abi",
                      "core/rust/crates/xiao-intrinsics"]:
        require(not any(path.startswith(forbidden + "/") for path in changed), "B3-4改动了语言绑定/ABI范围：" + forbidden)

    codegen = code_blobs["core/rust/crates/xiao-codegen-llvm/src/lib.rs"].decode("utf-8")
    abi_source = code_blobs["core/rust/crates/xiao-runtime-abi/src/lib.rs"].decode("utf-8")
    require(re.search(r"pub const CODEGEN_VERSION:\s*u32\s*=\s*20;", codegen) is not None, "B3-4生成器版本不是20")
    require(re.search(r"ABI_MINOR_VERSION[^=]*=\s*18", abi_source) is not None, "B3-4 ABI不是1.18")
    intrinsics = json.loads(code_blobs["core/rust/crates/xiao-intrinsics/intrinsics.json"].decode("utf-8"))["intrinsics"]
    require(sorted(row["id"] for row in intrinsics if row["status"] == "active") == list(range(1, 49)),
            "B3-4 intrinsic active范围偏离固定内部状态")
    require(next((row for row in intrinsics if row["id"] == 100 and row["status"] == "tombstone"), None) is not None,
            "B3-4 ID100墓碑变化")
    registry = json.loads(code_blobs["docs/module-registry.json"].decode("utf-8"))
    module = next(row for row in registry["modules"] if row["id"] == "stdlib.cmath")
    require(module["status"] == "planned" and module["code"] == [] and module["tests"] == [],
            "cmath registry必须planned且不冒充产品测试")
    tutorial = code_blobs["docs/UseDocs/library/cmath.md"].decode("utf-8").replace("\r\n", "\n")
    front = tutorial.split("\n---\n", 1)[0]
    require("status: planned\n" in front, "cmath教程不再planned")
    wrapper = code_blobs["stdlib/cmath.xiao"].decode("utf-8")
    require(len(re.findall(r"^def ", wrapper, re.M)) == 15, "cmath包装不是恰好15项")
    require(not any(name.startswith("tests/fixtures/docs/cmath/") for name in code_blobs),
            "B3-4不迁移B4正式fixture")
    return {"entry_commit": entry, "code_commit": code, "head_commit": None,
            "allowed_product_paths": sorted(ALLOWED_PATHS), "source": product_snapshot(repository, code),
            "runtime_abi": "1.18", "codegen_version": 20, "stdlib_interface": 1,
            "active_intrinsics": [1, 48], "cmath_status": "planned"}


def validate(root, repository, require_closeout=False):
    attribute = (root / ".gitattributes").read_text(encoding="utf-8")
    require(attribute == "* -text\n*.log -diff\n", "B3-4证据目录换行/日志属性不符")
    static = check_static_contract(repository)
    records, logs = {}, {}
    first_attempt = json.loads((root / "workspace_tests.json").read_text(encoding="utf-8"))
    first_log_bytes = (root / first_attempt["log"]).read_bytes()
    require(first_attempt["label"] == "workspace_tests" and first_attempt["exit_code"] == 101
            and first_attempt["argv"] == ["cargo", "+1.96.0", "test", "--workspace", "--locked"]
            and hashlib.sha256(first_log_bytes).hexdigest() == first_attempt["log_sha256"]
            and "could not find `Cargo.toml`" in first_log_bytes.decode("utf-8", errors="strict"),
            "workspace根目录错误的保留首轮记录与实际原因不符")
    for key in RUNS:
        records[key], logs[key] = read_run(root, repository, key)
        require(records[key]["source_before"] == records["toolchain"]["source_before"], "B3-4执行点源码身份不同：" + key)
    validate_captured_source(records["toolchain"]["source_before"], static["source"])
    wrapper = logs["wrapper"]
    require(re.search(r"test result: ok\. 8 passed; 0 failed; 0 ignored;", wrapper), "B3-4 wrapper不是8/0")
    require(wrapper.count("B3-WRAPPER-EXPORT\t") == 15 and "removals=15" in wrapper and "mutation_cases=10" in wrapper,
            "B3-4源码AST/移除/变异契约缺失")
    imports = [line for line in wrapper.splitlines() if "B3-WRAPPER-IMPORT " in line]
    require(len(imports) == 30 and all("executable=false" in line and "result=planned-rejected" in line for line in imports),
            "30条cmath路径必须全部明确planned拒绝")
    require("existing_hex_import_keeps_native_lowering ... ok" in wrapper
            and "unimported_module_cannot_pull_cmath_into_hex ... ok" in wrapper
            and "B3-CMATH-REACHABILITY root=stdlib.hex unreachable=stdlib.path dependency=stdlib.cmath result=success" in wrapper,
            "hex或不可达模块隔离回归缺失")
    require("test result: ok. 12 passed; 0 failed; 3 ignored;" in logs["workspace_tests_final"],
            "既有20C模块回归计数不符")
    concurrency = logs["concurrency"]
    require(re.search(r"test result: ok\. 29 passed; 0 failed; 0 ignored;", concurrency), "22C不是29/0")
    require("B3-CMATH-SPAWN path=std.cmath identity=stdlib.cmath result=planned-rejected" in concurrency
            and "B3-CMATH-SPAWN path=stdlib.cmath identity=stdlib.cmath result=planned-rejected" in concurrency
            and "B3-CMATH-XAR identity=stdlib.cmath aliases=std.cmath result=planned-rejected" in concurrency,
            "子VM/cmath归档别名边界缺失")
    require("B3-STDLIB-XAR identity=stdlib.text aliases=std.text index=success execution=planned-rejected" in concurrency,
            "归档标准库别名检查缺失")
    require(re.search(r"test result: ok\. 11 passed; 0 failed; 0 ignored;", logs["archive_wiring"]),
            "20E归档门禁计数不符")
    codegen = logs["codegen"]
    codegen_summaries = SUMMARY.findall(codegen)
    require(codegen_summaries and all(failed == "0" for _, failed, _ in codegen_summaries), "LLVM回归出现失败")
    require("test result: ok. 56 passed; 0 failed; 0 ignored;" in codegen
            and "test result: ok. 4 passed; 0 failed; 0 ignored;" in codegen,
            "LLVM单元或stdlib绑定测试计数不符")
    require(all(name + " ... ok" in codegen for name in [
        "unreachable_module_cannot_pull_in_planned_cmath",
        "transitive_imports_bind_dependencies_and_terminate_cycles",
        "namespace_selection_does_not_expand_unselected_siblings"]), "LLVM导入可达闭包用例缺失")
    require('"checks": 22' in logs["b3_cabi_history"] and '"code_commit": "89d782e1"' in logs["b3_cabi_history"],
            "B3-CABI固定历史审计不是22项通过")
    require('"passed": 18' in logs["b1_history"] and '"total": 18' in logs["b1_history"], "B1历史审计未过18/18")
    require('"checks": 14' in logs["b2_history"] and '"exact_checks": 4' in logs["b2_history"], "B2历史审计未核对14项/4精确目标")
    require("test result: FAILED" not in logs["workspace_tests_final"]
            and all(failed == "0" for _, failed, _ in SUMMARY.findall(logs["workspace_tests_final"])),
            "Rust workspace测试出现失败")
    require("Finished `dev` profile" in logs["clippy"] and "error:" not in logs["clippy"], "workspace Clippy失败")
    require(all(records[name]["exit_code"] == 0 and "[error]" not in logs[name]
                for name in ["repo_all", "repo_all_final", "repo_all_closeout", "repo_all_authoritative", "repo_all_final_final", "repo_all_authoritative_v2", "repo_all_review_final"]),
            "repo docs/usedocs/layout门禁失败")
    require(records["fmt"]["exit_code"] == 0, "cargo fmt失败")
    require(records["lock_check"]["exit_code"] == 0 and "error:" not in logs["lock_check"], "cargo lock gate失败")
    handoff = (root / "handoff-b4.md").read_text(encoding="utf-8")
    for index, (name, parameters) in enumerate(FROZEN.items()):
        expected = f"| {name} | {', '.join(parameters)} | {34 + index} |"
        require(expected in handoff, "B4交接函数/ID映射不符：" + name)
    if require_closeout:
        closeout = json.loads((root / "closeout.json").read_text(encoding="utf-8"))
        require(closeout["passed"] is True and closeout["b3_4_closed"] is True
                and closeout["language_available"] is False, "B3-4 closeout状态不符")
        require(closeout["cmath_status"] == closeout["registry_status"] == closeout["tutorial_status"] == "planned"
                and closeout["old_721_executed"] is False
                and closeout["historical_debts_touched"] is False
                and closeout["remote_ci_claimed"] is False
                and closeout["three_platform_public_product"] is False,
                "B3-4 closeout越过planned/历史/平台边界")
        require(closeout["gates"]["repo_all_review_final"]["exit_code"] == 0
                and closeout["gates"]["workspace_tests_final"]["exit_code"] == 0,
                "closeout未引用最终workspace/docs门禁")
        evidence = json.loads((root / "evidence.json").read_text(encoding="utf-8"))
        require(evidence["files"] == evidence_inventory(root), "B3-4证据清单与实际文件字节不符")
        require(evidence["scope"] == "B3-4 source-only planned wrapper closeout; language unavailable",
                "B3-4证据范围被扩大")
        negative = json.loads((root / "self-negative-result-final.json").read_text(encoding="utf-8"))
        require(negative.get("passed") is True and negative.get("read_only") is True
                and len(negative.get("cases", [])) == 3
                and {row.get("case") for row in negative["cases"]}
                == {"baseline-read-only", "unlisted-file", "language-status-overclaim"}
                and negative["cases"][1].get("expected_reason") == "B3-4证据清单与实际文件字节不符"
                and negative["cases"][2].get("expected_reason") == "B3-4 closeout状态不符",
                "B3-4自反例回执缺失或被改写")
    return {"passed": True, "source": static["source"], "identity": static,
            "test_summaries": {key: [list(row) for row in SUMMARY.findall(value)]
                               for key, value in logs.items()},
            "status": {"b3_4_closed": require_closeout, "language_available": False,
                       "cmath": "planned", "three_platform": False, "remote_ci": False}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--prepare", action="store_true", help="预收口核验；允许尚无清单")
    args = parser.parse_args()
    result = validate(args.root.resolve(), args.repository_root.resolve(), require_closeout=not args.prepare)
    print(json.dumps({"passed": True, "scope": "B3-4源码包装收口；非语言可用性",
                      "code_commit": result["identity"]["code_commit"],
                      "product_files": result["source"]["files"], "b3_4_closed": not args.prepare,
                      "language_available": False, "cmath": "planned",
                      "identity_source": "fixed Git commit blobs; canonical UTF-8/LF hashes"},
                     ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
