"""在临时副本验证 B3-0E 反例及审计只读性。"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

sys.dont_write_bytecode = True
from audit import ROOT, encoded, identity, inventory


def snapshot(root):
    return identity(encoded({"files": inventory(root), "manifest": identity((root / "evidence.json").read_bytes())}))["sha256"]


def write_json(root, name, record):
    (root / name).write_bytes(encoded(record))


def tamper(root, case):
    if case == "manifest-tampered":
        manifest = json.loads((root / "evidence.json").read_bytes())
        manifest["files"][".gitattributes"]["bytes"] += 1
        write_json(root, "evidence.json", manifest)
        return
    if case == "missing-attributes":
        (root / ".gitattributes").unlink()
    elif case == "replaced-golden":
        (root / "candidates/tests/fixtures/docs/cli/tutorial.stdout").write_bytes(b"wrong\n")
    elif case == "missing-input":
        (root / "candidates/tests/fixtures/docs/args/tutorial.input.json").unlink()
    elif case == "parse-as-execution":
        syntax = json.loads((root / "syntax.json").read_bytes())
        syntax["product_execution"] = True
        write_json(root, "syntax.json", syntax)
    elif case == "forged-approval":
        decisions = json.loads((root / "decisions.json").read_bytes())
        decisions["decisions"]["B3-0E-PATH"] = {"status": "approved", "answer": ""}
        write_json(root, "decisions.json", decisions)
    else:
        matrix = json.loads((root / "tutorials.json").read_bytes())
        if case == "forged-tutorial-hash":
            matrix["cases"]["time"]["assets"]["tutorial"]["sha256"] = "0" * 64
        elif case == "forged-verified":
            matrix["status"] = "verified"
        elif case == "forged-entry":
            matrix["cases"]["env"]["native"] = "different_entry"
        else:
            raise ValueError(case)
        write_json(root, "tutorials.json", matrix)
    write_json(root, "evidence.json", {"schema": 1, "files": inventory(root)})


def replay(folder, case, reason, prepare):
    root = folder / case
    shutil.copytree(ROOT, root)
    if reason:
        tamper(root, case)
    before = snapshot(root)
    command = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit.py"), "--evidence-root", str(root)]
    if prepare:
        command.append("--prepare")
    result = subprocess.run(command, capture_output=True, encoding="utf-8", timeout=60)
    after = snapshot(root)
    expected = f"B3-0E 拒绝：{reason}\n" if reason else ""
    if before != after or result.returncode != int(bool(reason)) or result.stderr != expected:
        raise RuntimeError(f"{case} 的审计结果不符，退出码 {result.returncode}\n{result.stderr}")
    return {"case": case, "exit": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
            "before": before, "after": after}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--output", type=Path, help="写入指定位置的新回执")
    args = parser.parse_args()
    before = snapshot(ROOT)
    cases = json.loads((ROOT / "negative-cases.json").read_bytes())
    with TemporaryDirectory(prefix="xiao-b3-0e-negative-") as temporary:
        rows = [replay(Path(temporary), case, reason, args.prepare) for case, reason in cases.items()]
    if snapshot(ROOT) != before:
        raise RuntimeError("自反例复跑期间证据目录字节发生变化")
    if args.output:
        with args.output.open("xb") as output:
            output.write(encoded({"cases": rows}))
    print(json.dumps({"read_only": True, "baseline_passed": True, "rejected_cases": len(rows) - 1}))


if __name__ == "__main__":
    main()
