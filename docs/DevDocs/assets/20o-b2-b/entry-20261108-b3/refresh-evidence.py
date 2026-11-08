"""显式重建最终字节清单；审计器绝不调用此脚本。"""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def rows(root):
    return [{'path': path.relative_to(root).as_posix(), 'bytes': path.stat().st_size,
             'sha256': hashlib.sha256(path.read_bytes()).hexdigest()} for path in sorted(root.rglob('*'))
            if path.is_file() and path != root / 'evidence.json' and '__pycache__' not in path.parts]


if __name__ == '__main__':
    files = rows(ROOT)
    (ROOT / 'evidence.json').write_text(json.dumps({'file_count': len(files), 'files': files}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps({'files': len(files), 'attributes_included': any(row['path'] == '.gitattributes' for row in files)}))
