"""隔离复制归档自反例：核对确切原因和只读性，不接受无关异常充当拒绝。"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]
REASONS = {'baseline': None, 'tampered_manifest': 'B3证据清单或摘要不符', 'missing_attributes': 'B3证据清单或摘要不符',
           'semantic_tamper_after_manifest_rebuild': 'B3版本或公开边界越界',
           'runtime_tamper_after_manifest_rebuild': 'B3原生Runtime身份与构建记录不符',
           'reference_rebound_after_manifest_rebuild': 'B3参考文件偏离被测源码清单',
           'baseline_record_after_manifest_rebuild': 'B3-0证据偏离已提交基线',
           'pow_summary_after_manifest_rebuild': 'B3幂行为差表汇总偏离明细',
           'exact_target_after_manifest_rebuild': 'B3精确入口命令或测试目标不符'}


def rows(root):
    return [{'path': p.relative_to(root).as_posix(), 'bytes': p.stat().st_size, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
            for p in sorted(root.rglob('*')) if p.is_file() and p != root / 'evidence.json' and '__pycache__' not in p.parts]


def snapshot(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob('*') if p.is_file()}


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


def main():
    task = Path(json.loads((ROOT / 'bootstrap.json').read_text(encoding='utf-8'))['task_root'])
    parent = task / 'audit-self-negative'
    parent.mkdir(exist_ok=True)
    results = {}
    for name, reason in REASONS.items():
        with tempfile.TemporaryDirectory(prefix=name + '-', dir=parent) as temporary:
            target = Path(temporary) / 'evidence'
            shutil.copytree(ROOT, target, ignore=shutil.ignore_patterns('__pycache__'))
            if name == 'tampered_manifest':
                manifest = json.loads((target / 'evidence.json').read_text(encoding='utf-8'))
                manifest['file_count'] += 1
                save(target / 'evidence.json', manifest)
            elif name == 'missing_attributes':
                (target / '.gitattributes').unlink()
            elif name == 'semantic_tamper_after_manifest_rebuild':
                closeout = json.loads((target / 'b1/closeout.json').read_text(encoding='utf-8'))
                closeout['abi'] = '1.19'
                save(target / 'b1/closeout.json', closeout)
            elif name == 'runtime_tamper_after_manifest_rebuild':
                surface = json.loads((target / 'b1/public-surface.json').read_text(encoding='utf-8'))
                surface['runtime']['sha256'] = '0' * 64
                save(target / 'b1/public-surface.json', surface)
            elif name == 'reference_rebound_after_manifest_rebuild':
                generator = target / 'b1/generate-reference.py'
                generator.write_bytes(generator.read_bytes() + b'\n# changed after the measured snapshot\n')
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
            elif name == 'baseline_record_after_manifest_rebuild':
                path = target / 'b0/build-driver.json'
                record = json.loads(path.read_text(encoding='utf-8'))
                record['duration_seconds'] += 1
                save(path, record)
                files = rows(target / 'b0')
                save(target / 'b0/evidence.json', {'file_count': len(files), 'files': files})
            elif name == 'pow_summary_after_manifest_rebuild':
                path = target / 'b1/pow-diff.json'
                document = json.loads(path.read_text(encoding='utf-8'))
                document['summary'] = {'same': 0, 'value_or_error_boundary': 0, 'value_bits': 0}
                save(path, document)
            elif name == 'exact_target_after_manifest_rebuild':
                path = target / 'b1/b2-vm.json'
                document = json.loads(path.read_text(encoding='utf-8'))
                index = document['argv'].index('lfloat_divide_has_vm_golden_outputs')
                document['argv'][index] = 'unrelated_test'
                save(path, document)
            if name.endswith('after_manifest_rebuild'):
                files = rows(target)
                save(target / 'evidence.json', {'file_count': len(files), 'files': files})
            before = snapshot(target)
            argv = [sys.executable, '-X', 'utf8', str(ROOT / 'audit.py'), '--root', str(target), '--source-root', str(SOURCE), '--core-only']
            env = dict(os.environ, PYTHONUTF8='1', PYTHONIOENCODING='utf-8')
            run = subprocess.run(argv, capture_output=True, text=True, encoding='utf-8', env=env)
            output = (run.stdout + run.stderr).strip()
            read_only = before == snapshot(target)
            passed = read_only and run.returncode == (1 if reason else 0)
            if reason:
                passed = passed and output == reason
            else:
                passed = passed and json.loads(output)['passed'] is True
            results[name] = {'argv': argv, 'exit_code': run.returncode, 'expected_reason': reason,
                             'output': output, 'read_only': read_only, 'passed': passed}
            print(json.dumps({'case': name, **results[name]}, ensure_ascii=True), flush=True)
    result = {'passed': all(row['passed'] for row in results.values()), 'cases': results,
              'scope': '复制归档仅在隔离临时目录内篡改；清单重建反例显式标注；默认审计另行执行'}
    save(ROOT / 'self-negative-result.json', result)
    if not result['passed']:
        raise SystemExit('B3自反例失败；不能收口')


if __name__ == '__main__':
    main()
