"""用独立Python整数公式生成A6固定输入和黄金；不调用Xiao/Rust数值实现。"""
from pathlib import Path
import json
import sys

sys.set_int_max_str_digits(0)
ROOT = Path(__file__).resolve().parent
D = 32768


def literal(text):
    return json.dumps(text, ensure_ascii=False)


def number(text, family='lfloat'):
    if family == 'lint':
        return f'lint(lfloat({literal(text)}))'
    return f'{family}({literal(text)})'


def decimal(coefficient, scale):
    if coefficient == 0:
        return '0'
    signed = str(coefficient)
    negative = signed.startswith('-')
    digits = signed.lstrip('-')
    plain = len(signed) if scale == 0 else (len(signed)+1 if 0 < scale < len(digits) else scale+2+negative)
    if scale < 0 or plain > 65536:
        return signed+'e'+str(-scale)
    if scale == 0:
        return signed
    if scale < len(digits):
        return ('-' if negative else '')+digits[:-scale]+'.'+digits[-scale:]
    return ('-' if negative else '')+'0.'+'0'*(scale-len(digits))+digits


def fibonacci(n):
    if n == 0:
        return 0, 1
    a, b = fibonacci(n//2)
    c = a*(2*b-a)
    d = a*a+b*b
    return (d, c+d) if n & 1 else (c, d)


CASES = []


def add(identity, family, result, expression, profiles, complexity='linear', dimension='coefficient_digits', body=None):
    CASES.append({'id': identity, 'family': family, 'result': result,
                  'expression': expression, 'body': body,
                  'complexity': complexity, 'dimension': dimension, 'profiles': profiles})


def profile(label, size, left, right, expected, result='lfloat', family='lfloat'):
    expected_source = literal(expected) if result == 'str' else (expected if result in ['bool', 'float', 'sfloat', 'int'] else number(expected, result))
    return {'label': label, 'size': size, 'maximum': label == 'maximum',
            'left': number(left, family) if family not in ['str', 'float', 'sfloat'] else (literal(left) if family == 'str' else left),
            'right': number(right, family) if family not in ['str', 'float', 'sfloat'] else (literal(right) if family == 'str' else right),
            'expected': expected_source}


def sizes(maximum=D, half=4096):
    return [('n', half), ('2n', 2*half), ('maximum', maximum)]


def main():
    for operation, expression, digit in [('add', 'left + right', '7'), ('subtract', 'left - right', '1')]:
        p = [profile(label, n, ('4' if operation == 'add' else '9')*n,
                     ('3' if operation == 'add' else '8')*n, digit*n, 'lint', 'lint') for label, n in sizes()]
        add('lint.'+operation, 'lint', 'lint', expression, p)
    p = [profile(label, n, '9'*n, '9'*n, '9'*(n-1)+'8'+'0'*(n-1)+'1', 'lint', 'lint') for label, n in sizes(D//2)]
    add('lint.multiply', 'lint', 'lint', 'left * right', p, 'superlinear', 'balanced_operand_digits')
    p = [profile(label, n, '1e'+str(n), '9'*n, '1') for label, n in sizes()]
    add('lfloat.cancel', 'lfloat', 'lfloat', 'left - right', p, dimension='aligned_digits')
    p = [profile(label, n, '9'*n+'e-'+str(n), '9'*n+'e-'+str(n), decimal((10**n-1)**2, 2*n)) for label, n in sizes(D//2)]
    add('lfloat.multiply', 'lfloat', 'lfloat', 'left * right', p, 'superlinear', 'balanced_operand_digits')
    p = []
    for label, n in sizes(D//2):
        a = -(10**(2*n)-1)
        b = 10**n+7
        p.append(profile(label, n, str(a), str(b), str(a//b)))
    add('lfloat.floor', 'lfloat', 'lfloat', 'left // right', p, 'superlinear', 'balanced_division_digits')
    p = [profile(label, n, '1e100000', '9'*(n-1)+'7e-100000', str(pow(10, 200000, 10**n-3))+'e-100000') for label, n in sizes()]
    assert all(int(str(row['expected']).split('(', 1)[1].split('e', 1)[0].strip('"')) != 0 for row in p)
    add('lfloat.modular', 'lfloat', 'lfloat', 'left % right', p, 'superlinear', 'modulus_digits_fixed_scale_gap_200000')
    p = [profile(label, n, '-'+('9'*n), '1e'+str(n), '1') for label, n in sizes()]
    add('lfloat.remainder-cancel', 'lfloat', 'lfloat', 'left % right', p, 'superlinear', 'virtual_denominator_digits')
    maximum_two_power = 46879
    assert len(str(5**maximum_two_power)) == D
    p = [profile(label, n, '1', str(2**n), str(5**n)+'e-'+str(n)) for label, n in sizes(maximum_two_power)]
    add('lfloat.finite-division', 'lfloat', 'lfloat', 'left / right', p, 'superlinear', 'denominator_binary_exponent')
    p = []
    for label, n in sizes():
        # F_k和F_(k+1)互质，分母不止2/5因子，触发密集gcd后034。
        k = int(n/0.20898764024997873)
        a, b = fibonacci(k)
        while len(str(b)) > n:
            k -= 1
            a, b = fibonacci(k)
        assert len(str(a)) <= n and len(str(b)) == n
        rest = b
        while rest % 2 == 0:
            rest //= 2
        while rest % 5 == 0:
            rest //= 5
        assert rest > 1
        p.append(profile(label, len(str(b)), str(a), str(b), 'X06-RUNTIME-034', 'str'))
    error_body = 'try\n    left / right\ncatch failure as ArithmeticError\n    return error_code(failure)\nreturn "unexpected-success"'
    add('lfloat.nonterminating-gcd', 'lfloat', 'str', '', p, 'superlinear', 'consecutive_fibonacci_digits', error_body)
    p = [profile(label, n, '2', str(n), str(2**n), 'lint', 'lint') for label, n in sizes(108852, 8192)]
    assert len(str(2**108852)) == D
    add('lint.power', 'lint', 'lint', 'left ** right', p, 'superlinear', 'actual_result_binary_exponent')
    p = [profile(label, n, '1.001', str(n), decimal(1001**n, 3*n)) for label, n in sizes(10921, 2048)]
    assert len(str(1001**10921)) == D
    add('lfloat.power', 'lfloat', 'lfloat', 'left ** right', p, 'superlinear', 'actual_decimal_coefficient_growth')
    p = [profile(label, n, '9'*min(n, D) if n <= D else '0'*(n-D)+'9'*D, '', '9'*min(n, D), 'lfloat', 'str') for label, n in sizes(65536)]
    add('text.parse', 'str', 'lfloat', 'lfloat(left)', p, dimension='raw_utf8_bytes')
    p = []
    for label, n in sizes(65534):
        digits = min(n, D)
        p.append(profile(label, n, '9'*digits+'e-'+str(n), '0', '0.'+'0'*(n-digits)+'9'*digits, 'str'))
    add('lfloat.format', 'lfloat', 'str', 'str(left)', p, dimension='output_fraction_digits')
    p = [profile(label, n, '9'*n, '0', '9'*n, 'lfloat', 'lint') for label, n in sizes()]
    add('lint.to-decimal', 'lint', 'lfloat', 'lfloat(left)', p)
    p = [profile(label, n, '9'*n, '0', '9'*n, 'lint') for label, n in sizes()]
    add('lfloat.to-integer', 'lfloat', 'lint', 'lint(left)', p)
    p = [profile(label, n, '1.'+'9'*(n-1), '0', '1', 'int') for label, n in sizes()]
    add('lfloat.to-int-truncate', 'lfloat', 'int', 'int(left)', p, 'superlinear')
    p = [profile(label, n, '9'*n+'e-'+str(n+323), '0', '1e-323', 'float') for label, n in sizes()]
    add('lfloat.to-float-subnormal', 'lfloat', 'float', 'float(left)', p, 'superlinear')
    p = [profile(label, n, '9'*n+'e-'+str(n+45), '0', 'sfloat(1e-45)', 'sfloat') for label, n in sizes()]
    add('lfloat.to-sfloat-subnormal', 'lfloat', 'sfloat', 'sfloat(left)', p, 'superlinear')
    for family, exponent, binary in [('float', 1074, 'float(lfloat("5e-324"))'), ('sfloat', 149, 'sfloat(lfloat("1e-45"))')]:
        p = [profile('maximum', 1, binary, family+'(0)', str(5**exponent)+'e-'+str(exponent), 'lfloat', family)]
        add(family+'.to-decimal', family, 'lfloat', 'lfloat(left)', p, dimension='fixed_binary_width')
    p = [profile(label, n, '9'*n+'e-100000', '9'*n+'e100000', 'true', 'bool') for label, n in sizes()]
    add('identity.compare', 'lfloat', 'bool', 'left < right', p)
    p = [profile(label, n, '9'*n, '9'*n, 'true', 'bool') for label, n in sizes()]
    add('identity.hash-set', 'lfloat', 'bool', '{left, right} == {left}', p)
    p = [profile(label, n, '9'*n, '9'*n, '9'*n, 'str') for label, n in sizes()]
    add('carrier.tuple', 'lfloat', 'str', '', p, body='values = (left, right)\nreturn str(values[0])')
    p = [profile(label, n, '4'*n+'e-'+str(n), '3'*n+'e-'+str(n), '0.'+'7'*n) for label, n in sizes()]
    add('lfloat.add', 'lfloat', 'lfloat', 'left + right', p)
    p = [profile(label, n, '9'*n, '0', '9'*n) for label, n in sizes()]
    add('carrier.thread-xcon', 'lfloat', 'lfloat', '', p,
        body='root = current_runtime()\nworker = spawn(root, __a6_echo, left)\nreturn join(worker)')
    add('carrier.pool-cycle-xcon', 'lfloat', 'lfloat', '', p,
        body='root = current_runtime()\npool = pool_create(root, 1)\njob = submit(pool, __a6_echo, left)\nactual = wait(job)\nshutdown(pool)\nreturn actual')
    result = {'schema': 1, 'reference': 'Python标准整数与独立闭式公式；不执行Xiao/Rust实现。浮点黄金是已知IEEE子正规数的源级精确目标。',
              'D': D, 'W': 65536, 'H': 100000, 'cases': CASES}
    path = ROOT/'catalog.json'
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n', encoding='utf-8', newline='\n')
    print(json.dumps({'cases': len(CASES), 'profiles': sum(len(case['profiles']) for case in CASES), 'bytes': path.stat().st_size}))


if __name__ == '__main__':
    main()
