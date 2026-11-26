"""从固定 Git 对象、原始平台产物与 CI 回执派生 B3-3 状态。"""

import argparse
import hashlib
from io import BytesIO
import json
import os
from pathlib import Path
import re
import subprocess
from zipfile import BadZipFile, ZipFile

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
PRODUCT = "4e480ebec6809db4a691288d1eb2042a0683a4e0"
BASE = "879ce625c84965dc4fe4bb11523434635be55826"
INPUT = "docs/DevDocs/assets/20o-b3/entry-20261125-b3-3"
WORKFLOW = ".github/workflows/b3-3-fs-runner.yml"
TARGETS = {
    "windows": "x86_64-pc-windows-msvc",
    "linux": "x86_64-unknown-linux-gnu",
    "macos": "aarch64-apple-darwin",
}
IDS = "core/rust/crates/xiao-intrinsics/intrinsics.json"
FROZEN = "docs/DevDocs/assets/20o-b3/entry-20261124-b3-0d/freeze-table.json"
GOLDEN = "tests/fixtures/docs/fs/tutorial.stdout"
PRIOR_EVIDENCE = "f8c2b0edfa6896c8114dcdaaa9a3763f38aa8b49"
PRIOR_ARCHIVE = "docs/DevDocs/assets/20o-b3/entry-20261125-b3-2-final/runs.zip"
CASING = ("std", "stdlib", "denied")
GATES = ("check:docs", "check:usedocs", "check:layout")
GENERATED = {"closeout.json", "doc-gates.json", "evidence.json", "self-negative-result.json"}
SCOPE = {
    "xiao-bytecode": ("src/encode/tests.rs", "tests/b4_cmath_tac.rs"),
    "xiao-codegen-llvm": ("src/artifact.rs", "src/dynamic.rs", "src/dynamic/cmath.rs",
                         "src/dynamic/expression.rs", "src/dynamic/predicate.rs", "src/lib.rs"),
    "xiao-driver": ("Cargo.toml", "src/frontend.rs", "src/module_loader.rs", "src/native.rs", "src/run.rs",
                    "tests/20o_b2_b2_divide.rs", "tests/20o_b2_b4_cmath_llvm.rs", "tests/20o_b2_b4_cmath_vm.rs",
                    "tests/20o_b2_b4_frontend.rs", "tests/20o_b2_b5_versions.rs", "tests/20o_b3_fs_tutorial.rs",
                    "tests/20o_b3_time_tutorial.rs", "tests/20o_static_float_equality.rs",
                    "tests/support/b3_args_cli_native.rs", "tests/support/b3_fs_native.rs"),
    "xiao-intrinsics": ("build.rs", "intrinsics.json", "src/lib.rs"),
    "xiao-ir": ("src/lower.rs",),
    "xiao-runtime-abi": ("src/lib.rs",),
    "xiao-runtime": ("src/abi.rs", "src/abi_fs.rs", "src/filesystem.rs", "src/filesystem_host.rs",
                     "src/filesystem_tests.rs", "src/lib.rs", "src/time.rs"),
    "xiao-types": ("src/checker/expression.rs",),
    "xiao-vm": ("src/run.rs", "src/semantics/exec.rs", "src/semantics/fs_intrinsics.rs",
                "src/semantics/time_intrinsics.rs"),
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def git(repo, *args):
    launcher = ["cmd", "/d", "/c", "git"] if os.name == "nt" else ["git"]
    return subprocess.check_output([*launcher, *args], cwd=repo)


def identity(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def manifest(root):
    return {file.relative_to(root).as_posix(): identity(file.read_bytes())
            for file in sorted(root.rglob("*"))
            if file.is_file() and file != root / "evidence.json"}


def read_json(archive, key):
    return json.loads(archive.read(key))


def read_object(archive, expected):
    digest = expected["sha256"]
    require(re.fullmatch("[0-9a-f]{64}", digest) is not None, "对象摘要格式不符")
    raw = archive.read("objects/" + digest)
    require(identity(raw) == {"sha256": digest, "bytes": expected["bytes"]}, "对象字节不符")
    return raw


def generated_paths():
    prefix = HERE.relative_to(REPO).as_posix()
    return {f"{prefix}/{name}" for name in GENERATED}


def document_identity(repo, commit):
    require(re.fullmatch("[0-9a-f]{40}", commit), "定稿文档提交无效")
    rows = git(repo, "ls-tree", "-rz", commit, "--", "docs/DevDocs", "docs/UseDocs").split(b"\0")
    excluded = {name.encode() for name in generated_paths()}
    recorded = [row for row in rows if row and row.partition(b"\t")[2] not in excluded]
    return {"commit": commit, "docs_tree_sha256": identity(b"\0".join(recorded) + b"\0")["sha256"]}


def check_doc_gates(root, repo):
    receipt = json.loads((root / "doc-gates.json").read_bytes())
    require(receipt["source"] == document_identity(repo, receipt["source"]["commit"])
            and receipt["matched_before_after"] is True, "定稿文档树身份不符")
    require([row["name"] for row in receipt["checks"]] == list(GATES), "文档门禁缺项")
    for row in receipt["checks"]:
        require(row["exit"] == 0 and row["argv"][1:] == ["run", row["name"]], "文档门禁未通过")
        for stream in ("stdout", "stderr"):
            digest = row[stream]
            require(identity(receipt["logs"][digest].encode())["sha256"] == digest, "文档日志字节不符")
    return receipt["source"]


def check_scope(repo):
    changed = set(git(repo, "diff", "--name-only", BASE, PRODUCT, "--", ".", ":(exclude)docs/DevDocs")
                  .decode().splitlines())
    allowed = {f"core/rust/crates/{crate}/{path}" for crate, paths in SCOPE.items() for path in paths}
    allowed.update({"docs/UseDocs/library/README.md", "docs/module-registry.json",
                    "stdlib/README.md", "stdlib/fs.md", "stdlib/fs.xiao"})
    allowed.update(f"tests/fixtures/docs/fs/{path}"
                   for path in (".gitattributes", "README.md", "tutorial.stdout", "tutorial.xiao"))
    require(changed == allowed, "产品路径超出 B3-3 范围")


def check_baseline(repo, blob, current):
    baseline = json.loads(blob(INPUT + "/baseline.json"))
    prior = json.loads(blob(
        "docs/DevDocs/assets/20o-b3/entry-20261125-b3-2-final/closeout.json", current,
    ))
    require(baseline["product_commit"] == prior["product_commit"] == BASE
            and prior["b3_2_passed"] and prior["b3_3_entry_ready"]
            and prior["windows_args_cli_coff"] == "complete", "B3-2 入口不符")
    require(baseline["b1_guards"] == prior["b1_guards"], "前批守卫来源不符")
    ir = "core/rust/crates/xiao-ir/src/lower.rs"
    marker = b"#[cfg(test)]"
    require(blob(ir) == blob(ir, baseline["maintenance_commit"])
            and blob(ir).split(marker, 1)[0] == blob(ir, BASE).split(marker, 1)[0],
            "B3-2 IR 维护差异超出测试")
    recorded = json.loads(blob(INPUT + "/decisions.json"))["decisions"]
    require(len(recorded) == 5 and all(row["status"] == "approved" for row in recorded),
            "五项开工裁定缺项")
    return baseline["b1_guards"]


def fixed_source(repo):
    def blob(path, commit=PRODUCT):
        return git(repo, "show", f"{commit}:{path}")

    check_scope(repo)
    guards = check_baseline(repo, blob, PRODUCT)
    rows_before = {row["id"]: row for row in json.loads(blob(IDS, BASE))["intrinsics"]}
    rows_after = {row["id"]: row for row in json.loads(blob(IDS))["intrinsics"]}
    frozen = {row["id"]: row for row in json.loads(blob(FROZEN))["intrinsics"] if row["module"] == "fs"}
    require(rows_before.keys() == rows_after.keys(), "intrinsic 总编号集合不符")
    for id_, row in rows_after.items():
        if id_ in frozen:
            require(row["status"] == "active" and row["kind"] == "fs"
                    and row["name"] == frozen[id_]["name"] and row["capability"] == "filesystem"
                    and "xiao_runtime_" + row["abi_binding"] == frozen[id_]["c_symbol"],
                    "fs 编号或授权不符")
        else:
            require(row == rows_before[id_], "B3-3 之外的编号变化")
    locks = {}
    for path in ("core/rust/Cargo.lock", "tests/benchmarks/Cargo.lock"):
        require(blob(path) == blob(path, BASE), "锁文件变化")
        locks[path] = identity(blob(path))
    for path in ("tests/fixtures/docs/fs/tutorial.xiao", GOLDEN):
        candidate = f"docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/candidates/{path}"
        require(blob(path) == blob(candidate, BASE), "教程字节与冻结候选不符")
    decision = json.loads(blob(INPUT + "/generator-decision.json"))
    require(decision["approved"] == {"abi": "1.20", "generator": 25, "stdlib_interface": 1},
            "补充裁定不符")
    for path, marker in (
        ("xiao-codegen-llvm", b"CODEGEN_VERSION: u32 = 25;"),
        ("xiao-runtime-abi", b"ABI_MAJOR_VERSION: u32 = 1;"),
        ("xiao-runtime-abi", b"ABI_MINOR_VERSION: u32 = 20;"),
        ("xiao-standard-library", b"STDLIB_INTERFACE_VERSION: u32 = 1;"),
    ):
        require(b"pub const " + marker in blob(f"core/rust/crates/{path}/src/lib.rs"), "产品版本不符")
    registry = json.loads(blob("docs/module-registry.json"))
    require([module["status"] for module in registry["modules"] if module["id"] == "stdlib.fs"]
            == ["planned"], "registry 登记不符")
    source = git(repo, "show", "-s", "--format=%T", PRODUCT).decode().strip()
    return source, locks, blob(GOLDEN), guards


def check_ci(archive, repo):
    results = {}
    for name in ("runner", "maintenance", "workspace"):
        run = read_json(archive, f"ci/{name}/run.json")
        jobs = read_json(archive, f"ci/{name}/jobs.json")["jobs"]
        require(run["status"] == "completed" and run["conclusion"] == "success"
                and jobs and all(job["conclusion"] == "success" for job in jobs),
                f"{name} CI 未通过")
        require(all(job["run_id"] == run["id"] and job["run_attempt"] == run["run_attempt"]
                    and job["head_sha"] == run["head_sha"] for job in jobs), "CI 作业身份不符")
        delta = git(repo, "diff", "--name-only", PRODUCT, run["head_sha"]).decode().splitlines()
        require(delta == [WORKFLOW], f"{name} CI 与固定产品有差异")
        if name == "runner":
            template = git(repo, "show", f"{PRODUCT}:{INPUT}/runner.yml")
            actual = git(repo, "show", f"{run['head_sha']}:{WORKFLOW}")
            require(actual == template.replace(b"B3_FS_PRODUCT_COMMIT", PRODUCT.encode()),
                    "Runner 模板未绑定固定产品")
        results[name] = {"run": run["id"], "attempt": run["run_attempt"],
                         "commit": run["head_sha"], "jobs": {job["name"]: job["id"] for job in jobs}}
    require(set(results["runner"]["jobs"]) == {"B3-3 fs (linux)", "B3-3 fs (macos)"}, "Runner 缺平台")
    require(len(results["maintenance"]["jobs"]) == 6, "维护门禁作业缺项")
    return results


def prior_artifacts(repo):
    records = {}
    data = git(repo, "show", f"{PRIOR_EVIDENCE}:{PRIOR_ARCHIVE}")
    with ZipFile(BytesIO(data)) as archive:
        for name in TARGETS:
            rows = read_json(archive, f"platforms/{name}.json")["artifacts"]
            baseline = next(row for row in rows if row["case"] == "args-std-o0")
            file = baseline["file"]
            raw = read_object(archive, file)
            require(baseline["generator"] == 24 and baseline["target"] == TARGETS[name],
                    "前批生成器原件不符")
            records[name] = {"commit": PRIOR_EVIDENCE, "archive": PRIOR_ARCHIVE,
                             "member": file["path"], **identity(raw)}
    return records


def check_platform(archive, name, source, locks, golden, ci, prior):
    report = read_json(archive, f"platforms/{name}.json")
    target = TARGETS[name]
    require(report["passed"] is True and report["source_commit"] == PRODUCT
            and report["source_tree"] == source and report["target"] == target
            and report["lockfiles"] == locks, "平台固定源码、锁或目标不符")
    require(report["older_artifact"] == prior, "旧生成器的真实原件身份不符")
    if name != "windows":
        require(report["ci"]["GITHUB_RUN_ID"] == str(ci["run"])
                and report["ci"]["GITHUB_RUN_ATTEMPT"] == str(ci["attempt"]), "平台 CI 身份不符")
    commands = report["commands"]
    require(set(commands) == {"rustc", "clang", "llvm-as", "llc", "build-runtime", "runtime-and-abi",
                              "vm", "native"} and all(row["exit"] == 0 for row in commands.values()),
            "平台执行回执缺项")
    stdout = {}
    for label, command in commands.items():
        stdout[label] = read_object(archive, command["stdout"]).decode("utf-8")
        read_object(archive, command["stderr"])
    for label in ("build-runtime", "runtime-and-abi", "vm", "native"):
        require("--release" in commands[label]["argv"] and target in commands[label]["argv"],
                "release 或目标命令缺项")
    require(f"host: {target}" in stdout["rustc"] and "release: 1.96.0" in stdout["rustc"],
            "Rust 版本不符")
    for tool in ("clang", "llvm-as", "llc"):
        fields = report["tools"][tool]
        require(fields["version"] == stdout[tool].splitlines()
                and fields["path"] == commands[tool]["argv"][0]
                and re.fullmatch("[0-9a-f]{64}", fields["sha256"]), "LLVM 工具身份不符")
    require(all(f"B3-3-VM module=fs alias={alias} exact=true demo_empty=true" in stdout["vm"]
                for alias in ("std", "stdlib")), "VM 教程未逐字通过")
    count = 7 if name == "macos" else 8
    require(f"{count} passed; 0 failed; 0 ignored" in stdout["runtime-and-abi"]
            and "B3-3-FS-SYMLINK passed" in stdout["runtime-and-abi"]
            and "B3-3-FS-UNICODE adapter=rejected" in stdout["runtime-and-abi"],
            "Runtime、C ABI 或宿主边界缺项")
    require("filesystem::tests::fs_atomic_no_replace_has_one_winner_and_no_staging_files ... ok"
            in stdout["runtime-and-abi"], "排他覆盖竞态验证缺项")
    require("filesystem::tests::fs_write_precommit_cancel_and_deadline_clean_up ... ok"
            in stdout["runtime-and-abi"], "取消、期限或清理边界缺项")
    if name == "linux":
        require("B3-3-FS-CROSS-VOLUME passed" in stdout["runtime-and-abi"], "跨卷拒绝未实测")
    require(report["runtime_after"] == {key: report["runtime"][key] for key in ("bytes", "sha256")},
            "Runtime 字节身份不符")
    versions = report["versions"]
    require(versions == json.loads(re.search(r"(?m)^B3-3-VERSION (.+)$", stdout["native"])[1])
            and versions["baseline"] == 24 and versions["current"] == 25
            and versions["older_metadata"] == "accepted"
            and versions["baseline_fingerprint"] != versions["current_fingerprint"],
            "生成器兼容或缓存指纹不符")
    logs = {row["case"]: row for row in map(json.loads, re.findall(r"(?m)^B3-3-NATIVE (.+)$", stdout["native"]))}
    expected = {f"fs-{kind}-o{level}" for kind in CASING for level in (0, 3)}
    rows = report["artifacts"]
    require(len(rows) == len(expected) and {row["case"] for row in rows} == set(logs) == expected,
            "原生双路径、优化等级或权限用例缺项")
    for row in rows:
        observed = logs[row["case"]]
        require(row["verification"] == "complete" and row["components"] == row["observed"]
                and row["unclassified"] == [] and "fs" in row["components"],
                "产物组件或 COFF 完整性不符")
        for key in ("target", "optimization", "capabilities", "exit", "demo_empty", "verification",
                    "components", "observed", "unclassified", "old_verifier", "generator"):
            require(row[key] == observed[key], "原生报告与原始运行日志不符")
        require(row["target"] == target and row["generator"] == 25 and row["exit"] == 0
                and row["demo_empty"] is True and row["old_verifier"] == "X17-XAR-007"
                and row["optimization"] == int(row["case"][-1]), "原生执行标记不符")
        denied = "denied" in row["case"]
        require(row["capabilities"] == (32 if denied else 33), "原生能力掩码不符")
        require(read_object(archive, row["stdout"]) == (b"X06-RUNTIME-026\n" * 5 if denied else golden)
                and read_object(archive, row["stderr"]) == b"", "原生教程或权限黄金不符")
        artifact = read_object(archive, row["artifact"])
        require(identity(artifact)["sha256"] == observed["sha256"], "原生产物与执行日志摘要不符")
        if name == "windows":
            mapping = read_object(archive, row["map"])
            require(artifact.startswith(b"MZ") and identity(mapping)["sha256"] == observed["coff_map_sha256"]
                    and all(b"xiao_runtime_fs_" + n in mapping for n in
                            (b"write_text", b"list_dir", b"remove", b"rename", b"metadata")),
                    "Windows COFF map 身份或符号不符")
        else:
            require("map" not in row, "非 Windows 平台包含 COFF map")
    if name == "windows":
        trace = read_object(archive, report["clang_commands"])
        require(b"x86_64-pc-windows-msvc" in trace and b"/MAP:" in trace, "缺少真实 MSVC 链接轨迹")
    return {"target": target, "runtime_sha256": report["runtime"]["sha256"],
            "runtime_c_abi_cases": count, "vm_cases": 2, "native_tutorials": 4,
            "native_capability_cases": 2, "product_verification": "complete"}


def derive(root, repo, prepared=False):
    source, locks, golden, guards = fixed_source(repo)
    prior = prior_artifacts(repo)
    with ZipFile(root / "runs.zip") as archive:
        names = archive.namelist()
        require(len(names) == len(set(names)), "归档出现重复日志对象")
        ci = check_ci(archive, repo)
        platforms = {name: check_platform(archive, name, source, locks, golden, ci["runner"], prior[name])
                     for name in TARGETS}
        job_logs = read_json(archive, "ci/job-logs.json")
        logs = {}
        for job, receipt in job_logs.items():
            file = receipt["object"]
            data = archive.read(file)
            require(file == "objects/" + identity(data)["sha256"] and len(data) == receipt["bytes"],
                    "CI 作业日志字节不符")
            logs[int(job)] = data
        link_job = ci["maintenance"]["jobs"]["Native differential regression (Linux)"]
        match = re.search(rb"RUNTIME-COMPONENTS exports=(\d+) target=x86_64-unknown-linux-gnu result=passed",
                          logs[link_job])
        require(match is not None, "Linux release Runtime 全量符号对账缺失")
        component_exports = int(match[1])
    if not prepared:
        replay = json.loads((root / "self-negative-result.json").read_bytes())
        require(replay == {"baseline_passed": True, "read_only": True, "rejected": NEGATIVE_REASONS},
                "自反例回执缺项或拒绝原因不符")
    return {
        "stage": "20O-B3-3", "product_commit": PRODUCT, "product_tree": source,
        "versions": {"abi": "1.20", "generator": 25, "stdlib_interface": 1},
        "b3_3_started": True, "b3_3a_entry_and_baseline_verified": True,
        "b3_3b_fs_intrinsics_implemented": True, "b3_3c_vm_tac_llvm_c_abi_native_consistent": True,
        "b3_3d_fs_tutorial_and_boundaries_passed": True, "b3_3e_evidence_audit_passed": not prepared,
        "b3_3_passed": not prepared, "b3_4_entry_ready": not prepared, "b3_4_started": False,
        "decisions_approved": True, "no_product_scope_creep": True,
        "document_source": check_doc_gates(root, repo), "b1_guards": guards,
        "linux_component_exports_checked": component_exports,
        "registry_status": "planned", "tutorial_status": "planned", "windows_fs_coff": "complete",
        "old_721_executed": False,
        "deferred": ["fs_restricted_mode_unscheduled", "env_and_log", "B3-7_public_acceptance",
                     "old_721_and_historical_debts"],
        "platforms": platforms, "ci": ci,
    }


NEGATIVE_REASONS = {
    "manifest_changed": "归档清单与实际文件不符",
    "attributes_missing": "缺少 .gitattributes",
    "forged_closeout": "收口状态与回执不符",
    "changed_capability": "原生报告与原始运行日志不符",
    "changed_tutorial": "原生教程或权限黄金不符",
    "missing_race": "排他覆盖竞态验证缺项",
    "changed_artifact": "对象字节不符",
    "changed_coff": "产物组件或 COFF 完整性不符",
    "changed_intrinsics": "平台固定源码、锁或目标不符",
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=HERE)
    parser.add_argument("--source-root", type=Path, default=REPO)
    parser.add_argument("--prepare", action="store_true")
    args = parser.parse_args()
    try:
        files = manifest(args.root)
        require(".gitattributes" in files, "缺少 .gitattributes")
        require(json.loads((args.root / "evidence.json").read_bytes()) == {"files": files},
                "归档清单与实际文件不符")
        result = derive(args.root, args.source_root, args.prepare)
        require(json.loads((args.root / "closeout.json").read_bytes()) == result, "收口状态与回执不符")
    except (KeyError, TypeError, AttributeError, ValueError, OSError, subprocess.SubprocessError, BadZipFile) as error:
        raise SystemExit(f"B3-3 拒绝：{error}") from None
    print(json.dumps({**result, "audit_files": len(files)}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
