"""B3-3A/B只读归档审计：固定产品Git对象、完整源码与真实测试角色。"""
import argparse
from collections import Counter
import datetime
import hashlib
import json
from pathlib import Path, PureWindowsPath
import re
import subprocess

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]
ENTRY = '5343baac'
CODE = '89d782e1'
PREFIX = 'docs/DevDocs/assets/20o-b2-b/entry-20261109-b3-cabi/'
PRODUCT = ['core/rust', 'stdlib', 'tests/fixtures', 'tests/benchmarks', '.github/workflows', '.cargo', 'tools', 'cli/ts',
           'docs/UseDocs', 'docs/module-registry.json', 'package.json', 'bun.lock', '.gitattributes']
POW_PATHS = ['value/ops.rs', 'value/mod.rs', 'value/arithmetic/mod.rs', 'value/arithmetic/cmath.rs', 'value/arithmetic/cmath_tests.rs']
ALLOWED = {'core/rust/crates/xiao-runtime/src/' + name for name in POW_PATHS + ['abi.rs', 'abi_cmath.rs', 'abi_cmath_tests.rs']}
ALLOWED.update('core/rust/crates/xiao-intrinsics/' + name for name in ['build.rs', 'intrinsics.json', 'src/lib.rs'])
ALLOWED.update('core/rust/crates/xiao-runtime-abi/' + name for name in ['README.md', 'src/lib.rs'])
REFERENCES = ['docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/' + name
              for name in ['reference-vectors.tsv', 'reference-inputs.json', 'generate-reference.py', 'pow-diff.json']]
BASE = ['build-runtime', 'build-driver', 'runtime', 'intrinsics', 'abi', 'pow', 'guards', 'history', 'reference-replay']
FINAL = ['regression', 'b1-native', 'b1-vm', 'b2-native', 'b2-vm', 'workspace-check', 'clippy', 'benchmark', 'fmt',
         'c-probe', 'public-boundary', 'pow-annotations-final', 'docs']
TARGETS = {'b1-vm': ('20o_b1_probe', 'b1_capabilities_have_vm_golden_outputs'),
           'b1-native': ('20o_b1_probe', 'b1_capabilities_match_native_golden_outputs'),
           'b2-vm': ('20o_b2_b2_divide', 'lfloat_divide_has_vm_golden_outputs'),
           'b2-native': ('20o_b2_b2_divide', 'lfloat_divide_matches_native_golden_outputs')}
SUMMARY = r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;'
FUNCTIONS = ['sqrt', 'floor', 'ceil', 'round', 'trunc', 'abs', 'min', 'max', 'pow', 'sin', 'cos', 'tan', 'exp', 'ln', 'log10']
NEGATIVE = {'baseline': None, 'manifest': 'B3-CABI证据清单或摘要不符', 'attributes': 'B3-CABI证据清单或摘要不符',
            'target': 'B3-CABI精确入口命令或测试名不符', 'runtime': 'B3-CABI Runtime身份与构建记录不符',
            'pow': 'B3-CABI pow分类或数值距离不符'}


