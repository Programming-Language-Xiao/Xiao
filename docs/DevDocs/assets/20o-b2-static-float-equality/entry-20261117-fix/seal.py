"""在最后文档门禁后按真实字节生成一次证据清单。"""
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


manifest_path = ROOT / "evidence-final-r4.json"
if manifest_path.exists():
    raise ValueError("最终证据清单禁止覆盖")
docs = json.loads((ROOT / "docs-gate-final-r13.json").read_text(encoding="utf-8"))
if docs["exit_code"] != 0 or not docs["source_unchanged"]:
    raise ValueError("最终DevDocs/产品源码门禁未通过")
plan_name = "docs/DevDocs/20o-b2-static-float-equality-fix-plan.md"
plan = (REPOSITORY / plan_name).read_bytes()
readme_name = (
    "docs/DevDocs/assets/20o-b2-static-float-equality/"
    "entry-20261117-fix/README.md"
)
if docs["inputs"].get(plan_name, {}).get("sha256") != sha(plan):
    raise ValueError("最终门禁后修订规划字节发生变化")
if docs["inputs"].get(readme_name, {}).get("sha256") != sha((ROOT / "README.md").read_bytes()):
    raise ValueError("最终门禁后证据README字节发生变化")
for path in [ROOT / ".gitattributes", *ROOT.glob("*.py"), *ROOT.glob("*.rs")]:
    name = path.relative_to(REPOSITORY).as_posix()
    if name not in docs["inputs"] or docs["inputs"][name]["sha256"] != sha(path.read_bytes()):
        raise ValueError("最终门禁未覆盖本批证据文件：" + name)
compressed = gzip.compress(plan, mtime=0)
digest = sha(compressed)
object_path = ROOT / "objects/sha256" / digest[:2] / (digest + ".bin")
object_path.parent.mkdir(parents=True, exist_ok=True)
if object_path.exists():
    if object_path.read_bytes() != compressed:
        raise ValueError("规划对象摘要冲突")
else:
    with object_path.open("xb") as output:
        output.write(compressed)
files = {
    path.relative_to(ROOT).as_posix(): identity(path.read_bytes())
    for path in sorted(ROOT.rglob("*")) if path.is_file()
}
manifest = {
    "scope": "按位静态浮点相等/不等修复的Windows本机验收；不是跨平台发布验收",
    "generated_utc": datetime.now(timezone.utc).isoformat(),
    "product_acceptance_passed": True, "generator_version": 22,
    "runtime_abi": 65555, "stdlib_interface_version": 1,
    "plan": identity(plan),
    "entry_commit": "aead9c5501177f17976bf8a54f324ac6091bbff7",
    "product_commit": "cfab977105273dcfdfc1831332b3a804ccc217d9",
    "plan_object": {
        "path": object_path.relative_to(ROOT).as_posix(),
        **identity(compressed), "encoding": "gzip",
        "raw_bytes": len(plan), "raw_sha256": sha(plan),
    },
    "files": files, "file_count": len(files),
    "contains_gitattributes": ".gitattributes" in files,
    "multi_platform_or_ci_passed": False, "repair_applied": True,
}
with manifest_path.open("xb") as output:
    output.write((json.dumps(manifest, ensure_ascii=False, indent=2) + chr(10)).encode("utf-8"))
print(json.dumps({"file_count": len(files), "plan_sha256": sha(plan),
                  "contains_gitattributes": manifest["contains_gitattributes"]},
                 ensure_ascii=False))
