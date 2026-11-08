"""F-9 离线参考 v1：精确 binary64 输入，MPFR 有向区间，最近偶数出口。"""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import random
import struct
import sys

import gmpy2 as g

SEED = 0xB203F90020261108
FUNCTIONS = ['sqrt', 'floor', 'ceil', 'round', 'trunc', 'abs', 'min', 'max', 'pow', 'sin', 'cos', 'tan', 'exp', 'ln', 'log10']
SIGN = 1 << 63
MASK = SIGN - 1
ERROR = 'X06-RUNTIME-009'


def bits(value):
    return struct.unpack('>Q', struct.pack('>d', value))[0]


def value(pattern):
    return struct.unpack('>d', struct.pack('>Q', pattern))[0]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def write(path, document):
    path.write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


def inputs():
    rng = random.Random(SEED)
    edge = {0, SIGN, 1, 2, 3, 0x000fffffffffffff, 0x0010000000000000, 0x0010000000000001,
            0x7fefffffffffffff, 0x7feffffffffffffe, 0x7ff0000000000000, 0xfff0000000000000,
            0x7ff8000000000001, 0xfff8000000000001, 0x7ff0000000000001,
            0x3ff0000000000000, 0x3fe0000000000000, 0x4000000000000000, 0x4340000000000000}
    # 输入常量只定义覆盖位置；黄金结果全部另经契约或 MPFR 区间计算。
    anchors = [-745.1332191019411, -744.4400719213812, -708.3964185322641, 709.782712893384,
               -1.0, 0.5, 1.0, 1.5, 2.5, 3.5, 1.5707963267948966, 3.141592653589793,
               6.283185307179586, 4503599627370495.5, 9007199254740992.0]
    for anchor in anchors:
        center = bits(abs(anchor))
        for delta in range(-4, 5):
            if 0 <= center + delta < 0x7ff0000000000000:
                edge.update([center + delta, SIGN | (center + delta)])
    edge |= {p | SIGN for p in list(edge) if p < SIGN}
    # 所有指数带都有精确二进制样本，包括极小数和超大三角参数约减。
    for exponent in range(0, 2047, 7):
        pattern = (exponent << 52) | rng.getrandbits(52)
        edge.update([pattern, pattern | SIGN])
    for _ in range(350):
        edge.add(rng.getrandbits(64))
    cases = []
    for name in FUNCTIONS:
        if name in ['min', 'max', 'pow']:
            continue
        for pattern in sorted(edge):
            cases.append({'function': name, 'a': f'{pattern:016x}', 'b': '-', 'category': 'edge-and-exponent-strata'})
    binary_edge = [0, SIGN, 1, SIGN | 1, 0x0010000000000000, 0x7fefffffffffffff,
                   bits(1.0), bits(-1.0), bits(2.0), bits(-2.0), bits(0.5), bits(-0.5),
                   bits(3.0), bits(-3.0), bits(float(1 << 52)), bits(float(1 << 53)), bits(-float(1 << 53)),
                   0x7ff0000000000000, 0xfff0000000000000, 0x7ff8000000000001]
    for name in ['min', 'max', 'pow']:
        pairs = {(a, b) for a in binary_edge for b in binary_edge}
        for _ in range(400):
            pairs.add((rng.getrandbits(64), rng.getrandbits(64)))
        if name == 'pow':
            for a in [bits(1.0) + d for d in range(-4, 5)] + [1, 0x000fffffffffffff, 0x7fefffffffffffff]:
                for b in [bits(x) for x in [-1075.0, -1074.0, -1022.0, -1.0, 0.5, 1.0, 1024.0]]:
                    pairs.add((a, b))
            for _ in range(200):
                pairs.add(((rng.randrange(1003, 1043) << 52) | rng.getrandbits(52),
                           (rng.randrange(1010, 1030) << 52) | rng.getrandbits(52) | (rng.randrange(2) << 63)))
        for a, b in sorted(pairs):
            cases.append({'function': name, 'a': f'{a:016x}', 'b': f'{b:016x}', 'category': 'cartesian-special-and-fixed-seed'})
    return {'schema': 1, 'generator_version': 1, 'seed': f'{SEED:016x}', 'input_encoding': 'IEEE754 binary64 big-endian u64 hex; no decimal parsing', 'cases': cases}


