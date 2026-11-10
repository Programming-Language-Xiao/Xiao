"""在临时证据副本中验证审计只读、清单篡改和语义冒进均被明确拒绝。"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

import audit


def inventory(root):
    return [(path.relative_to(root).as_posix(), hashlib.sha256(path.read_bytes()).hexdigest())
            for path in sorted(path for path in root.rglob("*") if path.is_file())]


def run(root):
    return subprocess.run([sys.executable, "-B", "-X", "utf8", str(root / "audit.py"),
                           "--root", str(root), "--repository-root", str(audit.REPOSITORY)],
                          cwd=audit.REPOSITORY, capture_output=True, text=True, encoding="utf-8")


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")


def rebuild_manifest(root):
    manifest = json.loads((root / "evidence.json").read_text(encoding="utf-8"))
    manifest["files"] = audit.evidence_inventory(root)
    write_json(root / "evidence.json", manifest)


def main():
    output = audit.ROOT / "self-negative-result-final.json"
    if output.exists():
        raise SystemExit("拒绝覆盖B3-4自反例结果")
    cases = []
    with tempfile.TemporaryDirectory(prefix="b3-4-closeout-self-negative-") as temp:
        baseline = Path(temp) / "baseline"
        shutil.copytree(audit.ROOT, baseline)
        before = inventory(baseline)
        result = run(baseline)
        after = inventory(baseline)
        if result.returncode != 0 or before != after:
            raise SystemExit("基线审计失败或改写证据：" + result.stdout + result.stderr)
        cases.append({"case": "baseline-read-only", "exit_code": result.returncode, "read_only": before == after})

        tampered = Path(temp) / "tampered-manifest"
        shutil.copytree(baseline, tampered)
        with (tampered / "unexpected.bin").open("xb") as stream:
            stream.write(b"tamper")
        result = run(tampered)
        reason = "B3-4证据清单与实际文件字节不符"
        if result.returncode != 1 or reason not in result.stdout + result.stderr:
            raise SystemExit("清单反例未按声明原因拒绝：" + result.stdout + result.stderr)
        cases.append({"case": "unlisted-file", "exit_code": result.returncode, "expected_reason": reason})

        semantic = Path(temp) / "semantic-overclaim"
        shutil.copytree(baseline, semantic)
        closeout_path = semantic / "closeout.json"
        closeout = json.loads(closeout_path.read_text(encoding="utf-8"))
        closeout["language_available"] = True
        write_json(closeout_path, closeout)
        rebuild_manifest(semantic)
        result = run(semantic)
        reason = "B3-4 closeout状态不符"
        if result.returncode != 1 or reason not in result.stdout + result.stderr:
            raise SystemExit("状态冒进反例未按声明原因拒绝：" + result.stdout + result.stderr)
        cases.append({"case": "language-status-overclaim", "exit_code": result.returncode, "expected_reason": reason})
    value = {"passed": True, "read_only": True, "cases": cases}
    with output.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(value, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(value, ensure_ascii=False))


if __name__ == "__main__":
    main()
