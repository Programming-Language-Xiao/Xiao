"""采集或只读复核 B3-0C 的固定入口回执。"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parent
REPO = ROOT.parents[4]
ASSETS = ROOT.parent.relative_to(REPO).as_posix()
START = "6d81883be600bb6cc7459e6b63b8282c46fc3e2b"
PRIOR = {
    "0a": ("ac6865ea029ad116b0087c556e59b0f5ed6d896e", f"{ASSETS}/entry-20261122-b3-0", "entry.json"),
    "0b": ("b8abebf368ef9180ad05756ae90f5d198580a028", f"{ASSETS}/entry-20261123-b3-0b", "closeout.json"),
}
COMMANDS = {
    "0a-audit": f"{PRIOR['0a'][1]}/audit_entry.py",
    "0b-audit": f"{PRIOR['0b'][1]}/audit_closeout.py",
    "0b-negatives": f"{PRIOR['0b'][1]}/self_negative.py",
}
DIRECTIONS = {"B3-0C-FS", "B3-0C-ENV", "B3-0C-LOG"}
PRODUCT_PATHS = ("core", "stdlib", "tests", "cli", "tools", "resources", ".cargo", ".github", ".gitattributes",
                 "package.json", "bun.lock", "tsconfig.json", "repository.manifest.json",
                 "docs/UseDocs", "docs/module-registry.json")


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def git(*args, repo=REPO):
    launcher = ["cmd", "/c", "git"] if os.name == "nt" else ["git"]
    return subprocess.check_output([*launcher, *args], cwd=repo)


def blob(commit, path):
    return git("show", f"{commit}:{path}")


def identity(raw):
    return {"bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def encoded(data):
    return (json.dumps(data, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()


def fixed_sources():
    sources = {}
    for key, (commit, path, status) in PRIOR.items():
        require(not git("diff", "--name-only", "-z", commit, START, "--", path), f"{key} 入口对象偏离固定封存")
        manifest = blob(commit, f"{path}/evidence.json")
        report = blob(commit, f"{path}/{status}")
        sources[key] = {"commit": commit, "path": path, "manifest": identity(manifest), "status": identity(report)}
    require(not git("diff", "--name-only", "-z", PRIOR["0b"][0], START, "--", *PRODUCT_PATHS),
            "规划提交的产品或正式夹具发生变化")
    return sources


def verify_worktree():
    paths = [value[1] for value in PRIOR.values()]
    require(not git("diff", "--name-only", "-z", START, "--", *paths, *PRODUCT_PATHS)
            and not git("ls-files", "--others", "--exclude-standard", "-z", "--", *paths, *PRODUCT_PATHS),
            "入口采集时前批证据或产品工作区发生变化")


def verify_baseline(record):
    sources = fixed_sources()
    require(record["entry_commit"] == START and record["sources"] == sources, "入口来源或清单摘要不符")
    rows = record["commands"]
    require(rows.keys() == COMMANDS.keys(), "入口复跑命令缺项")
    for name, script in COMMANDS.items():
        row = rows[name]
        require(row["argv"][1:] == ["-B", "-X", "utf8", script] and row["exit"] == 0
                and not row["stderr"] and row["started_utc"] <= row["finished_utc"], f"入口复跑失败：{name}")
    facts = {}
    for source, (commit, path, status) in PRIOR.items():
        original = json.loads(blob(commit, f"{path}/{status}"))
        manifest = json.loads(blob(commit, f"{path}/evidence.json"))
        expected = original | {"audit_files": len(manifest["files"])}
        require(json.loads(rows[f"{source}-audit"]["stdout"]) == expected, f"{source} 复跑结论与固定原件不符")
        facts[source] = original
    hosts = facts["0a"]["platforms"]
    guards = {"passed": 28, "actual": 4, "derived": 24}
    require(facts["0a"]["b3_0a_baseline_frozen"] is True and facts["0a"]["guards"] == guards
            and set(hosts) == {"windows", "linux", "macos"}
            and all(row["b1"] == {"entries": 20, "resource_targets": 18, "vm_exact": 1, "native_exact": 1}
                    and all(row["observed"][module] for module in ("fs", "env", "log")) for row in hosts.values()),
            "三平台观察或 B1 前置不全")
    closeout = facts["0b"]
    handoff = closeout["handoff"]
    require(closeout["b3_0b_passed"] is closeout["b3_0c_entry_ready"] is True
            and closeout["decision_count"] == 12 and closeout["baseline"]["product"] == facts["0a"]["product"]
            and closeout["baseline"]["guards"] == guards and handoff["next"] == "20O-B3-0C"
            and handoff["modules"] == ["fs", "env", "log"]
            and handoff["windows_product_coff_policy"] == "complete_required"
            and handoff["windows_product_coff_status"] == "pending", "B3-0B 交接条件不符")

    b_commit, b_path, _ = PRIOR["0b"]
    original = json.loads(blob(b_commit, f"{b_path}/self-negative-result.json"))
    replay = json.loads(rows["0b-negatives"]["stdout"])
    fields = ("case", "exit", "stderr", "read_only")
    require(replay["mode"] == original["mode"] == "sealed"
            and [[row[key] for key in fields] for row in replay["cases"]]
            == [[row[key] for key in fields] for row in original["cases"]]
            and all(row["before"] == row["after"] for row in replay["cases"]), "B3-0B 封存态负例复跑不符")
    negatives = sum(row["exit"] == 1 for row in replay["cases"])
    require(negatives == closeout["complete_checks"]["negatives"] == 7, "B3-0B 负例数量不符")
    return {"product": facts["0a"]["product"], "platforms": sorted(hosts), "b1_guards": guards,
            "b3_0b_cases": negatives, "b3_0c_started": True, "b3_0c_passed": False, "b3_0d_entry_ready": False,
            "windows_product_coff_policy": handoff["windows_product_coff_policy"],
            "windows_product_coff_status": handoff["windows_product_coff_status"]}


def collect(task):
    require(not task.is_relative_to(REPO), "采集目录须独立于仓库")
    verify_worktree()
    sources = fixed_sources()
    task.mkdir(parents=True)
    rows = {}
    for name, script in COMMANDS.items():
        argv = [sys.executable, "-B", "-X", "utf8", script]
        started = datetime.now(timezone.utc).isoformat()
        result = subprocess.run(argv, cwd=REPO, encoding="utf-8", capture_output=True, timeout=120)
        rows[name] = {"argv": argv, "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
                      "started_utc": started, "finished_utc": datetime.now(timezone.utc).isoformat()}
    record = {"entry_commit": START, "sources": sources, "commands": rows}
    (task / "baseline.json").write_bytes(encoded(record))
    verify_worktree()
    return verify_baseline(record)


def audit():
    result = verify_baseline(json.loads((ROOT / "baseline.json").read_bytes()))
    record = json.loads((ROOT / "decisions.json").read_bytes())
    directions = record["direction_decisions"]
    require(record["stage"] == "20O-B3-0C" and directions.keys() == DIRECTIONS, "首轮裁定台账不全")
    pending = []
    for key, row in directions.items():
        if row["status"] == "pending":
            require(row["answer"] is row["question_item_id"] is None, f"待答项混入裁定：{key}")
            pending.append(key)
        else:
            require(row["status"] == "answered" and row["answer"] and row["question_item_id"], f"答复来源不全：{key}")
    return result | {"direction_decisions_pending": sorted(pending)}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collect", type=Path, metavar="TASK", help="在仓库外的全新任务目录保留入口原件")
    args = parser.parse_args()
    try:
        result = collect(args.collect.resolve()) if args.collect else audit()
        print(encoded(result).decode(), end="")
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"B3-0C 入口拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
