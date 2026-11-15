"""B4-4只读审计：固定Git对象、真实双路径重放、隔离环境与内容寻址闭包。"""
import argparse
from datetime import datetime
from functools import lru_cache
import json
from pathlib import Path, PureWindowsPath
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile

sys.dont_write_bytecode = True
from record import environment as BASE_ENV
from record import ROOT, REPOSITORY, PREFIX, PRODUCT_ROOTS, BASE, canonical, docs_snapshot, fixed_files, git, identity, load, require, sha, write_json
from suite import COMMANDS, CARGO, HISTORY

BASELINE = "c93f74654c5a956bd9dfbe2083b28dbbe6b52d6a"
FIRST_CODE = "e2f2422224c59a454e0a9a227d49f1ecd3e183ab"
PREVIOUS_CODE = "eb65a3b607e046656e7943e49e6e9c70f3007f3f"
CODE = "bb0b8196479e90d28d676f1bdefd04403e4ba6eb"
PREVIOUS = "fixed-final"
FINAL = "fixed-verified"
REVISIONS = {"baseline": BASELINE, "fixed": FIRST_CODE, PREVIOUS: PREVIOUS_CODE, FINAL: CODE}
RUNS = {"baseline": {"entry"}, "fixed": {"runtime-build", "native-library"}, PREVIOUS: set(COMMANDS), FINAL: set(COMMANDS)}
ALLOWED = {
    "core/rust/crates/xiao-driver/README.md", "core/rust/crates/xiao-driver/src/README.md",
    "core/rust/crates/xiao-driver/src/archive_closure.rs", "core/rust/crates/xiao-driver/src/archive_run.rs",
    "core/rust/crates/xiao-driver/src/lib.rs", "core/rust/crates/xiao-driver/src/module_loader.rs",
    "core/rust/crates/xiao-driver/src/protocol/run.rs", "core/rust/crates/xiao-driver/tests/20o_b2_b3_wrapper.rs",
    "core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_boundary.rs", "core/rust/crates/xiao-driver/tests/20o_b2_b4_cmath_library.rs",
    "core/rust/crates/xiao-driver/tests/22c_vm_concurrency.rs", "core/rust/crates/xiao-driver/tests/README.md",
    "core/rust/crates/xiao-driver/tests/support/b4_cmath_library_native.rs",
    "core/rust/crates/xiao-xar/README.md", "core/rust/crates/xiao-xar/src/runner.rs",
    "docs/UseDocs/library/README.md", "docs/UseDocs/library/cmath.md", "stdlib/README.md", "stdlib/cmath.md",
    "tests/fixtures/docs/numeric/README.md", "tests/fixtures/docs/cmath/README.md", "tests/fixtures/docs/numeric/cmath-library.stdout", "tests/fixtures/docs/numeric/cmath-library.xiao",
}
REASONS = {
    "manifest": "B4-4证据清单不符", "attributes": "B4-4证据属性缺失或变化",
    "source": "B4-4固定产品身份不符", "environment": "B4-4隔离环境或路径不符",
    "docs": "B4-4文档字节绑定不符", "runtime": "B4-4 Runtime身份不符",
    "state": "B4-4阶段或运行结果不符", "orphan": "B4-4存在未引用日志对象",
    "artifact": "B4-4真实产物摘要不符", "review": "B4-4自反例结果不符",
}


NEGATIVES = {k: v for k, v in REASONS.items() if k != "review"}


def inventory(root):
    result = {}
    for path in sorted(root.rglob("*")):
        require(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(), "证据不能含链接")
        require("__pycache__" not in path.parts, "证据不能含Python缓存")
        relative = path.relative_to(root).as_posix()
        if path.is_file() and relative != "evidence.json":
            raw = path.read_bytes()
            result[relative] = {"bytes": len(raw), "sha256": sha(raw)}
    return result


def all_entries(root):
    return {p.relative_to(root).as_posix(): sha(p.read_bytes()) if p.is_file() else "directory"
            for p in sorted(root.rglob("*"))}


@lru_cache(maxsize=None)
def source_at(repository, revision):
    return fixed_files(Path(repository), revision)


@lru_cache(maxsize=None)
def docs_at(repository, revision):
    return docs_snapshot(Path(repository), revision)


