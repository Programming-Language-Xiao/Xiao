"""参考生成器自检：零符号、精确输入、IEEE出口及精度升级/不可验证边界。"""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

import gmpy2 as g

spec = importlib.util.spec_from_file_location('reference', Path(__file__).with_name('generate-reference.py'))
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)


def case(name, a, b='-'):
    return {'function': name, 'a': a, 'b': b}


class ReferenceTests(unittest.TestCase):
    def test_all_integral_roundings_preserve_negative_zero(self):
        for name in ['floor', 'ceil', 'round', 'trunc']:
            with self.subTest(name=name):
                self.assertEqual(reference.reference(case(name, '8000000000000000'))[0], '8000000000000000')
        for name in ['ceil', 'round', 'trunc']:
            self.assertEqual(reference.reference(case(name, '8000000000000001'))[0], '8000000000000000')

    def test_input_is_exact_binary64(self):
        with g.context(precision=256):
            value = g.mpfr(reference.value(0x3fb999999999999a))
            self.assertEqual(value.as_integer_ratio(), (g.mpz(3602879701896397), g.mpz(36028797018963968)))

    def test_binary64_rounding_midpoints_and_subnormals(self):
        with g.context(precision=256):
            self.assertEqual(reference.binary64(g.mpfr(1) + g.exp2(-53)), 0x3ff0000000000000)
            self.assertEqual(reference.binary64(g.mpfr(1) + 3 * g.exp2(-53)), 0x3ff0000000000002)
            self.assertEqual(reference.binary64(g.exp2(-1075)), 0)
            self.assertEqual(reference.binary64(-g.exp2(-1075)), 1 << 63)
            self.assertEqual(reference.binary64(3 * g.exp2(-1075)), 2)

    def test_nonfinite_is_rejected_before_pow_zero(self):
        self.assertEqual(reference.reference(case('pow', '7ff8000000000001', '0000000000000000'))[0], reference.ERROR)
        self.assertEqual(reference.reference(case('pow', '7ff0000000000000', '0000000000000000'))[0], reference.ERROR)

    def test_sqrt_requires_a_unique_rounded_interval(self):
        result = reference.reference(case('sqrt', '4000000000000000'))
        self.assertEqual(result[0], '3ff6a09e667f3bcd')
        self.assertEqual(result[3], result[4])

    def test_precision_doubles_until_bounds_agree(self):
        with patch.object(reference, 'binary64', side_effect=[1, 2, 3, 4, 5, 5]) as convert:
            result = reference.reference(case('sqrt', '4000000000000000'))
            self.assertEqual(result[2], 1024)
            self.assertEqual(convert.call_count, 6)

    def test_unresolved_interval_is_not_guessed(self):
        with patch.object(reference, 'binary64', side_effect=[1, 2] * 5) as convert:
            result = reference.reference(case('sqrt', '4000000000000000'))
            self.assertEqual(result[0], 'UNVERIFIABLE')
            self.assertEqual(result[2], 4096)
            self.assertEqual(convert.call_count, 10)


if __name__ == '__main__':
    unittest.main(verbosity=2)
