"""只读复核A5固定源码、原始执行记录、精确入口和实际CI作业。"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[4]
SCOPE = ['core/rust', 'stdlib', 'tests/fixtures', 'tests/benchmarks', '.github/workflows',
         '.cargo', 'tools', 'cli/ts', 'docs/UseDocs', 'docs/module-registry.json',
         'package.json', 'bun.lock', '.gitattributes']
REQUIRED = {'close-language-tests', 'close-package-tests', 'close-driver-unit-tests',
            'close-clippy', 'close-rustfmt', 'close-runtime-release', 'close-doc-adapter',
            'close-b1-contract', 'close-b1-resources', 'close-b1-vm-exact', 'close-b1-native-exact',
            'close-a1-vm-exact', 'close-a1-native-exact', 'close-workspace-check',
            'close-benchmarks-check', 'close-docs-closeout', 'close-b1-guards'}


def expected_markers():
    cases = ['a5_integer_arithmetic', 'a5_decimal_arithmetic', 'a5_conversions',
             'a5_carriers', 'a5_errors', 'a5_matrix', 'a5_bindings']
    errors = {'narrow': '009', 'syntax': '012', 'zero': '013', 'admission': '033',
              'quotient': '033', 'nonterminating': '034'}
    vm = [f'B2-A5-LANGUAGE case={name} backend=vm' for name in cases]
    vm += [f'B2-A5-ERROR case={name} backend=vm code=X06-RUNTIME-{code}' for name, code in errors.items()]
    vm += ['B2-A5-ERROR case=allocation backend=vm code=X06-RUNTIME-011 source_unchanged=true']
    vm += [f'B2-A5-REPLAY case={name} format=xiaoc' for name in cases if name != 'a5_carriers']
    vm += [f'B2-A5-REPLAY case={name} format=xar' for name in cases]
    native = [f'B2-A5-LANGUAGE case={name} backend=native optimization={level} artifact=' for name in cases for level in [0, 3]]
    native += [f'B2-A5-ERROR case={name} backend=native optimization={level} code=X06-RUNTIME-{code} artifact=' for name, code in errors.items() for level in [0, 3]]
    native += [f'B2-A5-CLEANUP case={name} optimization={level} temporary=once artifact=' for name in ['cast-success', 'cast-failure', 'right-failure'] for level in [0, 3]]
    native += [f'B2-A5-ALLOCATION optimization={level} code=X06-RUNTIME-011 out=unchanged source_unchanged=true artifact=' for level in [0, 3]]
    return {'vm': vm, 'native': native}


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def path(name):
    result = (ROOT / name).resolve()
    require(isinstance(name, str) and not Path(name).is_absolute()
            and result.is_relative_to(ROOT.resolve()), '证据引用越界：' + str(name))
    return result


def read(name):
    return json.loads(path(name).read_text(encoding='utf-8'))


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(raw):
    try:
        raw.decode('utf-8')
        return raw if b'\0' in raw else raw.replace(b'\r\n', b'\n')
    except UnicodeDecodeError:
        return raw


def inventory(write=False):
    observed = {item.relative_to(ROOT).as_posix(): item for item in ROOT.rglob('*')
                if item.is_file() and item != ROOT / 'evidence.json'}
    if write:
        rows = [{'path': name, 'bytes': len(item.read_bytes()), 'sha256': sha(item.read_bytes())}
                for name, item in sorted(observed.items())]
        (ROOT / 'evidence.json').write_text(json.dumps({'scope': '本目录全部实际文件，清单不递归计自身',
            'file_count': len(rows), 'files': rows}, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
        return len(rows)
    manifest = read('evidence.json')
    names = [item['path'] for item in manifest['files']]
    require(len(names) == len(set(names)) == manifest['file_count'] == len(observed)
            and set(names) == set(observed), '证据清单缺项或重复')
    for item in manifest['files']:
        raw = path(item['path']).read_bytes()
        require(len(raw) == item['bytes'] and sha(raw) == item['sha256'], '证据字节摘要不符：' + item['path'])
    return len(names)


def source(snapshot, commit):
    require(re.fullmatch('[0-9a-f]{40}', commit) is not None, '固定源码提交格式不符')
    require(snapshot['scope'] == SCOPE, '源码输入范围漂移')
    expected = subprocess.check_output(['cmd', '/c', 'git', 'ls-tree', '-r', '--name-only', '-z', commit, '--', *SCOPE], cwd=REPOSITORY)
    names = [row['path'] for row in snapshot['files']]
    require(len(names) == len(set(names)) and set(names) == {item.decode() for item in expected.split(b'\0') if item}, '完整源码清单缺项或重复')
    require(sha(json.dumps(snapshot['files'], ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()) == snapshot['raw_tree_sha256'], '原始源码树摘要不符')
    pairs = [[row['path'], row['utf8_lf_or_binary_sha256']] for row in snapshot['files']]
    require(sha(json.dumps(pairs, ensure_ascii=False, separators=(',', ':')).encode()) == snapshot['utf8_lf_tree_sha256'], '规范源码树摘要不符')
    requests = ''.join(commit + ':' + name + '\n' for name in names).encode()
    blobs = subprocess.check_output(['cmd', '/c', 'git', 'cat-file', '--batch'], cwd=REPOSITORY, input=requests)
    offset = 0
    for item in snapshot['files']:
        end = blobs.index(b'\n', offset)
        header = blobs[offset:end].split()
        require(len(header) == 3 and header[1] == b'blob', '固定提交缺少源码：' + item['path'])
        length = int(header[2])
        raw = blobs[end+1:end+1+length]
        require(sha(canonical(raw)) == item['utf8_lf_or_binary_sha256'], '输入偏离固定代码提交：' + item['path'])
        offset = end+length+2
    require(offset == len(blobs), 'Git源码对象流长度不符')


def clean_log(raw):
    text = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', raw.decode('utf-8', errors='replace'))
    return '\n'.join(re.sub(r'^\d{4}-\d\d-\d\dT\S+\s+', '', line).rstrip('\r') for line in text.splitlines())


def exact_section(text, target):
    lines = text.splitlines()
    passed = [i for i, line in enumerate(lines) if re.fullmatch('test ' + re.escape(target) + r' \.\.\. ok', line)]
    require(len(passed) == 1, '精确入口未恰好实际通过一次：' + target)
    end = passed[0]
    starts = [i for i, line in enumerate(lines[:end]) if line == 'running 1 test']
    require(bool(starts), '精确入口缺少单项起点：' + target)
    start = starts[-1]
    summaries = [line for line in lines[start:] if line.startswith('test result:')]
    require(bool(summaries) and re.match(r'^test result: ok\. 1 passed; 0 failed; 0 ignored;', summaries[0]) is not None, '精确入口计数/忽略状态不符：' + target)
    stop = next(i for i in range(end+1, len(lines)) if lines[i].startswith('test result:'))
    return lines[start:stop+1]


def markers(lines, expected):
    for marker in expected:
        require(sum(marker in line and line.endswith(' result=success') for line in lines) == 1, 'A5标记缺失或重复：' + marker)


def execution_identity(job):
    return (job['name'], job['head_sha'], job['started_at'], job['completed_at'],
            job['runner_id'], job['runner_name'],
            tuple((step['number'], step['name'], step.get('conclusion'),
                   step.get('started_at'), step.get('completed_at')) for step in job['steps']))


def origin_job(record, jobs, commit, run):
    matches = [job for job in jobs if job['id'] == record['job_id']]
    require(len(matches) == 1, '实际日志作业被聚合作业ID替代或缺失')
    job = matches[0]
    require(job['name'] == record['job_name'] and job['head_sha'] == record['code_commit'] == commit, '实际日志作业名称或源码身份不符')
    require(job['run_id'] == record['run_id'] == run['id'] and job['run_attempt'] == record['run_attempt'], '实际日志作业运行/次数不符')
    candidates = [item for item in jobs if item['name'] == job['name']]
    aggregate = max(candidates, key=lambda item: (item['run_attempt'], item['id']))
    require(aggregate['id'] == record['aggregate_job_id'] and aggregate['run_attempt'] == record['aggregate_run_attempt'], '未绑定平台最新汇总映射')
    require(execution_identity(aggregate) == execution_identity(job), '实际日志不属于最新汇总映射保留的执行')
    same_execution = [item for item in candidates if execution_identity(item) == execution_identity(job)]
    actual = min(same_execution, key=lambda item: (item['run_attempt'], item['id']))
    require(job['id'] == actual['id'], '实际日志作业不是该次执行的原始作业')
    require(record['execution_retained'] == (job['id'] != aggregate['id']), '保留执行与新执行标记不符')
    require(job['conclusion'] == record['conclusion'] == 'success', '实际日志作业未通过')
    raw = path(record['log']).read_bytes()
    require(sha(raw) == record['log_sha256'], 'CI原始日志摘要不符')
    return clean_log(raw)


def main():
    files = inventory()
    closeout = read('closeout.json')
    checks = read('checks.json')
    require(closeout['passed'] and closeout['scope'] == 'A5语言全链路', 'A5尚未有效收口')
    require(closeout['abi'] == '1.18' and closeout['codegen'] == 20 and closeout['active_intrinsic_max'] == 32, '版本边界漂移')
    require(not closeout['a6_a7_started'] and not closeout['b1_721_executed']
            and closeout['numeric_tutorial_scope'] == closeout['cmath'] == 'planned', '本批状态或后续范围越界')
    snapshot = read(closeout['product_manifest'])
    source(snapshot, closeout['code_commit'])
    require(len(checks['effective']) == len(set(checks['effective'])) and set(checks['effective']) == set(closeout['effective_checks']) == REQUIRED, '有效检查清单缺项或重复')
    require(checks['markers'] == expected_markers(), 'A5标记精确清单漂移')
    for name in checks['effective']:
        record = read(name + '.json')
        raw = path(record['log']).read_bytes()
        require(record['name'] == name and record['exit_code'] == 0 and sha(raw) == record['sha256'], '实际命令未通过或日志摘要不符：' + name)
        require(record['source_unchanged'] and record['source_before'] == record['source_after'], '执行期间源码变化：' + name)
        require(record['source_after']['utf8_lf_tree_sha256'] == snapshot['utf8_lf_tree_sha256'], '有效命令不属于最终产品输入：' + name)
        argv = record['argv']
        if Path(argv[0]).name.lower() in {'cargo', 'cargo.exe'}:
            require('--locked' in argv and '+1.96.0' in argv, 'Cargo未锁定工具/依赖：' + name)
        text = clean_log(raw)
        if name in checks['test_counts']:
            expected = checks['test_counts'][name]
            require(re.search(r'^test result: ok\. ' + str(expected) + r' passed; 0 failed;', text, re.M) is not None, '实际测试计数不符：' + name)
        if name in checks['exact_targets']:
            require('--exact' in argv, '实际命令未使用--exact：' + name)
            section = exact_section(text, checks['exact_targets'][name])
            if name in checks['marker_groups']:
                markers(section, checks['markers'][checks['marker_groups'][name]])
    guard = read('close-b1-guards-report.json')
    require(guard.get('passed') == guard.get('total') == 28, '28条既有守卫未通过')
    require(len(guard['cases']) == 28 and all(row['Passed'] is True for row in guard['cases']), '既有守卫逐项结果不完整')
    require(set(closeout['ci_runs']) == {'workspace', 'maintenance'}, '代码CI工作流清单缺项')
    require(set(closeout['ci_records']) == {'vm', 'windows', 'linux', 'macos'}, '三平台/VM实际作业清单缺项')
    for role, name in closeout['ci_runs'].items():
        run = read(name)
        require(run['head_sha'] == closeout['code_commit'] and run['status'] == 'completed' and run['conclusion'] == 'success', '代码CI未通过：' + role)
        require(run['name'] == {'workspace': 'Xiao workspace gates', 'maintenance': 'Xiao maintenance regression'}[role], 'CI工作流角色不符：' + role)
    run = read(closeout['ci_runs']['maintenance'])
    jobs = read(closeout['ci_jobs'])['jobs']
    for role, name in closeout['ci_records'].items():
        record = read(name)
        expected_job = {'vm': 'Rust workspace default tests (Linux)',
                        'windows': 'Native differential regression (Windows)',
                        'linux': 'Native differential regression (Linux)',
                        'macos': 'Native differential regression (macOS)'}[role]
        require(record['job_name'] == expected_job, 'CI平台角色与作业名称不符：' + role)
        log = origin_job(record, jobs, closeout['code_commit'], run)
        target = 'b1_capabilities_have_vm_golden_outputs' if role == 'vm' else 'b1_capabilities_match_native_golden_outputs'
        markers(exact_section(log, target), checks['markers']['vm' if role == 'vm' else 'native'])
    native = read('native-artifacts.json')
    require(native['code_commit'] == closeout['code_commit'] and len(native['artifacts']) == 34, '本地原生产物清单不完整')
    rows = native['artifacts']
    require(len({row['path'] for row in rows}) == 34 and len({row['marker'] for row in rows}) == 34, '本地原生产物路径或标记重复')
    require(all(row['bytes'] > 0 and re.fullmatch('[0-9a-f]{64}', row['sha256']) for row in rows), '本地原生产物身份不完整')
    identity = read('product-identity.json')
    require(identity['code_commit'] == closeout['code_commit'] and identity['runtime_build_record'] == 'close-runtime-release.json', '工具或Runtime来源不符')
    require(not closeout['independent_debts_closed'], '独立债务被本批核销')
    debt = read('independent-22c-record.json')
    first = read('ci-attempt1-maintenance-run.json')
    failed_job = read('ci-attempt1-macos.json')
    failed_log = path(debt['log']).read_bytes()
    require(not debt['closed'] and first['head_sha'] == closeout['code_commit']
            and first['conclusion'] == 'failure' and failed_job['id'] == debt['job_id']
            and failed_job['conclusion'] == 'failure', '首轮独立22C失败身份不符')
    require(sha(failed_log) == debt['log_sha256'] and debt['test'] in failed_log.decode('utf-8'), '首轮独立22C失败日志缺失')
    print(json.dumps({'passed': True, 'scope': closeout['scope'], 'files': files,
                      'source_files': len(snapshot['files']), 'code_commit': closeout['code_commit'],
                      'checks': len(checks['effective']), 'ci_platforms': list(closeout['ci_records'])}, ensure_ascii=False))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--manifest', action='store_true')
    parser.add_argument('--archive-root', type=Path)
    parser.add_argument('--repository-root', type=Path)
    args = parser.parse_args()
    if args.archive_root:
        ROOT = args.archive_root.resolve()
    if args.repository_root:
        REPOSITORY = args.repository_root.resolve()
    if args.manifest:
        print(json.dumps({'files': inventory(True)}, ensure_ascii=False))
    else:
        main()