def counts(text):
    result, current = {}, None
    for line in text.splitlines():
        running = re.search(r"Running .*\((?:.*[\\/])([^\\/]+)-[0-9a-f]+\.exe\)", line)
        if running:
            current = running[1]
        if line.lstrip().startswith("Doc-tests "):
            current = line.split("Doc-tests ", 1)[1].strip() + ":doc"
        match = re.match(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", line)
        if match:
            require(current is not None and current not in result, "测试目标重复或缺失")
            result[current] = (int(match[1]), int(match[2]))
    return result


def expected_command(variant, label):
    if variant == "baseline":
        return CARGO + ["-p", "xiao-driver", "--test", "20o_b2_b3_wrapper", "--test",
                        "20o_b2_b4_cmath_boundary", "--test", "22c_vm_concurrency", "--", "--nocapture"]
    return COMMANDS[label]


def read_run(root, repository, variant, label, source, environment):
    row = load(root / variant / (label + ".json"))
    exit_code = 101 if variant == "fixed" and label == "native-library" else 1 if variant == PREVIOUS and label == "final-docs" else 0
    require(row["variant"] == variant and row["label"] == label and row["argv"] == expected_command(variant, label)
            and row["exit_code"] == exit_code, REASONS["state"])
    require(row["pid"] > 0 and datetime.fromisoformat(row["started_utc"]) < datetime.fromisoformat(row["finished_utc"])
            and row["cwd"] == environment["source_root"], REASONS["state"])
    require(row["environment_sha256"] == sha((root / variant / "environment.json").read_bytes()), REASONS["environment"])
    require(row["source_before"] == row["source_after"] == source["sha256"]
            and row["source_unchanged"] is True, REASONS["source"])
    commit = row["docs_commit"]
    docs = docs_at(str(repository), commit)
    require(row["docs_commit_tree"] == docs and row["docs_before"] == row["docs_after"]
            and row["docs_unchanged"] is True and row["docs_before"]["paths"] == docs["paths"]
            and row["docs_before"]["canonical_sha256"] == docs["canonical_sha256"], REASONS["docs"])
    require(row["runner_sha256"] == sha(git(repository, "show", commit + ":" + PREFIX + "/record.py"))
            and row["base_runner_sha256"] == sha(git(repository, "show", commit + ":docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-3/record.py")),
            "B4-4采集器Git对象不符")
    digest = row["log_sha256"]
    reference = f"logs/sha256/{digest[:2]}/{digest}.log"
    require(re.fullmatch(r"[0-9a-f]{64}", digest) and row["log_ref"] == reference, "B4-4日志路径不符")
    raw = (root / reference).read_bytes()
    require(len(raw) == row["log_bytes"] and sha(raw) == digest, "B4-4日志摘要不符")
    return row, raw.decode("utf-8")


def check_sources(root, repository):
    for variant, revision in REVISIONS.items():
        revision = git(repository, "rev-parse", revision).decode().strip()
        source = load(root / variant / "source.json")
        require(source["revision"] == revision and source["sha256"] == identity(source["files"])
                and {p: r["canonical_sha256"] for p, r in source["files"].items()} == source_at(str(repository), revision), REASONS["source"])
        environment = load(root / variant / "environment.json")
        task = PureWindowsPath(r"D:\project\Xiao\.b4-4\20261115") / variant
        require(PureWindowsPath(environment["task_root"]) == task, REASONS["environment"])
        overrides = environment["overrides"]
        for key, relative in [("CARGO_TARGET_DIR", "target"), ("XIAO_HOME", "home"), ("TEMP", "temp"),
                              ("TMP", "temp"), ("XIAO_RUNTIME_LIBRARY", "target/release/xiao_runtime.lib")]:
            require(PureWindowsPath(overrides[key]) == task / relative, REASONS["environment"])
        require(environment == BASE_ENV(variant, Path(str(task)))
                and environment["source_root"] == r"D:\project\Xiao\Xiao"
                and environment["removed_environment_prefixes"] == ["XIAO_", "RUST_BIGDECIMAL_"]
                and PureWindowsPath(overrides["XIAO_STDLIB_ROOT"]) == PureWindowsPath(environment["source_root"]) / "stdlib"
                and overrides["XIAO_B1_F10"] == overrides["XIAO_B2_A_PERF"] == "0"
                and overrides["RUSTUP_TOOLCHAIN"] == "1.96.0", REASONS["environment"])
    changed = set(git(repository, "diff", "--name-only", BASELINE, CODE, "--", *PRODUCT_ROOTS).decode().splitlines())
    require(changed == ALLOWED, "B4-4受保护产品范围不符")
    # 容器/ABI/生成器/锁与registry均在完整受保护集合中，只有明确的22条路径可变。
    abi = git(repository, "show", CODE + ":core/rust/crates/xiao-runtime-abi/src/lib.rs").decode()
    cg = git(repository, "show", CODE + ":core/rust/crates/xiao-codegen-llvm/src/lib.rs").decode()
    table = json.loads(git(repository, "show", CODE + ":core/rust/crates/xiao-intrinsics/intrinsics.json"))["intrinsics"]
    registry = json.loads(git(repository, "show", CODE + ":docs/module-registry.json"))
    require(re.search(r"ABI_MINOR_VERSION[^=]*=\s*18", abi) and re.search(r"CODEGEN_VERSION:\s*u32\s*=\s*21", cg)
            and sorted(r["id"] for r in table if r["status"] == "active") == list(range(1, 49))
            and next(r for r in table if r["id"] == 100)["status"] == "tombstone"
            and next(r for r in registry["modules"] if r["id"] == "stdlib.cmath")["status"] == "planned", REASONS["state"])


def check_artifacts(root, environment):
    data = load(root / FINAL / "artifact-manifest.json")
    rows, references = data["files"], set()
    require({k: sum(r["kind"] == k for r in rows) for k in [".xiaoc", ".xar", ".ll", ".exe"]}
            == {".xiaoc": 4, ".xar": 4, ".ll": 4, ".exe": 4}, REASONS["artifact"])
    temporary = PureWindowsPath(environment["overrides"]["TEMP"])
    for row in rows:
        path = PureWindowsPath(row["path"])
        require(path.is_relative_to(temporary) and ".." not in path.parts and row["bytes"] > 0
                and row["application_source_removed"] is True and re.fullmatch(r"[0-9a-f]{64}", row["sha256"]), REASONS["artifact"])
        if row["kind"] == ".exe":
            require("object" not in row, REASONS["artifact"])
            continue
        expected = "artifacts/sha256/" + row["sha256"] + row["kind"]
        require(row["object"] == expected, REASONS["artifact"])
        raw = (root / expected).read_bytes()
        require(sha(raw) == row["sha256"] and len(raw) == row["bytes"], REASONS["artifact"])
        references.add(expected)
        if row["kind"] == ".xar":
            with zipfile.ZipFile(root / expected) as archive:
                require(archive.namelist() == row["members"] and "META-INF/xiao/index.pb" in row["members"], REASONS["artifact"])
    require(references == {p.relative_to(root).as_posix() for p in (root / "artifacts").rglob("*") if p.is_file()}, REASONS["artifact"])
    return {"xiaoc": 4, "xar": 4, "llvm_ir": 4, "native_executable_hashes": 4}


def conclusions(root, records, logs, live_runtime):
    build = records[FINAL, "runtime-build"]
    runtime = build["runtime_after"]
    require(build["runtime_before"]["exists"] is False and runtime["exists"] is True
            and runtime["bytes"] > 0 and re.fullmatch(r"[0-9a-f]{64}", runtime["sha256"]), REASONS["runtime"])
    for variant, labels in RUNS.items():
        for label in labels:
            row = records[variant, label]
            if variant == FINAL and label != "runtime-build":
                require(build["finished_utc"] <= row["started_utc"] and row["runtime_before"] == row["runtime_after"] == runtime, REASONS["runtime"])
    if live_runtime:
        path = Path(runtime["path"])
        require(path.is_file() and len(path.read_bytes()) == runtime["bytes"] and sha(path.read_bytes()) == runtime["sha256"], REASONS["runtime"])
    require(counts(logs["baseline", "entry"]) == {"20o_b2_b3_wrapper": (8, 0), "20o_b2_b4_cmath_boundary": (3, 0), "22c_vm_concurrency": (29, 0)}, REASONS["state"])
    require("名称 abs 没有动态值槽" in logs["fixed", "native-library"] and "FAILED" in logs["fixed", "native-library"], "B4-4原生模块值历史失败丢失")
    expected = {"20o_b2_b4_cmath_library": (5, 1), "20o_b2_b4_cmath_llvm": (4, 1), "20o_b2_b4_cmath_vm": (7, 0),
                "20o_b2_b4_frontend": (3, 0), "20o_b2_b4_cmath_boundary": (4, 0), "20o_b2_b3_wrapper": (8, 0),
                "20o_b2_b2_divide": (2, 1), "20o_b2_a1_identity": (1, 1), "20o_b2_a_contract": (12, 1),
                "20o_b2_a_resources": (8, 0), "22c_vm_concurrency": (29, 0)}
    require(counts(logs[FINAL, "regressions"]) == expected, "B4-4跨阶段回归计数不符")
    require(counts(logs[FINAL, "driver-unit"]) == {"xiao_driver": (66, 1)}
            and "B4-4-VM-REMOVAL removed=15 paths=2 retained=passed unknown_source=rejected result=passed" in logs[FINAL, "driver-unit"], "B4-4驱动器真实移除回归缺失")
    regression = logs[FINAL, "regressions"]
    require(len(re.findall(r"B4-4-IMPORT path=(std|stdlib)\.cmath function=\w+ identity=stdlib.cmath executable=true result=passed", regression)) == 30
            and regression.count("source,xiaoc,xar=passed source_removed=true official_bundled=false ids_checked=true") == 2
            and regression.count("source,xiaoc,xar=passed source_removed=true official_bundled=false ids_checked=false") == 2
            and "B4-4-LLVM-REMOVAL functions=15 paths=2 source_body=removed fallback=rejected result=passed" in regression
            and "B4-4-SIGNATURES functions=15 paths=2 invalid_types=8 each_parameter=checked shapes=checked result=passed" in regression,
            "B4-4双路径/重放/签名/移除矩阵缺失")
    require(counts(logs[FINAL, "native-library"]) == {"20o_b2_b4_cmath_library": (1, 0)}
            and {(a, b) for a, b in re.findall(r"B4-4-NATIVE path=(std|stdlib)\.cmath optimization=([03]).*result=passed", logs[FINAL, "native-library"])}
            == {("std", "0"), ("std", "3"), ("stdlib", "0"), ("stdlib", "3")}, "B4-4原生双路径矩阵缺失")
    require(counts(logs[FINAL, "native-matrix"]) == {"20o_b2_b4_cmath_llvm": (1, 0)}
            and len(re.findall(r"B4-3-MATRIX dynamic=(?:true|false) optimization=[03].*cases=18318 error_cases=3153 pow_inputs=1075 bit_or_code=exact", logs[FINAL, "native-matrix"])) == 4, "B4-4既有LLVM矩阵回归缺失")
    for label, name in [("b1-vm", "b1_capabilities_have_vm_golden_outputs"), ("b1-native", "b1_capabilities_match_native_golden_outputs")]:
        require(counts(logs[FINAL, label]) == {"20o_b1_probe": (1, 0)}
                and re.search(r"test " + name + r" \.\.\. ok", logs[FINAL, label]), "B4-4 B1精确入口不符")
    require("B2-B2-DIVIDE backend=native optimization=0,3 intrinsic=33 result=success" in logs[FINAL, "b1-native"], "B4-4 B2原生回归缺失")
    core = counts(logs[FINAL, "core-tests"])
    require(core.get("xiao_runtime") == (272, 0) and core.get("xiao_vm") == (35, 0)
            and core.get("xiao_bytecode") == (33, 0) and core.get("xiao_codegen_llvm") == (56, 0)
            and core.get("xiao_intrinsics") == (8, 0) and core.get("xiao_xar") == (23, 0)
            and len(core) == 36 and sum(v[0] for v in core.values()) == 646
            and sum(v[1] for v in core.values()) == 16, "B4-4核心测试目标缺失")
    for name in HISTORY:
        result = json.loads(logs[FINAL, "audit-" + name])
        require(result.get("passed") == result.get("total") == 18 if name == "b1" else result.get("passed") is True,
                "B4-4历史审计失败：" + name)
    toolchain = json.loads(logs[FINAL, "toolchain"])
    require("rustc 1.96.0" in toolchain["rustc"]["stdout"] and "x86_64-pc-windows-msvc" in toolchain["targets"]["stdout"], "B4-4工具链身份不符")
    require(json.loads(logs[FINAL, "collector"]) == {"passed": True, "identical_outputs_reused": True, "distinct_objects": 2,
            "tampered_object_rejected": True, "existing_task_root_rejected": True}, "B4-4采集器自验失败")
    artifacts = check_artifacts(root, load(root / FINAL / "environment.json"))
    require(json.loads(logs[FINAL, "artifacts"]) == {"passed": True, "files": 16, "source_removed": True,
            "manifest_sha256": sha((root / FINAL / "artifact-manifest.json").read_bytes())}, REASONS["artifact"])
    old_docs = json.loads(logs[PREVIOUS, "final-docs"])
    require(old_docs["passed"] is False and {r["code"] for r in old_docs["diagnostics"] if r["severity"] == "error"}
            == {"A0-LAYOUT-001", "A0-LAYOUT-002", "A0-COVERAGE-001", "A0-COVERAGE-002"}, "B4-4首次文档失败未保留")
    docs = json.loads(logs[FINAL, "final-docs"])
    require(docs["passed"] is True and not any(r["severity"] == "error" for r in docs["diagnostics"]), "B4-4最终文档门禁失败")
    return {"passed": True, "stage": "B4-4", "b4_4_closed": True, "b4_closed": False,
            "language_available": False, "dual_path_internal_executable": True, "cmath": "planned",
            "runtime_abi": "1.18", "codegen_version": 21, "stdlib_interface": 1,
            "baseline_commit": BASELINE, "product_commit": CODE, "product_files": 1064,
            "changed_product_paths": sorted(ALLOWED), "runtime": runtime, "artifacts": artifacts,
            "regressions": {k: list(v) for k, v in expected.items()}, "core_test_targets": {k: list(v) for k, v in core.items()},
            "historical_audits": sorted(HISTORY), "native_module_values": "existing unsupported boundary retained; selected imports tested",
            "platforms_executed": ["Windows x86_64-pc-windows-msvc"],
            "not_executed": ["Linux", "macOS", "remote CI", "performance budget", "B1 721 F10"],
            "next_stage": "B4-5 (not started)", "capture_interruption": "retained; no PID/time receipt; excluded", "runs": {v + "/" + l: {"pid": r["pid"], "exit_code": r["exit_code"],
            "log_sha256": r["log_sha256"], "docs_commit": r["docs_commit"]} for (v, l), r in sorted(records.items())}}


def verify(root, repository, live_runtime=False, check_inventory=True, check_closeout=True, check_review=True):
    before = all_entries(root)
    require((root / ".gitattributes").is_file() and (root / ".gitattributes").read_bytes() == b"* -text -whitespace\n*.log -diff\n", REASONS["attributes"])
    if check_inventory:
        require(load(root / "evidence.json") == {"files": inventory(root)}, REASONS["manifest"])
    check_sources(root, repository)
    records, logs, referenced = {}, {}, set()
    for variant, labels in RUNS.items():
        source, environment = load(root / variant / "source.json"), load(root / variant / "environment.json")
        actual = {p.stem for p in (root / variant).glob("*.json")} - {"source", "environment", "artifacts"}
        require(actual == labels, REASONS["state"])
        for label in sorted(labels):
            row, text = read_run(root, repository, variant, label, source, environment)
            records[variant, label], logs[variant, label] = row, text
            referenced.add(row["log_ref"])
    interruption = load(root / "capture-interruption.json")
    interrupted_raw = (root / interruption["log_ref"]).read_bytes()
    require(interruption["accepted_as_run"] is False
            and interruption["original_manifest_path"] == "fixed-final/artifacts.json"
            and interruption["preserved_manifest_path"] == "fixed-final/artifact-manifest.json"
            and interruption["manifest_sha256"] == sha((root / PREVIOUS / "artifact-manifest.json").read_bytes())
            and interruption["log_sha256"] == sha(interrupted_raw) and interruption["log_bytes"] == len(interrupted_raw),
            "B4-4采集器中断记录不符")
    referenced.add(interruption["log_ref"])
    require(referenced == {p.relative_to(root).as_posix() for p in (root / "logs").rglob("*") if p.is_file()}, REASONS["orphan"])
    # 最终docs运行必须绑定本批全部静态工具/README；后续HEAD可变化，审计读取固定Git对象。
    commit = records[FINAL, "final-docs"]["docs_commit"]
    for path in root.iterdir():
        if path.suffix in {".py", ".md"}:
            require(sha(canonical(path.read_bytes())) == sha(canonical(git(repository, "show", commit + ":" + PREFIX + "/" + path.name))), REASONS["docs"])
    result = conclusions(root, records, logs, live_runtime)
    if check_closeout:
        require(load(root / "closeout.json") == result, REASONS["state"])
    if check_review:
        expected = [{"case": "baseline", "exit_code": 0, "reason": None, "stderr": "", "read_only": True}] + [
            {"case": name, "exit_code": 1, "reason": REASONS[name], "stderr": REASONS[name] + "\n", "read_only": True} for name in NEGATIVES]
        require(load(root / "self-negative.json") == {"passed": True, "cases": expected}, REASONS["review"])
    require(before == all_entries(root), "B4-4审计改写证据")
    return result


def save_inventory(root):
    (root / "evidence.json").write_text(json.dumps({"files": inventory(root)}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def negative_cases(root, repository):
    cases = []
    with tempfile.TemporaryDirectory(prefix="b4-4-negative-") as temporary:
        for name in ["baseline", *NEGATIVES]:
            copy = Path(temporary) / name
            shutil.copytree(root, copy)
            if name == "manifest":
                with (copy / "closeout.json").open("ab") as out:
                    out.write(b"\n")
            elif name == "attributes":
                (copy / ".gitattributes").unlink()
            elif name in {"source", "environment", "docs", "runtime", "state"}:
                path = copy / (FINAL + "/source.json" if name == "source" else FINAL + "/environment.json" if name == "environment"
                               else FINAL + "/native-library.json")
                data = load(path)
                if name == "source": data["revision"] = BASELINE
                elif name == "environment": data["overrides"]["TEMP"] = r"D:\old-task\temp"
                elif name == "docs": data["docs_before"]["canonical_sha256"] = "0" * 64
                elif name == "runtime": data["runtime_before"]["sha256"] = "0" * 64
                elif name == "state": data["exit_code"] = 99
                path.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")
            elif name == "orphan":
                raw = b"unreferenced\n"
                digest = sha(raw)
                path = copy / f"logs/sha256/{digest[:2]}/{digest}.log"
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)
            elif name == "artifact":
                path = next((copy / "artifacts").rglob("*.xar"))
                with path.open("ab") as out: out.write(b"tampered")
            if name not in {"baseline", "manifest"}:
                save_inventory(copy)
            before = all_entries(copy)
            result = subprocess.run([sys.executable, "-B", "-X", "utf8", str(Path(__file__).resolve()), "--root", str(copy),
                                     "--repository-root", str(repository), "--core-only"], capture_output=True, encoding="utf-8")
            require(before == all_entries(copy), "反例审计改写目录")
            reason = None if name == "baseline" else REASONS[name]
            require(result.returncode == (0 if reason is None else 1), "反例退出码不符：" + name + " " + result.stderr)
            if reason is not None:
                require(result.stderr.strip() == reason, "反例实际拒绝原因不符：" + name + " " + result.stderr)
            cases.append({"case": name, "exit_code": result.returncode, "reason": reason, "stderr": result.stderr, "read_only": True})
    return {"passed": True, "cases": cases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY)
    parser.add_argument("--live-runtime", action="store_true")
    parser.add_argument("--collect", action="store_true")
    parser.add_argument("--self-negative", action="store_true")
    parser.add_argument("--refresh-inventory", action="store_true")
    parser.add_argument("--core-only", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    root, repository = args.root.resolve(), args.repository_root.resolve()
    if args.collect:
        require(not (root / "closeout.json").exists() and not (root / "evidence.json").exists(), "禁止覆盖收口结果")
        result = verify(root, repository, args.live_runtime, False, False, False)
        write_json(root / "closeout.json", result)
        save_inventory(root)
        actual = negative_cases(root, repository)
        write_json(root / "self-negative.json", actual)
        save_inventory(root)
        result = verify(root, repository, args.live_runtime)
    elif args.refresh_inventory:
        verify(root, repository, args.live_runtime, check_inventory=False)
        save_inventory(root)
        result = verify(root, repository, args.live_runtime)
    elif args.self_negative:
        print(json.dumps(negative_cases(root, repository), ensure_ascii=False))
        return
    else:
        result = verify(root, repository, args.live_runtime, check_review=not args.core_only)
    print(json.dumps({"passed": result["passed"], "b4_4_closed": result["b4_4_closed"] and not args.core_only, "b4_closed": False,
                      "files": len(inventory(root)), "runs": len(result["runs"]), "product_commit": result["product_commit"]}, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, AssertionError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
