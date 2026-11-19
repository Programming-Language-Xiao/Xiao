"""只读审计FIX.2平台回执、代码边界、最终docs门禁和实字节清单。"""
import argparse
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
    missing = sorted(set(manifest["files"]) - set(actual))
    require(not missing, "FIX.2清单文件缺失：" + ", ".join(missing))
    require(manifest["file_count_excluding_manifest"] == len(actual),
            "FIX.2清单文件数不符")
    require(manifest["files"] == actual, "FIX.2清单与实际字节不符")

    result = finish.read(ROOT, "result.json")
    b6_status = result["b6_handoff"]
    require(b6_status["b6_1_must_rerun_in_own_task_root"] is True
            and b6_status["b6_1_historical_status_rewritten"] is False
            and b6_status["b6_2_started"] is False,
            "FIX.2交接越过B6-1自有重跑")
    windows_rows = result["native_wrapper_rows"]["windows"]
    require(result["fix0"]["windows_verification"] == "UnverifiedCoffExports"
            and len(windows_rows) == 4
            and all(row["verification"] == "UnverifiedCoffExports" for row in windows_rows),
            "FIX.2 Windows组件完整性被冒进标记")
    require(result == finish.collect(), "FIX.2结果与原始回执重算不一致")

    commit = git("log", "-1", "--format=%H", "--", PREFIX + "/evidence.json").decode().strip()
    require(commit and git("show", commit + ":" + PREFIX + "/evidence.json")
            == (ROOT / "evidence.json").read_bytes(), "FIX.2固定Git清单字节不符")

    require(manifest["docs_receipt"] == finish.DOCS_RECEIPT,
            "FIX.2清单与文档门禁回执身份不符")
    docs = finish.read(ROOT, manifest["docs_receipt"])
    require(docs["exit_code"] == 0 and docs["report_passed"] is True
            and docs["source_unchanged"] is True,
            "FIX.2最终文档门禁未通过或源码发生变化")
    require(docs["result_sha256"] == sha((ROOT / "result.json").read_bytes()),
            "FIX.2文档门禁未绑定最终结果")
    docs_log = finish.unpack(docs["log"])
    report = json.loads(docs_log.decode("utf-8"))
    require(report["passed"] is True, "FIX.2文档门禁原始报告未通过")
    docs_after = finish.unpack(docs["docs_after"])
    require(docs_after == finish.unpack(docs["docs_before"]),
            "FIX.2文档门禁前后输入字节发生变化")
    docs_inputs = json.loads(docs_after)
    required = set(finish.DOC_FILES) | {
        PREFIX + "/" + name for name in before
        if not name.startswith("objects/")
        and name not in {manifest["docs_receipt"], "evidence.json"}
    }
    require(set(docs_inputs) == required, "FIX.2文档门禁输入集合不全")
    # 文档会随后续阶段更新，历史回执只与封存提交的Git对象对照。
    fixed_docs = finish.git_source_snapshot(commit, tuple(docs_inputs))
    require(set(fixed_docs) == set(docs_inputs), "FIX.2封存Git文档路径集合不符")
    for name, digest in fixed_docs.items():
        require(docs_inputs[name]["lf_sha256"] == digest,
                "FIX.2门禁输入偏离封存Git对象：" + name)
    require(inventory(ROOT) == before, "FIX.2审计修改了证据")
    return {
        "passed": True, "read_only": True, "sealed": True,
        "stage": "B4-3-FIX.2", "fix2_closed": True,
        "code_commit": result["code_commit"],
        "b6_1_rerun_required": True, "b6_2_started": False,
        "file_count_excluding_manifest": len(actual),
    }


def main():
    global ROOT, REPOSITORY
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT,
                        help="证据目录，可指定临时副本")
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY,
                        help="保存固定Git对象的仓库")
    args = parser.parse_args()
    ROOT, REPOSITORY = args.root.resolve(), args.repository_root.resolve()
    finish.ROOT, finish.REPOSITORY = ROOT, REPOSITORY
    before = inventory(ROOT)
    try:
        outcome = audit()
        code = 0
    except Exception as error:
        outcome = {"passed": False, "reason": str(error)}
        code = 1
    outcome["read_only"] = inventory(ROOT) == before
    if not outcome["read_only"]:
        outcome.update(passed=False, reason="FIX.2审计修改了证据")
        code = 1
    sys.stdout.reconfigure(encoding="utf-8")
    print(json.dumps(outcome, ensure_ascii=False))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
