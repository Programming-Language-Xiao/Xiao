"""用临时副本复跑 B3-2A 只读审计和守卫回执核验。"""

import json
from pathlib import Path
import shutil
import sys
from tempfile import TemporaryDirectory
from zipfile import ZipFile, ZipInfo

sys.dont_write_bytecode = True
from audit import NEGATIVE_CASES, NEGATIVE_RECEIPT, ROOT, audit, encoded, fixed_inputs, identity, inventory
from audit import require, verify_windows


def snapshot(root):
    files = inventory(root)
    manifest = root / "evidence.json"
    if manifest.is_file():
        files[manifest.name] = identity(manifest.read_bytes())
    return identity(encoded(files))["sha256"]


def check(case, root, operation):
    before = snapshot(root)
    reason = None
    try:
        operation()
    except RuntimeError as error:
        reason = str(error)
    expected = NEGATIVE_CASES[case]
    require(reason == expected, f"{case} 审计结果为 {reason!r}，预期 {expected!r}")
    after = snapshot(root)
    require(after == before, f"{case} 改写了待审文件")
    return {"case": case, "accepted": reason is None, "reason": reason,
            "before_sha256": before, "after_sha256": after}


def guard_case(root, parent, case, edit, upstream, locks):
    target = parent / case
    target.mkdir()
    archive = "windows-baseline.zip"
    with ZipFile(root / archive) as original, ZipFile(target / archive, "x") as changed:
        report = json.loads(original.read("result.json"))
        guards = json.loads(original.read("b1-guards-report.json"))
        edit(guards)
        raw = encoded(guards)
        report["b1_guard_report"].update(identity(raw))
        replacements = {"result.json": encoded(report), "b1-guards-report.json": raw}
        for name in original.namelist():
            # 固定临时 ZIP 的时间、宿主标记与存储方式，便于跨 Python 复核字节。
            entry = ZipInfo(name)
            entry.create_system = 0
            data = replacements[name] if name in replacements else original.read(name)
            changed.writestr(entry, data)
    return check(case, target, lambda: verify_windows(target, upstream, locks))


def run_cases(root):
    rows = [check("unchanged", root, lambda: audit(root, prepare=True))]
    upstream, locks, _, _ = fixed_inputs()
    source = json.loads((root / "doc-gates.json").read_bytes())["source"]
    with TemporaryDirectory(prefix="xiao-b3-2a-negative-") as directory:
        target = Path(directory)
        copy = target / "document-tree-sha256"
        shutil.copytree(root, copy)

        path = copy / "doc-gates.json"
        gates = json.loads(path.read_bytes())
        gates["source"]["docs_tree_sha256"] = "0" * 64
        path.write_bytes(encoded(gates))
        (copy / "evidence.json").write_bytes(encoded({"schema": 1, "files": inventory(copy)}))
        rows.append(check("document-tree-sha256", copy, lambda: audit(copy, prepare=True)))

        rows.append(guard_case(root, target, "guard-script-sha256",
                               lambda guards: guards.update(scriptSha256="0" * 64), upstream, locks))

        def duplicate_case(guards):
            guards["cases"][-1] = guards["cases"][0]

        rows.append(guard_case(root, target, "duplicate-guard-case", duplicate_case, upstream, locks))
    return {"schema": 1, "source": source, "cases": rows}


def main():
    original = snapshot(ROOT)
    audit(ROOT)
    with TemporaryDirectory(prefix="xiao-b3-2a-replay-") as directory:
        target = Path(directory) / "evidence"
        shutil.copytree(ROOT, target, ignore=shutil.ignore_patterns(NEGATIVE_RECEIPT, "evidence.json"))
        (target / "evidence.json").write_bytes(encoded({"schema": 1, "files": inventory(target)}))
        observed = run_cases(target)
    require(encoded(observed) == (ROOT / NEGATIVE_RECEIPT).read_bytes(), "复跑结果与自反例封存回执不符")
    require(snapshot(ROOT) == original, "自反例复跑改写了封存目录")
    print(encoded({"baseline_passed": observed["cases"][0]["accepted"],
                   "rejected_cases": sum(not row["accepted"] for row in observed["cases"]),
                   "read_only": True}).decode(), end="")


if __name__ == "__main__":
    main()
