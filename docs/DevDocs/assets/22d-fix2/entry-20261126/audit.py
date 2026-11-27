"""复核封存轨迹、三平台矩阵与 B3-3 产品级复验。"""

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
from zipfile import ZipFile

import repeat as samples
from repeat import require


HERE = Path(__file__).resolve().parent
HISTORY = HERE.parents[1] / "20o-b2-b" / "entry-20261119-b6-4-followup" / "followup.zip"
LOCK_WAIT = samples.LOCK_WAIT
CASES = samples.SINGLE_CASES
TARGETS = {
    "windows": "x86_64-pc-windows-msvc",
    "linux": "x86_64-unknown-linux-gnu",
    "macos": "aarch64-apple-darwin",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_json(archive, name):
    return json.loads(archive.read(name))


def check_file(archive, prefix, receipt):
    relative = receipt["path"].replace("\\", "/")
    path = f"{prefix}/{relative}"
    data = archive.read(path)
    require(len(data) == receipt["bytes"] and digest(data) == receipt["sha256"], path)
    return data


def shared_lifecycles(raw):
    objects, _ = samples.group_trace_actions(raw)
    return sorted(actions for actions in objects.values() if actions[0] == "shared_release")


def check_matrix(archive, prefix, expected, product, ci=None, platform=None):
    data = read_json(archive, f"{prefix}/matrix.json")
    require(data["product"] == product and data["passed"] and data["runtime_unchanged"]
            and data.get("source_unchanged", True), prefix)
    if ci:
        require(data["target"] == platform["target"] and data["ci"]["ImageVersion"] == platform["runner_image"]
                and data["ci"]["GITHUB_SHA"] == ci["sha"]
                and int(data["ci"]["GITHUB_RUN_ID"]) == ci["run"]
                and int(data["ci"]["GITHUB_RUN_ATTEMPT"]) == ci["attempt"], prefix)
    rounds = {(row["case"], row["round"]): row for row in data["rounds"]}
    require(len(rounds) == len(data["rounds"]) and set(rounds) == {
        (case, n) for case, count in expected.items() for n in range(1, count + 1)
    }, prefix)
    grades = set()
    for (case, number), result in rounds.items():
        directory = PurePosixPath(case or "full", f"{number:03}")
        require(result["receipt"] == str(directory / "receipt.json"), f"差分轮次回执路径不符：{directory}")
        row = read_json(archive, f"{prefix}/{result['receipt']}")
        require(result["passed"] and row["passed"] and row["round"] == number and row["case"] == case, result["receipt"])
        require(row["saved_log"] == str(directory / "output.log")
                and all(PurePosixPath(receipt["path"]).parent.parent == directory / "trace" for receipt in row["traces"]),
                f"差分轮次引用了其他轮次的原始文件：{directory}")
        log = archive.read(f"{prefix}/{row['saved_log']}")
        require(digest(log) == row["log_sha256"], row["saved_log"])
        traces = {receipt["path"]: check_file(archive, prefix, receipt) for receipt in row["traces"]}
        samples.validate_round(row, log, traces)
        grades.update(record["verification"] for record in row["native"])
    return sorted(grades)


def check_fs(archive, platform, product, run):
    prefix = f"ci/fs/{platform}"
    data = read_json(archive, f"{prefix}/result.json")
    require(data["passed"] and data["source_commit"] == product and len(data["artifacts"]) == 6
            and data["target"] == TARGETS[platform]
            and int(data["ci"]["GITHUB_RUN_ID"]) == run["run"]
            and int(data["ci"]["GITHUB_RUN_ATTEMPT"]) == run["attempt"], prefix)
    expected = {f"fs-{alias}-o{level}" for alias in ("std", "stdlib", "denied") for level in (0, 3)}
    require({row["case"] for row in data["artifacts"]} == expected, "fs 六个用例缺失或重复")
    require(len({row["artifact"]["path"] for row in data["artifacts"]}) == 6, "fs 产物路径重复")
    require([item["baseline"] for item in data["versions"]] == [24, 25]
            and all(row["current"] == 26 and row["older_metadata"] == "accepted"
                    and row["baseline_fingerprint"] != row["current_fingerprint"] for row in data["versions"]), prefix)

    def command_stdout(name):
        command = data["commands"][name]
        receipt = command["stdout"]
        log = check_file(archive, prefix, {**receipt, "path": f"logs/{receipt['sha256']}.log"})
        require(command["exit"] == 0, f"{prefix}/{name}")
        return log

    native = command_stdout("native")
    require(b"test result: ok. 1 passed; 0 failed; 0 ignored;" in native, "fs 原生精确入口未通过")
    records = [json.loads(line.removeprefix(b"B3-3-NATIVE ")) for line in native.splitlines()
               if line.startswith(b"B3-3-NATIVE ")]
    by_case = {row["case"]: row for row in records}
    require(len(records) == len(expected) and by_case.keys() == expected, "fs 原生日志用例缺失或重复")
    versions = [json.loads(line.removeprefix(b"B3-3-VERSION ")) for line in native.splitlines()
                if line.startswith(b"B3-3-VERSION ")]
    require(versions == data["versions"], "fs 生成器兼容回执与原生日志不符")
    task_root = PurePosixPath(data["task_root"].replace("\\", "/"))
    golden = (samples.ROOT / "tests/fixtures/docs/fs/tutorial.stdout").read_bytes()
    for row in data["artifacts"]:
        case = row["case"]
        denied = "denied" in case
        require(row["generator"] == 26 and row["verification"] == "complete"
                and row["target"] == TARGETS[platform] and row["exit"] == 0 and row["demo_empty"]
                and row["capabilities"] == (32 if denied else 33)
                and row["optimization"] == int(case[-1])
                and row["components"] == row["observed"] == ["fs", "rc", "value", "weak"]
                and not row["unclassified"] and row["old_verifier"] == "X17-XAR-007", case)
        record = by_case[case]
        artifact = row["artifact"]
        require(record.pop("path").replace("\\", "/") == str(task_root / artifact["path"])
                and record.pop("sha256") == artifact["sha256"], f"fs 产物与原生日志不符：{case}")
        check_file(archive, prefix, artifact)
        artifact_dir = PurePosixPath(artifact["path"]).parent
        for stream in ("stdout", "stderr"):
            require(row[stream]["path"] == str(artifact_dir / f"{stream}.bin"), f"fs 输出路径不符：{case}")
        require(check_file(archive, prefix, row["stdout"]) == (b"X06-RUNTIME-026\n" * 5 if denied else golden)
                and check_file(archive, prefix, row["stderr"]) == b"", f"fs 产物输出与黄金不符：{case}")
        map_path, map_digest = record.pop("coff_map"), record.pop("coff_map_sha256")
        if platform == "windows":
            require(map_path and map_path.replace("\\", "/") == str(task_root / row["map"]["path"])
                    and map_digest == row["map"]["sha256"], f"fs 链接图与原生日志不符：{case}")
            check_file(archive, prefix, row["map"])
        else:
            require(map_path is None and map_digest is None and "map" not in row, f"fs 链接图平台不符：{case}")
        require(record == {key: value for key, value in row.items() if key not in ("artifact", "stdout", "stderr", "map")},
                f"fs 产物验证回执与原生日志不符：{case}")
    runtime = command_stdout("runtime-and-abi")
    require(b"B3-3-FS-RENAME-SAME-PATH passed" in runtime and b"B3-3-FS-RENAME-DANGLING passed" in runtime,
            f"{prefix}/runtime-and-abi")
    vm = command_stdout("vm")
    require(all(f"B3-3-VM module=fs alias={alias} exact=true demo_empty=true".encode() in vm
                for alias in ("std", "stdlib")), f"{prefix}/vm")


def check_original_traces(archive):
    with ZipFile(HISTORY) as history:
        index = read_json(history, "evidence.json")
        origins = read_json(archive, "original/aliases/origins.json")
        require(origins.keys() == {
            f"{case}.{backend}-release-trace.tsv" for case in CASES[1:] for backend in ("native", "vm")
        }, "F2/F3 历史 TSV 缺项")
        for case in CASES[1:]:
            pair = {}
            for backend in ("native", "vm"):
                name = f"{case}.{backend}-release-trace.tsv"
                item = origins[name]
                raw = archive.read(f"original/aliases/{name}")
                require(digest(raw) == item["sha256"] == index[item["member"]]["sha256"], name)
                pair[backend] = samples.trace_projection(raw)
                require(pair[backend]["objects"] == [("strong_release", "destroy")] * 3, name)
            samples.compare_trace(case, pair["native"], pair["vm"])

    backends = {backend: f"{LOCK_WAIT}.{backend}-release-trace.tsv" for backend in ("native", "vm")}
    shared = ["shared_release", "shared_destroy", "weak_release"]
    for phase, expected in (("precheck", [["shared_release"], shared]), ("precheck-cleanup", [shared, shared])):
        prefix = f"original/f1/{phase}"
        require(shared_lifecycles(archive.read(f"{prefix}/{backends['native']}")) == expected
                and shared_lifecycles(archive.read(f"{prefix}/{backends['vm']}")) == [shared, shared], prefix)
        require(archive.read(f"{prefix}/native.stdout").replace(b"\r\n", b"\n") == b"lock-finally\n"
                and b"xiao-error class=recoverable code=X06-RUNTIME-027 " in archive.read(f"{prefix}/native.stderr")
                and f"22D-NATIVE {LOCK_WAIT} exit=exit code: 3 ".encode() in archive.read(f"{prefix}/cargo.log"), prefix)
    baseline = read_json(archive, "original/baseline/summary-100.json")
    require([row["round"] for row in baseline] == list(range(1, 101)), "基线 100 轮")
    for row in baseline:
        prefix = f"original/baseline/f1-{row['round']:03d}"
        require(row["trace"].keys() == set(backends.values()), f"F1 基线两侧 TSV 缺项：{prefix}")
        log = archive.read(f"{prefix}/output.log")
        require(row["exit"] == 0 and row["case_seen"] and digest(log) == row["log_sha256"]
                and f"22D-CASE {LOCK_WAIT}: ok".encode() in log
                and b"test result: ok. 1 passed; 0 failed; 0 ignored;" in log, prefix)
        for name in backends.values():
            item = row["trace"][name]
            relative = item["path"].replace("\\", "/")
            trace = archive.read(f"{prefix}/{relative}")
            require(PurePosixPath(relative).name == name and digest(trace) == item["sha256"]
                    and shared_lifecycles(trace) == [shared, shared] and item["terminal"] == [shared[1:]] * 2, prefix)


def audit(root):
    status = json.loads((root / "status.json").read_text(encoding="utf-8"))
    bundle = root / "evidence.zip"
    require(digest(bundle.read_bytes()) == status["evidence_sha256"], "证据归档摘要")
    product = status["product"]
    ci = status["ci"]
    with ZipFile(bundle) as archive:
        require(len(archive.namelist()) == len(set(archive.namelist())), "归档成员重复")
        check_original_traces(archive)
        replay = archive.read("local/logs/critical-local.log")
        require(all(f"22D-PROJECTION P-{n}: ok".encode() in replay for n in range(1, 10))
                and all(f"22D-REPLAY {case}: ok".encode() in replay for case in CASES[1:]), "投影回放")
        check_matrix(archive, "local/f1", {LOCK_WAIT: 100}, product)
        check_matrix(archive, "local/matrix", {None: 10, **dict.fromkeys(CASES, 50)}, product)
        for platform, singles in (("windows", 50), ("linux", 30), ("macos", 30)):
            metadata = status["platforms"][platform]
            grades = check_matrix(archive, f"ci/trace/{platform}", {None: 10, **dict.fromkeys(CASES, singles)},
                                  product, ci["evidence"], metadata)
            require(grades == metadata["verification"] and metadata["fs_complete"] == 6
                    and metadata["rounds"] == {"full": 10, **dict.fromkeys(CASES, singles)}, platform)
            check_fs(archive, platform, product, ci["evidence"])
            job = status["platforms"][platform]["native_maintenance_job"]
            log = archive.read(f"ci/logs/ci-maintenance-{job}.log")
            require(all(marker in log for marker in (
                b"test native_concurrency_llvm_assembly_fixtures ... ok",
                b"test native_concurrency_shared_c_layout_probe ... ok",
                b"22D-CASE detached-failure: ok",
                b"native_blocked_lock_wait_is_cancelled_by_host_token ... ok",
                b"native_cleanup_checkpoint_defers_repeated_cancellation ... ok",
                b"test result: ok. 29 passed; 0 failed; 0 ignored;",
            )), f"{platform} 原生维护精确入口")
            require(all(
                f"20O-B1-IMP-PROBE case={library} backend=vm/native optimization={level} result=success".encode() in log
                for library in ("math", "statistics", "fmt", "hex", "base64", "url", "csv")
                for level in (0, 3)
            ), f"{platform} B1 七库精确入口")
        for key in ("evidence", "maintenance", "workspace"):
            row = ci[key]
            data = read_json(archive, f"ci/runs/{row['run']}.json")
            require(data["databaseId"] == row["run"] and data["conclusion"] == row["conclusion"] == "success"
                    and data["attempt"] == row["attempt"]
                    and data["headSha"] == row["sha"] and row["jobs"] == {
                        job["name"]: job["databaseId"] for job in data["jobs"] if job["conclusion"] == "success"
                    } and all(job["status"] == "completed" and job["conclusion"] == "success" for job in data["jobs"]), key)
        for platform, os_name in (("windows", "windows-2025"), ("linux", "ubuntu-24.04"), ("macos", "macos-15")):
            row = status["platforms"][platform]
            require(row["target"] == TARGETS[platform]
                    and row["trace_job"] == ci["evidence"]["jobs"][f"{os_name} / release-trace"]
                    and row["fs_job"] == ci["evidence"]["jobs"][f"{os_name} / fs"]
                    and row["native_maintenance_job"] == ci["maintenance"]["jobs"][
                        f"Native differential regression ({'macOS' if platform == 'macos' else platform.title()})"],
                    f"{platform} 作业身份未绑定")
    require(status["22d_open"] is False and status["b3_4_started"] is False, "阶段状态")
    print("22D-FIX.2 证据对账通过")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=HERE)
    args = parser.parse_args()
    audit(args.root.resolve())


if __name__ == "__main__":
    main()
