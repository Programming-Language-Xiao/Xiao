"""只在所有最终检查及代码CI真实通过后生成有效收口，不改写失败记录。"""
import datetime
import hashlib
import json
from pathlib import Path
import re
import subprocess

TASK = Path(__file__).resolve().parents[1]
SOURCE = TASK / 'source'
ROOT = SOURCE / 'docs/DevDocs/assets/20o-b2-a2-a3/entry-20261106'
CODE = '5ef17bb23453f1e308a56f390734467822db3231'
CHECKS = ['final-arithmetic-admission', 'close-package-tests', 'close-clippy', 'close-rustfmt',
          'close-build-runtime-release', 'close-build-runtime-adapter', 'close-b1-contract',
          'close-b1-resources', 'close-b1-vm-exact', 'close-b1-native-exact', 'close-workspace-check',
          'close-benchmarks-check', 'close-docs-final', 'close-b1-guards', 'close-a1-vm-exact', 'close-a1-native-exact']
COUNTS = {'final-arithmetic-admission': 21, 'close-b1-contract': 9, 'close-b1-resources': 17,
          'close-b1-vm-exact': 1, 'close-b1-native-exact': 1, 'close-a1-vm-exact': 1, 'close-a1-native-exact': 1}


def read(name):
    return json.loads((ROOT / name).read_text(encoding='utf-8'))


def write(name, value):
    (ROOT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


records = [read(name + '.json') for name in CHECKS]
assert all(record['exit_code'] == 0 and record['source_unchanged'] and record['source_before'] == record['source_after'] for record in records)
identities = {record['source_before']['manifest'] for record in records}
assert len(identities) == 1
snapshot = read(identities.pop())
for role in ['maintenance', 'workspace']:
    run = read('ci-' + role + '-run.json')
    assert run['head_sha'] == CODE and run['status'] == 'completed' and run['conclusion'] == 'success'
pattern = r'(?m)^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\r\n]*'
for name, count in COUNTS.items():
    log = (ROOT / read(name + '.json')['log']).read_text(encoding='utf-8')
    assert re.findall(pattern, log) == [('ok', str(count), '0', '0')], name
boundary = read('boundary.json')
assert boundary['passed'] and not boundary['unclassified_paths'] and not boundary['protected_changes']
artifacts = []
for item in read('product-identity.json')['artifacts']:
    raw = Path(item['path']).read_bytes()
    assert sha(raw) == item['sha256'] and len(raw) == item['bytes']
    artifacts.append(item)
native = []
log = (ROOT / 'close-a1-native-exact.log').read_text(encoding='utf-8')
for path in re.findall(r'B2-A1-IDENTITY case=\S+ backend=native optimization=[03] artifact=(.+?) result=success', log):
    artifact = Path(path)
    assert artifact.is_relative_to(TASK)
    raw = artifact.read_bytes()
    native.append({'path': str(artifact), 'bytes': len(raw), 'sha256': sha(raw)})
assert len(native) == 6
write('native-artifacts-final.json', {'scope': 'A1已有三个完整示例的O0/O3回归，不是A5新算术语言差分',
    'runtime_and_adapter_unchanged_after_tests': True, 'tools': artifacts, 'artifacts': native, 'internal_only': True})
all_commands = []
for path in sorted(ROOT.glob('*.json')):
    try:
        row = json.loads(path.read_text(encoding='utf-8'))
    except (ValueError, UnicodeError):
        continue
    if isinstance(row, dict) and {'name', 'argv', 'cwd', 'exit_code', 'started_utc'}.issubset(row):
        all_commands.append(row)
all_commands.sort(key=lambda row: row['started_utc'])
write('commands.json', {'scope': '本任务run-step实际执行记录；每条保留原始JSON/日志/源码输入，不包含远端CI作业的本地PID',
    'commands': all_commands})
write('checks.json', {'entry_checks': ['a1-entry-audit', 'a1-entry-tests', 'b1-entry-contract', 'b1-entry-resources'],
    'effective_checks': CHECKS, 'exact_test_counts': COUNTS,
    'historical_failures': [row['name'] for row in all_commands if row['exit_code'] != 0],
    'historical_successes_not_final_snapshot': [row['name'] for row in all_commands if row['name'] not in CHECKS and row['exit_code'] == 0],
    'source_scope': snapshot['scope'], 'source_files': len(snapshot['files']),
    'note': '整份源码清单覆盖产品、UseDocs、registry、工具与锁；DevDocs滚动证据不计入产品输入树'})
write('closeout.json', {'passed': True, 'scope': 'Runtime内核', 'recorded_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'code_commit': CODE, 'a2_commit': '9ef111a5', 'effective_checks': CHECKS, 'exact_test_counts': COUNTS,
    'raw_source_tree_sha256': snapshot['raw_tree_sha256'], 'utf8_lf_source_tree_sha256': snapshot['utf8_lf_tree_sha256'],
    'source_files': len(snapshot['files']), 'guard_report': 'close-b1-guards-report.json',
    'abi': '1.18', 'codegen': 20, 'active_intrinsic_max': 32, 'cmath': 'planned', 'numeric_tutorial_scope': 'planned',
    'a4_through_a7_started': False, 'b1_721_executed': False,
    'ci_runs': {'workspace': 'ci-workspace-run.json', 'maintenance': 'ci-maintenance-run.json'},
    'ci_maintenance_jobs': 'ci-maintenance-jobs.json', 'ci_arithmetic_record': 'ci-kernel-linux.json',
    'ci_scope': '21项内核Windows本地/Linux默认CI；三平台现有原生维护回归。不得写成A5新语言/三平台差分或A6性能完成',
    'debts': '22C/22D及19/11/22独立保留；本轮绿色不关闭历史债务',
    'documentation_ci_scope': '本归档绑定5ef17bb2代码CI；后续纯文档收口提交不冒称已经包含在该运行中',
    'next_entry': 'A4转换矩阵；A5完整语言接线、A6性能、A7整批收口后置'})
print(json.dumps({'passed': True, 'effective_checks': len(CHECKS), 'source_files': len(snapshot['files']), 'code_commit': CODE}, ensure_ascii=False))
