"""临时副本反例：要求退出码、实际拒绝文本及只读性同时正确。"""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
import audit


def run(root, repository):
    before = audit.scan(root)
    child = subprocess.run([
        sys.executable, "-B", "-X", "utf8", str(Path(__file__).with_name("audit.py")),
        "--root", str(root), "--repository-root", str(repository),
    ], capture_output=True, encoding="utf-8", errors="strict")
    audit.require(before == audit.scan(root), "自反例审计修改了输入目录")
    output = json.loads(child.stdout)
    audit.require(output["read_only"] is True and not child.stderr,
                  "审计非只读或发生未处理异常")
    return {"exit_code": child.returncode, **output}


def main():
    root = Path(__file__).resolve().parent
    repository = root.parents[4]
    baseline = run(root, repository)
    audit.require(baseline["exit_code"] == 0 and baseline["passed"], "真实归档审计未通过")
    results = {"baseline": baseline}
    with tempfile.TemporaryDirectory(prefix="xiao-url-evidence-negative-") as temporary:
        for case, expected in (
            ("manifest", "清单字节偏离固定Git对象"),
            ("missing", "已登记文件缺失：.gitattributes"),
            ("statistics", "证据文件字节与清单不符：result.json"),
            ("rebuilt_manifest", "清单字节偏离固定Git对象"),
        ):
            target = Path(temporary) / case
            shutil.copytree(root, target)
            if case == "manifest":
                path = target / "evidence.json"
                path.write_bytes(path.read_bytes() + b"\n")
            elif case == "missing":
                (target / ".gitattributes").unlink()
            else:
                path = target / "result.json"
                value = audit.load(path)
                value["workspace"]["passed"] += 1
                path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n",
                                encoding="utf-8", newline="\n")
                if case == "rebuilt_manifest":
                    manifest = audit.load(target / "evidence.json")
                    manifest["files"]["result.json"] = audit.scan(target)["result.json"]
                    (target / "evidence.json").write_text(
                        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
                        encoding="utf-8", newline="\n")
            result = run(target, repository)
            audit.require(result["exit_code"] == 1 and result["passed"] is False
                          and result["reason"] == expected,
                          "反例未按完整预期原因拒绝：" + case + "：" + str(result))
            results[case] = {"expected_reason": expected, **result}
    print(json.dumps({"passed": True, "cases": results}, ensure_ascii=False))


if __name__ == "__main__":
    main()
