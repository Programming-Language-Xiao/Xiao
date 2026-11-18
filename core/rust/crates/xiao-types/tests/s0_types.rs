//! S0 静态标量类型语义的规格测试。

use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::{
    ARITHMETIC_ERROR_CODE, ASSIGNMENT_TYPE_MISMATCH_CODE, INVALID_CONVERSION_CODE,
    RuntimeCheckKind, Type, TypeChecker,
};
use xiao_types::{
    DUPLICATE_DECLARATION_CODE, INVALID_OPERANDS_CODE, NON_CONSTANT_CODE, UNINITIALIZED_READ_CODE,
};

/// 解析并检查一段 Xiao 源码。
fn check(source_text: &str) -> xiao_types::TypeCheckResult {
    let source = SourceFile::from_text(source_text);
    let parsed = Parser::new(&source).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    TypeChecker::check(&source, parsed.program.as_ref().expect("program"))
}

#[test]
/// 验证推断绑定、显式声明和同类型重赋值。
fn infers_and_locks_scalar_bindings() {
    let result = check("a = 1\na = 2\nint b = a\nstr text\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert_eq!(
        result.binding("ascii:a").expect("a").scheme.ty,
        Type::scalar(xiao_types::ScalarType::Int)
    );
    assert!(!result.binding("ascii:text").expect("text").initialized);
}

#[test]
/// 验证不兼容重赋值、读取未初始化和重复声明会累积诊断。
fn reports_binding_errors() {
    let result = check("int a = 1\na = \"bad\"\nstr b\nb\nint a = 2\n");
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&ASSIGNMENT_TYPE_MISMATCH_CODE));
    assert!(codes.contains(&UNINITIALIZED_READ_CODE));
    assert!(codes.contains(&DUPLICATE_DECLARATION_CODE));
}

#[test]
/// 验证布尔奇偶加减、数值提升和除法结果类型。
fn checks_boolean_and_numeric_operations() {
    let result = check("flag = true\ncount = 3\nnext = flag + count\nratio = count / 2\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert_eq!(
        result.binding("ascii:next").expect("next").scheme.ty,
        Type::scalar(xiao_types::ScalarType::Bool)
    );
    assert_eq!(
        result.binding("ascii:ratio").expect("ratio").scheme.ty,
        Type::scalar(xiao_types::ScalarType::Float)
    );
}

#[test]
/// 验证反向布尔运算和静态除零被拒绝。
fn rejects_invalid_boolean_and_arithmetic() {
    let result = check("a = 1 + true\nb = true + 1 / 0\n");
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&ARITHMETIC_ERROR_CODE));
    assert!(codes.contains(&INVALID_OPERANDS_CODE));
}

#[test]
/// 验证 `as` 与构造式共享 str/bool 转换矩阵和运行时标记。
fn shares_conversion_matrix() {
    let result = check(
        "str raw = input(\"value\")\nbool first = raw as bool\nbool second = bool(raw)\nstr rendered = first as str\n",
    );
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert!(
        result
            .runtime_checks
            .iter()
            .any(|check| check.kind == RuntimeCheckKind::StringBoolean)
    );
    assert_eq!(
        result
            .binding("ascii:rendered")
            .expect("rendered")
            .scheme
            .ty,
        Type::scalar(xiao_types::ScalarType::Str)
    );
}

#[test]
/// 验证非法字符串到布尔转换和隐式窄化不会静默通过。
fn rejects_invalid_conversion_and_narrowing() {
    let result = check("bool bad = \"maybe\"\nsint narrow = 9999999999\n");
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&INVALID_CONVERSION_CODE));
    assert!(codes.contains(&ARITHMETIC_ERROR_CODE));
    assert_eq!(
        codes
            .iter()
            .filter(|code| **code == INVALID_CONVERSION_CODE)
            .count(),
        1,
        "静态非法 bool 拼写只应产生一个转换诊断"
    );
}

#[test]
/// 验证 const 只接受编译期可求值表达式并保留常量绑定。
fn checks_compile_time_constants() {
    let result = check("const answer = 1 + 2\nconst invalid = input(\"x\")\n");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == NON_CONSTANT_CODE)
    );
    assert!(result.binding("ascii:answer").expect("answer").constant);
}

#[test]
/// 验证常量字面量可以安全落入较窄整数槽，而浮点到整数仍被拒绝。
fn checks_literal_widths() {
    let fitting = check("sint small = 1\nsfloat fraction = 1.5\n");
    assert!(
        fitting.is_success(),
        "diagnostics: {:?}",
        fitting.diagnostics
    );
    let narrowing = check("int whole = 1.5\n");
    assert!(!narrowing.is_success());
    assert!(
        narrowing
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == ARITHMETIC_ERROR_CODE)
    );
}

