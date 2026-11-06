"""固定种子、任意十进制及精确中点±最小十进制步长；不引入产品依赖。"""
import hashlib
import json
from pathlib import Path
import random

TASK = Path(__file__).resolve().parents[1]
ROOT = TASK/'source/docs/DevDocs/assets/20o-b2-a4-self-review/entry-20261106'
rng = random.Random(0xA4_20261106)
cases = ['0', '-0', '0e100000', '-0e-100000', '0.1', '-0.1', '1.50', '+1e2', '1e308',
         '1e309', '1e-324', '1e-325', '1e100000', '-1e-100000']
for _ in range(2400):
    digits = str(rng.randrange(1, 10)) + ''.join(str(rng.randrange(10)) for _ in range(rng.randrange(0, 120)))
    point = rng.randrange(len(digits)+1)
    if point:
        digits = digits[:point] + '.' + digits[point:]
    cases.append(('-' if rng.randrange(2) else '+')+digits+'e'+str(rng.randrange(-450, 401)))
for precision, minimum, maximum in [(53, -1074, 1023), (24, -149, 127)]:
    for _ in range(300):
        exponent = rng.randrange(minimum, maximum+1)-precision
        mantissa = 2*rng.randrange(1 << (precision-1), 1 << precision)+1
        coefficient, scale = (mantissa << exponent, 0) if exponent >= 0 else (mantissa*5**(-exponent), -exponent)
        for delta in [-1, 0, 1]:
            for sign in ['', '-']:
                cases.append(sign+str(coefficient+delta)+'e'+str(-scale))
for order in [-325, -324, -323, 0, 38, 307, 308, 309]:
    scale = 32768-1-order
    for digits in ['9'*32768, '1'+'0'*32766+'1']:
        for sign in ['', '-']:
            cases.append(sign+digits+'e'+str(-scale))
cases = list(dict.fromkeys(cases))
raw = ('\n'.join(cases)+'\n').encode()
path = TASK/'conversion-vectors.txt'
path.write_bytes(raw)
(ROOT/'conversion-vectors.txt').write_bytes(raw)
(ROOT/'vectors.json').write_text(json.dumps({'seed':'0xA4_20261106','vectors':len(cases),'comparisons':2*len(cases),
    'sha256':hashlib.sha256(raw).hexdigest(),'reference':'Rust 1.96.0 FromStr<f32/f64>，独立于被测BigUint有理数算法；零按F-8规范化',
    'scope':'Runtime对照，不是A5语言差分或A6性能'},ensure_ascii=False,indent=2)+'\n',encoding='utf-8',newline='\n')
print(json.dumps({'vectors':len(cases),'comparisons':2*len(cases),'bytes':len(raw)}))
