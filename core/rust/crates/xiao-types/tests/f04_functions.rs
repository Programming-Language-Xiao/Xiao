//! 04-B/04-C 函数与控制流静态类型规格测试。

use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::{
    CATCH_FATAL_CODE, CATCH_ORDER_CODE, CONDITION_TYPE_CODE, FUNCTION_CALL_CODE,
    FUNCTION_INFERENCE_CODE, FUNCTION_RETURN_CODE, LOOP_CONTROL_CODE, RAISE_TYPE_CODE,
    RuntimeCheckKind, ScalarType, Type, TypeChecker,
};

/// 解析并静态检查一个应当可用于 04 阶段测试的源码样例。
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
/// 注解函数应登记签名并允许位置/关键字调用。
fn checks_annotated_function_call() {
    let result = check(
        "def add(int left, int right=1) -> int\n    return left + right\nvalue = add(2, right=3)\n",
    );
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let signature = result
        .function_signatures()
        .get("ascii:add")
        .expect("function signature");
    assert_eq!(signature.return_type, Type::scalar(ScalarType::Int));
    assert_eq!(signature.parameters.len(), 2);
    assert_eq!(
        result.binding("ascii:value").expect("value").scheme.ty,
        Type::scalar(ScalarType::Int)
    );
}

#[test]
/// 无注解参数可以通过函数体和调用点统一，并支持前向引用。
fn infers_recursive_and_forward_function_types() {
    let result = check("value = twice(2)\ndef twice(x) -> int\n    return x + x\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert_eq!(
        result
            .function_signatures()
            .get("ascii:twice")
            .expect("twice")
            .return_type,
        Type::scalar(ScalarType::Int)
    );
}

#[test]
/// 默认参数、关键字约束和返回类型冲突应使用稳定函数诊断。
fn diagnoses_function_call_and_return_errors() {
    let result = check("def f(int value) -> int\n    return true\nf()\nf(value=1, other=2)\n");
    let codes = result
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&FUNCTION_RETURN_CODE));
    assert!(codes.contains(&FUNCTION_CALL_CODE));
}

#[test]
/// 条件必须为 bool，动态条件则留下运行时布尔检查。
fn checks_boolean_conditions() {
    let invalid = check("if 1\n    value = 1\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == CONDITION_TYPE_CODE)
    );
    let dynamic = check("if value\n    value = 1\n");
    assert!(
        dynamic
            .runtime_checks()
            .iter()
            .any(|check| check.kind == RuntimeCheckKind::BooleanCondition)
    );
}

#[test]
/// 循环只接受可迭代容器，break/continue 不能越出循环。
fn checks_loops_and_control_statements() {
    let result =
        check("for item in [1, 2]\n    continue\nwhile true\n    break\nbreak\ncontinue\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == LOOP_CONTROL_CODE)
    );
    let invalid = check("for item in 1\n    break\n");
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == xiao_types::ITERABLE_TYPE_CODE)
    );
}

#[test]
/// 无法约束的参数和返回变量必须报告推断失败，而不是静默动态化。
fn rejects_unresolved_function_inference() {
    let result = check("def identity(value)\n    return value\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == FUNCTION_INFERENCE_CODE)
    );
}

#[test]
/// 定义先于调用时，函数收尾阶段应等待后续调用提供参数约束。
fn infers_function_types_from_later_calls() {
    let result = check("def twice(x)\n    return x + x\nvalue = twice(2)\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    let signature = result
        .function_signatures()
        .get("ascii:twice")
        .expect("twice");
    assert_eq!(signature.parameters[0].ty, Type::scalar(ScalarType::Int));
    assert_eq!(signature.return_type, Type::scalar(ScalarType::Int));
}

#[test]
/// 异构有序容器仍可迭代，循环变量在本阶段退化为动态边界。
fn accepts_heterogeneous_iterables() {
    let result = check("for item in [1, \"x\"]\n    print(item)\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
}

#[test]
/// 反引号函数名称应使用与定义相同的规范化键完成调用匹配。
fn checks_backticked_function_call() {
    let result = check("def `处理`(int value) -> int\n    return value\nanswer = `处理`(1)\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert!(result.function_signatures().contains_key("backtick:处理"));
    assert_eq!(
        result.binding("ascii:answer").expect("answer").scheme.ty,
        Type::scalar(ScalarType::Int)
    );
}

#[test]
/// 直接位置实参应统一到 `*args` 的元素类型，而不是数组外壳类型。
fn checks_direct_varargs_arguments() {
    let result = check("def collect(*items) -> none\n    return\ncollect(1, 2)\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
}

#[test]
/// 函数外的返回和循环控制语句必须分别给出稳定诊断。
fn rejects_control_statements_outside_owner() {
    let result = check("return 1\nbreak\ncontinue\n");
    let codes = result
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&FUNCTION_RETURN_CODE));
    assert!(
        codes
            .iter()
            .filter(|code| **code == LOOP_CONTROL_CODE)
            .count()
            >= 2
    );
}

#[test]
/// 参数名称遮蔽函数名称时，函数定义回写仍应更新外层函数绑定。
fn preserves_function_binding_when_parameter_has_same_name() {
    let result = check("def f(int f) -> int\n    return f\nanswer = f(1)\n");
    assert!(result.is_success(), "diagnostics: {:?}", result.diagnostics);
    assert_eq!(
        result.binding("ascii:answer").expect("answer").scheme.ty,
        Type::scalar(ScalarType::Int)
    );
}

#[test]
/// `raise` 的静态值必须是可恢复错误边界，普通标量应被拒绝。
fn rejects_non_error_raise_value() {
    let result = check("raise 1\n");
    assert!(
        result
            .diagnostics()
            .iter()
            .any(|d| d.code() == RAISE_TYPE_CODE)
    );
}

#[test]
/// `FatalError` 不得被普通 `catch` 捕获，宽泛处理器必须放在具体类型之后。
fn enforces_catch_recovery_boundaries() {
    let fatal = check("try\n    raise error\ncatch fatal as FatalError\n    print(fatal)\n");
    assert!(
        fatal
            .diagnostics()
            .iter()
            .any(|d| d.code() == CATCH_FATAL_CODE)
    );

    let order = check(
        "try\n    raise error\ncatch any as Error\n    print(any)\ncatch specific as ArithmeticError\n    print(specific)\n",
    );
    assert!(
        order
            .diagnostics()
            .iter()
            .any(|d| d.code() == CATCH_ORDER_CODE)
    );
}
