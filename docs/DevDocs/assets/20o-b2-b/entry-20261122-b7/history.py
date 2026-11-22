"""前序审计入口和固定 Git 证据身份。"""

import json
from pathlib import PureWindowsPath

BASE = "docs/DevDocs/assets"
EVIDENCE_COMMIT = "af96e8231abb7108ab7d93feeacafd490bd64695"
STAGES = {
    "a0": ("20o-b2-a0/entry-20261105", "d0e4a5fb", "verify-evidence.py", "evidence.json", "result.json", "closeout.json"),
    "a1": ("20o-b2-a1/entry-20261106", "39b2c1ea", "audit.py", "evidence.json"),
    "a2-a3": ("20o-b2-a2-a3/entry-20261106", "5ef17bb2", "audit.py", "evidence.json"),
    "a4": ("20o-b2-a4/entry-20261106", "1838fae1", "audit.py", "evidence.json"),
    "a5": ("20o-b2-a5/entry-20261106", "f21715d3", "audit.py", "evidence.json"),
    "a6": ("20o-b2-a6/entry-20261107", "e7bf7a49", "audit.py", "evidence.json"),
    "a7": ("20o-b2-a7/entry-20261107", "5749d6e0", "audit.py", "evidence.json", "closeout.json"),
}
B = "20o-b2-b/entry-"
STAGES.update({
    "b0": (B + "20261107", "d08b8460", "audit.py", "evidence.json", ".gitattributes"),
    "b1": (B + "20261108-b1", "35872782", "audit.py", "evidence.json"),
    "b2": (B + "20261108-b2", "37b92c53", "audit.py", "evidence.json"),
    "b3-reference": (B + "20261108-b3", "88886dd5", "audit.py", "evidence.json"),
    "b3-cabi": (B + "20261109-b3-cabi", "89d782e1", "audit.py", "evidence.json"),
    "b3-wrapper": (B + "20261110-b3-wrapper-final", "7d8b85e4", "audit.py", "evidence.json"),
    "b4-1": (B + "20261110-b4-0-1", "c7a45e10", "audit.py", "evidence.json"),
    "b4-2a": (B + "20261112-b4-2a-fix1", "c4610211", "audit.py", "evidence.json"),
    "b4-2b": (B + "20261113-b4-2b", "246bea46", "audit.py", "evidence.json"),
    "b4-3": (B + "20261113-b4-3", "e0d5993e", "audit.py", "evidence.json"),
    "b4-3-fix1": (B + "20261115-b4-3-fix1", "cfa79381", "audit.py", "evidence.json"),
    "b4-3-fix2": (B + "20261118-b4-component-fix", "43f18274", "audit.py", "evidence.json"),
    "b4-4": (B + "20261115-b4-4", "bb0b8196", "audit.py", "evidence.json"),
    "b4-5": (B + "20261116-b4-5-rerun", "cfa79381", "audit.py", "evidence.json", "closeout.json"),
    "b5": (B + "20261117-b5-closeout", "67319d61", "audit.py", "evidence.json", "closeout.json"),
    "b6-1": (B + "20261118-b6-1-rerun", "43f18274", "audit.py", "evidence.json", "result.json"),
    "b6-1-macos": (B + "20261118-b6-1-macos", "ef456f17", "audit.py", "evidence.json"),
    "b6-2": (B + "20261118-b6-2", "79ea0375", "audit.py", "summary.json", "measurements.zip", "review/source-boundary.json"),
    "b6-3": (B + "20261118-b6-3", "58176ff5", "audit.py", "summary.json", "boundaries.zip"),
    "b6-4": (B + "20261119-b6-4", "58176ff5", "evidence.py", "summary.json", "ci.zip"),
    "b6-4-followup": (B + "20261119-b6-4-followup", "780398e5", "audit.py", "status.json", "followup.zip", "local-checks.zip"),
    "b6-5": (B + "20261120-b6-5", "e79fab3c", "audit.py", "status.json", "handoff.json",
             "closeout.zip", "repo-check-locked.zip"),
    "b6-5-review": (B + "20261120-b6-5", "e79fab3c", "self_negative.py", "review.json"),
})


def inspect(bundle, git, digest, require):
    result = {}
    for label, (directory, revision, script, *sources) in STAGES.items():
        path = f"{BASE}/{directory}"
        receipt = bundle.read(f"history/{label}.json")
        report = bundle.read(f"history/{label}.stdout")
        require(receipt["exit"] == 0 and receipt["pid"] > 0 and receipt["started_utc"] <= receipt["finished_utc"]
                and receipt["argv"][1:4] == ["-B", "-X", "utf8"]
                and PureWindowsPath(receipt["argv"][4]) == PureWindowsPath(receipt["cwd"]) / path / script
                and not bundle.raw(f"history/{label}.stderr"), f"前序审计进程或入口不符：{label}")
        success = report.get("passed") is True or report.get("verified") is True
        if label in ("b0", "b1"):
            success = report["passed"] == report["total"] == 18
        if label == "b6-5":
            success = report["b6_5_passed"] is report["b7_entry_ready"] is True and report["b6_accepted"] is False
        require(success, f"前序审计未通过：{label}")
        source = git("rev-parse", "--verify", revision).decode("ascii").strip()
        reported = report.get("product", report.get("code_commit", report.get("baseline_commit")))
        require(reported is None or source.startswith(reported) or label == "a7", f"前序产品身份不符：{label}")
        tree = git("rev-parse", f"{EVIDENCE_COMMIT}:{path}").decode("ascii").strip()
        files = {item: digest(git("show", f"{EVIDENCE_COMMIT}:{path}/{item}")) for item in (script, *sources)}
        result[label] = {"source": source, "evidence_commit": EVIDENCE_COMMIT, "directory": path,
                         "tree": tree, "files": files}
    a0 = f"{BASE}/{STAGES['a0'][0]}"
    prior = json.loads(git("show", f"{EVIDENCE_COMMIT}:{a0}/result.json"))
    repaired = json.loads(git("show", f"{EVIDENCE_COMMIT}:{a0}/closeout.json"))
    require(prior["passed"] is False and repaired["passed"] is True, "A0 原始失败及有效收口身份不符")
    require(bundle.read("history/a7.stdout")["b1_721_executed"] is False, "A7 历史 721 边界不符")
    inherited = bundle.read("history/b6-5.stdout")
    b65 = f"{BASE}/{STAGES['b6-5'][0]}"
    require(inherited == json.loads(git("show", f"{EVIDENCE_COMMIT}:{b65}/status.json")),
            "B6-5 回执与固定状态不符")
    perf = bundle.read("history/b6-2.stdout")
    require(perf["perf_pass"] is True and perf["successful_processes"] == 8235
            and len(perf["coverage"]) == 9 and all(
                row["counts"] == {"passed": 183} and row["url"]["maximum"]["status"] == "passed"
                for row in perf["coverage"].values()
            ), "B6-2 历史性能覆盖不符")
    return result, inherited
