//! 04-A 函数与控制流语法规格测试。

use xiao_source::SourceFile;
use xiao_syntax::{CallArgumentKind, EntryMode, FunctionParameterKind, Parser, Statement};
use xiao_syntax::{
    INVALID_PARAMETER_CODE, MISSING_BLOCK_CODE, MISSING_ERROR_HANDLER_CODE,
    MISSING_RAISE_VALUE_CODE,
};

/// 解析一个应当无错误的 04 阶段源码样例。
fn parse_ok(source: &str) -> xiao_syntax::Program {
    let result = Parser::new(&SourceFile::from_text(source)).parse();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    result.program.expect("program")
}

#[test]
/// 完整参数集应保留参数种类、默认值和返回类型注解。
fn parses_complete_function_signature() {
    let program = parse_ok(
        "def f(int a, str b=\"x\", /, c=1, *args, bool flag=false, **kwargs) -> int\n    return a\n",
    );
    let Statement::Function {
        parameters,
        return_type,
        body,
        ..
    } = &program.statements[0]
    else {
        panic!("expected function");
    };
    assert_eq!(parameters.len(), 6);
    assert_eq!(parameters[0].kind, FunctionParameterKind::PositionalOnly);
    assert_eq!(parameters[1].kind, FunctionParameterKind::PositionalOnly);
    assert_eq!(
        parameters[2].kind,
        FunctionParameterKind::PositionalOrKeyword
    );
    assert_eq!(parameters[3].kind, FunctionParameterKind::VarArgs);
    assert_eq!(parameters[4].kind, FunctionParameterKind::KeywordOnly);
    assert_eq!(parameters[5].kind, FunctionParameterKind::VarKeywords);
    assert!(parameters[1].default.is_some());
    assert_eq!(
        return_type,
        &Some(xiao_syntax::FunctionTypeAnnotation::Scalar(
            xiao_syntax::ScalarType::Int
        ))
    );
    assert!(matches!(body[0], Statement::Return { .. }));
}

#[test]
/// 调用参数应区分位置、关键字和两种展开形式。
fn parses_named_and_expanded_call_arguments() {
    let program = parse_ok("result = f(1, b=2, *items, **options)\n");
    let Some(xiao_syntax::Expression::Call { arguments, .. }) =
        program.statements[0].try_expression()
    else {
        panic!("expected call");
    };
    assert_eq!(arguments.len(), 4);
    assert_eq!(arguments[0].kind, CallArgumentKind::Positional);
    assert_eq!(arguments[1].kind, CallArgumentKind::Keyword);
    assert_eq!(arguments[2].kind, CallArgumentKind::Star);
    assert_eq!(arguments[3].kind, CallArgumentKind::DoubleStar);
    assert!(arguments[1].name.is_some());
}

#[test]
/// 条件、循环和嵌套缩进体应形成递归语句 AST。
fn parses_control_flow_blocks() {
    let program = parse_ok(
        "if ready\n    for item in items\n        if item\n            continue\n        else\n            break\n    elif fallback\n        while ready\n            ready = false\n    else\n        print(\"none\")\n",
    );
    let Statement::If {
        body,
        elif_branches,
        else_body,
        ..
    } = &program.statements[0]
    else {
        panic!("expected if");
    };
    assert_eq!(elif_branches.len(), 1);
    assert!(else_body.is_some());
    assert!(matches!(body[0], Statement::For { .. }));
}

#[test]
/// `[main]` 只作为顶层入口元数据，普通脚本保持脚本模式。
fn records_entry_mode() {
    assert!(parse_ok("print(1)\n").entry_mode.is_script());
    assert!(matches!(
        parse_ok("[main]\nprint(1)\n").entry_mode,
        EntryMode::Project { .. }
    ));
}

#[test]
/// 缺少缩进体和非法条件头应给出 04 阶段稳定诊断。
fn diagnoses_missing_block_and_control_tail() {
    let result = Parser::new(&SourceFile::from_text("if 1\nnext = 2\n")).parse();
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&MISSING_BLOCK_CODE));
}

