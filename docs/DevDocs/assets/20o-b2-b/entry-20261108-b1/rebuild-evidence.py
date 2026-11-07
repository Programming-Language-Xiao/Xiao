"""按最终目录实际字节生成 evidence.json；自身清单条目不包含 evidence.json。"""

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parent
rows = []
for path in sorted(ROOT.rglob("*")):
    if path.is_file() and path.name != "evidence.json" and "__pycache__" not in path.parts:
        rows.append(
            {
                "path": path.relative_to(ROOT).as_posix(),
                "bytes": path.stat().st_size,
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
        )
manifest = {
    "scope": "仅B1本目录最终文件；清单自身不计入自身摘要",
    "file_count": len(rows),
    "files": rows,
}
(ROOT / "evidence.json").write_text(
    json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
)
print(json.dumps({"written": "evidence.json", "files": len(rows)}, ensure_ascii=False))
