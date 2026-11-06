"""临时复制完整归档，逐案验证真实拒绝原因；不修改原始执行日志。"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')


def read(root, name):
    return json.loads((root/name).read_text(encoding='utf-8'))


def log_mutation(root, name, transform):
    record = read(root, name+'.json')
    log = root/record['log']
    text = log.read_text(encoding='utf-8')
    log.write_text(transform(text), encoding='utf-8', newline='\n')
    record['sha256'] = hashlib.sha256(log.read_bytes()).hexdigest()
    write(root/(name+'.json'), record)


def missing_vm(root):
    log_mutation(root, 'close-b1-vm-exact', lambda text: '\n'.join(
        line for line in text.splitlines() if not line.startswith('B2-A5-LANGUAGE case=a5_bindings backend=vm'))+'\n')


def duplicate_native(root):
    def transform(text):
        selected = next(line for line in text.splitlines() if line.startswith('B2-A5-LANGUAGE case=a5_bindings backend=native optimization=0'))
        return text.replace('test b1_capabilities_match_native_golden_outputs ... ok', selected+'\n'+'test b1_capabilities_match_native_golden_outputs ... ok')
    log_mutation(root, 'close-b1-native-exact', transform)


def ignored_native(root):
    log_mutation(root, 'close-b1-native-exact', lambda text: text.replace('1 passed; 0 failed; 0 ignored;', '1 passed; 0 failed; 1 ignored;'))


def aggregate_id(root):
    closeout = read(root, 'closeout.json')
    record = read(root, closeout['ci_records']['vm'])
    record['job_id'] = read(root, closeout['ci_runs']['maintenance'])['id']
    write(root/closeout['ci_records']['vm'], record)


def omitted_source(root):
    closeout = read(root, 'closeout.json')
    manifest = read(root, closeout['product_manifest'])
    manifest['files'] = [row for row in manifest['files'] if row['path'] != 'stdlib/math.xiao']
    manifest['raw_tree_sha256'] = hashlib.sha256(json.dumps(manifest['files'], ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    pairs = [[row['path'], row['utf8_lf_or_binary_sha256']] for row in manifest['files']]
    manifest['utf8_lf_tree_sha256'] = hashlib.sha256(json.dumps(pairs, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
    write(root/closeout['product_manifest'], manifest)


def mapped_job_id(root):
    closeout = read(root, 'closeout.json')
    record = read(root, closeout['ci_records']['vm'])
    assert record['execution_retained']
    jobs = read(root, closeout['ci_jobs'])['jobs']
    mapped = next(job for job in jobs if job['id'] == record['aggregate_job_id'])
    record['job_id'] = mapped['id']
    record['run_attempt'] = mapped['run_attempt']
    record['execution_retained'] = False
    write(root/closeout['ci_records']['vm'], record)


def omitted_check(root):
    checks = read(root, 'checks.json')
    checks['effective'].remove('close-clippy')
    write(root/'checks.json', checks)
    closeout = read(root, 'closeout.json')
    closeout['effective_checks'].remove('close-clippy')
    write(root/'closeout.json', closeout)


def omitted_platform(root):
    closeout = read(root, 'closeout.json')
    closeout['ci_records'].pop('macos')
    write(root/'closeout.json', closeout)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--temporary-parent', type=Path, required=True)
    args = parser.parse_args()
    temporary = Path(tempfile.mkdtemp(prefix='a5-audit-', dir=args.temporary_parent.resolve()))
    cases = [
        ('positive', None, None),
        ('missing-vm-marker', missing_vm, 'A5标记缺失或重复：B2-A5-LANGUAGE case=a5_bindings backend=vm'),
        ('duplicate-native-marker', duplicate_native, 'A5标记缺失或重复：B2-A5-LANGUAGE case=a5_bindings backend=native optimization=0 artifact='),
        ('ignored-native', ignored_native, '精确入口计数/忽略状态不符：b1_capabilities_match_native_golden_outputs'),
        ('aggregate-job-id', aggregate_id, '实际日志作业被聚合作业ID替代或缺失'),
        ('mapped-job-id', mapped_job_id, '实际日志作业不是该次执行的原始作业'),
        ('omitted-product-source', omitted_source, '完整源码清单缺项或重复'),
        ('omitted-clippy', omitted_check, '有效检查清单缺项或重复'),
        ('omitted-macos', omitted_platform, '三平台/VM实际作业清单缺项'),
    ]
    reports = []
    observed = []
    for name, mutate, expected in cases:
        directory = temporary/name
        shutil.copytree(ROOT, directory)
        if mutate:
            mutate(directory)
            updated = subprocess.run([sys.executable, '-X', 'utf8', str(ROOT/'audit.py'), '--archive-root', str(directory), '--manifest'], capture_output=True)
            assert updated.returncode == 0, updated.stderr.decode('utf-8')
        command = [sys.executable, '-X', 'utf8', str(ROOT/'audit.py'), '--archive-root', str(directory), '--repository-root', str(REPOSITORY)]
        result = subprocess.run(command, capture_output=True)
        actual = result.stderr.decode('utf-8').strip()
        passed = result.returncode == (0 if expected is None else 1) and (actual == expected if expected else not actual)
        observed.append((name, result.stdout, result.stderr))
        reports.append({'case': name, 'argv': command, 'exit_code': result.returncode,
                        'expected_error': expected, 'actual_error': actual, 'passed': passed,
                        'stdout_sha256': hashlib.sha256(result.stdout).hexdigest(),
                        'stderr_sha256': hashlib.sha256(result.stderr).hexdigest()})
    output = ROOT/'self-review'
    output.mkdir(exist_ok=True)
    for name, stdout, stderr in observed:
        (output/(name+'.stdout.log')).write_bytes(stdout)
        (output/(name+'.stderr.log')).write_bytes(stderr)
    report = {'scope': '只验证完整归档审计的正反例；不是新的产品或CI运行',
              'temporary_root': str(temporary), 'original_manifest_sha256': hashlib.sha256((ROOT/'evidence.json').read_bytes()).hexdigest(),
              'audit_sha256': hashlib.sha256((ROOT/'audit.py').read_bytes()).hexdigest(),
              'passed': all(row['passed'] for row in reports), 'cases': reports}
    write(output/'negative-cases.json', report)
    print(json.dumps({'passed': report['passed'], 'cases': len(reports), 'temporary_root': str(temporary)}, ensure_ascii=False))
    if not report['passed']:
        raise SystemExit('自审反例没有按实际预期原因拒绝')


if __name__ == '__main__':
    main()
