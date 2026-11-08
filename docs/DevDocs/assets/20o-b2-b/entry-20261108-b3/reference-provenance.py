"""登记离线参考工具来源；下载归档只放独立任务根，不加入产品依赖。"""
import hashlib
import importlib.metadata
import json
from pathlib import Path
import sys
import urllib.request
import zipfile

import gmpy2

ROOT = Path(__file__).resolve().parent
TASK = Path(json.loads((ROOT / 'bootstrap.json').read_text(encoding='utf-8'))['task_root'])
ARCHIVES = TASK / 'reference-tools'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def fetch(url):
    with urllib.request.urlopen(url, timeout=60) as response:
        return response.read()


def main():
    assert (gmpy2.version(), gmpy2.mpfr_version(), gmpy2.mp_version()) == ('2.3.2', 'MPFR 4.2.2', 'GMP 6.3.0')
    ARCHIVES.mkdir(exist_ok=True)
    metadata_raw = fetch('https://pypi.org/pypi/gmpy2/2.3.2/json')
    (ARCHIVES / 'pypi-gmpy2-2.3.2.json').write_bytes(metadata_raw)
    metadata = json.loads(metadata_raw)
    selected = [row for row in metadata['urls'] if row['filename'].endswith('.tar.gz') or row['filename'].endswith('cp312-cp312-win_amd64.whl')]
    records = []
    for row in selected:
        raw = fetch(row['url'])
        assert sha(raw) == row['digests']['sha256']
        (ARCHIVES / row['filename']).write_bytes(raw)
        records.append({'filename': row['filename'], 'url': row['url'], 'sha256': sha(raw), 'bytes': len(raw), 'pypi_digest_verified': True})
    dist = importlib.metadata.distribution('gmpy2')
    installed = []
    wheel = next(row for row in records if row['filename'].endswith('.whl'))
    with zipfile.ZipFile(ARCHIVES / wheel['filename']) as archive:
        for filename in archive.namelist():
            if filename.endswith(('.pyd', '.dll')):
                binary = Path(dist.locate_file(filename))
                raw = binary.read_bytes()
                assert raw == archive.read(filename), filename
                installed.append({'relative_path': filename, 'sha256': sha(raw), 'bytes': len(raw), 'matches_published_wheel': True})
    assert installed
    result = {'python': sys.version, 'platform': sys.platform, 'gmpy2': gmpy2.version(), 'mpfr': gmpy2.mpfr_version(), 'gmp': gmpy2.mp_version(),
              'build_method': 'PyPI 发布者预构建 cp312 Windows x64 wheel；未自行构建 MPFR/GMP；实际加载版本由模块查询并断言。',
              'source_archive': 'gmpy2 2.3.2 sdist；MPFR/GMP 为 wheel 捆绑库，未声称独立重建其二进制。',
              'metadata_url': 'https://pypi.org/pypi/gmpy2/2.3.2/json', 'metadata_sha256': sha(metadata_raw),
              'archives': records, 'loaded_binaries': installed, 'archive_directory': str(ARCHIVES),
              'offline_only': True, 'product_dependency_added': False}
    (ROOT / 'b1/tool-provenance.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(result, ensure_ascii=True))


if __name__ == '__main__':
    main()
