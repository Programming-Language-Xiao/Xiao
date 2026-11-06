"""用本任务新编译的rlib构建独立对照，绑定源码和产物摘要。"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

task = Path(__file__).resolve().parents[1]
source = task/'source'
root = source/'docs/DevDocs/assets/20o-b2-a4-self-review/entry-20261106'
target = task/'cargo-target/debug'
runtime = [path for path in [target/'libxiao_runtime.rlib', target/'xiao_runtime.rlib'] if path.exists()]
if not runtime:
    runtime = list((target/'deps').glob('libxiao_runtime-*.rlib'))
syntax = list((target/'deps').glob('libxiao_syntax-*.rlib'))
assert len(runtime) == len(syntax) == 1, (runtime, syntax)
setup = json.loads((task/'environment.json').read_text(encoding='utf-8'))
output = task/'conversion-probe.exe'
argv = [setup['overrides']['RUSTC'], '--edition=2024', '--crate-name', 'a4_conversion_review',
        str(task/'scripts/conversion-probe.rs'), '-L', 'dependency='+str(target/'deps'),
        '--extern', 'xiao_runtime='+str(runtime[0]), '--extern', 'xiao_syntax='+str(syntax[0]), '-o', str(output)]
result = subprocess.run([sys.executable, '-X', 'utf8', '-u', str(task/'scripts/run-step.py'), 'review-probe-compile', '--', *argv], cwd=source)
assert result.returncode == 0
identity = {'scope': '独立对照程序及本任务新编译rlib；不是复用旧原生产物', 'files': []}
for path in [runtime[0], syntax[0], output, task/'scripts/conversion-probe.rs', task/'conversion-vectors.txt']:
    raw = path.read_bytes()
    identity['files'].append({'path':str(path), 'bytes':len(raw), 'sha256':hashlib.sha256(raw).hexdigest()})
(root/'probe-identity.json').write_text(json.dumps(identity, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
result = subprocess.run([sys.executable, '-X', 'utf8', '-u', str(task/'scripts/run-step.py'), 'review-conversion-differential', '--', str(output), str(task/'conversion-vectors.txt')], cwd=source)
raise SystemExit(result.returncode)
