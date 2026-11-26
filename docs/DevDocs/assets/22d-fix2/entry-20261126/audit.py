"""复核封存轨迹、三平台矩阵与 B3-3 产品级复验。"""

from collections import Counter
import hashlib
import json
from pathlib import Path
from zipfile import ZipFile


HERE = Path(__file__).resolve().parent
HISTORY = HERE.parents[1] / "20o-b2-b" / "entry-20261119-b6-4-followup" / "followup.zip"
LOCK_WAIT = "native-lock-wait-027"
CASES = (LOCK_WAIT, "xcon-bounded-string-success", "spawn-join-str-multiargument")


def require(condition, message):
    if not condition:
        raise ValueError(message)


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


def shared_terminal(raw):
    events = Counter(line.rsplit("\t", 1)[1] for line in raw.decode().splitlines())
    return events["shared_destroy"], events["weak_release"]


def check_matrix(archive, prefix, expected, product, ci=None):
    data = read_json(archive, f"{prefix}/matrix.json")
    require(data["product"] == product and data["passed"] and data["runtime_unchanged"], prefix)
    if ci:
        require(data["ci"]["GITHUB_SHA"] == ci["sha"] and data["ci"]["ImageVersion"]
                and int(data["ci"]["GITHUB_RUN_ID"]) == ci["run"]
                and int(data["ci"]["GITHUB_RUN_ATTEMPT"]) == ci["attempt"], prefix)
    rounds = {(row["case"], row["round"]): row for row in data["rounds"]}
    require(len(rounds) == len(data["rounds"]) and set(rounds) == {
        (case, n) for case, count in expected.items() for n in range(1, count + 1)
    }, prefix)
    grades = Counter()
    for (case, number), result in rounds.items():
        row = read_json(archive, f"{prefix}/{result['receipt']}")
        require(result["passed"] and row["passed"] and row["exit"] == 0 and row["round"] == number
                and row["case"] == case and row["case_start"] == row["case_ok"], result["receipt"])
        log = archive.read(f"{prefix}/{row['saved_log']}")
        require(digest(log) == row["log_sha256"] and b"test result: ok. 1 passed; 0 failed; 0 ignored;" in log,
                row["saved_log"])
        require(len(row["case_ok"]) == len(row["native"]) == (1 if case else 29), row["saved_log"])
        require(len(row["summaries"]) == (1 if case else 30) and len(row["traces"]) == (2 if case else 59),
                result["receipt"])
        grades.update(record["verification"] for record in row["native"])
        for receipt in row["traces"]:
            trace = check_file(archive, prefix, receipt)
            if Path(receipt["path"]).name.startswith(LOCK_WAIT):
                require(shared_terminal(trace) == (2, 2), receipt["path"])
    return sorted(grades)


def check_fs(archive, platform, product, run):
    prefix = f"ci/fs/{platform}"
    data = read_json(archive, f"{prefix}/result.json")
    require(data["passed"] and data["source_commit"] == product and len(data["artifacts"]) == 6
            and int(data["ci"]["GITHUB_RUN_ID"]) == run["run"]
            and int(data["ci"]["GITHUB_RUN_ATTEMPT"]) == run["attempt"], prefix)
    require([item["baseline"] for item in data["versions"]] == [24, 25], prefix)
    for row in data["artifacts"]:
        require(row["generator"] == 26 and row["verification"] == "complete", row["case"])
        for kind in ("artifact", "stdout", "stderr"):
            check_file(archive, prefix, row[kind])
        if platform == "windows":
            check_file(archive, prefix, row["map"])
    for name, marker in (("runtime-and-abi", b"B3-3-FS-RENAME-SAME-PATH passed"),
                         ("runtime-and-abi", b"B3-3-FS-RENAME-DANGLING passed"),
                         ("vm", b"B3-3-VM module=fs alias=stdlib exact=true demo_empty=true")):
        sha = data["commands"][name]["stdout"]["sha256"]
        log = archive.read(f"{prefix}/logs/{sha}.log")
        require(digest(log) == sha and marker in log, f"{prefix}/{name}")


def main():
    status = json.loads((HERE / "status.json").read_text(encoding="utf-8"))
    bundle = HERE / "evidence.zip"
    require(digest(bundle.read_bytes()) == status["evidence_sha256"], "证据归档摘要")
    product = status["product"]
    ci = status["ci"]
    with ZipFile(bundle) as archive:
        with ZipFile(HISTORY) as history:
            index = read_json(history, "evidence.json")
            origins = read_json(archive, "original/aliases/origins.json")
            require(len(origins) == 4, "F2/F3 历史 TSV")
            for name, item in origins.items():
                require(digest(archive.read(f"original/aliases/{name}")) == item["sha256"]
                        == index[item["member"]]["sha256"], name)
        replay = archive.read("local/logs/critical-local.log")
        require(all(f"22D-PROJECTION P-{n}: ok".encode() in replay for n in range(1, 10))
                and all(f"22D-REPLAY {case}: ok".encode() in replay for case in CASES[1:]), "投影回放")
        for phase, expected in (("precheck", (1, 1)), ("precheck-cleanup", (2, 2))):
            prefix = f"original/f1/{phase}"
            require(shared_terminal(archive.read(f"{prefix}/{LOCK_WAIT}.native-release-trace.tsv")) == expected
                    and shared_terminal(archive.read(f"{prefix}/{LOCK_WAIT}.vm-release-trace.tsv")) == (2, 2), prefix)
        baseline = read_json(archive, "original/baseline/summary-100.json")
        require([row["round"] for row in baseline] == list(range(1, 101)), "基线 100 轮")
        for row in baseline:
            prefix = f"original/baseline/f1-{row['round']:03d}"
            require(row["exit"] == 0 and digest(archive.read(f"{prefix}/output.log")) == row["log_sha256"], prefix)
            for item in row["trace"].values():
                trace = archive.read(prefix + "/" + item["path"].replace("\\", "/"))
                require(digest(trace) == item["sha256"] and shared_terminal(trace) == (2, 2), prefix)
        check_matrix(archive, "local/f1", {LOCK_WAIT: 100}, product)
        check_matrix(archive, "local/matrix", {None: 10, **dict.fromkeys(CASES, 50)}, product)
        for platform, singles in (("windows", 50), ("linux", 30), ("macos", 30)):
            grades = check_matrix(archive, f"ci/trace/{platform}", {None: 10, **dict.fromkeys(CASES, singles)},
                                  product, ci["evidence"])
            require(grades == status["platforms"][platform]["verification"], platform)
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
            require(data["conclusion"] == "success" and data["attempt"] == row["attempt"]
                    and data["headSha"] == row["sha"] and row["jobs"] == {
                        job["name"]: job["databaseId"] for job in data["jobs"] if job["conclusion"] == "success"
                    }, key)
    require(status["22d_open"] is False and status["b3_4_started"] is False, "阶段状态")
    print("22D-FIX.2 证据对账通过")


if __name__ == "__main__":
    main()
