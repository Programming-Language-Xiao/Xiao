"""按最终 B0 目录实际字节重建 evidence.json。"""

import argparse
import hashlib
import json
from pathlib import Path


DEFAULT_ROOT = Path(__file__).resolve().parent


def main(root):
    rows = []
    for path in sorted(root.rglob("*")):
        if path.is_file() and path != root / "evidence.json" and "__pycache__" not in path.parts:
            rows.append({
                "path": path.relative_to(root).as_posix(),
                "bytes": path.stat().st_size,
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            })
    manifest = {
        "scope": "仅B0本目录最终文件；清单自身不计入自身摘要",
        "file_count": len(rows),
        "files": rows,
    }
    (root / "evidence.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    print(json.dumps({"written": "evidence.json", "files": len(rows)}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    args = parser.parse_args()
    main(args.root.resolve())
