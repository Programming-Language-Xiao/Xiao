"""在临时副本核对缺项、统计、来源、历史与状态反例的实际拒绝原因。"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import audit


def execute(root, repository, prepare):
    before = audit.inventory(root)
    argv = [sys.executable, "-B", "-X", "utf8", str(audit.ROOT / "audit.py"),
            "--root", str(root), "--repository-root", str(repository)]
    if prepare:
        argv += ["--prepare"]
    child = subprocess.run(argv, capture_output=True, encoding="utf-8", errors="strict")
    audit.require(not child.stderr and before == audit.inventory(root),
                  "自反例发生未处理异常或改写证据")
    result = json.loads(child.stdout)
    audit.require(result["read_only"] is True, "自反例审计不是只读")
    return {"exit_code": child.returncode, **result}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    baseline = execute(audit.ROOT, audit.REPOSITORY, args.prepare)
    audit.require(baseline["exit_code"] == 0 and baseline["passed"], "真实收口审计未通过")
    rows = [{"case": "baseline", "mode": "prepare" if args.prepare else "sealed",
             "expected_reason": None, **baseline}]
    semantic = {
        "missing": "收口必需文件缺失：.gitattributes",
        "source": "收口引用偏离固定Git来源",
        "component": "四类收口证据缺失或被替换",
        "historical": "历史失败/重建来源边界被改写",
        "statistics": "当前默认workspace统计不符",
        "state": "收口状态或发布边界冒进",
    }
    cases = dict(semantic)
    if not args.prepare:
        cases.update(manifest="收口清单偏离固定Git对象",
                     rehashed="收口清单偏离固定Git对象")
    with tempfile.TemporaryDirectory(prefix="xiao-b5-close-negative-") as temporary:
        for case, reason in cases.items():
            root = Path(temporary) / case
            shutil.copytree(audit.ROOT, root)
            path = root / "closeout.json"
            value = audit.load(path)
            if case == "missing":
                (root / ".gitattributes").unlink()
            elif case == "manifest":
                manifest = root / "evidence.json"
                manifest.write_bytes(manifest.read_bytes() + b"\n")
            else:
                if case == "source":
                    value["sources"]["matrix"]["commit"] = audit.ENTRY
                elif case == "component":
                    del value["components"]["current_default_workspace"]
                elif case == "historical":
                    value["historical_boundary"]["historical_originals_available"] = True
                elif case in ("statistics", "rehashed"):
                    value["current_workspace"]["passed"] += 1
                else:
                    value["b6_started"] = True
                path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n",
                                encoding="utf-8", newline="\n")
                if case == "rehashed":
                    manifest = root / "evidence.json"
                    manifest.write_text(json.dumps({
                        "files": {name: row for name, row in audit.inventory(root).items()
                                  if name != "evidence.json"}}, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
            # 语义反例绕过封存摘要，直接证明错误被识别；清单反例必须用默认封存模式。
            prepare = case in semantic
            result = execute(root, audit.REPOSITORY, prepare)
            audit.require(result["exit_code"] == 1 and result["passed"] is False
                          and result["reason"] == reason,
                          "自反例未按完整预期原因拒绝：" + case + "：" + str(result))
            rows.append({"case": case, "mode": "prepare" if prepare else "sealed",
                         "expected_reason": reason, **result})
    result = {"passed": True, "cases": rows}
    if args.output:
        from collect import write
        write(args.output, result)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
