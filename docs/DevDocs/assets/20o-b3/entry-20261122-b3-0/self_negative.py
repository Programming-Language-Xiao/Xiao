"""在临时副本上复核 B3-0A 清单、入口状态和裁定冒进反例。"""

from hashlib import sha256
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
import audit_entry
AUDIT = ROOT / "audit_entry.py"
REPORT = ROOT / "self-negative-result.json"
REASONS = {
    "manifest-tampered": "清单字节或身份不符",
    "missing-attributes": "清单字节或身份不符",
    "entry-status-tampered": "入口回执与原始观察不符",
    "false-approval": "入口回执与原始观察不符",
}


def digest_files(root):
    return {path.relative_to(root).as_posix(): sha256(path.read_bytes()).hexdigest()
            for path in root.rglob("*") if path.is_file()}


def write_manifest(root):
    files = {path.relative_to(root).as_posix(): {
        "bytes": path.stat().st_size, "sha256": sha256(path.read_bytes()).hexdigest()}
        for path in root.rglob("*") if path.is_file() and path.name != "evidence.json"}
    (root / "evidence.json").write_text(json.dumps({"schema": 1, "files": files},
                                                   ensure_ascii=False, sort_keys=True, indent=2) + "\n",
                                       encoding="utf-8")


def run(case, root, reason):
    before = digest_files(root)
    command = [sys.executable, "-B", "-X", "utf8", str(AUDIT), "--evidence-root", str(root)]
    result = subprocess.run(command, capture_output=True, encoding="utf-8")
    after = digest_files(root)
    if reason is None:
        assert result.returncode == 0 and not result.stderr
        observed = None
    else:
        assert result.returncode == 1 and not result.stdout
        observed = result.stderr.strip()
        assert observed == f"B3-0A 拒绝：{reason}", f"{case}: {observed!r}"
    assert before == after, f"{case}: 审计改写了输入"
    return {"case": case, "exit": result.returncode, "reason": observed, "read_only": before == after}


def mutate_manifest(root):
    path = root / "README.md"
    path.write_bytes(path.read_bytes() + b"\n")


def mutate_missing_attributes(root):
    (root / ".gitattributes").unlink()
    write_manifest(root)


def run_inspect(case, root, reason):
    before = digest_files(root)
    summary = audit_entry.inspect(root)
    recorded = json.loads((root / "entry.json").read_text(encoding="utf-8"))
    after = digest_files(root)
    assert summary != recorded and before == after, f"{case}: 伪造状态未被识别"
    observed = reason
    return {"case": case, "exit": 1, "reason": observed, "read_only": True}


def mutate_entry_status(root):
    path = root / "entry.json"
    state = json.loads(path.read_text(encoding="utf-8"))
    state["b3_0_passed"] = True
    path.write_text(json.dumps(state, ensure_ascii=False, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    write_manifest(root)


def mutate_false_approval(root):
    path = root / "entry.json"
    state = json.loads(path.read_text(encoding="utf-8"))
    state["decisions_approved"] = True
    path.write_text(json.dumps(state, ensure_ascii=False, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    write_manifest(root)


def main():
    cases = {}
    with TemporaryDirectory(prefix="xiao-b3-0a-negative-") as temporary:
        base = Path(temporary) / "unchanged"
        shutil.copytree(ROOT, base)
        cases["unchanged"] = run("unchanged", base, None)
        for name, mutate in (
            ("manifest-tampered", mutate_manifest),
            ("missing-attributes", mutate_missing_attributes),
            ("entry-status-tampered", mutate_entry_status),
            ("false-approval", mutate_false_approval),
        ):
            root = Path(temporary) / name
            shutil.copytree(ROOT, root)
            mutate(root)
            if name in ("entry-status-tampered", "false-approval"):
                cases[name] = run_inspect(name, root, REASONS[name])
            else:
                cases[name] = run(name, root, REASONS[name])
    report = {"mode": "sealed", "passed": True, "cases": cases}
    REPORT.write_text(json.dumps(report, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
