"""新目录复核参考工具只读性及17,243条重生成；输出保存为独立自审记录。"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
FILES = ['generate-reference.py', 'test-reference.py', 'replay-reference.py', 'reference-inputs.json',
         'reference-vectors.tsv', 'reference-manifest.json']


def snapshot(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob('*') if p.is_file()}


def main():
    task = Path(json.loads((ROOT / 'bootstrap.json').read_text(encoding='utf-8'))['task_root']) / 'reference-self-review'
    task.mkdir(exist_ok=True)
    env = dict(os.environ, PYTHONUTF8='1', PYTHONIOENCODING='utf-8')
    for key in ['PYTHONDONTWRITEBYTECODE', 'PYTHONPYCACHEPREFIX']:
        env.pop(key, None)
    cases = {}
    with tempfile.TemporaryDirectory(prefix='fresh-reference-', dir=task) as temporary:
        target = Path(temporary)
        for name in FILES:
            shutil.copy2(ROOT / 'b1' / name, target / name)
        sources = snapshot(target)
        for name in ['test-reference.py', 'replay-reference.py']:
            before = snapshot(target)
            argv = [sys.executable, '-X', 'utf8', str(target / name)]
            run = subprocess.run(argv, capture_output=True, text=True, encoding='utf-8', env=env)
            after = snapshot(target)
            output = (run.stdout + run.stderr).strip()
            passed = run.returncode == 0 and before == after
            if name == 'test-reference.py':
                passed = passed and 'Ran 7 tests' in output and output.endswith('OK')
            else:
                passed = passed and json.loads(run.stdout.splitlines()[-1])['passed'] is True
            cases[name] = {'argv': argv, 'exit_code': run.returncode, 'read_only': before == after,
                           'new_files': sorted(set(after) - set(before)), 'output': output, 'passed': passed}
            print(json.dumps({'case': name, **cases[name]}, ensure_ascii=True), flush=True)
    result = {'passed': all(row['passed'] for row in cases.values()), 'source_sha256': sources, 'cases': cases,
              'scope': '全新复制目录，未设置禁止字节码缓存的环境变量；子命令前后所有文件字节和集合一致'}
    (HERE / 'reference-tools-result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    if not result['passed']:
        raise SystemExit('参考工具只读复核失败')


if __name__ == '__main__':
    main()