#[test]
/// 验证超出 i128 的十进制字面量仍可作为 lint 保存。
fn accepts_unbounded_integer_literal() {
    let result = check("lint huge = 12345678901234567890123456789012345678901234567890\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert_eq!(
        result.binding("ascii:huge").expect("huge").scheme.ty,
        Type::scalar(xiao_types::ScalarType::Lint)
    );
}

#[test]
/// 验证常量字符串的非法布尔拼写在编译期直接报转换错误。
fn rejects_invalid_static_boolean_conversion() {
    let result = check("bad = bool(\"maybe\")\n");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == INVALID_CONVERSION_CODE)
    );
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == NON_CONSTANT_CODE)
    );
}

#[test]
/// 验证显式浮点到整数转换按向零截断，并为动态值保留范围检查。
fn supports_explicit_float_to_integer_truncation() {
    let result = check(
        "from_literal = 1.9 as int\nfrom_constructor = int(1.5)\nvalue = 2.5\nconverted = value as sint\n",
    );
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert_eq!(
        result
            .binding("from_literal")
            .expect("from_literal")
            .scheme
            .ty,
        Type::scalar(xiao_types::ScalarType::Int)
    );
    assert_eq!(
        result
            .binding("from_constructor")
            .expect("from_constructor")
            .scheme
            .ty,
        Type::scalar(xiao_types::ScalarType::Int)
    );
    assert!(
        result
            .runtime_checks
            .iter()
            .any(|check| check.kind == RuntimeCheckKind::NumericRange)
    );
}

#[test]
/// 验证隐式窄化不会注入可执行的运行时检查。
fn rejects_implicit_narrowing_without_runtime_marker() {
    let result = check("value = 1\nsmall = value as sint\nsint invalid = value\n");
    assert!(!result.is_success());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == ASSIGNMENT_TYPE_MISMATCH_CODE)
    );
    assert_eq!(
        result
            .runtime_checks
            .iter()
            .filter(|check| check.kind == RuntimeCheckKind::NumericRange)
            .count(),
        1,
        "只有显式 as 才应产生范围检查"
    );
}

#[test]
/// 验证常量比较、混合数值折叠和默认 64 位整数溢出诊断。
fn folds_constants_and_reports_result_width_overflow() {
    let result = check(
        "const mixed = 1 + 2.5\nconst comparison = 1 < 2\nconst overflow = 9223372036854775807 + 1\n",
    );
    let overflow_codes = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code() == ARITHMETIC_ERROR_CODE)
        .count();
    assert_eq!(overflow_codes, 1, "diagnostics: {:?}", result.diagnostics);
    assert_eq!(
        result.binding("mixed").expect("mixed").scheme.ty,
        Type::scalar(xiao_types::ScalarType::Float)
    );
    assert_eq!(
        result.binding("comparison").expect("comparison").scheme.ty,
        Type::scalar(xiao_types::ScalarType::Bool)
    );
}

#[test]
/// 验证显式常量类型会锁定声明类型，而不是沿用初始化字面量类型。
fn preserves_explicit_const_type() {
    let result = check("const sint small = 1\nconst sfloat fraction = 1.5\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert_eq!(
        result.binding("small").expect("small").scheme.ty,
        Type::scalar(xiao_types::ScalarType::Sint)
    );
    assert_eq!(
        result.binding("fraction").expect("fraction").scheme.ty,
        Type::scalar(xiao_types::ScalarType::Sfloat)
    );
}

#[test]
/// 验证 `bool("maybe")` 只报告转换错误，且 `bool` 反引号名称不被当作内建函数。
fn distinguishes_backtick_name_from_boolean_constructor() {
    let result = check("`bool` = \"custom\"\nvalue = `bool`\nbad = bool(\"maybe\")\n");
    let invalid_conversions = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code() == INVALID_CONVERSION_CODE)
        .count();
    assert_eq!(
        invalid_conversions, 1,
        "diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(
        result
            .binding("backtick:bool")
            .expect("反引号名称")
            .scheme
            .ty,
        Type::scalar(xiao_types::ScalarType::Str)
    );
    assert_eq!(
        result.binding("value").expect("value").scheme.ty,
        Type::scalar(xiao_types::ScalarType::Str)
    );
}
