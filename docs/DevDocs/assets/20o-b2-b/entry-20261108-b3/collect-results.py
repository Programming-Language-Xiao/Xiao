"""从实际执行记录生成B3-1收口文件；与只读audit分开运行。"""
from collections import Counter
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]
CHECKS = ['build-runtime', 'kernel-tests', 'reference-self-tests', 'reference-replay', 'runtime-all',
          'unit-contract', 'workspace-check', 'benchmark', 'clippy', 'fmt', 'guards',
          'b1-vm', 'b1-native', 'b2-vm', 'b2-native', 'public-boundary', 'docs-final']
EXACT = ['b1-vm', 'b1-native', 'b2-vm', 'b2-native']
ALLOWED = ['core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs',
           'core/rust/crates/xiao-runtime/src/value/arithmetic/cmath_tests.rs']


def read(name):
    return json.loads((ROOT / name).read_text(encoding='utf-8'))


def write(name, document):
    (ROOT / name).write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


def main(prepare=False):
    records = [read('b1/' + name + '.json') for name in CHECKS if not (prepare and name == 'docs-final')]
    assert all(r['exit_code'] == 0 and r['source_before'] == r['source_after'] and r['source_unchanged'] for r in records)
    assert len({r['source_before']['manifest'] for r in records}) == 1
    for name in EXACT:
        log = (ROOT / 'b1' / (name + '.log')).read_text(encoding='utf-8')
        assert re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', log, re.M) == [('1', '0', '0')]
    kernel_log = (ROOT / 'b1/kernel-tests.log').read_text(encoding='utf-8')
    metrics = [{'function': n, 'cases': int(c), 'max_ulp': int(u), 'limit': int(t)} for n, c, u, t in re.findall(r'^B3-CMATH function=(\w+) cases=(\d+) max_ulp=(\d+) limit=(\d+) result=success$', kernel_log, re.M)]
    reference = read('b1/reference-manifest.json')
    assert len(metrics) == 15 and sum(row['cases'] for row in metrics) == reference['cases'] == 17243
    differences = []
    for line in kernel_log.splitlines():
        if line.startswith('B3-POW-DIFF\t'):
            _, name, a, b, old, new, expected = line.split('\t')
            differences.append({'id': name, 'a_bits': a, 'b_bits': b, 'legacy_float_power': old,
                                'private_cmath_pow': new, 'reference': expected, 'same_bits_or_error': old == new})
    assert len(differences) == reference['functions']['pow'] == 1075
    changes = Counter('same' if row['same_bits_or_error'] else 'value_or_error_boundary' if row['legacy_float_power'].startswith('X') != row['private_cmath_pow'].startswith('X') else 'value_bits' for row in differences)
    write('b1/pow-diff.json', {'platform': 'Windows x86_64-msvc', 'legacy_path': 'RuntimeValue::Float.power -> ops.rs f64::powf',
          'new_path': '私有 cmath::pow -> libm 0.2.16；本批未切换公开调用点', 'source_log': 'kernel-tests.log',
          'cases': len(differences), 'summary': dict(changes), 'rows': differences})
    code = subprocess.check_output(['cmd', '/c', 'git', 'rev-parse', '88886dd5'], cwd=SOURCE).decode().strip()
    products = [{'path': name, 'raw_sha256': hashlib.sha256((SOURCE / name).read_bytes()).hexdigest(),
                 'utf8_lf_sha256': hashlib.sha256((SOURCE / name).read_bytes().replace(b'\r\n', b'\n')).hexdigest()} for name in ALLOWED]
    write('code-identity.json', {'entry_commit': read('bootstrap.json')['entry_commit'], 'baseline_commit': 'ca63a792',
          'code_commit': code, 'allowed_product_paths': ALLOWED, 'products': products,
          'source_manifest': records[0]['source_before']['manifest'], 'reference_inputs': ['b1/reference-vectors.tsv', 'b1/reference-inputs.json', 'b1/generate-reference.py']})
    write('b1/kernel-metrics.json', {'cases': reference['cases'], 'functions': sorted(metrics, key=lambda row: row['function']),
          'source_log': 'kernel-tests.log', 'initial_failure': 'kernel-initial.log', 'initial_failure_cause':
          '参考工具floor/ceil/trunc便捷接口返回mpz丢失负零；改用MPFR rint_*，没有修改或放宽产品零语义。'})
    surface = read('b1/public-surface.json')
    write('b1/closeout.json', {'passed': not prepare, 'scope': 'B3-1 F-9参考与私有内核，尚未公开接线',
          'code_commit': code, 'effective_checks': CHECKS, 'exact_checks': EXACT,
          'source_manifest': records[0]['source_before']['manifest'], 'reference_cases': reference['cases'],
          'reference_unverifiable': reference['outcomes'].get('unverifiable', 0), 'reference_self_tests': 7,
          'kernel_tests': 5, 'runtime_unit_passed': 259, 'driver_unit_passed': 64, 'intrinsic_passed': 7,
          'contract_passed': 9, 'resource_passed': 17, 'guards': '28/28',
          'runtime': surface['runtime'], 'abi': '1.18', 'codegen': 20, 'stdlib_interface': 1,
          'active_intrinsic_max': 33, 'cmath': 'planned', 'ops_callsite_changed': False,
          'b1_721_executed': False, 'b2_a_perf': False, 'ci_claimed': False,
          'platforms': {'windows': '本批本机实测通过', 'linux': '本批未执行', 'macos': '本批未执行'},
          'next': 'B3-2 Runtime/C ABI；B4语言全链路；B5集中1.19/21'})
    print(json.dumps({'passed': not prepare, 'prepare_only': prepare, 'reference_cases': reference['cases'], 'pow_summary': dict(changes), 'checks': len(records), 'code_commit': code}, ensure_ascii=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepare', action='store_true', help='生成链接所需文件；收口明确passed=false，最终文档门禁通过后再正式生成')
    main(parser.parse_args().prepare)
