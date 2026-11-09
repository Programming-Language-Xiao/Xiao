"""把真实共享Runtime结果与冻结B3-1旧结果逐组对账；不重算或覆盖历史宿主样本。"""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT.parents[4]
OLD_PATH = 'docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/pow-diff.json'
ERROR = 'X06-RUNTIME-009'


def value(bits):
    return struct.unpack('>d', struct.pack('>Q', int(bits, 16)))[0]


def rule(a, b, actual):
    if not math.isfinite(a) or not math.isfinite(b):
        assert actual == ERROR
        return 'F-2:nonfinite-input-first'
    if b == 0:
        assert actual == '3ff0000000000000'
        return 'F-2:zero-exponent'
    if a == 0 and b < 0:
        assert actual == ERROR
        return 'F-2:zero-negative-exponent-domain'
    if a < 0 and not b.is_integer():
        assert actual == ERROR
        return 'F-2:negative-base-noninteger-domain'
    if a == 0:
        expected = '8000000000000000' if math.copysign(1, a) < 0 and b.is_integer() and b % 2 else '0000000000000000'
        assert actual == expected
        return 'F-2:zero-base-sign-and-parity'
    if actual == ERROR:
        return 'F-2:nonfinite-result-overflow'
    if value(actual) == 0:
        return 'F-2/F-9:ieee-underflow-and-signed-zero'
    return 'F-9:shared-libm-and-four-ulp'


def main(phase, result_name='pow-transition.json'):
    destination = ROOT / phase / result_name
    if Path(result_name).name != result_name or destination.exists():
        raise SystemExit('结果必须是新文件名，拒绝覆盖既有证据')
    raw = (SOURCE / OLD_PATH).read_bytes()
    frozen = subprocess.check_output(['cmd', '/c', 'git', 'show', '96f28de0:' + OLD_PATH], cwd=SOURCE)
    assert raw == frozen, 'B3-1历史差表必须保持原始字节'
    baseline = json.loads(raw)
    record = json.loads((ROOT / phase / ('pow.json' if phase == 'baseline' else 'kernel-tests.json')).read_text(encoding='utf-8'))
    log = (ROOT / record['log']).read_bytes()
    assert record['exit_code'] == 0 and record['source_unchanged'] and hashlib.sha256(log).hexdigest() == record['sha256']
    current = {}
    for line in log.decode('utf-8').splitlines():
        # --nocapture的测试进度可能先打印在同一行；标记之后才是完整数据。
        if 'B3-POW-SHARED\t' in line:
            name, a, b, public, kernel, reference = line.split('B3-POW-SHARED\t', 1)[1].split('\t')
            assert name not in current
            current[name] = (a, b, public, kernel, reference)
    assert len(current) == baseline['cases'] == 1075
    rows, changes, rules = [], Counter(), Counter()
    for old in baseline['rows']:
        a, b, public, kernel, reference = current[old['id']]
        assert (a, b, reference) == (old['a_bits'], old['b_bits'], old['reference'])
        assert public == kernel == old['private_cmath_pow'], '共享内核或已验参考行为漂移'
        category = 'same' if public == old['legacy_float_power'] else 'value_or_error_boundary' if public.startswith('X') != old['legacy_float_power'].startswith('X') else 'value_bits'
        basis = rule(value(a), value(b), public)
        finite = math.isfinite(value(a)) and math.isfinite(value(b))
        def distance(left, right):
            if left.startswith('X') or right.startswith('X'):
                return None
            x, y = value(left), value(right)
            assert (x == 0) == (y == 0) and math.copysign(1, x) == math.copysign(1, y)
            return abs(int(left, 16) - int(right, 16))
        new_reference_ulp = distance(public, reference)
        if new_reference_ulp is not None:
            assert new_reference_ulp <= 4
        old_new_ulp = distance(old['legacy_float_power'], public)
        if category == 'value_bits':
            assert old_new_ulp == 1
        if category == 'value_or_error_boundary':
            assert not finite and basis == 'F-2:nonfinite-input-first'
        rows.append({'old_new_ulp': old_new_ulp, 'new_reference_ulp': new_reference_ulp, 'id': old['id'], 'a_bits': a, 'b_bits': b, 'legacy_float_power': old['legacy_float_power'],
                     'new_runtime_float_power': public, 'private_cmath_pow': kernel, 'reference': reference,
                     'difference': category, 'frozen_rule': basis, 'new_decision_required': False,
                     'reachability': {'rust_runtime_float_api': True, 'normal_source_finite_inputs': finite,
                                      'note': '有限输入可由字面量构造；完整语言/LLVM静态共核留B4' if finite else '正常源码构造拒绝非有限值；Runtime值可构造，C ABI原始Float载荷留本批专项覆盖'}})
        changes[category] += 1
        rules[basis] += 1
    assert dict(changes) == baseline['summary'] == {'same': 1011, 'value_or_error_boundary': 40, 'value_bits': 24}
    result = {'passed': True, 'phase': phase, 'cases': len(rows), 'summary': dict(changes), 'rules': dict(rules),
              'baseline_git': '96f28de0', 'baseline_path': OLD_PATH, 'baseline_sha256': hashlib.sha256(raw).hexdigest(),
              'runtime_source_manifest': record['source_before']['manifest'], 'runtime_log': record['log'],
              'source_language_status': '本批共享Runtime算子；LLVM静态llvm.pow.f64与cmath语言全链路留B4，未声称已全部接线',
              'b2_a_regression': '独立执行点及VM/原生维护入口另行验收，不用该表代替语言测试', 'rows': rows}
    with destination.open('x', encoding='utf-8', newline='\n') as stream:
        stream.write(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'passed': True, 'cases': len(rows), 'summary': dict(changes), 'rules': dict(rules)}, ensure_ascii=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['baseline', 'pow', 'abi'])
    parser.add_argument('--result', default='pow-transition.json')
    args = parser.parse_args()
    main(args.phase, args.result)
