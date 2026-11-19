"""在临时副本复现 B4-2B 原审计的汇总、历史状态与报告缺失旁路；不改原归档。"""
import argparse
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
ORIGINAL = ROOT.parent / "entry-20261113-b4-2b"
sys.path.insert(0, str(ORIGINAL))
import audit as original
from record import write_json, sha, store_content_log


def replace_log(root, variant, label, transform):
    """改变一个日志对象，同步其独立回执及派生汇总，避免仅由摘要漂移拒绝。"""
    path = root / variant / (label + ".json")
    row = original.load(path)
    old = row["log_ref"]
    changed = transform((root / old).read_text(encoding="utf-8")).encode("utf-8")
    reference, digest = store_content_log(root, changed)
    row.update(log_ref=reference, log_sha256=digest, log_bytes=len(changed))
    path.write_text(json.dumps(row), encoding="utf-8")
    closeout = original.load(root / "closeout.json")
    closeout["runs"][variant + "/" + label]["log_sha256"] = digest
    (root / "closeout.json").write_text(json.dumps(closeout), encoding="utf-8")
    (root / old).unlink()


def apply_mutation(root, case):
    """只改变反例所需字段，之后重新生成真实字节清单。"""
    if case == "redistributed_counts":
        replace_log(root, "revised", "regressions", lambda text: text.replace(
            "8 passed; 0 failed; 0 ignored;", "9 passed; 0 failed; 0 ignored;", 1).replace(
            "29 passed; 0 failed; 0 ignored;", "28 passed; 0 failed; 0 ignored;", 1))
    elif case == "historical_state":
        def change(text):
            value = json.loads(text)
            value["language_available"] = True
            value["cmath"] = "verified"
            return json.dumps(value)
        replace_log(root, "fixed", "b34", change)
    elif case == "missing_negative_report":
        (root / "self-negative-findings.json").unlink()
    (root / "evidence.json").write_text(json.dumps({"files": original.inventory(root)}), encoding="utf-8")


def run():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expect-rejections", action="store_true")
    args = parser.parse_args()
    cases = []
    for case in ["baseline", "redistributed_counts", "historical_state", "missing_negative_report"]:
        with tempfile.TemporaryDirectory(prefix="xiao-b4-2b-before-review-") as temporary:
            copied = Path(temporary) / "evidence"
            shutil.copytree(ORIGINAL, copied)
            apply_mutation(copied, case)
            before = original.all_entries(copied)
            process = subprocess.run([sys.executable, "-B", "-X", "utf8", str(copied / "audit.py"),
                "--root", str(copied), "--repository-root", str(original.REPOSITORY)],
                capture_output=True, encoding="utf-8", text=True)
            original.require(before == original.all_entries(copied), "复现审计改写了证据副本")
            raw = (process.stdout + process.stderr).encode("utf-8")
            reference, digest = store_content_log(ROOT, raw)
            if args.expect_rejections:
                reason = {"redistributed_counts": "逐目标测试计数不符：revised/regressions",
                          "historical_state": "历史审计精确结果不符：b34",
                          "missing_negative_report": "缺少已验收的自反例报告"}.get(case)
                if reason is None:
                    original.require(process.returncode == 0 and json.loads(process.stdout)["self_negative_checked"] is True, "复验基线不通过")
                else:
                    original.require(process.returncode == 1 and process.stdout.strip() == reason and not process.stderr,
                                     "反例未按精确原因拒绝：" + case + process.stdout + process.stderr)
            cases.append({"case": case, "exit_code": process.returncode, "read_only": True,
                          "log_ref": reference, "log_sha256": digest, "stdout": process.stdout.strip()})
    write_json(args.output, {"original_archive_commit": "81e45783", "cases": cases,
                            "auditor_sha256": sha((ORIGINAL / "audit.py").read_bytes()),
                            "expected_rejections_checked": args.expect_rejections})
    print(json.dumps({"cases": [{"case": row["case"], "exit_code": row["exit_code"]} for row in cases]}, ensure_ascii=False))


if __name__ == "__main__":
    run()