#[test]
/// 嵌套条件链不能吞掉外层代码块的反缩进。
fn preserves_nested_if_dedents() {
    let program = parse_ok(
        "def f(bool ready)\n    if ready\n        if ready\n            return 1\n        else\n            return 2\n    else\n        return 3\nvalue = 4\n",
    );
    assert_eq!(program.statements.len(), 2);
    let Statement::Function { body, .. } = &program.statements[0] else {
        panic!("expected function");
    };
    assert!(matches!(body[0], Statement::If { .. }));
}

#[test]
/// 同一名称空间中的重复参数应在语法层给出稳定诊断。
fn diagnoses_duplicate_parameters() {
    let result = Parser::new(&SourceFile::from_text(
        "def f(int value, str value)\n    return value\n",
    ))
    .parse();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == INVALID_PARAMETER_CODE)
    );
}

#[test]
/// 反引号函数名称、默认参数和尾部关键字参数应保留可恢复结构。
fn parses_backticked_function_name_and_keyword_only_marker() {
    let program = parse_ok("def `处理`(int value, *, str label=\"ok\") -> int\n    return value\n");
    let Statement::Function {
        name, parameters, ..
    } = &program.statements[0]
    else {
        panic!("expected function");
    };
    assert!(name.backticked);
    assert_eq!(
        parameters[0].kind,
        FunctionParameterKind::PositionalOrKeyword
    );
    assert_eq!(parameters[1].kind, FunctionParameterKind::KeywordOnly);
}

#[test]
/// 额外缩进层结束后，外层函数和后续顶层语句都应保留。
fn preserves_function_dedent_before_following_statement() {
    let program = parse_ok(
        "def f(bool ready) -> int\n    if ready\n        return 1\n    else\n        return 2\nvalue = 3\n",
    );
    assert_eq!(program.statements.len(), 2);
    assert!(matches!(program.statements[0], Statement::Function { .. }));
    assert!(matches!(
        program.statements[1],
        Statement::Assignment { .. }
    ));
}

#[test]
/// 位置专用标记没有逗号分隔时必须拒绝，而不能改变前一个参数的种类。
fn rejects_unseparated_positional_marker() {
    let result = Parser::new(&SourceFile::from_text(
        "def f(int value /, int other)\n    return value\n",
    ))
    .parse();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == INVALID_PARAMETER_CODE)
    );
}

#[test]
/// `try`、多个 `catch`、`finally` 和 `raise` 应保留完整递归结构。
fn parses_error_control_flow() {
    let source = SourceFile::from_text(
        "try\n    raise error\ncatch err as ArithmeticError\n    print(err)\ncatch other as Error\n    print(other)\nfinally\n    cleanup = true\n",
    );
    let result = Parser::new(&source).parse();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    let program = result.program.expect("program");
    let Statement::Try {
        body,
        catches,
        finally_body,
        ..
    } = &program.statements[0]
    else {
        panic!("expected try");
    };
    assert!(matches!(body[0], Statement::Raise { .. }));
    assert_eq!(catches.len(), 2);
    assert_eq!(catches[0].error_type.text(&source), "ArithmeticError");
    assert!(finally_body.is_some());
}

#[test]
/// 错误控制流语句继续接收与其他语句一致的文档注释区间。
fn attaches_docs_to_error_control_flow() {
    let program = parse_ok("### 处理错误 ###\ntry\n    raise error\nfinally\n    cleanup = true\n");
    let Statement::Try { leading_docs, .. } = &program.statements[0] else {
        panic!("expected try");
    };
    assert_eq!(leading_docs.len(), 1);
}

#[test]
/// `try` 缺少处理器和 `raise` 缺少表达式必须给出稳定诊断。
fn diagnoses_invalid_error_control_flow() {
    let result = Parser::new(&SourceFile::from_text("try\n    value = 1\nraise\n")).parse();
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&MISSING_ERROR_HANDLER_CODE));
    assert!(codes.contains(&MISSING_RAISE_VALUE_CODE));
}

#[test]
/// 嵌套循环和错误控制流收尾产生的多个顶层 Dedent 不应误报独立反缩进。
fn parses_nested_try_and_loop_dedents() {
    let program = parse_ok(
        "try\n    while true\n        try\n            break\n        finally\n            inner = \"done\"\n    finally\n        outer = \"done\"\n",
    );
    assert!(matches!(
        program.statements.first(),
        Some(Statement::Try { .. })
    ));
}
