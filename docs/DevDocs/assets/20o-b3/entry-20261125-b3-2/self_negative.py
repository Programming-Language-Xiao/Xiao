"""用临时副本复跑 B3-2A 只读审计和守卫回执核验。"""

import json
from pathlib import Path
import shutil
import sys
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True
from audit import ROOT, audit, encoded, fixed_inputs, identity, inventory, require, verify_windows


def snapshot(root):
    return inventory(root) | {"evidence.json": identity((root / "evidence.json").read_bytes())}


def check(label, root, operation, expected=None):
    before = snapshot(root)
    try:
        operation()
    except RuntimeError as error:
        require(expected == str(error), f"{label} 拒绝原因不符：{error}")
    else:
        require(expected is None, f"{label} 应拒绝：{expected}")
    require(snapshot(root) == before, f"{label} 改写了待审文件")


def guard_case(root, edit, upstream, locks):
    with TemporaryDirectory(prefix="guard-variants-", dir=root) as directory:
        target = Path(directory)
        archive_path = target / "windows-baseline.zip"
        with (ZipFile(root / "windows-baseline.zip") as original,
              ZipFile(archive_path, "x", ZIP_DEFLATED) as changed):
            report = json.loads(original.read("result.json"))
            guards = json.loads(original.read("b1-guards-report.json"))
            edit(guards)
            raw = encoded(guards)
            report["b1_guard_report"].update(identity(raw))
            for name in original.namelist():
                if name not in ("result.json", "b1-guards-report.json"):
                    changed.writestr(name, original.read(name))
            changed.writestr("result.json", encoded(report))
            changed.writestr("b1-guards-report.json", raw)
        before = identity(archive_path.read_bytes())
        try:
            verify_windows(target, upstream, locks)
        finally:
            require(identity(archive_path.read_bytes()) == before, "守卫审计改写归档")


def main():
    original = snapshot(ROOT)
    check("原样基线", ROOT, lambda: audit(ROOT))
    upstream, locks, _, _ = fixed_inputs()
    with TemporaryDirectory(prefix="xiao-b3-2a-negative-") as directory:
        target = Path(directory)
        copy = target / "evidence"
        shutil.copytree(ROOT, copy)

        path = copy / "doc-gates.json"
        gates = json.loads(path.read_bytes())
        gates["source"]["docs_tree_sha256"] = "0" * 64
        path.write_bytes(encoded(gates))
        (copy / "evidence.json").write_bytes(encoded({"schema": 1, "files": inventory(copy)}))
        check("文档树摘要", copy, lambda: audit(copy), "定稿文档树或工作树回执不符")

        check("守卫脚本摘要", copy,
              lambda: guard_case(copy, lambda guards: guards.update(scriptSha256="0" * 64), upstream, locks),
              "本机 B1 守卫脚本、工作流或回执字节不符")

        def duplicate_case(guards):
            guards["cases"][-1] = guards["cases"][0]

        check("守卫用例重复", copy,
              lambda: guard_case(copy, duplicate_case, upstream, locks),
              "本机 B1 模拟守卫的用例集合或退出结果不符")
    require(snapshot(ROOT) == original, "自反例复跑改写了封存目录")
    print(encoded({"baseline_passed": True, "rejected_cases": 3, "read_only": True}).decode(), end="")


if __name__ == "__main__":
    main()
