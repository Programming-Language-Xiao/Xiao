"""只读复核 B3-3 排他重命名修复、三平台回执及当前交接。"""

import argparse
from importlib.util import module_from_spec, spec_from_file_location
import json
from pathlib import Path
import re
import subprocess
import sys
from zipfile import BadZipFile, ZipFile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
PREFIX = HERE.relative_to(REPO).as_posix()
ORIGINAL = HERE.with_name("entry-20261125-b3-3-final")
CLOSEOUT = "a32c1695996e3bf7c1fbe91091bbd5690259d8a5"
PRODUCT = "2460e75fbdac448b51b7941e937fb3df24e0cb70"
FIRST_RUNNER = "61387cf90685bc4bb856e2c6f2f95e88a9b66c6b"
CI_RUNS = {"runner": 45807842820, "maintenance": 45807842760, "workspace": 45808384106}
FIRST_RUN = 45806308281
FIRST_WINDOWS_JOB = 139867799304
LINUX_COMPONENTS_JOB = 139867799354
GENERATED = {"status.json", "doc-gates.json", "evidence.json"}
MARKERS = (b"B3-3-FS-RENAME-SAME-PATH passed", b"B3-3-FS-RENAME-DANGLING passed")


def load_original(name):
    spec = spec_from_file_location(f"b3_fs_original_{name}", ORIGINAL / f"{name}.py")
    module = module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# 在独立模块实例上复用固定平台、版本及 COFF 的验收规则。
checks = load_original("audit")
checks.PRODUCT = PRODUCT
git, identity, manifest, require = checks.git, checks.identity, checks.manifest, checks.require


def check_product(repo):
    original_prefix = ORIGINAL.relative_to(REPO).as_posix()
    original_manifest = git(repo, "show", f"{CLOSEOUT}:{original_prefix}/evidence.json")
    require((ORIGINAL / "evidence.json").read_bytes() == original_manifest
            and json.loads(original_manifest) == {"files": manifest(ORIGINAL)}, "原收口证据字节变化")
    previous = json.loads((ORIGINAL / "closeout.json").read_bytes())
    require(previous["b3_3_passed"] and previous["b3_4_entry_ready"], "原收口状态不符")
    delta = git(repo, "diff", "--name-only", CLOSEOUT, PRODUCT).decode().splitlines()
    allowed = {"core/rust/crates/xiao-runtime/src/" + name
               for name in ("filesystem.rs", "filesystem_tests.rs")}
    require(set(delta) == allowed, "修复源码超出排他重命名范围")
    return previous


def document_source(repo, commit):
    require(re.fullmatch("[0-9a-f]{40}", commit), "定稿文档提交无效")
    rows = git(repo, "ls-tree", "-rz", commit, "--", "docs/DevDocs", "docs/UseDocs").split(b"\0")
    excluded = {f"{PREFIX}/{file}".encode() for file in GENERATED}
    rows = [row for row in rows if row and row.partition(b"\t")[2] not in excluded]
    return {"commit": commit, "docs_tree_sha256": identity(b"\0".join(rows) + b"\0")["sha256"]}


def check_doc_gates(root, repo):
    result = json.loads((root / "doc-gates.json").read_bytes())
    source = result["source"]
    require(source == document_source(repo, source["commit"]) and result["matched_before_after"] is True,
            "定稿文档树身份不符")
    require([row["name"] for row in result["checks"]] == list(checks.GATES), "文档门禁缺项")
    for row in result["checks"]:
        require(row["exit"] == 0 and row["argv"][1:] == ["run", row["name"]], "文档门禁未通过")
        for stream in ("stdout", "stderr"):
            digest = row[stream]
            require(identity(result["logs"][digest].encode())["sha256"] == digest, "文档日志字节不符")
    return source


def check_first_run(archive):
    run = checks.read_json(archive, "ci/first-runner/run.json")
    jobs = checks.read_json(archive, "ci/first-runner/jobs.json")["jobs"]
    require(run["id"] == FIRST_RUN and run["head_sha"] == FIRST_RUNNER and run["conclusion"] == "failure",
            "首轮 CI 身份不符")
    macos = next(job for job in jobs if job["name"] == "B3-3 fs (macos)")
    require(macos["run_id"] == FIRST_RUN and macos["run_attempt"] == run["run_attempt"]
            and macos["head_sha"] == FIRST_RUNNER and macos["conclusion"] == "failure", "首轮 macOS 作业不符")
    receipt = checks.read_json(archive, "ci/first-macos-log.json")
    require(receipt["job"] == macos["id"], "首轮 macOS 日志身份不符")
    log = checks.read_object(archive, receipt)
    require(b"fs_rename_and_remove_preserve_nonempty_directories" in log
            and b"called \x60Result::unwrap_err()\x60 on an \x60Ok\x60 value: None" in log, "首轮 macOS 回执缺项")
    return {"run": FIRST_RUN, "attempt": run["run_attempt"], "job": macos["id"], "conclusion": "failure"}


def attempt_jobs(archive, name, attempt, final_attempt):
    suffix = "jobs.json" if attempt == final_attempt else f"attempt-{attempt}-jobs.json"
    return checks.read_json(archive, f"ci/{name}/{suffix}")["jobs"]


