"""B6审计反例：固定失败状态不能被伪造通过，清单错误需识别实际原因。"""
import json
import argparse
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import audit


def run(root, prepare):
    before = audit.inventory(root)
    command = [
        sys.executable, "-B", "-X", "utf8", str(audit.ROOT / "audit.py"),
        "--root", str(root), "--repository-root", str(audit.REPOSITORY),
    ]
    if prepare:
        command.append("--prepare")
    proc = subprocess.run(command, capture_output=True, encoding="utf-8", errors="strict")
    audit.require(not proc.stderr and before == audit.inventory(root),
                  "反例审计抛出未捕获异常或改写证据")
    outcome = json.loads(proc.stdout)
    audit.require(outcome["read_only"], "反例审计不是只读")
    return {"exit_code": proc.returncode, **outcome}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    root = audit.ROOT
    baseline = run(root, prepare=args.prepare)
    audit.require(baseline["exit_code"] == 0 and baseline["passed"]
                  and baseline["stage_passed"] is False,
                  "B6基线预审未保持受阻状态")
    cases = [
        ("missing", "B6证据必需文件缺失：.gitattributes"),
        ("status", "B6阶段状态或平台阻断被改写：stage_passed"),
        ("linux", "未验证或失败平台被冒进升级"),
    ]
    if not args.prepare:
        cases.extend([
            ("file_count", "B6清单文件数与实际字节不符"),
            ("manifest", "B6清单与实际字节不符"),
            ("rehash", "B6固定Git清单字节不符"),
        ])
    rows = [{"case": "baseline", "expected_reason": None, **baseline}]
    with tempfile.TemporaryDirectory(prefix="xiao-b6-semantic-negative-") as temporary:
        for case, expected in cases:
            copy = Path(temporary) / case
            shutil.copytree(root, copy)
            if case == "missing":
                (copy / ".gitattributes").unlink()
            elif case == "file_count":
                path = copy / "evidence.json"
                manifest = json.loads(path.read_text(encoding="utf-8"))
                manifest["file_count_excluding_manifest"] += 1
                path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
                                encoding="utf-8", newline="\n")
            elif case == "manifest":
                path = copy / "evidence.json"
                manifest = json.loads(path.read_text(encoding="utf-8"))
                manifest["files"]["README.md"]["sha256"] = "0" * 64
                path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
                                encoding="utf-8", newline="\n")
            else:
                path = copy / "result-final-r5.json"
                result = json.loads(path.read_text(encoding="utf-8"))
                if case == "status":
                    result["stage_passed"] = True
                elif case == "linux":
                    result["linux"]["native_cmath_wrappers_o0_o3"]["passed"] = True
                else:
                    result["b6_closed"] = True
                path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n",
                                encoding="utf-8", newline="\n")
                if case == "rehash":
                    manifest_path = copy / "evidence.json"
                    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
                    manifest["files"]["result-final-r5.json"] = audit.identity(path.read_bytes())
                    manifest_path.write_text(
                        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
            prepare = args.prepare or case in ("missing", "status", "linux")
            outcome = run(copy, prepare=prepare)
            audit.require(outcome["exit_code"] == 1 and outcome["passed"] is False
                          and outcome["reason"] == expected,
                          "B6语义反例未按预期原因拒绝：" + case)
            rows.append({"case": case, "expected_reason": expected,
                         "mode": "prepare" if prepare else "sealed", **outcome})
    report = {"passed": True, "cases": rows}
    if args.output:
        audit.record.write_new(args.output, report)
    print(json.dumps(report, ensure_ascii=False))


if __name__ == "__main__":
    main()
