"""B3自审复现：只篡改隔离复制目录；保留修复前实际接受结果。"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
SOURCE = Path('D:/project/Xiao/Xiao')
ROOT = SOURCE / 'docs/DevDocs/assets/20o-b2-b/entry-20261108-b3'


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


def refresh(root):
    rows = [{'path': p.relative_to(root).as_posix(), 'bytes': p.stat().st_size, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
            for p in sorted(root.rglob('*')) if p.is_file() and p != root / 'evidence.json' and '__pycache__' not in p.parts]
    save(root / 'evidence.json', {'file_count': len(rows), 'files': rows})


def file_map(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob('*') if p.is_file()}


def main():
    commit = subprocess.check_output(['cmd', '/c', 'git', 'rev-parse', 'HEAD'], cwd=SOURCE).decode().strip()
    assert commit.startswith('8213e3e4')
    result = {'source_commit': commit, 'scope': '修复前审计的真实接受结果，非修复后通过证据', 'cases': {}}
    env = dict(os.environ, PYTHONUTF8='1', PYTHONIOENCODING='utf-8')
    for name in ['baseline', 'reference_generator_rebound', 'baseline_record_rewritten', 'pow_summary_rewritten', 'exact_target_rewritten']:
        with tempfile.TemporaryDirectory(prefix=name + '-', dir=HERE) as temp:
            target = Path(temp) / 'evidence'
            shutil.copytree(ROOT, target, ignore=shutil.ignore_patterns('__pycache__'))
            if name == 'reference_generator_rebound':
                generator = target / 'b1/generate-reference.py'
                generator.write_bytes(generator.read_bytes() + b'\n# self-review: changed after the measured snapshot\n')
                manifest = json.loads((target / 'b1/reference-manifest.json').read_text(encoding='utf-8'))
                manifest['generator_sha256'] = hashlib.sha256(generator.read_bytes()).hexdigest()
                save(target / 'b1/reference-manifest.json', manifest)
                log = target / 'b1/reference-replay.log'
                lines = log.read_text(encoding='utf-8').splitlines()
                replay = json.loads(lines[-1])
                replay['files']['reference-manifest.json'] = hashlib.sha256((target / 'b1/reference-manifest.json').read_bytes()).hexdigest()
                lines[-1] = json.dumps(replay, ensure_ascii=True)
                log.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
                record = json.loads((target / 'b1/reference-replay.json').read_text(encoding='utf-8'))
                record['sha256'] = hashlib.sha256(log.read_bytes()).hexdigest()
                save(target / 'b1/reference-replay.json', record)
            elif name == 'baseline_record_rewritten':
                path = target / 'b0/build-driver.json'
                record = json.loads(path.read_text(encoding='utf-8'))
                record['duration_seconds'] += 1
                save(path, record)
                refresh(target / 'b0')
            elif name == 'pow_summary_rewritten':
                path = target / 'b1/pow-diff.json'
                document = json.loads(path.read_text(encoding='utf-8'))
                document['summary'] = {'same': 0, 'value_or_error_boundary': 0, 'value_bits': 0}
                save(path, document)
            elif name == 'exact_target_rewritten':
                path = target / 'b1/b2-vm.json'
                document = json.loads(path.read_text(encoding='utf-8'))
                i = document['argv'].index('lfloat_divide_has_vm_golden_outputs')
                document['argv'][i] = 'unrelated_test'
                save(path, document)
            refresh(target)
            before = file_map(target)
            argv = [sys.executable, '-X', 'utf8', str(ROOT / 'audit.py'), '--root', str(target), '--source-root', str(SOURCE), '--core-only']
            run = subprocess.run(argv, capture_output=True, text=True, encoding='utf-8', env=env)
            row = {'argv': argv, 'exit_code': run.returncode, 'output': (run.stdout + run.stderr).strip(), 'read_only': before == file_map(target)}
            result['cases'][name] = row
            print(json.dumps({'case': name, **row}, ensure_ascii=True), flush=True)
    with tempfile.TemporaryDirectory(prefix='reference-self-test-', dir=HERE) as temp:
        target = Path(temp)
        for name in ['test-reference.py', 'generate-reference.py']:
            shutil.copy2(ROOT / 'b1' / name, target / name)
        before = file_map(target)
        argv = [sys.executable, '-X', 'utf8', str(target / 'test-reference.py')]
        run = subprocess.run(argv, capture_output=True, text=True, encoding='utf-8', env=env)
        after = file_map(target)
        result['cases']['reference_self_test_read_only'] = {'argv': argv, 'exit_code': run.returncode,
            'output': (run.stdout + run.stderr).strip(), 'new_files': sorted(set(after) - set(before)), 'read_only': before == after}
    save(HERE / 'before-fix.json', result)
    print(json.dumps({'output': str(HERE / 'before-fix.json')}, ensure_ascii=True))


if __name__ == '__main__':
    main()
