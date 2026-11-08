"""B3-0/B3-1只读归档审计：固定Git对象，不要求当前HEAD，不生成或重写任何证据。"""
import argparse
from collections import Counter
import datetime
import hashlib
import json
from pathlib import Path, PureWindowsPath
import re
import subprocess

ROOT = Path(__file__).resolve().parent
ENTRY = 'ae1744e586990f59d6714b9ba67e77505de3c3ab'
CODE = '88886dd5'
PREFIX = 'docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/'
PRODUCT_ROOTS = ['core/rust', 'stdlib', 'tests/fixtures', 'tests/benchmarks', '.github/workflows', '.cargo',
                 'tools', 'cli/ts', 'docs/UseDocs', 'docs/module-registry.json', 'package.json', 'bun.lock', '.gitattributes']
ALLOWED = {'core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs',
           'core/rust/crates/xiao-runtime/src/value/arithmetic/cmath_tests.rs'}
REFERENCE = ['b1/reference-vectors.tsv', 'b1/reference-inputs.json', 'b1/generate-reference.py']
B0 = ['build-runtime', 'build-driver', 'runtime-all', 'unit-contract', 'guards', 'workspace-check', 'benchmark',
      'clippy', 'fmt', 'docs', 'b1-vm', 'b1-native', 'b2-vm', 'b2-native', 'public-boundary-final', 'inherited-b2-audit']
B1 = ['build-runtime', 'kernel-tests', 'reference-self-tests', 'reference-replay', 'runtime-all', 'unit-contract',
      'workspace-check', 'benchmark', 'clippy', 'fmt', 'guards', 'b1-vm', 'b1-native', 'b2-vm', 'b2-native', 'public-boundary', 'docs-final']
EXACT = ['b1-vm', 'b1-native', 'b2-vm', 'b2-native']
SUMMARY = r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;'
FUNCTIONS = {'sqrt', 'floor', 'ceil', 'round', 'trunc', 'abs', 'min', 'max', 'pow', 'sin', 'cos', 'tan', 'exp', 'ln', 'log10'}
EXACT_FUNCTIONS = {'sqrt', 'floor', 'ceil', 'round', 'trunc', 'abs', 'min', 'max'}
NEGATIVE = {'baseline': None, 'tampered_manifest': 'B3证据清单或摘要不符', 'missing_attributes': 'B3证据清单或摘要不符',
            'semantic_tamper_after_manifest_rebuild': 'B3版本或公开边界越界',
            'runtime_tamper_after_manifest_rebuild': 'B3原生Runtime身份与构建记录不符'}


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


def manifest_rows(root):
    return [{'path': path.relative_to(root).as_posix(), 'bytes': path.stat().st_size, 'sha256': sha(path.read_bytes())}
            for path in sorted(root.rglob('*')) if path.is_file() and path != root / 'evidence.json' and '__pycache__' not in path.parts]


