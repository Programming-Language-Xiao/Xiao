"""在临时副本复现审计缺口，不改写 FIX1 历史来源、运行或结果。"""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent
EVIDENCE = ROOT.parent
REPOSITORY = EVIDENCE.parents[4]
sys.path.insert(0, str(EVIDENCE))
from record import load, sha, write_json


def inventory(root):
    return {path.relative_to(root).as_posix(): {"bytes": path.stat().st_size, "sha256": sha(path.read_bytes())}
            for path in sorted(root.rglob("*")) if path.is_file() and path.name != "evidence.json"
            and "__pycache__" not in path.parts}


def run_case(case):
    with tempfile.TemporaryDirectory(prefix="xiao-fix1-review-") as temporary:
        copied = Path(temporary) / "evidence"
        shutil.copytree(EVIDENCE, copied, ignore=shutil.ignore_patterns("self-review-20261112"))
        args = []
        if case == "refresh_changes_language_state":
            closeout = load(copied / "closeout.json")
            closeout["language_available"] = True
            (copied / "closeout.json").write_text(json.dumps(closeout), encoding="utf-8")
            args = ["--refresh-results"]
        elif case == "unlisted_nested_evidence":
            nested = copied / "unlisted/evidence.json"
            nested.parent.mkdir()
            nested.write_text('{"forged":true}', encoding="utf-8")
        elif case == "parent_escape":
            env_path = copied / "fixed/environment.json"
            config = load(env_path)
            config["overrides"]["CARGO_TARGET_DIR"] = config["task_root"] + "\\..\\shared-target"
            env_path.write_text(json.dumps(config), encoding="utf-8")
            digest = sha(env_path.read_bytes())
            for path in (copied / "fixed").glob("*.json"):
                record = load(path)
                if "environment_sha256" in record:
                    record["environment_sha256"] = digest
                    path.write_text(json.dumps(record), encoding="utf-8")
            manifest = load(copied / "evidence.json")
            manifest["files"] = inventory(copied)
            (copied / "evidence.json").write_text(json.dumps(manifest), encoding="utf-8")
        # 排除自审目录后重建旧口径清单，避免其它文件影响此次反例。
        manifest = load(copied / "evidence.json")
        manifest["files"] = inventory(copied)
        (copied / "evidence.json").write_text(json.dumps(manifest), encoding="utf-8")
        child = subprocess.run([sys.executable, "-X", "utf8", str(copied / "audit.py"),
                                "--root", str(copied), "--repository-root", str(REPOSITORY), *args],
                               text=True, encoding="utf-8", capture_output=True)
        return {"case": case, "argv_options": args, "exit_code": child.returncode,
                "stdout": child.stdout, "stderr": child.stderr}


if __name__ == "__main__":
    rows = [run_case(case) for case in ["refresh_changes_language_state", "unlisted_nested_evidence", "parent_escape"]]
    report = {"audit_sha256": sha((EVIDENCE / "audit.py").read_bytes()), "cases": rows}
    write_json(ROOT / "before-negative.json", report)
    print(json.dumps(report, ensure_ascii=False))
