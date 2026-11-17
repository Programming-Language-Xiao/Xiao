"""只读审计FIX.2平台回执、代码边界、最终docs门禁和实字节清单。"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
import finish

ROOT, REPOSITORY = finish.ROOT, finish.REPOSITORY
PREFIX = ROOT.relative_to(REPOSITORY).as_posix()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def inventory(root):
    return {
        path.relative_to(root).as_posix(): {
            "bytes": len(path.read_bytes()), "sha256": sha(path.read_bytes()),
        }
        for path in sorted(root.rglob("*")) if path.is_file()
    }


def git(*args):
    process = subprocess.run(
        ["cmd", "/c", "git", "-C", str(REPOSITORY), *args],
        capture_output=True,
    )
    require(process.returncode == 0, "FIX.2固定Git对象读取失败：" +
            process.stderr.decode("utf-8", errors="replace"))
    return process.stdout


def audit():
    before = inventory(ROOT)
    require(".gitattributes" in before, "FIX.2证据缺少.gitattributes")
    manifest = finish.read(ROOT, "evidence.json")
    actual = {name: row for name, row in before.items() if name != "evidence.json"}
    require(manifest["file_count_excluding_manifest"] == len(actual),
            "FIX.2清单文件数不符")
    require(manifest["files"] == actual, "FIX.2清单与实际字节不符")

    commit = git("log", "-1", "--format=%H", "--", PREFIX + "/evidence.json").decode().strip()
    require(commit and git("show", commit + ":" + PREFIX + "/evidence.json")
            == (ROOT / "evidence.json").read_bytes(), "FIX.2固定Git清单字节不符")

    result = finish.read(ROOT, "result.json")
    require(result == finish.collect(), "FIX.2结果与原始回执重算不一致")
    docs = finish.read(ROOT, finish.DOCS_RECEIPT)
    require(docs["exit_code"] == 0 and docs["report_passed"] is True
            and docs["source_unchanged"] is True,
            "FIX.2最终文档门禁未通过或源码发生变化")
    require(docs["result_sha256"] == sha((ROOT / "result.json").read_bytes()),
            "FIX.2文档门禁未绑定最终结果")
    docs_log = finish.unpack(docs["log"])
    report = json.loads(docs_log.decode("utf-8"))
    require(report["passed"] is True, "FIX.2文档门禁原始报告未通过")
    docs_after = finish.unpack(docs["docs_after"])
    require(docs_after == finish.canonical(finish.docs_snapshot()),
            "FIX.2文档门禁后输入树发生变化")

    b6_status = result["b6_handoff"]
    require(b6_status["b6_1_must_rerun_in_own_task_root"] is True
            and b6_status["b6_1_historical_status_rewritten"] is False
            and b6_status["b6_2_started"] is False,
            "FIX.2交接越过B6-1自有重跑")
    require(inventory(ROOT) == before, "FIX.2审计修改了证据")
    return {
        "passed": True, "read_only": True, "sealed": True,
        "stage": "B4-3-FIX.2", "fix2_closed": True,
        "code_commit": result["code_commit"],
        "b6_1_rerun_required": True, "b6_2_started": False,
        "file_count_excluding_manifest": len(actual),
    }


def main():
    try:
        outcome = audit()
        code = 0
    except Exception as error:
        outcome = {"passed": False, "read_only": True, "reason": str(error)}
        code = 1
    sys.stdout.reconfigure(encoding="utf-8")
    print(json.dumps(outcome, ensure_ascii=False))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