def main(root, source, core_only=False, live_runtime=False):
    def read(name):
        return json.loads((root / name).read_text(encoding='utf-8'))

    def git(*args, data=None):
        return subprocess.check_output(['cmd', '/c', 'git', *args], cwd=source, input=data)

    def verify_manifest(directory):
        document = json.loads((directory / 'evidence.json').read_text(encoding='utf-8'))
        rows = manifest_rows(directory)
        require(document == {'file_count': len(rows), 'files': rows}, 'B3证据清单或摘要不符')
        return rows

    rows = verify_manifest(root)
    require(any(row['path'] == '.gitattributes' for row in rows), 'B3证据清单或摘要不符')
    verify_manifest(root / 'b0')
    bootstrap, identity = read('bootstrap.json'), read('code-identity.json')
    code = git('rev-parse', CODE).decode().strip()
    require(bootstrap['entry_commit'] == identity['entry_commit'] == ENTRY and identity['code_commit'] == code,
            'B3固定Git入口或代码身份不符')
    changed = set(git('diff', '--name-only', ENTRY, code).decode().splitlines())
    require({path for path in changed if not path.startswith(PREFIX)} == ALLOWED == set(identity['allowed_product_paths']),
            'B3代码改动越出允许集合')
    require({r['path'] for r in identity['products']} == ALLOWED and identity['reference_inputs'] == REFERENCE, 'B3产品摘要集合不完整')
    for product in identity['products']:
        require(sha(canonical(git('show', code + ':' + product['path']))) == product['utf8_lf_sha256'], 'B3产品摘要偏离固定代码对象')
    require(not git('diff', '--name-only', identity['baseline_commit'], code, '--', PREFIX + 'b0').strip(), 'B3-0历史基线被改写')
    inherited = bootstrap['inherited_editorial_delta']
    previous = set(git('diff', '--name-only', bootstrap['b2_code'], ENTRY).decode().splitlines())
    require({p for p in previous if not p.startswith('docs/DevDocs/')} == {inherited['path']}, 'B3入口继承了未登记的产品差异')
    inherited_blob = git('show', ENTRY + ':' + inherited['path'])
    require(inherited_blob == git('show', inherited['source_commit'] + ':' + inherited['path']) and sha(inherited_blob) == inherited['sha256'],
            'B3既有UseDocs勘误身份不符')

    setup = read('environment.json')['overrides']
    task = PureWindowsPath(bootstrap['task_root'])
    for key in ['CARGO_TARGET_DIR', 'XIAO_HOME', 'TEMP', 'TMP', 'XIAO_RUNTIME_LIBRARY']:
        require(PureWindowsPath(setup[key]).is_relative_to(task), 'B3环境路径越出独立任务根：' + key)
    require(PureWindowsPath(setup['XIAO_RUNTIME_LIBRARY']).is_relative_to(PureWindowsPath(setup['CARGO_TARGET_DIR'])), 'B3Runtime不在独立target内')
    require(PureWindowsPath(setup['XIAO_STDLIB_ROOT']) == PureWindowsPath(bootstrap['source_root']) / 'stdlib', 'B3标准库加载根身份不符')
    require(setup['XIAO_B1_F10'] == setup['XIAO_B2_A_PERF'] == '0' and setup['RUSTUP_TOOLCHAIN'] == '1.96.0', 'B3环境开关或工具链越界')

    phase_logs, phase_runtime = {}, {}
    for phase, names, commit in [('b0', B0, ENTRY), ('b1', B1, code)]:
        closeout, surface = read(phase + '/closeout.json'), read(phase + '/public-surface.json')
        require(closeout['passed'] is True and closeout['effective_checks'] == names and closeout['exact_checks'] == EXACT, 'B3有效执行点集合不完整')
        require(closeout['abi'] == '1.18' and closeout['codegen'] == 20 and closeout['active_intrinsic_max'] == 33
                and closeout['cmath'] == 'planned' and not closeout['b1_721_executed'] and not closeout['b2_a_perf'] and not closeout['ci_claimed'],
                'B3版本或公开边界越界')
        require((closeout['runtime_unit_passed'], closeout['driver_unit_passed'], closeout['intrinsic_passed'], closeout['contract_passed'], closeout['resource_passed'], closeout['guards'])
                == (259, 64, 7, 9, 17, '28/28'), 'B3收口测试计数不符')
        if phase == 'b1':
            require(closeout['code_commit'] == code and closeout['source_manifest'] == identity['source_manifest']
                    and closeout['reference_cases'] == 17243 and closeout['reference_unverifiable'] == 0
                    and closeout['reference_self_tests'] == 7 and closeout['kernel_tests'] == 5
                    and closeout['stdlib_interface'] == 1 and not closeout['ops_callsite_changed'], 'B3-1收口身份或覆盖不符')
        logs, records, manifests = {}, {}, set()
        for name in names:
            record = read(phase + '/' + name + '.json')
            require(record['log'] == phase + '/' + name + '.log' and record['phase'] == phase and record['name'] == name, 'B3日志角色不符')
            raw = (root / record['log']).read_bytes()
            require(record['exit_code'] == 0 and record['sha256'] == sha(raw), 'B3有效执行失败或日志摘要不符：' + phase + '/' + name)
            require(record['source_unchanged'] and record['source_before'] == record['source_after'] and record['runner_sha256'] == sha((root / 'run-step.py').read_bytes()),
                    'B3执行期间源码或执行器身份变化')
            require(record['cwd'] == bootstrap['source_root'] and record['pid'] > 0, 'B3执行路径或PID身份不符')
            logs[name], records[name] = raw.decode('utf-8'), record
            manifests.add(record['source_before']['manifest'])
        require(len(manifests) == 1 and closeout['source_manifest'] == next(iter(manifests)), 'B3有效执行没有绑定同一源码')
        snapshot_name = next(iter(manifests))
        snapshot = read(snapshot_name)
        require(snapshot_name == 'source-manifests/' + snapshot['raw_tree_sha256'] + '.json', 'B3源码清单文件名不符')
        require(snapshot['scope'] == PRODUCT_ROOTS and snapshot['reference_inputs'] == REFERENCE, 'B3源码保护范围不完整')
        source_rows = snapshot['files']
        raw_tree = sha(json.dumps(source_rows, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode())
        canonical_tree = sha(json.dumps([[r['path'], r['utf8_lf_or_binary_sha256']] for r in source_rows], ensure_ascii=False, separators=(',', ':')).encode())
        require(snapshot['raw_tree_sha256'] == raw_tree and snapshot['utf8_lf_tree_sha256'] == canonical_tree, 'B3源码树摘要不符')
        tracked = {p.decode('utf-8') for p in git('ls-tree', '-r', '--name-only', '-z', commit, '--', *PRODUCT_ROOTS).split(b'\0') if p}
        if phase == 'b1':
            tracked.update(PREFIX + name for name in REFERENCE)
        require([r['path'] for r in source_rows] == sorted(tracked), 'B3源码清单没有覆盖完整产品树与参考')
        requests = ''.join(commit + ':' + r['path'] + '\n' for r in source_rows).encode()
        stream = git('cat-file', '--batch', data=requests)
        offset = 0
        for row in source_rows:
            end = stream.index(b'\n', offset)
            header = stream[offset:end].split()
            require(len(header) == 3 and header[1] == b'blob', 'B3固定代码对象缺文件：' + row['path'])
            length = int(header[2])
            require(sha(canonical(stream[end + 1:end + 1 + length])) == row['utf8_lf_or_binary_sha256'], 'B3被测源码偏离固定Git代码对象：' + row['path'])
            offset = end + length + 2
        require(offset == len(stream), 'B3源码对象流长度不符')
        for record in records.values():
            require(record['source_before'] == {'manifest': snapshot_name, 'raw_tree_sha256': raw_tree,
                    'utf8_lf_tree_sha256': canonical_tree, 'files': len(source_rows)}, 'B3执行记录的源码身份不符')
        for name in EXACT:
            require('--exact' in records[name]['argv'] and re.findall(SUMMARY, logs[name], re.M) == [('1', '0', '0')], 'B3精确入口不是恰好1 passed')
            require(('--ignored' in records[name]['argv']) == name.endswith('native'), 'B3原生精确入口参数不符')
        expected_unit = [('64', '0', '1'), ('9', '0', '0'), ('17', '0', '0'), ('2', '0', '1'), ('7', '0', '0'), ('259', '0', '0')]
        require(re.findall(SUMMARY, logs['unit-contract'], re.M) == expected_unit, 'B3单元/契约/资源计数漂移')
        require(re.findall(SUMMARY, logs['runtime-all'], re.M) == [('259', '0', '0'), ('9', '0', '0'), ('1', '0', '0'), ('1', '0', '0'), ('1', '0', '1'), ('17', '0', '0'), ('4', '0', '0')],
                'B3Runtime完整测试计数漂移')
        guards = read(phase + '/b1-guards-report.json')
        require(guards['passed'] == guards['total'] == len(guards['cases']) == 28 and all(c['Passed'] for c in guards['cases']), 'B3守卫不是28/28')
        for name, backend in [('b1-vm', 'vm'), ('b1-native', 'native')]:
            require('B2-B2-DIVIDE backend=' + backend in logs[name], 'B3维护入口遗漏B2单入口示例')
        require(surface['active_intrinsics'] == list(range(1, 34)) and surface['absent_intrinsics'] == list(range(34, 49))
                and surface['tombstones'][0]['id'] == 100 and surface['tombstones'][0]['status'] == 'tombstone'
                and surface['versions'] == {'abi': 18, 'codegen': 20, 'stdlib_interface': 1} and surface['cmath_status'] == 'planned'
                and not surface['cmath_product_exists'], 'B3版本或公开边界越界')
        table = json.loads(git('show', commit + ':core/rust/crates/xiao-intrinsics/intrinsics.json'))['intrinsics']
        require(sorted(r['id'] for r in table if r['status'] == 'active') == surface['active_intrinsics']
                and all(r['id'] not in range(34, 49) for r in table), 'B3公开intrinsic越界')
        registry_raw = git('show', commit + ':docs/module-registry.json')
        registry = json.loads(registry_raw)
        registry_source = next(r for r in source_rows if r['path'] == 'docs/module-registry.json')
        require(registry_source['raw_sha256'] == surface['registry_sha256']
                and sha(canonical(registry_raw)) == registry_source['utf8_lf_or_binary_sha256']
                and next(r for r in registry['modules'] if r['id'] == 'stdlib.cmath')['status'] == 'planned'
                and not git('ls-tree', '--name-only', commit, '--', 'stdlib/cmath.xiao').strip(), 'B3registry身份不符')
        for path, constant, value in [('core/rust/crates/xiao-runtime-abi/src/lib.rs', 'ABI_MINOR_VERSION', 18),
                                      ('core/rust/crates/xiao-codegen-llvm/src/lib.rs', 'CODEGEN_VERSION', 20),
                                      ('core/rust/crates/xiao-standard-library/src/lib.rs', 'STDLIB_INTERFACE_VERSION', 1)]:
            require(re.search(r'pub const ' + constant + r': u32 = (\d+);', git('show', commit + ':' + path).decode()).group(1) == str(value), 'B3版本或公开边界越界')
        runtime, build = surface['runtime'], records['build-runtime']
        runtime_core = {k: runtime[k] for k in ['path', 'bytes', 'sha256', 'mtime_ns']}
        require(closeout['runtime'] == runtime and build['runtime_after'] == dict(runtime_core, exists=True), 'B3原生Runtime身份与构建记录不符')
        require(PureWindowsPath(runtime['path']) == PureWindowsPath(setup['XIAO_RUNTIME_LIBRARY'])
                and PureWindowsPath(runtime['archived_copy']).is_relative_to(task), 'B3原生Runtime身份与构建记录不符')
        timestamp = runtime['mtime_ns'] / 1e9
        start = datetime.datetime.fromisoformat(build['started_utc']).timestamp()
        finish = datetime.datetime.fromisoformat(build['finished_utc']).timestamp()
        require(start - 2 <= timestamp <= finish + 2, 'B3Runtime不在本批构建窗口')
        for name in ['b1-native', 'b2-native']:
            require(records[name]['runtime_before'] == records[name]['runtime_after'] == dict(runtime_core, exists=True), 'B3原生Runtime身份与构建记录不符')
            require(records[name]['started_utc'] >= build['finished_utc'], 'B3原生执行早于新Runtime构建')
        symbols = read(phase + '/native-symbols.json')
        require(symbols['runtime'] == runtime and symbols['exit_code'] == 0 and 'xiao_runtime_lfloat_divide' in symbols['symbols']
                and not any(n.startswith('xiao_runtime_cmath_') for n in symbols['symbols']), 'B3Runtime C符号边界不符')
        if live_runtime:
            archive = Path(runtime['archived_copy'])
            require(archive.is_file() and sha(archive.read_bytes()) == runtime['sha256'], 'B3本机Runtime存档缺失或摘要不符')
        phase_logs[phase], phase_runtime[phase] = logs, runtime
    require(not read('b0/build-runtime.json')['runtime_before']['exists'], 'B3-0未从全新Runtime开始')
    require(read('b1/build-runtime.json')['runtime_before']['sha256'] == phase_runtime['b0']['sha256'], 'B3-1构建来源Runtime身份不符')

    reference = read('b1/reference-manifest.json')
    require((reference['gmpy2'], reference['mpfr'], reference['gmp']) == ('2.3.2', 'MPFR 4.2.2', 'GMP 6.3.0'), 'B3参考版本不符')
    require(reference['generator_sha256'] == sha((root / REFERENCE[2]).read_bytes())
            and reference['inputs_sha256'] == sha((root / REFERENCE[1]).read_bytes())
            and reference['vectors_sha256'] == sha((root / REFERENCE[0]).read_bytes()), 'B3参考生成器或向量摘要不符')
    inputs = read(REFERENCE[1])
    vector_rows = [line.split('\t') for line in (root / REFERENCE[0]).read_text(encoding='utf-8').splitlines() if not line.startswith('#')]
    require(len(vector_rows) == len(inputs['cases']) == reference['cases'] == 17243 and inputs['seed'] == reference['seed'] == 'b203f90020261108', 'B3参考覆盖或种子不符')
    for index, (row, case) in enumerate(zip(vector_rows, inputs['cases'])):
        require(len(row) == 9 and row[:4] == [f'v{index:05}', case['function'], case['a'], case['b']]
                and row[7] == row[8] and row[4] != 'UNVERIFIABLE' and row[5] in ['contract_special', 'mpfr_interval']
                and (int(row[6]) == 0 if row[5] == 'contract_special' else int(row[6]) in [256, 512, 1024, 2048, 4096]), 'B3参考输入输出未配对或区间未收敛')
    counts = Counter(row[1] for row in vector_rows)
    require(dict(counts) == reference['functions'] and set(counts) == FUNCTIONS and not reference['outcomes'].get('unverifiable', 0), 'B3参考缺少函数或存在未验项')
    logs = phase_logs['b1']
    require(re.findall(SUMMARY, logs['kernel-tests'], re.M) == [('5', '0', '0')] and 'Ran 7 tests' in logs['reference-self-tests'] and logs['reference-self-tests'].rstrip().endswith('OK'), 'B3内核或参考自检未通过')
    metrics = [{'function': f, 'cases': int(c), 'max_ulp': int(u), 'limit': int(t)} for f, c, u, t in re.findall(r'^B3-CMATH function=(\w+) cases=(\d+) max_ulp=(\d+) limit=(\d+) result=success$', logs['kernel-tests'], re.M)]
    require(len(metrics) == 15 and {r['function']: r['cases'] for r in metrics} == dict(counts), 'B3内核实际覆盖不完整')
    require(all(r['limit'] == (0 if r['function'] in EXACT_FUNCTIONS else 4) and r['max_ulp'] <= r['limit'] for r in metrics), 'B3内核误差越出冻结预算')
    require(read('b1/kernel-metrics.json')['functions'] == sorted(metrics, key=lambda r: r['function']), 'B3内核统计偏离原始日志')
    replay = json.loads(logs['reference-replay'].splitlines()[-1])
    require(replay['passed'] and not replay['originals_overwritten'] and replay['files'] == {name: sha((root / 'b1' / name).read_bytes()) for name in ['reference-inputs.json', 'reference-vectors.tsv', 'reference-manifest.json']}, 'B3参考重生成不一致')
    difference = read('b1/pow-diff.json')
    actual_differences = [line.split('\t')[1:] for line in logs['kernel-tests'].splitlines() if line.startswith('B3-POW-DIFF\t')]
    require(difference['cases'] == len(difference['rows']) == len(actual_differences) == 1075, 'B3幂行为差表不完整')
    for row, actual in zip(difference['rows'], actual_differences):
        require([row[k] for k in ['id', 'a_bits', 'b_bits', 'legacy_float_power', 'private_cmath_pow', 'reference']] == actual
                and row['same_bits_or_error'] == (actual[3] == actual[4]), 'B3幂行为差表偏离实际算子调用')
    provenance = read('b1/tool-provenance.json')
    require(json.loads((root / 'b0/reference-tools.log').read_text(encoding='utf-8')) == provenance, 'B3工具来源摘要偏离实际取证日志')
    require((provenance['mpfr'], provenance['gmp']) == ('MPFR 4.2.2', 'GMP 6.3.0') and len(provenance['archives']) == 2
            and all(r['pypi_digest_verified'] for r in provenance['archives']) and all(r['matches_published_wheel'] for r in provenance['loaded_binaries'])
            and provenance['offline_only'] and not provenance['product_dependency_added'], 'B3参考工具来源不完整')
    require(read('b1/kernel-initial.json')['exit_code'] == 101 and 'floor a=8000000000000000' in (root / 'b1/kernel-initial.log').read_text(encoding='utf-8'), 'B3初轮参考缺陷历史未保留')
    if not core_only:
        negative = read('self-negative-result.json')
        require(negative['passed'] is True and set(negative['cases']) == set(NEGATIVE), 'B3自反例集合不完整')
        for name, reason in NEGATIVE.items():
            case = negative['cases'][name]
            require(case['passed'] is True and case['read_only'] is True and case['exit_code'] == (1 if reason else 0), 'B3自反例未证明只读拒绝')
            if reason:
                require(case['output'] == reason and case['expected_reason'] == reason, 'B3自反例拒绝原因不符')
    print(json.dumps({'passed': True, 'core_only': core_only, 'files': len(rows), 'baseline_commit': ENTRY[:8], 'code_commit': code[:8],
                      'b0_checks': len(B0), 'b1_checks': len(B1), 'exact_entries_per_phase': 4, 'reference_cases': 17243,
                      'max_ulp': max(r['max_ulp'] for r in metrics), 'live_runtime_checked': live_runtime,
                      'scope': 'B3-0/B3-1 私有内核；尚未公开接线'}, ensure_ascii=False))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--source-root', type=Path, default=ROOT.parents[4], help='仅用于读取固定Git对象，可指向主仓库')
    parser.add_argument('--core-only', action='store_true', help='自反例内部入口；显式省略自反例报告验证，不是最终收口')
    parser.add_argument('--live-runtime', action='store_true', help='另外校验独立任务根内的两份本机Runtime存档')
    args = parser.parse_args()
    main(args.root.resolve(), args.source_root.resolve(), args.core_only, args.live_runtime)
