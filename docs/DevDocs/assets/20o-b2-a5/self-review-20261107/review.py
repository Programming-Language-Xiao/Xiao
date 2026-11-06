"""复制A5归档，对修复前后审计器执行相同反例，保存实际退出码和原始输出。"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent
ARCHIVE = ROOT.parent/'entry-20261106'
REPOSITORY = ROOT.parents[4]


def read(root, name):
    return json.loads((root/name).read_text(encoding='utf-8'))


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')


def metadata(root, name, change):
    value = read(root, name)
    change(value)
    write(root/name, value)


def log_change(root, name, change):
    record = read(root, name+'.json')
    path = root/record['log']
    path.write_text(change(path.read_text(encoding='utf-8')), encoding='utf-8', newline='\n')
    record['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
    write(root/(name+'.json'), record)


def missing_counts(root):
    metadata(root, 'checks.json', lambda value: value.update(test_counts={}))
    log_change(root, 'close-language-tests', lambda text: text.replace('12 passed; 0 failed; 1 ignored;', '0 passed; 0 failed; 13 ignored;'))


def altered_counts(root):
    metadata(root, 'checks.json', lambda value: value['test_counts'].update({'close-language-tests': 11}))
    metadata(root, 'closeout.json', lambda value: value['exact_test_counts'].update({'close-language-tests': 11}))
    log_change(root, 'close-language-tests', lambda text: text.replace('12 passed; 0 failed; 1 ignored;', '11 passed; 0 failed; 2 ignored;'))


def missing_exact(root):
    metadata(root, 'checks.json', lambda value: value.update(exact_targets={}, marker_groups={}))
    for name in ['close-b1-vm-exact', 'close-b1-native-exact', 'close-a1-vm-exact', 'close-a1-native-exact']:
        metadata(root, name+'.json', lambda value: value.update(argv=[arg for arg in value['argv'] if arg != '--exact']))


def missing_groups(root):
    metadata(root, 'checks.json', lambda value: value.update(marker_groups={}))
    log_change(root, 'close-b1-vm-exact', lambda text: '\n'.join(line for line in text.splitlines()
        if not line.startswith('B2-A5-LANGUAGE case=a5_bindings backend=vm'))+'\n')


def duplicate_summary(root):
    def duplicate(text):
        summary = next(line for line in text.splitlines() if line.startswith('test result:'))
        return text+'\n'+summary+'\n'
    log_change(root, 'close-b1-native-exact', duplicate)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--temporary-parent', type=Path, required=True)
    parser.add_argument('--output-directory', type=Path, required=True)
    args = parser.parse_args()
    output = args.output_directory.resolve()
    output.mkdir(exist_ok=False)
    temporary = Path(tempfile.mkdtemp(prefix='strict-audit-', dir=args.temporary_parent.resolve()))
    cases = [
        ('positive', None, None),
        ('missing-count-contract', missing_counts, '测试计数契约缺项或漂移'),
        ('altered-count-contract', altered_counts, '测试计数契约缺项或漂移'),
        ('missing-exact-contract', missing_exact, '精确入口契约缺项或漂移'),
        ('missing-marker-group', missing_groups, '精确入口契约缺项或漂移'),
        ('missing-exact-argument', lambda root: metadata(root, 'close-b1-vm-exact.json', lambda value: value.update(argv=[arg for arg in value['argv'] if arg != '--exact'])), '实际Cargo命令与冻结执行点不符：close-b1-vm-exact'),
        ('runtime-command-substituted', lambda root: metadata(root, 'close-runtime-release.json', lambda value: value.update(argv=['python', '-c', 'pass'])), '实际Cargo命令与冻结执行点不符：close-runtime-release'),
        ('runtime-check-instead-of-build', lambda root: metadata(root, 'close-runtime-release.json', lambda value: value['argv'].__setitem__(2, 'check')), '实际Cargo命令与冻结执行点不符：close-runtime-release'),
        ('docs-subset-instead-of-all', lambda root: metadata(root, 'close-docs-closeout.json', lambda value: value['argv'].__setitem__(-1, 'docs')), '实际文档命令与冻结执行点不符'),
        ('rustfmt-input-omitted', lambda root: metadata(root, 'close-rustfmt.json', lambda value: value['argv'].pop()), '格式检查的完整Rust输入清单不符'),
        ('duplicate-local-summary', duplicate_summary, '实际测试计数不符：close-b1-native-exact'),
        ('null-log-reference', lambda root: metadata(root, 'close-language-tests.json', lambda value: value.update(log=None)), '证据引用不是非空路径文本：None'),
        ('empty-command', lambda root: metadata(root, 'close-runtime-release.json', lambda value: value.update(argv=[])), '实际命令不是参数数组：close-runtime-release'),
        ('wrong-source-directory', lambda root: metadata(root, 'close-clippy.json', lambda value: value.update(cwd='D:/different-source')), '实际命令工作目录与源码输入不符：close-clippy'),
    ]
    results = []
    for name, change, expected in cases:
        directory = temporary/name
        shutil.copytree(ARCHIVE, directory)
        if change:
            change(directory)
            result = subprocess.run([sys.executable, '-X', 'utf8', str(ARCHIVE/'audit.py'), '--archive-root', str(directory), '--manifest'], capture_output=True)
            assert result.returncode == 0, result.stderr.decode('utf-8')
        item = {'case': name, 'expected_error': expected, 'archive_root': str(directory)}
        for phase, script in [('before', ROOT/'audit-before.py'), ('after', ARCHIVE/'audit.py')]:
            argv = [sys.executable, '-X', 'utf8', str(script), '--archive-root', str(directory), '--repository-root', str(REPOSITORY)]
            result = subprocess.run(argv, capture_output=True)
            (output/(name+'-'+phase+'.stdout.log')).write_bytes(result.stdout)
            (output/(name+'-'+phase+'.stderr.log')).write_bytes(result.stderr)
            actual = result.stderr.decode('utf-8').strip()
            item[phase] = {'argv': argv, 'exit_code': result.returncode, 'actual_error': actual,
                           'stdout_sha256': hashlib.sha256(result.stdout).hexdigest(),
                           'stderr_sha256': hashlib.sha256(result.stderr).hexdigest()}
        after = item['after']
        item['passed'] = after['exit_code'] == (0 if expected is None else 1) and after['actual_error'] == (expected or '')
        results.append(item)
    report = {'scope': '同一临时归档分别执行修复前后审计器；负例重新生成目录/日志摘要，要求具体契约拒绝。',
              'temporary_root': str(temporary), 'input_manifest_sha256': hashlib.sha256((ARCHIVE/'evidence.json').read_bytes()).hexdigest(),
              'before_auditor_sha256': hashlib.sha256((ROOT/'audit-before.py').read_bytes()).hexdigest(),
              'after_auditor_sha256': hashlib.sha256((ARCHIVE/'audit.py').read_bytes()).hexdigest(),
              'passed': all(row['passed'] for row in results), 'cases': results}
    write(output/'results.json', report)
    print(json.dumps({'passed': report['passed'], 'cases': len(results), 'temporary_root': str(temporary)}, ensure_ascii=False))
    if not report['passed']:
        raise SystemExit('反例未按实际预期原因拒绝')


if __name__ == '__main__':
    main()