def contract(name, a_bits, b_bits):
    a, b = value(a_bits), value(b_bits) if b_bits is not None else None
    if not math.isfinite(a) or (b is not None and not math.isfinite(b)):
        return ERROR
    if name == 'sqrt' and a < 0 or name in ['ln', 'log10'] and a <= 0:
        return ERROR
    if name == 'abs':
        return f'{a_bits & MASK:016x}'
    if name in ['min', 'max']:
        if a == 0 and b == 0:
            return f'{((a_bits | b_bits) if name == "min" else (a_bits & b_bits)) & SIGN:016x}'
        chosen = (a_bits if a <= b else b_bits) if name == 'min' else (a_bits if a >= b else b_bits)
        return f'{chosen:016x}'
    if name == 'pow':
        if b == 0:
            return '3ff0000000000000'
        if a == 0:
            if b < 0:
                return ERROR
            return f'{SIGN if (a_bits & SIGN and b.is_integer() and b % 2 != 0) else 0:016x}'
        if a < 0 and not b.is_integer():
            return ERROR
    return None


def operation(name, a, b):
    if name == 'round':
        # 整数舍入模式是契约的最近偶数，独立于区间端点方向。
        with g.context(g.get_context(), round=g.RoundToNearest):
            return g.rint(a)
    if name == 'abs':
        return abs(a)
    if name == 'pow':
        return a ** b
    if name in ['floor', 'ceil', 'trunc']:
        # floor/ceil/trunc 的 gmpy2 便捷接口返回 mpz，会丢失负零。
        # rint_* 直接保留 MPFR 浮点结果与零符号。
        return getattr(g, 'rint_' + name)(a)
    return getattr(g, {'ln': 'log'}.get(name, name))(a)


def binary64(number):
    with g.context(g.ieee(64)):
        return bits(float(+number))


def reference(case):
    name, a_pattern = case['function'], int(case['a'], 16)
    b_pattern = None if case['b'] == '-' else int(case['b'], 16)
    special = contract(name, a_pattern, b_pattern)
    if special is not None:
        return special, 'contract_special', 0, special, special
    for precision in [256, 512, 1024, 2048, 4096]:
        bounds = []
        for rounding in [g.RoundDown, g.RoundUp]:
            with g.context(precision=precision, round=rounding, emin=-1073741823, emax=1073741823):
                a = g.mpfr(value(a_pattern))
                b = None if b_pattern is None else g.mpfr(value(b_pattern))
                bounds.append(binary64(operation(name, a, b)))
        if bounds[0] == bounds[1]:
            pattern = bounds[0]
            expected = f'{pattern:016x}' if math.isfinite(value(pattern)) else ERROR
            return expected, 'mpfr_interval', precision, f'{bounds[0]:016x}', f'{bounds[1]:016x}'
    return 'UNVERIFIABLE', 'mpfr_interval', 4096, f'{bounds[0]:016x}', f'{bounds[1]:016x}'


def generate(directory):
    assert (g.version(), g.mpfr_version(), g.mp_version()) == ('2.3.2', 'MPFR 4.2.2', 'GMP 6.3.0')
    directory.mkdir(parents=True, exist_ok=True)
    document = inputs()
    write(directory / 'reference-inputs.json', document)
    rows = ['# id\tfunction\ta_bits\tb_bits\texpected_bits_or_error\toracle\tprecision\tlower_binary64\tupper_binary64']
    outcomes, precision_counts, functions = Counter(), Counter(), Counter()
    for i, case in enumerate(document['cases']):
        expected, oracle, precision, lower, upper = reference(case)
        rows.append('\t'.join([f'v{i:05}', case['function'], case['a'], case['b'], expected, oracle, str(precision), lower, upper]))
        outcomes['unverifiable' if expected == 'UNVERIFIABLE' else 'error' if expected == ERROR else 'value'] += 1
        precision_counts[str(precision)] += 1
        functions[case['function']] += 1
    (directory / 'reference-vectors.tsv').write_text('\n'.join(rows) + '\n', encoding='utf-8', newline='\n')
    result = {'generator_version': 1, 'generator_sha256': sha(Path(__file__).read_bytes()), 'seed': document['seed'],
              'gmpy2': g.version(), 'mpfr': g.mpfr_version(), 'gmp': g.mp_version(),
              'rounding': '256/512/1024/2048/4096-bit RoundDown/RoundUp -> IEEE binary64 nearest-even; accept iff both bit patterns equal',
              'cases': len(document['cases']), 'functions': dict(functions), 'outcomes': dict(outcomes), 'precision_counts': dict(precision_counts),
              'inputs_sha256': sha((directory / 'reference-inputs.json').read_bytes()), 'vectors_sha256': sha((directory / 'reference-vectors.tsv').read_bytes()),
              'tool_provenance': 'tool-provenance.json', 'production_dependencies': 'unchanged'}
    write(directory / 'reference-manifest.json', result)
    print(json.dumps(result, ensure_ascii=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path(__file__).resolve().parent)
    generate(parser.parse_args().output)
