"""只读复核第二轮A5自审的旧工具身份、实际输出、门禁及完整文件清单。"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
ARCHIVE = ROOT.parent/'entry-20261106'


def read(name):
    return json.loads((ROOT/name).read_text(encoding='utf-8'))


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def inventory(create=False):
    files = {p.relative_to(ROOT).as_posix(): p for p in ROOT.rglob('*') if p.is_file() and p != ROOT/'evidence.json'}
    if create:
        rows = [{'path': name, 'bytes': p.stat().st_size, 'sha256': sha(p.read_bytes())} for name, p in sorted(files.items())]
        (ROOT/'evidence.json').write_text(json.dumps({'file_count': len(rows), 'files': rows}, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
        return len(rows)
    evidence = read('evidence.json')
    assert len(evidence['files']) == evidence['file_count'] == len(files)
    assert set(files) == {row['path'] for row in evidence['files']}
    for row in evidence['files']:
        path = (ROOT/row['path']).resolve()
        assert path.is_relative_to(ROOT.resolve())
        raw = path.read_bytes()
        assert len(raw) == row['bytes'] and sha(raw) == row['sha256'], row['path']
    return len(files)


def main():
    count = inventory()
    previous = subprocess.check_output(['cmd', '/c', 'git', 'show', '3dd492d16c6991c795b2fe91f0f4af9bda892e78:docs/DevDocs/assets/20o-b2-a5/entry-20261106/audit.py'], cwd=REPOSITORY)
    assert previous == (ROOT/'audit-before.py').read_bytes()
    current = (ARCHIVE/'audit.py').read_bytes()
    report = read('regression/results.json')
    assert report['passed'] and len(report['cases']) == 14
    assert report['before_auditor_sha256'] == sha(previous) and report['after_auditor_sha256'] == sha(current)
    assert report['input_manifest_sha256'] == sha((ROOT/'input-evidence.json').read_bytes())
    assert len({case['case'] for case in report['cases']}) == 14
    for case in report['cases']:
        assert case['passed']
        for phase in ['before', 'after']:
            for stream in ['stdout', 'stderr']:
                raw = (ROOT/'regression'/(case['case']+'-'+phase+'.'+stream+'.log')).read_bytes()
                assert sha(raw) == case[phase][stream+'_sha256']
            actual = (ROOT/'regression'/(case['case']+'-'+phase+'.stderr.log')).read_bytes().decode('utf-8').strip()
            assert actual == case[phase]['actual_error']
        expected = case['expected_error']
        assert case['after']['exit_code'] == (0 if expected is None else 1)
        assert case['after']['actual_error'] == (expected or '')
    legacy = read('legacy-regression/negative-cases.json')
    assert legacy['passed'] and len(legacy['cases']) == 9 and legacy['audit_sha256'] == sha(current)
    for case in legacy['cases']:
        assert case['passed'] and case['actual_error'] == (case['expected_error'] or '')
        for stream in ['stdout', 'stderr']:
            raw = (ROOT/'legacy-regression'/(case['case']+'.'+stream+'.log')).read_bytes()
            assert sha(raw) == case[stream+'_sha256']
        actual = (ROOT/'legacy-regression'/(case['case']+'.stderr.log')).read_bytes().decode('utf-8').strip()
        assert actual == case['actual_error']
    docs = read('docs-gate.json')
    assert docs['exit_code'] == 0 and not docs['product_changes_before'] and not docs['product_changes_after']
    assert sha((ROOT/docs['log']).read_bytes()) == docs['log_sha256']
    boundary = read('boundary.json')
    assert not subprocess.check_output(boundary['command'], cwd=REPOSITORY).strip()
    result = subprocess.run(['python', '-X', 'utf8', str(ARCHIVE/'audit.py')], cwd=REPOSITORY, capture_output=True)
    assert result.returncode == 0, result.stderr.decode('utf-8')
    print(json.dumps({'passed': True, 'files': count, 'new_cases': 14, 'legacy_cases': 9,
                      'product_commit': 'f21715d3', 'product_changes': []}, ensure_ascii=False))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--manifest', action='store_true')
    args = parser.parse_args()
    if args.manifest:
        print(json.dumps({'files': inventory(True)}))
    else:
        main()
