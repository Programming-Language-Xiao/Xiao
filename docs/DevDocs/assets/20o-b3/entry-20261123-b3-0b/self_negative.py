"""在临时副本执行 B3-0B 实际审计，核对拒绝文本和只读性。"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

sys.dont_write_bytecode = True
from audit_closeout import (MANIFEST, PLAN, REASONS, ROOT, SEALED_NEGATIVE_REASON, base, blob,
                            identity, json_bytes, require)
from preflight import rewrite_manifest, snapshot


def replace_field(path, key, value):
    payload = json.loads(path.read_bytes())
    payload[key] = value
    path.write_bytes(json_bytes(payload))


def tamper(root, name):
    if name == "manifest-tampered":
        with (root / MANIFEST).open("ab") as output:
            output.write(b"\n")
        return
    if name == "missing-attributes":
        (root / ".gitattributes").unlink()
    elif name == "forged-status":
        path = root / "closeout.json"
        ready = json.loads(path.read_bytes())["b3_0c_entry_ready"]
        replace_field(path, "b3_0c_entry_ready", not ready)
    elif name == "forged-observation":
        path = root / "preflight.json"
        payload = json.loads(path.read_bytes())
        payload["bun_argv"]["literal_and_unpaired_same_result"] = False
        path.write_bytes(json_bytes(payload))
    elif name == "direction-as-freeze":
        path = root / "decisions.json"
        payload = json.loads(path.read_bytes())
        payload["detail_decisions"] = {}
        for row in payload["direction_decisions"].values():
            row["status"] = "frozen"
        path.write_bytes(json_bytes(payload))
    elif name == "replaced-doc-blob":
        path = root / "doc-gate.json"
        payload = json.loads(path.read_bytes())
        document = "docs/DevDocs/20o-b3-0b-time-args-cli-plan.md"
        old_bytes = blob(PLAN, document)
        for key in ("before", "after"):
            payload[key]["markdown"][document] = identity(old_bytes)
        path.write_bytes(json_bytes(payload))
    elif name == "forged-negative-report":
        path = root / "self-negative-result.json"
        payload = json.loads(path.read_bytes())
        payload["cases"][1]["before"] = payload["cases"][1]["after"] = "0" * 64
        path.write_bytes(json_bytes(payload))
    else:
        raise ValueError(f"未知反例：{name}")
    rewrite_manifest(root)


def replay(parent, name, reason, preflight):
    root = parent / name
    shutil.copytree(ROOT, root)
    if name != "unchanged":
        tamper(root, name)
    before = base.sha(json_bytes(snapshot(root)))
    argv = [sys.executable, "-B", "-X", "utf8", str(ROOT / "audit_closeout.py"), "--evidence-root", str(root)]
    if preflight:
        argv.append("--preflight")
    result = subprocess.run(argv, capture_output=True, encoding="utf-8", timeout=60)
    after = base.sha(json_bytes(snapshot(root)))
    expected = f"B3-0B 拒绝：{reason}\n" if reason else ""
    require(result.returncode == int(reason is not None) and result.stderr == expected and before == after,
            f"{name} 的拒绝或只读性不符：{result.returncode} {result.stderr}")
    if reason:
        require(not result.stdout, f"拒绝时出现成功输出：{name}")
    else:
        status = json.loads(result.stdout)
        passed = not preflight and status["decisions_approved"]
        require(status["b3_0b_passed"] is passed and status["b3_0c_entry_ready"] is passed,
                "正向用例的状态不符")
    return {"case": name, "argv": argv, "exit": result.returncode, "stderr": result.stderr,
            "read_only": True, "before": before, "after": after}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    cases = {"unchanged": None} | REASONS
    if not args.preflight:
        cases["forged-negative-report"] = SEALED_NEGATIVE_REASON
    with TemporaryDirectory(prefix="xiao-b3-0b-negative-") as temporary, ThreadPoolExecutor(max_workers=3) as pool:
        results = list(pool.map(lambda item: replay(Path(temporary), *item, args.preflight), cases.items()))
    raw = json_bytes({"mode": "preflight" if args.preflight else "sealed", "cases": results})
    if args.output:
        with args.output.open("xb") as output:
            output.write(raw)
    print(raw.decode(), end="")
