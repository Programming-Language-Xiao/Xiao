"""按定稿实际字节一次生成清单；不改写采集回执。"""
from datetime import datetime, timezone
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def identity(raw):
    return {"bytes": len(raw), "sha256": sha(raw)}


manifest_path = ROOT / "evidence.json"
if manifest_path.exists():
    raise ValueError("最终证据清单禁止覆盖")
docs = json.loads((ROOT / "docs-gate-final-verified.json").read_text(encoding="utf-8"))
if docs["exit_code"] != 0 or not docs["inputs_unchanged"]:
    raise ValueError("最终文档门禁未通过")
plan_name = "docs/DevDocs/20o-b2-static-float-equality-fix-plan.md"
plan = (REPOSITORY / plan_name).read_bytes()
if docs["inputs"][plan_name] != identity(plan):
    raise ValueError("规划在最终文档门禁后发生变化")
for path in [ROOT / "README.md", *ROOT.glob("*.py"), *ROOT.glob("*.rs")]:
    name = path.relative_to(REPOSITORY).as_posix()
    if docs["inputs"][name] != identity(path.read_bytes()):
        raise ValueError("说明或采集工具在最终文档门禁后发生变化：" + name)
packed = gzip.compress(plan, mtime=0)
digest = sha(packed)
object_path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
object_path.parent.mkdir(parents=True, exist_ok=True)
if object_path.exists():
    if object_path.read_bytes() != packed:
        raise ValueError("规划归档对象摘要冲突")
else:
    with object_path.open("xb") as output:
        output.write(packed)
files = {path.relative_to(ROOT).as_posix(): identity(path.read_bytes())
         for path in sorted(ROOT.rglob("*")) if path.is_file()}
manifest = {
    "scope": "未修复基线与待裁定规划，不是产品修复验收",
    "generated_utc": datetime.now(timezone.utc).isoformat(),
    "plan": identity(plan),
    "plan_object": {
        "path": object_path.relative_to(ROOT).as_posix(), **identity(packed),
        "encoding": "gzip", "raw_bytes": len(plan), "raw_sha256": sha(plan),
    },
    "files": files, "file_count": len(files),
    "repair_applied": False, "product_acceptance_passed": False,
}
with manifest_path.open("xb") as output:
    output.write((json.dumps(manifest, ensure_ascii=False, indent=2) + chr(10)).encode("utf-8"))
print(json.dumps({"files": len(files), "plan_sha256": sha(plan)}, ensure_ascii=False))
