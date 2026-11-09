"""修复前隔离复现：不修改真实源码或被测Runtime。"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
ENTRY = json.loads((HERE / 'entry.json').read_text(encoding='utf-8'))
SOURCE = Path(ENTRY['source'])
ROOT = SOURCE / 'docs/DevDocs/assets/20o-b2-b/entry-20261109-b3-cabi'


def write(path, document):
    path.write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


def snapshot(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob('*') if p.is_file()}


results = {}
for name in ['baseline', 'closeout_counts', 'pow_rule_distance', 'c_optimization', 'source_record_identity', 'git_objects_only']:
    with tempfile.TemporaryDirectory(prefix=name + '-', dir=HERE) as temporary:
        root = Path(temporary) / 'evidence'
        shutil.copytree(ROOT, root, ignore=shutil.ignore_patterns('__pycache__'))
        def mutate(path, function):
            document = json.loads((root / path).read_text(encoding='utf-8'))
            function(document)
            write(root / path, document)
        if name == 'closeout_counts':
            mutate('closeout.json', lambda d: d.update(runtime_tests=0, formal_abi_cases=1, external_c_checks_each=0))
        elif name == 'pow_rule_distance':
            def change(d):
                row = next(r for r in d['rows'] if r['difference'] == 'same' and r['new_reference_ulp'] is not None)
                row.update(frozen_rule='F-2:invented-unfrozen-rule', new_reference_ulp=999)
            mutate('baseline/pow-switch-manifest.json', change)
        elif name == 'c_optimization':
            def change(d):
                row = next(r for r in d['records'] if r['name'] == 'compile-o0')
                row['argv'][row['argv'].index('-O0')] = '-O3'
            mutate('abi/c-probe-result.json', change)
        elif name == 'source_record_identity':
            def change(d):
                d['source_before']['raw_tree_sha256'] = '0' * 64
                d['source_after']['raw_tree_sha256'] = '0' * 64
            mutate('abi/b2-vm.json', change)
        rows = [{'path': p.relative_to(root).as_posix(), 'bytes': p.stat().st_size, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
                for p in sorted(root.rglob('*')) if p.is_file() and p != root / 'evidence.json' and '__pycache__' not in p.parts]
        write(root / 'evidence.json', {'file_count': len(rows), 'files': rows})
        before = snapshot(root)
        source = HERE / 'objects.git' if name == 'git_objects_only' else SOURCE
        argv = [sys.executable, '-X', 'utf8', str(ROOT / 'audit.py'), '--root', str(root), '--source-root', str(source), '--core-only']
        run = subprocess.run(argv, capture_output=True, text=True, encoding='utf-8', env=dict(os.environ, PYTHONUTF8='1', PYTHONIOENCODING='utf-8'))
        results[name] = {'argv': argv, 'exit_code': run.returncode, 'output': (run.stdout + run.stderr).strip(), 'read_only': before == snapshot(root)}
        print(json.dumps({'case': name, **results[name]}, ensure_ascii=True), flush=True)
write(HERE / 'before-result.json', {'entry_commit': ENTRY['entry_commit'], 'cases': results,
      'scope': '修复前隔离目录真实审计输出；Git对象目录无参考工作树文件'})
