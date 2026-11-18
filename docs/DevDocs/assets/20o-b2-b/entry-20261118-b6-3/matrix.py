"""从 F-3/F-12 的整数关系生成独立边界输入和黄金。"""

import hashlib
import json
import sys


D, W, H = 32768, 65536, 100000
PLATFORMS = ("windows", "linux", "macos")
LANGUAGE_BACKENDS = ("vm", "o0", "o3")
CMATH = ("sqrt", "floor", "ceil", "round", "trunc", "abs", "min", "max",
         "pow", "sin", "cos", "tan", "exp", "ln", "log10")


def literal(value):
    return json.dumps(value, ensure_ascii=False)


def decimal(text):
    return f"lfloat({literal(text)})"


def integer(text):
    return f"lint({decimal(text)})"


def output(text):
    data = text.encode("utf-8")
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
            "preview": text if len(text) <= 200 else text[:65] + "…" + text[-65:]}


def cases():
    sys.set_int_max_str_digits(W)
    rows = []

    def program(identifier, rule, source, *, stdout="", code=None, span=None, diagnostic_detail=None):
        start = len(source[:source.rindex(span)].encode()) if span else None
        location = [start, start + len(span.encode())] if span else None
        rows.append(({
            "id": identifier, "rule": rule, "backends": LANGUAGE_BACKENDS,
            "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
            "expected": {"exit": 3 if code else 0, "stdout": output(stdout),
                         "code": f"X06-RUNTIME-{code}" if code else None, "span": location,
                         "diagnostic_detail": diagnostic_detail},
        }, source))

    def expr(identifier, rule, expression, expected, prefix=""):
        if expected in {"002", "009", "011", "012", "013", "033", "034"}:
            program(identifier, rule, prefix + f"print({expression})\n", code=expected, span=expression)
        else:
            program(identifier, rule, prefix + f"print({expression})\n", stdout=expected + "\n")

    for identifier, value, expected in (
        ("bytes-exact", "0" * W, "0"),
        ("bytes-over-before-syntax", "0" * W + "x", "033"),
        ("utf8-bytes-before-ascii-syntax", "界" * (W // 3 + 1), "033"),
        ("syntax-before-digits", "9" * (D + 1) + "x", "012"),
        ("digits-limit", "9" * D, "9" * D),
        ("digits-over", "9" * (D + 1), "033"),
        ("leading-zeros", "0" * D + "1", "1"),
        ("exponent-bound", "1e100000", "1e100000"),
        ("exponent-over", "0e100001", "033"),
        ("display-scale-positive-bound", "1e-100000", "1e-100000"),
        ("display-scale-positive-over", "1e-100001", "033"),
        ("display-scale-fraction-over", "1.0e-100000", "033"),
        ("canonical-scale-over", "10e100000", "033"),
        ("zero-scale-exception", "0.0e-100000", "0"),
        ("plain-text-bound", "1e-65534", "0." + "0" * 65533 + "1"),
        ("plain-text-fallback", "1e-65535", "1e-65535"),
        ("unicode-digit-syntax", "１", "012"),
    ):
        expr(identifier, "6.1.1/5.2/8.1–8.2", decimal(value), expected)

    for identifier, rule, left, op, right, expected in (
        ("zero-keeps-scale", "6.1.2", "0e-100000", "+", "1.50", "1.50"),
        ("strong-cancel", "6.1.2", "1e32768", "-", "9" * D, "1"),
        ("result-digits-over", "6.1.2", "9" * D, "+", "1", "033"),
        ("alignment-work-w", "6.1.2", "1e32767", "+", "1e-32767", "033"),
        ("alignment-work-w-plus-one", "6.1.2", "1e32768", "+", "1e-32767", "033"),
        ("balanced-product", "6.1.2", "9" * (D // 2), "*", "9" * (D // 2),
         "9" * (D // 2 - 1) + "8" + "0" * (D // 2 - 1) + "1"),
        ("product-scale-bound", "6.1.2", "1e-99999", "*", "0.1", "1e-100000"),
        ("product-display-scale-over", "6.1.2", "1e-100000", "*", "0.1", "033"),
        ("product-canonical-scale-over", "6.1.2", "2e100000", "*", "5", "033"),
        ("divide-2-factor-d", "6.1.3", "1", "/", str(2**46880), "0." + str(5**46880).zfill(46880)),
        ("divide-2-factor-d-plus-one", "6.1.3", "1", "/", str(2**46881), "033"),
        ("divide-5-factors", "6.1.3", "1", "/", "125", "0.008"),
        ("divide-reduced", "6.1.3", "3.00", "/", "120", "0.025"),
        ("divide-retains-display-scale", "6.1.3", "1.00", "/", "2", "0.50"),
        ("divide-nonterminating-before-scale", "6.1.3/5.2", "1e100000", "/", "3e-100000", "034"),
        ("divide-zero-before-work", "6.1.3/5.2", "1e100000", "/", "0", "013"),
        ("floor-d-scale", "6.1.6", "1e32767", "//", "1", "1" + "0" * (D - 1)),
        ("floor-h-delta", "6.1.6", "1e100000", "//", "1e-100000", "033"),
        ("floor-small-negative", "6.1.6", "-1e-100000", "//", "1e100000", "-1"),
        ("floor-negative-carry", "6.1.6", "-9.99", "//", "1", "-10"),
        ("remainder-h-delta-zero", "6.1.7", "1e100000", "%", "1e-100000", "0"),
        ("remainder-h-delta-nonzero", "6.1.7", "1e100000", "%", "3e-100000", "1e-100000"),
        ("remainder-d-plus-one-cancels", "6.1.7", "-" + "9" * D, "%", "1e32768", "1"),
        ("remainder-d-plus-one-over", "6.1.7", "-" + "9" * D, "%", "2e32768", "033"),
        ("remainder-h-delta-200001-input", "6.1.1/6.1.7", "1e100000", "%", "1e-100001", None),
        ("power-lfloat-scale-bound", "6.1.5", "0.1", "**", "100000", "1e-100000"),
        ("power-lfloat-scale-over", "6.1.5", "0.1", "**", "100001", "033"),
        ("power-fractional-domain", "6.1.5/5.2", "1", "**", "1.5", "009"),
        ("power-unit-fast-path", "6.1.5", "-1", "**", "1000001.0", "-1"),
    ):
        if expected is None:
            expr(identifier, rule, decimal(right), "033", f"a = {decimal(left)}\n")
        else:
            expr(identifier, rule, f"a {op} b", expected, f"a = {decimal(left)}\nb = {decimal(right)}\n")

    for family, value in (("lint", integer), ("lfloat", decimal)):
        for n, detail in ((1_000_000, "位数下界"), (1_000_001, "指数超过1000000")):
            program(f"power-{family}-{n}", "6.1.5",
                    f"a = {value('12')}\nb = {value(str(n))}\nprint(a ** b)\n",
                    code="033", span="a ** b", diagnostic_detail=detail)
    expr("power-lint-result-d", "6.1.5", f"{integer('2')} ** {integer('108852')}", str(2**108852))
    expr("convert-result-digits-over", "6.1.1/F-8", f"lint({decimal('1e32768')})", "033")
    expr("convert-fixed-overflow", "5.2/F-8", f"int({decimal('9223372036854775808')})", "009")
    expr("convert-signed-underflow", "F-8", f"float({decimal('-1e-100000')})", "-0")

    prefix = 'one = lfloat("1")\nthree = lfloat("3")\nzero = lfloat("0")\n'
    for identifier, arguments, expected in (
        ("precision-min", 'one, three, 1, "half_even"', "0.3"),
        ("precision-max-short", 'one, one, 32768, "half_even"', "1"),
        ("precision-over", 'one, three, 32769, "half_even"', "012"),
        ("rounding-before-zero", 'one, zero, 1, "bad"', "012"),
        ("explicit-zero", 'one, zero, 1, "half_even"', "013"),
        ("explicit-scale", 'lfloat("1e100000"), lfloat("1e-100000"), 1, "half_even"', "033"),
    ):
        expr(identifier, "6.1.4/5.2", f"lfloat_divide({arguments})", expected, prefix)
    call = 'lfloat_divide(one, zero, 0, "bad")'
    program("precision-before-rounding-and-zero", "6.1.4/5.2", prefix + f"print({call})\n",
            code="012", span=call, diagnostic_detail="precision必须在1到32768之间")
    expr("explicit-work-w", "6.1.4", 'lfloat_divide(a, b, 32768, "away_from_zero")',
         "1" + "0" * (D - 2) + "1e-65535",
         f'a = lfloat("1")\nb = {decimal("9" * D)}\n')
    rounding = {
        "half_even": (2, -2), "half_up": (3, -3), "half_down": (2, -2),
        "toward_zero": (2, -2), "away_from_zero": (3, -3),
        "floor": (2, -3), "ceiling": (3, -2),
    }
    program("seven-roundings", "6.1.4/F-1", "".join(
        f"print(lfloat_divide({decimal(a)}, lfloat(2), 1, {literal(mode)}))\n"
        for mode in rounding for a in ("5", "-5")),
        stdout="".join(f"{value}\n" for pair in rounding.values() for value in pair))
    program("named-order-and-atomicity", "5.2/F-12", '''def note(str label, lfloat item) -> lfloat
    print(label)
    return item

left = lfloat("1.50")
right = lfloat("0")
value = lfloat("2.25")
try
    value = lfloat_divide(denominator = note("denominator", right), numerator = note("numerator", left), rounding = "bad", precision = 0)
catch error as Error
    print(error_code(error))
finally
    print("cleanup")
print(value)
print(left)
print(right)
''', stdout="denominator\nnumerator\ncleanup\nX06-RUNTIME-012\n2.25\n1.50\n0\n")

    for identifier, function, arguments, expected in (
        ("sqrt-domain", "sqrt", "-1.0", "009"),
        ("sqrt-negative-zero", "sqrt", "-0.0", "-0"),
        ("floor-negative-small", "floor", "-0.5", "-1"),
        ("ceil-negative-small", "ceil", "-0.5", "-0"),
        ("round-ties-even", "round", "2.5", "2"),
        ("trunc-negative-small", "trunc", "-0.5", "-0"),
        ("abs-negative-zero", "abs", "-0.0", "0"),
        ("min-zero-sign", "min", "0.0, -0.0", "-0"),
        ("max-zero-sign", "max", "-0.0, 0.0", "0"),
        ("sin-subnormal", "sin", "-5e-324", "-5e-324"),
        ("cos-negative-zero", "cos", "-0.0", "1"),
        ("tan-negative-zero", "tan", "-0.0", "-0"),
        ("exp-overflow", "exp", "1000.0", "009"),
        ("exp-underflow", "exp", "-1000.0", "0"),
        ("ln-zero", "ln", "0.0", "009"),
        ("log10-negative", "log10", "-1.0", "009"),
        ("pow-zero-negative", "pow", "0.0, -1.0", "009"),
        ("pow-negative-zero", "pow", "-0.0, 3.0", "-0"),
        ("pow-negative-underflow", "pow", "-1e-300, 3.0", "-0"),
        ("pow-negative-fractional", "pow", "-2.0, 0.5", "009"),
    ):
        if identifier == "sin-subnormal":
            expr(identifier, "F-2/F-9", f"{function}({arguments}) == float(-5e-324)", "true",
                 f"from std.cmath import {function}\n")
        else:
            # Wrapper domain errors originate inside the official module. The exact language
            # call-site spans are covered by the direct-primitive VM/native regressions.
            call = f"{function}({arguments})"
            if expected == "009":
                program(identifier, "F-2/F-9", f"from std.cmath import {function}\nprint({call})\n",
                        code=expected)
            else:
                expr(identifier, "F-2/F-9", call, expected, f"from std.cmath import {function}\n")
    expr("float-pow-negative-underflow", "F-2/F-9", "float(-1e-300) ** float(3)", "-0")
    return rows


REUSE = {
    "runtime": {
        "arithmetic_decimal_work_limits_and_cancellation_keep_inputs_unchanged": "6.1.2 W/W+1 trace, D, cancellation",
        "arithmetic_decimal_exact_division_keeps_failure_identities_and_order": "6.1.3 034 before work and zero ordering",
        "arithmetic_decimal_non_finite_precedes_operator_domain_after_inputs": "5.2 input, nonfinite, operator domain",
        "arithmetic_decimal_remainder_d_plus_one_boundary_is_checked_exactly": "6.1.7 D+1 actual remainder and sign",
        "arithmetic_decimal_dense_modulus_reaches_the_work_boundary": "6.1.7 mod_pow W trace without quotient",
        "arithmetic_decimal_finite_division_and_power_exact_limits": "6.1.3/6.1.5 finite factors, D/H",
        "arithmetic_integer_power_bounds_preserve_fast_paths": "6.1.5 huge valid unit exponents, domain and result limit",
        "accepts_exact_precision_bounds_and_all_rounding_modes": "6.1.4 p=1..D and seven modes",
        "rejects_precision_and_rounding_before_any_division_work": "5.2 precision precedes rounding and work",
        "division_by_zero_and_work_limits_are_stable": "6.1.4 divide zero and working length",
        "all_entries_reject_nonfinite_inputs_and_keep_owned_output_on_error": "F-2/F-5 NaN/+Inf/-Inf for all 15",
        "all_entries_reject_required_null_pointers_without_mutating_state": "F-5 ABI status 1",
        "all_entries_reject_unknown_tags_aliases_and_malformed_boolean": "F-5 ABI status 2",
        "all_entries_reject_missing_or_mismatched_handles_without_pending_changes": "F-5 ABI status 3",
        "all_entries_replace_owned_output_and_keep_pending_on_success": "F-5 ABI status 0, old out weak lifetime",
        "all_abi_results_match_formal_reference_errors_signs_and_ulp": "F-9 17,243 checked ABI vectors",
        "external_raw_text_keeps_error_order_and_ownership": "F-12 raw left/right admission",
        "structural_failures_do_not_touch_pending_or_output": "F-5 states 1/2/3/6 and divide allocation 011",
        "numeric_error_abi_preserves_identity_and_span": "F-12 errors 009/013 exact ABI source offsets",
        "platform_failure_report_is_machine_readable": "existing isolated Fatal hardware child",
    },
    "vm-regressions": {
        "controlled_allocation_failure_is_distinct_from_numeric_limit": "5.2 allocation 011 and unchanged borrowed/out",
        "mixed_error_priority_is_stable_at_the_language_entry": "5.2 type/admission/nonfinite/zero",
        "abi_semantic_failures_preserve_owned_out_and_inputs": "F-5 semantic state 6 and owned out",
        "successful_abi_replacement_releases_old_out_without_clearing_pending": "F-5 state 0 release and slot identity",
        "decoded_vm_calls_match_runtime_bits_and_errors_for_all_ids": "F-2 VM injected nonfinite and shared direct entries",
        "domain_errors_keep_decoded_span_and_observable_cleanup": "F-12 exact VM span and finally",
        "named_argument_expression_failures_keep_source_evaluation_order": "F-12 expression evaluation order",
        "vm_calls_leave_runtime_abi_pending_error_unchanged": "F-12 VM fault and ABI pending slot remain separate",
        "failing_cmath_calls_do_not_leak_heap_arguments_or_partial_results": "F-12 VM lifetime counters",
        "b6_numeric_and_cmath_errors_are_catchable_and_atomic_on_both_aliases": "B6-1 retained two alias finally regression",
        "a5_complete_vm_entry_covers_resources_and_portable_replay": "F-12 VM language offsets and failpoint 011",
    },
}


REGRESSION = {
    "b1-vm": ["20o_b1_probe", "b1_capabilities_have_vm_golden_outputs", False],
    "b1-native": ["20o_b1_probe", "b1_capabilities_match_native_golden_outputs", True],
    "cmath-native": ["20o_b2_b4_cmath_llvm", "native_cmath_static_dynamic_o0_o3_and_shared_pow", True],
    "cmath-wrappers": ["20o_b2_b4_cmath_library", "cmath_library_matches_native_o0_o3", True],
    "numeric-native": ["20o_b2_a_contract", "a5_language_values_match_native_golden_outputs", True],
    "divide-native": ["20o_b2_b2_divide", "lfloat_divide_matches_native_golden_outputs", True],
}


def catalog():
    return {
        "schema": 1, "platforms": PLATFORMS, "D": D, "W": W, "H": H,
        "language": [row for row, _ in cases()], "existing_assertions": REUSE,
        "regression_exact": REGRESSION,
        "b1_resource_tests": 18, "b1_contract_tests": 9, "guard_replays": [4, 24],
        "cmath_abi_entries": CMATH,
        "c_abi": {
            "divide-ok": [0, "0.5"], "divide-null": [1, "B6_SENTINEL"],
            "divide-label": [2, "B6_SENTINEL"], "divide-handle": [3, "B6_SENTINEL"],
            "divide-type": [6, "X06-RUNTIME-002"], "divide-parameter": [6, "X06-RUNTIME-012"],
            "divide-left-admission": [6, "X06-RUNTIME-033"], "divide-right-admission": [6, "X06-RUNTIME-012"],
            "divide-zero": [6, "X06-RUNTIME-013"], "divide-result-scale": [6, "X06-RUNTIME-033"],
            "fatal": [8, "X07-FATAL-006"],
        },
    }