def check_ci(archive, repo):
    receipts = {}
    for name, run_id in CI_RUNS.items():
        run = checks.read_json(archive, f"ci/{name}/run.json")
        head = run["head_sha"]
        require(run["id"] == run_id and run["status"] == "completed"
                and run["conclusion"] == "success", f"{name} CI 未通过")
        delta = git(repo, "diff", "--name-only", PRODUCT, head).decode().splitlines()
        require(delta == [checks.WORKFLOW], f"{name} CI 与固定产品有差异")
        jobs = {}
        final = run["run_attempt"]
        for attempt in range(1, final + 1):
            for row in attempt_jobs(archive, name, attempt, final):
                require((row["run_id"], row["head_sha"], row["run_attempt"]) == (run_id, head, attempt),
                        "CI 作业身份不符")
                jobs[row["name"]] = row
        require(jobs and all(row["conclusion"] == "success" for row in jobs.values()),
                f"{name} CI 有作业未通过")
        receipts[name] = {
            "run": run_id, "commit": head, "attempt": final,
            "jobs": {job: row["id"] for job, row in jobs.items()},
            "job_attempts": {job: row["run_attempt"] for job, row in jobs.items()},
        }
    template = git(repo, "show", f"{PRODUCT}:{checks.INPUT}/runner.yml")
    runner = git(repo, "show", f"{receipts['runner']['commit']}:{checks.WORKFLOW}")
    require(runner == template.replace(b"B3_FS_PRODUCT_COMMIT", PRODUCT.encode()), "Runner 模板未绑定固定产品")
    require(set(receipts["runner"]["jobs"]) == {"B3-3 fs (linux)", "B3-3 fs (macos)"}, "Runner 缺平台")
    require(len(receipts["maintenance"]["jobs"]) == 6, "维护门禁作业缺项")
    require(set(receipts["workspace"]["jobs"]) == {"Linux workspace gates"}, "工作区门禁缺项")
    return receipts


def check_22d(archive, maintenance):
    failed = next(row for row in attempt_jobs(archive, "maintenance", 1, maintenance["attempt"])
                  if row["id"] == FIRST_WINDOWS_JOB)
    name = "Native differential regression (Windows)"
    require(failed["name"] == name and failed["conclusion"] == "failure"
            and maintenance["job_attempts"][name] > 1, "22D 首次失败或后续重跑回执缺项")
    receipt = checks.read_json(archive, "ci/first-windows-maintenance-log.json")
    require(receipt["job"] == FIRST_WINDOWS_JOB, "22D 失败日志身份不符")
    log = checks.read_object(archive, receipt)
    require("xcon-bounded-string-success: VM/native 完整释放对象顺序".encode() in log,
            "22D 失败断言不符")
    return {"run": maintenance["run"], "failed_attempt": 1, "failed_job": FIRST_WINDOWS_JOB,
            "passed_attempt": maintenance["job_attempts"][name], "passed_job": maintenance["jobs"][name],
            "case": "xcon-bounded-string-success", "22d_open": True}


def check_rename_cases(log):
    require(all(marker in log for marker in MARKERS), "排他重命名回执缺项")


def derive(root, repo):
    previous = check_product(repo)
    tree, locks, golden, _ = checks.fixed_source(repo)
    prior = checks.prior_artifacts(repo)
    with ZipFile(root / "runs.zip") as archive:
        require(len(archive.namelist()) == len(set(archive.namelist())), "归档对象重复")
        ci = check_ci(archive, repo)
        platforms = {}
        for name in checks.TARGETS:
            platforms[name] = checks.check_platform(archive, name, tree, locks, golden, ci["runner"], prior[name])
            platform = checks.read_json(archive, f"platforms/{name}.json")
            receipt = platform["commands"]["runtime-and-abi"]["stdout"]
            check_rename_cases(checks.read_object(archive, receipt))
        first = check_first_run(archive)
        windows_maintenance = check_22d(archive, ci["maintenance"])
        linux = next(row for row in attempt_jobs(archive, "maintenance", 1, ci["maintenance"]["attempt"])
                     if row["id"] == LINUX_COMPONENTS_JOB)
        receipt = checks.read_json(archive, "ci/linux-components-log.json")
        require(linux["name"] == "Native differential regression (Linux)" and linux["conclusion"] == "success"
                and receipt["job"] == LINUX_COMPONENTS_JOB, "Linux 组件作业身份不符")
        log = checks.read_object(archive, receipt)
        exports = re.search(rb"RUNTIME-COMPONENTS exports=(\d+) target=x86_64-unknown-linux-gnu result=passed", log)
        require(exports and int(exports[1]) == previous["linux_component_exports_checked"], "Linux 组件对账回执缺失")
    return {"product_commit": PRODUCT, "product_tree": tree, "versions": previous["versions"],
            "previous_product_commit": previous["product_commit"], "first_macos_run": first,
            "windows_maintenance": windows_maintenance,
            "b3_3_passed": True, "b3_4_entry_ready": True, "b3_4_started": False,
            "windows_fs_coff": "complete", "linux_component_exports_checked": int(exports[1]),
            "linux_component_exports_job": LINUX_COMPONENTS_JOB,
            "registry_status": previous["registry_status"], "tutorial_status": previous["tutorial_status"],
            "document_source": check_doc_gates(root, repo), "platforms": platforms, "ci": ci}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=HERE)
    parser.add_argument("--source-root", type=Path, default=REPO)
    args = parser.parse_args()
    try:
        files = manifest(args.root)
        require(".gitattributes" in files, "缺少 .gitattributes")
        require(json.loads((args.root / "evidence.json").read_bytes()) == {"files": files}, "清单与字节不符")
        status = derive(args.root, args.source_root)
        require(json.loads((args.root / "status.json").read_bytes()) == status, "交接状态与实测回执不符")
    except (KeyError, StopIteration, TypeError, ValueError, OSError, subprocess.SubprocessError, BadZipFile) as error:
        raise SystemExit(f"B3-3 复验拒绝：{error}") from None
    print(json.dumps({**status, "audit_files": len(files)}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
