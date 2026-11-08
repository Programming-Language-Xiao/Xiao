"""在新临时目录完整重生成F-9参考，并逐字节比较；不覆盖已签入参考。"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent


def main():
    with tempfile.TemporaryDirectory(prefix='b3-reference-replay-') as temporary:
        target = Path(temporary)
        argv = [sys.executable, '-X', 'utf8', str(ROOT / 'generate-reference.py'), '--output', str(target)]
        run = subprocess.run(argv, capture_output=True, text=True, encoding='utf-8', check=True)
        print(run.stdout.rstrip())
        matched = {}
        for name in ['reference-inputs.json', 'reference-vectors.tsv', 'reference-manifest.json']:
            actual = (target / name).read_bytes()
            assert actual == (ROOT / name).read_bytes(), '重生成不同：' + name
            matched[name] = hashlib.sha256(actual).hexdigest()
        print(json.dumps({'passed': True, 'argv': argv, 'temporary_directory': temporary,
                          'files': matched, 'originals_overwritten': False}, ensure_ascii=True))


if __name__ == '__main__':
    main()
