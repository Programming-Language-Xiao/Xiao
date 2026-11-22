"""只读核对 B2-C 的公共证据和收口结论。"""

import argparse
import hashlib
import json
from pathlib import Path
import sys
from zipfile import BadZipFile, ZipFile

sys.dont_write_bytecode = True
from entry_audit import ANCHOR, ASSETS, PREFIX, ROOT, base, blob, derive, inspect, json_bytes
from gate_audit import STATE, verify_engineering, verify_status

CLOSEOUT = "closeout.json"
MANIFEST = "evidence.json"
NEGATIVES = "self-negative-result.json"
FROZEN_TOOLS = ("measure.py", "candidate.py", "versions.py", "download.py", ".gitattributes")
REQUIRED = {*FROZEN_TOOLS, "README.md", "audit.py", "seal.py", "entry_audit.py", "gate_audit.py",
            "performance_audit.py", "coff.py", "self_negative.py", "cmath-tutorial.stdout",
            "cmath-tutorial.xiao", "execution.zip", "gates.zip", CLOSEOUT}
REJECTION_REASONS = {
    "manifest-tampered": "清单字节或身份不符",
    "missing-gitattributes": "封存缺少文件：.gitattributes",
    "f10-missing": "F10 负载、规模、平台或优化级别集合不符",
    "f10-duplicate": "F10 负载、规模、平台或优化级别集合不符",
    "candidate-missing-o3": "候选缺 O0/O3",
    "candidate-missing-platform": "公开候选缺平台",
    "forged-source": "测量源码或工具链身份不符",
    "forged-runtime": "测量 Runtime 身份不符",
    "forged-ci-job": "CI 作业或上传原件身份不符：137179416348",
    "forged-status": "收口回执与逐项派生结论不符",
}


def inventory(root):
    return {file.relative_to(root).as_posix(): base.identity(file)
            for file in sorted(root.rglob("*")) if file.is_file() and file != root / MANIFEST}


def verify_files(root, preflight):
    observed = inventory(root)
    required = REQUIRED if preflight else REQUIRED | {NEGATIVES}
    base.require(required <= observed.keys(), f"封存缺少文件：{', '.join(sorted(required - observed.keys()))}")
    base.require((root / MANIFEST).read_bytes() == json_bytes({"schema": 1, "files": observed}),
                 "清单字节或身份不符")
    return len(observed)


def verify_negatives(report):
    expected = {"unchanged": (0, None)} | {name: (1, reason) for name, reason in REJECTION_REASONS.items()}
    rows = report["cases"]
    base.require(len(rows) == len(expected) and {row["case"] for row in rows} == expected.keys()
                 and report["mode"] == "preflight", "自反例执行集合不符")
    for row in rows:
        name = row["case"]
        base.require((row["exit"], row["rejection"]) == expected[name]
                     and row["read_only"] is True and row["before"] == row["after"]
                     and len(row["before"]) == 64 and row["argv"][-1] == "--preflight",
                     f"自反例实际拒绝原因或只读性不符：{name}")


def verify_history():
    a0 = f"{ASSETS}/20o-b2-a0/entry-20261105"
    original = json.loads(blob(ANCHOR, f"{a0}/result.json"))
    valid = json.loads(blob(ANCHOR, f"{a0}/closeout.json"))
    base.require(original["passed"] is False and valid["passed"] is True, "A0 原始失败或有效收口身份不符")
    b5_path = f"{ASSETS}/20o-b2-b/entry-20261117-b5-closeout/closeout.json"
    b5 = json.loads(blob(ANCHOR, b5_path))
    historical = b5["historical_boundary"]
    base.require(b5["b5_closed"] is True and historical["historical_originals_available"] is False
                 and historical["baseline_kind"] == "reconstructed_from_B1_source", "B5 历史原件的来源身份不符")
    return {
        "a0_original_passed": False, "a0_closeout_passed": True, "b1_721_historical": True, "b1_originals_missing": True,
        "b5": b5["components"],
        "independent": ["B1", "22C", "22D", "19", "11", "22"],
    }


def compute(root, preflight=False):
    result = inspect(root, root / "execution.zip")
    for name in (*FROZEN_TOOLS, "execution.zip"):
        git_name = f"{STATE}:{PREFIX}/{name}"
        git_sha = base.git("rev-parse", git_name).strip()
        raw = (root / name).read_bytes()
        observed = hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest().encode()
        base.require(observed == git_sha, f"固定 C4 证据对象不符：{name}")
    status = verify_status(root)
    with ZipFile(root / "gates.zip") as archive:
        engineering = verify_engineering(archive, result["dependencies"])
    if not preflight:
        verify_negatives(json.loads((root / NEGATIVES).read_bytes()))
    result.update({
        "status": status,
        "engineering": engineering,
        "history": verify_history(),
        "gates_archive": base.identity(root / "gates.zip"),
        "c4_registry_tutorial_passed": True,
        "c5_closeout_audit_passed": not preflight,
        "public_scope": {"runtime_abi": "1.19", "generator": 22, "stdlib_interface": 1,
                         "cmath_wrappers": 15, "lfloat_divide": True, "public_native_platforms": 3,
                         "windows_same_link_candidates": 10, "windows_product_coff_verified": False},
        "handoff": {"next": "20O-B3", "plan": "docs/DevDocs/20o-b3-host-libraries-plan.md", "started": False},
    })
    return result | derive(result)


def audit(root, preflight=False):
    files = verify_files(root, preflight)
    result = compute(root, preflight)
    base.require((root / CLOSEOUT).read_bytes() == json_bytes(result), "收口回执与逐项派生结论不符")
    return result | {"audit_files": files}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-root", type=Path, default=ROOT)
    parser.add_argument("--preflight", action="store_true", help="检查待自反例的回执，C5 始终为 false")
    args = parser.parse_args()
    try:
        print(json_bytes(audit(args.evidence_root.resolve(), args.preflight)).decode(), end="")
    except (ValueError, OSError, KeyError, BadZipFile) as error:
        print(f"B2-C 拒绝：{error}", file=sys.stderr)
        raise SystemExit(1) from None
