"""只读复核 B6-1 macOS 的失败记录和专项通过证据。"""

import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parents[4] / "tools/gates"))

from b2_b6_macos import file_identity, require, snapshot, source_digest


SUCCESS = "44243357695"
RUNNER = "4bc244c90b41da2aa314d3da05e2d30bd647cc5f"
SOURCE = "ef456f17e29ca3955f2106847b85b018f946cb44"
JOBS = {"native": 134596281404, "resources": 134596281121}
FAILED = ("44241879381", "44242377713")
SUITES = {
    "native": ("runtime-build", "wrappers", "core", "errors", "divide", "b1"),
    "resources": ("runtime-build", "runtime", "vm", "codegen", "driver", "b1-vm", "fmt", "clippy"),
}


def main():
    entries = json.loads((ROOT / "evidence.json").read_bytes())["files"]
    objects = {}
    for name, identity in entries.items():
        digest = identity["sha256"]
        path = ROOT / "objects" / digest[:2] / digest
        if digest not in objects:
            require(file_identity(path) == identity, f"归档对象变化：{name}")
            objects[digest] = path
    require(set(objects.values()) == {path for path in (ROOT / "objects").rglob("*") if path.is_file()},
            "归档对象清单不匹配")

    def lookup(name):
        return json.loads(objects[entries[name]["sha256"]].read_bytes())

    for run_id in FAILED:
        run = lookup(f"{run_id}/workflow.json")
        require(run["conclusion"] == "failure" and f"{run_id}/failure.log" in entries,
                f"失败运行的原始记录缺失：{run_id}")
    run = lookup(f"{SUCCESS}/workflow.json")
    require(run["headSha"] == RUNNER and run["conclusion"] == "success" and run["attempt"] == 1
            and {job["name"]: job["databaseId"] for job in run["jobs"] if job["conclusion"] == "success"}
            == {f"B6-1 macOS {suite}": job for suite, job in JOBS.items()}, "成功运行的作业不匹配")
    api = lookup(f"{SUCCESS}/artifact-api.json")
    require({item["name"] for item in api["artifacts"]} ==
            {f"b6-1-macos-{suite}-{SUCCESS}-1" for suite in SUITES}, "成功运行的产物不匹配")
    files = snapshot(SOURCE)
    require(len(files) == 1059 and source_digest(files) ==
            "4040e675c82aa18ca99f6e675b7658f49497ba8fc9f0ec285659af31dfd3f4db", "固定源码不匹配")
    for suite, labels in SUITES.items():
        prefix = f"{SUCCESS}/artifacts/b6-1-macos-{suite}-{SUCCESS}-1/"
        require(lookup(prefix + "source.json") == files, f"作业源码不匹配：{suite}")
        result = lookup(prefix + "result.json")
        environment = lookup(prefix + "environment.json")
        require(result["passed"] and result["source_sha256"] == source_digest(files)
                and result["runner_commit"] == environment["runner_commit"] == RUNNER
                and environment["target"] == "aarch64-apple-darwin"
                and environment["fixed"]["product_commit"] == SOURCE, f"作业身份不匹配：{suite}")
        for label in labels:
            receipt = lookup(prefix + label + ".json")
            log = receipt["log"]
            require(receipt["passed"] and receipt["exit_code"] == 0
                    and receipt["source_after_sha256"] == result["source_sha256"]
                    and receipt["runtime_after"] == result["runtime"]
                    and entries[prefix + log["name"]] == {key: log[key] for key in ("bytes", "sha256")},
                    f"命令与实际日志不匹配：{suite}/{label}")
        if suite == "native":
            rows = lookup(prefix + "wrappers.json")["native_rows"]
            require(len(rows) == 4 and {(row["module"], row["optimization"]) for row in rows}
                    == {(module, level) for module in ("std.cmath", "stdlib.cmath") for level in (0, 3)}
                    and all(row["verification"] == "Complete"
                            and row["observed"] == row["declared"] == ["containers", "rc", "value", "weak"]
                            for row in rows), "Mach-O 四格组件回执不匹配")
        else:
            require(lookup(prefix + "runtime.json")["formal_reference_cases"] == 17243, "正式参考缺项")
    print(json.dumps({"passed": True, "runs": 3, "passing_jobs": len(JOBS), "files": len(entries),
                      "unique_objects": len(objects), "source_commit": SOURCE}))


if __name__ == "__main__":
    main()
