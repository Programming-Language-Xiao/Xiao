"""重建 B0 closeout.json；审计脚本只读，不调用本生成器。"""

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4].resolve()
bootstrap = json.loads((ROOT / "bootstrap.json").read_text(encoding="utf-8"))
environment = json.loads((ROOT / "environment.json").read_text(encoding="utf-8"))
closeout = json.loads((ROOT / "closeout.json").read_text(encoding="utf-8"))
task_root = Path(bootstrap["task_root"]).resolve()
source_root = Path(bootstrap["source_root"]).resolve()
runtime_path = Path(environment["overrides"]["XIAO_RUNTIME_LIBRARY"]).resolve()
if not task_root.is_dir() or not source_root.is_dir() or not runtime_path.is_file():
    raise SystemExit("B0任务根、源码根或Runtime文件不存在")
if task_root == REPOSITORY or task_root.is_relative_to(REPOSITORY):
    raise SystemExit("B0任务根不能位于仓库内")
if not source_root.is_relative_to(task_root) or not runtime_path.is_relative_to(task_root):
    raise SystemExit("B0任务根 containment 不符")
closeout.update(
    {
        "actual_head": bootstrap["entry_commit"],
        "task_root": str(task_root),
        "source_root": str(source_root),
        "fresh_target": environment.get("fresh_target") is True,
        "fresh_xiao_home": environment.get("fresh_xiao_home") is True,
        "runtime_identity": {
            "path": str(runtime_path),
            "bytes": runtime_path.stat().st_size,
            "sha256": hashlib.sha256(runtime_path.read_bytes()).hexdigest(),
            "build_record": "b0-build-runtime.json",
        },
    }
)
(ROOT / "closeout.json").write_text(
    json.dumps(closeout, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
)
print(json.dumps({"written": "closeout.json", "runtime": closeout["runtime_identity"]}, ensure_ascii=False))
