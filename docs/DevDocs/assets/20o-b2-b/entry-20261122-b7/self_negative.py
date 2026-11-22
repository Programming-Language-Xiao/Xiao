"""在临时副本回放 B7 封存反例和状态组合。"""

from hashlib import sha256
from importlib.util import module_from_spec, spec_from_file_location
from io import BytesIO
from itertools import product
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory
from zipfile import ZIP_DEFLATED, ZipFile

sys.dont_write_bytecode = True
spec = spec_from_file_location("b7_audit", Path(__file__).with_name("audit.py"))
audit = module_from_spec(spec)
spec.loader.exec_module(audit)

FIELDS = (*audit.BASIC, "open_items_assigned")


def matrix():
    counts = {"checked": 0, "b7_passed": 0, "b2c_entry_ready": 0}
    for flags in product((False, True), repeat=len(FIELDS)):
        state = audit.derive(dict(zip(FIELDS, flags)))
        audit.require(state == {"b7_passed": False not in flags[:-1], "b2c_entry_ready": False not in flags},
                      "状态派生组合不符")
        counts["checked"] += 1
        for name in ("b7_passed", "b2c_entry_ready"):
            counts[name] += int(state[name])
    return counts


def files(root):
    return {path.relative_to(root).as_posix(): audit.digest(path.read_bytes())
            for path in root.rglob("*") if path.is_file()}


def regenerate(root):
    entries = files(root)
    entries.pop("evidence.json")
    (root / "evidence.json").write_bytes(audit.json_bytes(entries))


def repack(root, change=None):
    path = root / "records.zip"
    with ZipFile(BytesIO(path.read_bytes())) as original:
        index = json.loads(original.read("evidence.json"))
        objects = {name: original.read(name) for name in original.namelist() if name.startswith("objects/")}
    if change:
        name, edit = change
        raw = edit(objects["objects/" + index[name]["sha256"]])
        index[name] = audit.digest(raw)
        objects["objects/" + sha256(raw).hexdigest()] = raw
    output = BytesIO()
    with ZipFile(output, "w", compression=ZIP_DEFLATED) as replacement:
        for name in sorted({"objects/" + row["sha256"] for row in index.values()}):
            replacement.writestr(name, objects[name])
        replacement.writestr("evidence.json", audit.json_bytes(index))
    path.write_bytes(output.getvalue())
    state = json.loads((root / "closeout.json").read_bytes())
    state["archive"] = audit.digest(path.read_bytes()) | {
        "paths": len(index), "objects": len({row["sha256"] for row in index.values()}),
    }
    (root / "closeout.json").write_bytes(audit.json_bytes(state))
    regenerate(root)


def false_status(root):
    path = root / "closeout.json"
    state = json.loads(path.read_bytes())
    state["inherited"]["22d_open"] = False
    path.write_bytes(audit.json_bytes(state))
    regenerate(root)


def false_source(raw):
    row = json.loads(raw)
    row["source"]["commit"] = "0" * 40
    return audit.json_bytes(row)


def incomplete_coverage(raw):
    row = json.loads(raw)
    row.pop("b1-contract")
    return audit.json_bytes(row)


def invoke(root):
    before = files(root)
    result = subprocess.run([sys.executable, "-B", "-X", "utf8", str(audit.ROOT / "audit.py"), "--root", str(root)],
                            capture_output=True, encoding="utf-8", timeout=180)
    audit.require(before == files(root), "B7 审计改写了证据")
    return result


def main():
    negative = {
        "manifest-changed": (lambda root: (root / "evidence.json").write_bytes(b"{}\n"),
                             "B7 封存文件清单或字节不符"),
        "missing-attributes": (lambda root: (root / ".gitattributes").unlink(), "B7 封存文件清单或字节不符"),
        "forged-b6-state": (false_status, "封存状态与实际证据派生结果不符"),
        "forged-source": (lambda root: repack(root, ("windows/environment.json", false_source)),
                          "平台或任务目录身份不符：windows"),
        "subset-as-full": (lambda root: repack(root, ("windows/results.json", incomplete_coverage)),
                           "回归入口集合不符：windows"),
        "repacked-with-manifest": (repack, "B7 清单与固定 Git 对象不符"),
    }
    baseline = invoke(audit.ROOT)
    audit.require(baseline.returncode == 0 and not baseline.stderr and json.loads(baseline.stdout)["b7_passed"] is True,
                  f"B7 审计基线失败：{baseline.stderr}")
    results = {"baseline": {"exit": 0, "read_only": True}}
    with TemporaryDirectory(prefix="xiao-b7-audit-") as temporary:
        for name, (mutate, expected) in negative.items():
            root = Path(temporary) / name
            shutil.copytree(audit.ROOT, root)
            mutate(root)
            result = invoke(root)
            observed = result.stderr.strip()
            audit.require(result.returncode == 1 and not result.stdout and observed == expected,
                          f"{name} 未按预期拒绝，应为 {expected}，实际 {observed}")
            results[name] = {"exit": result.returncode, "reason": observed, "read_only": True}
    print(audit.json_bytes({"passed": True, "matrix": matrix(), "cases": results}).decode(), end="")


if __name__ == "__main__":
    main()