def require(condition, reason):
    if not condition:
        raise SystemExit(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(raw):
    try:
        raw.decode('utf-8')
    except UnicodeDecodeError:
        return raw
    return raw if b'\0' in raw else raw.replace(b'\r\n', b'\n')


def files(root):
    return [{'path': p.relative_to(root).as_posix(), 'bytes': p.stat().st_size, 'sha256': sha(p.read_bytes())}
            for p in sorted(root.rglob('*')) if p.is_file() and p != root / 'evidence.json' and '__pycache__' not in p.parts]


def main(root, source, core_only=False, live=False):
    def read(name):
        return json.loads((root / name).read_text(encoding='utf-8'))

    def git(*args, data=None):
        return subprocess.check_output(['cmd', '/c', 'git', *args], cwd=source, input=data)

    rows = files(root)
    require(read('evidence.json') == {'file_count': len(rows), 'files': rows}
            and any(r['path'] == '.gitattributes' for r in rows), 'B3-CABI证据清单或摘要不符')
    entry, code = (git('rev-parse', revision).decode().strip() for revision in [ENTRY, CODE])
    closeout, identity, bootstrap = read('closeout.json'), read('code-identity.json'), read('bootstrap.json')
    require(closeout['passed'] is True and closeout['code_commit'] == identity['code_commit'] == code
            and identity['entry_commit'] == bootstrap['entry_commit'] == entry, 'B3-CABI固定Git身份不符')
    changed = set(git('diff', '--name-only', entry, code).decode().splitlines())
    require(changed == ALLOWED and set(identity['allowed_product_paths']) == ALLOWED, 'B3-CABI产品改动越界')
    require(closeout['abi'] == '1.18' and closeout['codegen'] == 20 and closeout['stdlib_interface'] == 1
            and closeout['active_max'] == 48 and closeout['cmath'] == 'planned' and closeout['b3_4_started'] is False
            and not closeout['remote_ci_claimed'] and not closeout['b1_721_executed'] and not closeout['b2_a_perf'], 'B3-CABI收口范围越界')
    require(closeout['effective_checks'] == {'baseline': BASE, 'abi': FINAL}, 'B3-CABI执行点集合不完整')
    logs, records, snapshots = {}, {}, set()
    for phase, names in [('baseline', BASE), ('abi', FINAL)]:
        for name in names:
            key = phase + '/' + name
            record = read(key + '.json')
            require(record['log'] == key + '.log' and record['name'] == name and record['phase'] == phase, 'B3-CABI日志角色不符')
            raw = (root / record['log']).read_bytes()
            require(record['exit_code'] == 0 and sha(raw) == record['sha256'] and record['source_unchanged']
                    and record['source_before'] == record['source_after'] and record['runner_sha256'] == sha((root / 'run-step.py').read_bytes()),
                    'B3-CABI有效执行失败或记录被改写：' + key)
            require(record['cwd'] == bootstrap['source_root'] and record['pid'] > 0, 'B3-CABI执行路径或PID不符')
            snapshots.add(record['source_before']['manifest'])
            logs[key], records[key] = raw.decode('utf-8'), record
    require(len(snapshots) == 1 and identity['source_manifest'] == closeout['source_manifest'] == next(iter(snapshots)), 'B3-CABI有效执行不同源')
    snapshot = read(next(iter(snapshots)))
    require(snapshot['scope'] == PRODUCT and snapshot['reference_inputs'] == REFERENCES, 'B3-CABI源码范围不完整')
    source_rows = snapshot['files']
    require(snapshot['raw_tree_sha256'] == sha(json.dumps(source_rows, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode())
            and snapshot['utf8_lf_tree_sha256'] == sha(json.dumps([[r['path'], r['utf8_lf_or_binary_sha256']] for r in source_rows], ensure_ascii=False, separators=(',', ':')).encode()), 'B3-CABI源码清单摘要不符')
    tracked = {name.decode('utf-8') for name in git('ls-tree', '-r', '--name-only', '-z', code, '--', *PRODUCT).split(b'\0') if name}
    require([r['path'] for r in source_rows] == sorted(tracked | set(REFERENCES)), 'B3-CABI未覆盖完整产品与参考')
    requests = ''.join(code + ':' + r['path'] + '\n' for r in source_rows).encode()
    stream, offset = git('cat-file', '--batch', data=requests), 0
    for row in source_rows:
        end = stream.index(b'\n', offset)
        header = stream[offset:end].split()
        require(len(header) == 3 and header[1] == b'blob', 'B3-CABI Git对象缺文件')
        length = int(header[2]); raw = stream[end + 1:end + 1 + length]; offset = end + length + 2
        require(sha(canonical(raw)) == row['utf8_lf_or_binary_sha256'], 'B3-CABI源码偏离固定Git对象：' + row['path'])
        if row['path'] in REFERENCES:
            require((source / row['path']).read_bytes() == raw, 'B3-CABI参考偏离固定Git对象')
    require(offset == len(stream), 'B3-CABI对象流长度不符')
    for name, (target, function) in TARGETS.items():
        record = records['abi/' + name]
        flags = ['--ignored', '--exact', '--nocapture'] if name.endswith('native') else ['--exact', '--nocapture']
        argv = ['cargo', '+1.96.0', 'test', '--locked', '--manifest-path', 'core/rust/Cargo.toml', '-p', 'xiao-driver', '--test', target, function, '--', *flags]
        require(record['argv'] == argv and re.search(r'^test ' + re.escape(function) + r' \.\.\. ok\r?$', logs['abi/' + name], re.M), 'B3-CABI精确入口命令或测试名不符')
        require(re.findall(SUMMARY, logs['abi/' + name], re.M) == [('1', '0', '0')], 'B3-CABI精确入口不是恰好1 passed')
    require(re.findall(SUMMARY, logs['baseline/runtime'], re.M) == [('269','0','0'),('9','0','0'),('1','0','0'),('1','0','0'),('1','0','1'),('17','0','0'),('4','0','0')], 'B3-CABI Runtime计数不符')
    require(re.findall(SUMMARY, logs['baseline/abi'], re.M) == [('10','0','0')]
            and re.findall(SUMMARY, logs['baseline/pow'], re.M) == [('5','0','0')]
            and re.findall(SUMMARY, logs['baseline/intrinsics'], re.M) == [('8','0','0')], 'B3-CABI专项测试计数不符')
    require(re.findall(SUMMARY, logs['abi/regression'], re.M) == [('64','0','1'),('9','0','0'),('17','0','0'),('1','0','1'),('12','0','1'),('8','0','0')], 'B3-CABI B1/B2-A回归计数不符')
    guards = read('baseline/b1-guards-report.json')
    require(guards['passed'] == guards['total'] == len(guards['cases']) == 28 and all(c['Passed'] for c in guards['cases']), 'B3-CABI守卫不是28/28')
    metrics = re.findall(r'B3-CMATH function=(\w+) cases=(\d+) max_ulp=(\d+) limit=(\d+) result=success', logs['baseline/pow'])
    abi_counts = re.findall(r'B3-CMATH-ABI function=(\w+) cases=(\d+) result=success', logs['baseline/abi'])
    require(len(metrics) == len(abi_counts) == 15 and sum(int(r[1]) for r in abi_counts) == 17243
            and {r[0]: int(r[1]) for r in metrics} == {r[0]: int(r[1]) for r in abi_counts}, 'B3-CABI正式参考覆盖不完整')
    require(all(int(r[2]) <= int(r[3]) == (0 if r[0] in ['sqrt','floor','ceil','round','trunc','abs','min','max'] else 4) for r in metrics), 'B3-CABI参考误差预算不符')
    pow_manifest = read('baseline/pow-switch-manifest.json')
    require(pow_manifest['passed'] is True and pow_manifest['cases'] == len(pow_manifest['rows']) == 1075
            and pow_manifest['summary'] == {'same':1011,'value_or_error_boundary':40,'value_bits':24}, 'B3-CABI pow分类或数值距离不符')
    history = json.loads((source / REFERENCES[3]).read_text(encoding='utf-8'))
    require(pow_manifest['baseline_sha256'] == sha((source / REFERENCES[3]).read_bytes()), 'B3-CABI旧pow摘要不符')
    actual_pow = [line.split('B3-POW-SHARED\t',1)[1].split('\t') for line in logs['baseline/pow'].splitlines() if 'B3-POW-SHARED\t' in line]
    require(len(actual_pow) == 1075 and len({r[0] for r in actual_pow}) == 1075, 'B3-CABI共享pow日志不完整')
    actual_by_id = {r[0]: r for r in actual_pow}
    categories = Counter()
    for row, old in zip(pow_manifest['rows'], history['rows']):
        actual = actual_by_id[row['id']]
        require(row['id'] == old['id'] and [row[k] for k in ['a_bits','b_bits','legacy_float_power','reference']] == [old[k] for k in ['a_bits','b_bits','legacy_float_power','reference']]
                and actual[1:] == [row['a_bits'],row['b_bits'],row['new_runtime_float_power'],row['private_cmath_pow'],row['reference']]
                and row['new_runtime_float_power'] == row['private_cmath_pow'] == old['private_cmath_pow'], 'B3-CABI pow明细偏离原始日志')
        kind = 'same' if row['legacy_float_power'] == row['new_runtime_float_power'] else 'value_or_error_boundary' if row['legacy_float_power'].startswith('X') != row['new_runtime_float_power'].startswith('X') else 'value_bits'
        require(kind == row['difference'] and not row['new_decision_required'] and row['frozen_rule'].startswith(('F-2:', 'F-9:', 'F-2/F-9:')), 'B3-CABI pow分类或数值距离不符')
        if kind == 'value_bits':
            require(row['old_new_ulp'] == abs(int(row['legacy_float_power'],16)-int(row['new_runtime_float_power'],16)) == 1 and row['new_reference_ulp'] <= 4, 'B3-CABI pow分类或数值距离不符')
        if kind == 'value_or_error_boundary':
            require(row['frozen_rule'] == 'F-2:nonfinite-input-first', 'B3-CABI pow分类或数值距离不符')
        categories[kind] += 1
    require(dict(categories) == pow_manifest['summary'], 'B3-CABI pow分类或数值距离不符')
    surface, symbols = read('abi/public-surface.json'), read('abi/native-symbols.json')
    require(surface['active_intrinsics'] == list(range(1,49)) and surface['versions'] == {'abi':18,'codegen':20,'stdlib_interface':1}
            and not surface['cmath_product_exists'] and surface['cmath_status'] == 'planned' and not git('ls-tree','--name-only',code,'--','stdlib/cmath.xiao').strip(), 'B3-CABI公开面或包装越界')
    table_path = 'core/rust/crates/xiao-intrinsics/intrinsics.json'
    table = json.loads(git('show',code+':'+table_path))['intrinsics']
    original = json.loads(git('show',entry+':'+table_path))['intrinsics']
    require([r for r in table if r['id']<34 or r['id']==100] == original
            and [r['name'] for r in table if 34<=r['id']<=48] == ['cmath_'+n for n in FUNCTIONS], 'B3-CABI既有ID或声明身份被改写')
    runtime, build = surface['runtime'], records['baseline/build-runtime']
    runtime_record = {key:runtime[key] for key in ['path','bytes','sha256','mtime_ns']}
    require(closeout['runtime'] == runtime and build['runtime_before']['exists'] is False and build['runtime_after'] == dict(runtime_record, exists=True), 'B3-CABI Runtime身份与构建记录不符')
    require(datetime.datetime.fromisoformat(build['started_utc']).timestamp()-2 <= runtime['mtime_ns']/1e9 <= datetime.datetime.fromisoformat(build['finished_utc']).timestamp()+2, 'B3-CABI Runtime构建窗口不符')
    setup = read('environment.json')['overrides']; task = PureWindowsPath(bootstrap['task_root'])
    for key in ['CARGO_TARGET_DIR','XIAO_HOME','TEMP','TMP','XIAO_RUNTIME_LIBRARY']:
        require(PureWindowsPath(setup[key]).is_relative_to(task), 'B3-CABI任务目录包含关系不符')
    require(setup['XIAO_B1_F10'] == setup['XIAO_B2_A_PERF'] == '0' and PureWindowsPath(setup['XIAO_STDLIB_ROOT']) == PureWindowsPath(bootstrap['source_root'])/'stdlib', 'B3-CABI环境开关或标准库根不符')
    for key in ['abi/b1-native','abi/b2-native','abi/c-probe']:
        require(records[key]['runtime_before'] == records[key]['runtime_after'] == dict(runtime_record,exists=True), 'B3-CABI Runtime身份与构建记录不符')
    require(symbols['runtime'] == runtime and symbols['exit_code'] == 0 and {s for s in symbols['symbols'] if s.startswith('xiao_runtime_cmath_')} == {'xiao_runtime_cmath_'+n for n in FUNCTIONS}, 'B3-CABI C符号集合不符')
    probe = read('abi/c-probe-result.json')
    require(probe['passed'] and probe['failure'] is None and probe['runtime_unchanged'] and probe['inputs_unchanged']
            and probe['runtime_before'] == probe['runtime_after'] == runtime_record, 'B3-CABI外部C身份或结论不符')
    require(probe['tool_sha256'] == sha((root/'run-c-probe.py').read_bytes()) and probe['source_sha256'] == sha((root/'cmath-abi-probe.c').read_bytes()), 'B3-CABI外部C来源不符')
    require([r['name'] for r in probe['records']] == ['native-libs','compile-o0','run-o0','compile-o3','run-o3'] and all(r['exit_code']==0 for r in probe['records']), 'B3-CABI外部C执行点不完整')
    require(all(re.fullmatch(r'B3-CMATH-C-PROBE entries=15 checks=231 statuses=0,1,2,3,6 result=success\s*',r['output']) for r in probe['records'] if r['name'].startswith('run-')), 'B3-CABI外部C计数不符')
    if live:
        for name in ['path','archived_copy']:
            require(Path(runtime[name]).is_file() and sha(Path(runtime[name]).read_bytes()) == runtime['sha256'], 'B3-CABI本机Runtime存档不符')
    if not core_only:
        negative = read('negative-result.json')
        require(negative['passed'] and set(negative['cases']) == set(NEGATIVE), 'B3-CABI自反例不完整')
        for name, reason in NEGATIVE.items():
            case = negative['cases'][name]
            require(case['passed'] and case['read_only'] and case['exit_code'] == (1 if reason else 0)
                    and (case['output'] == reason if reason else json.loads(case['output'])['passed']), 'B3-CABI自反例原因或只读性不符')
    print(json.dumps({'passed':True,'core_only':core_only,'files':len(rows),'code_commit':code[:8],'checks':len(BASE)+len(FINAL),
                      'exact':4,'formal_abi_cases':17243,'runtime_tests':269,'active_max':48,'live_runtime_checked':live,
                      'scope':'B3-2A与B3-3A/B内部；B3-4/B4未开始'},ensure_ascii=False))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,default=ROOT)
    parser.add_argument('--source-root',type=Path,default=SOURCE)
    parser.add_argument('--core-only',action='store_true',help='仅供自反例内部复跑；不是最终审计')
    parser.add_argument('--live-runtime',action='store_true')
    args = parser.parse_args()
    main(args.root.resolve(),args.source_root.resolve(),args.core_only,args.live_runtime)
