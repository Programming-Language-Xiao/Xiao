//! P1 表达式核心与索引选择器的规格测试。

use serde::Deserialize;
use xiao_source::SourceFile;
use xiao_syntax::{
    AssignmentOperator, BinaryOperator, Expression, Parser, PathSegment, RandomMode, SelectorItem,
    Statement, UnaryOperator,
};

/// 一条 P1 规格快照用例。
#[derive(Debug, Deserialize)]
struct SnapshotCase {
    /// 用例名称。
    name: String,
    /// 输入源码。
    source: String,
    /// 期望状态。
    expect: String,
    /// 期望诊断编号。
    diagnostics: Vec<String>,
}

/// 一份 P1 规格快照文件。
#[derive(Debug, Deserialize)]
struct SnapshotFile {
    /// 阶段标识。
    stage: String,
    /// 用例列表。
    cases: Vec<SnapshotCase>,
}

/// 验证 P1 JSON 快照中的源码和诊断契约。
fn assert_snapshot(raw: &str) {
    let snapshot: SnapshotFile = serde_json::from_str(raw).expect("P1 快照 JSON 必须有效");
    assert_eq!(snapshot.stage, "P1");
    for case in snapshot.cases {
        let result = Parser::new(&SourceFile::from_text(&case.source)).parse();
        let actual = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(actual, case.diagnostics, "snapshot case: {}", case.name);
        if case.expect == "success" {
            assert!(result.is_success(), "snapshot case: {}", case.name);
        } else {
            assert!(result.has_errors(), "snapshot case: {}", case.name);
        }
    }
}

/// 解析单条源码并返回其语句节点。
fn one_statement(source: &str) -> Statement {
    let result = Parser::new(&SourceFile::from_text(source)).parse();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    result
        .program
        .expect("parser should return a program")
        .statements
        .into_iter()
        .next()
        .expect("source should contain one statement")
}

#[test]
/// 验证 Pratt 优先级、幂运算右结合和一元表达式。
fn parses_expression_precedence() {
    let statement = one_statement("result = -a ** b + c * d and not ready\n");
    let Statement::Assignment { value, .. } = statement else {
        panic!("expected simple assignment");
    };
    let Expression::Binary {
        operator: BinaryOperator::And,
        left,
        right,
        ..
    } = value
    else {
        panic!("expected logical and at root");
    };
    assert!(matches!(
        *right,
        Expression::Unary {
            operator: UnaryOperator::Not,
            ..
        }
    ));
    let Expression::Binary {
        operator: BinaryOperator::Add,
        left: add_left,
        right: add_right,
        ..
    } = *left
    else {
        panic!("expected addition below and");
    };
    assert!(matches!(
        *add_right,
        Expression::Binary {
            operator: BinaryOperator::Multiply,
            ..
        }
    ));
    assert!(matches!(
        *add_left,
        Expression::Unary {
            operator: UnaryOperator::Minus,
            operand,
            ..
        } if matches!(*operand, Expression::Binary { operator: BinaryOperator::Power, .. })
    ));
}

#[test]
/// 验证调用、尾逗号、成员访问和标量 `as` 转换可以串联。
fn parses_calls_members_and_casts() {
    let statement = one_statement("value = factory(a + b,).item as bool\n");
    let Statement::Assignment { value, .. } = statement else {
        panic!("expected assignment");
    };
    let Expression::Cast {
        expression, target, ..
    } = value
    else {
        panic!("expected cast");
    };
    assert_eq!(target, xiao_syntax::ScalarType::Bool);
    let Expression::Member { object, .. } = *expression else {
        panic!("expected member access");
    };
    assert!(matches!(*object, Expression::Call { arguments, .. } if arguments.len() == 1));
}

#[test]
/// 验证八种标量类型都能作为 `as` 的语法目标，容器转换保留为普通调用。
fn parses_all_scalar_cast_targets() {
    let targets = [
        ("int", xiao_syntax::ScalarType::Int),
        ("sint", xiao_syntax::ScalarType::Sint),
        ("lint", xiao_syntax::ScalarType::Lint),
        ("float", xiao_syntax::ScalarType::Float),
        ("sfloat", xiao_syntax::ScalarType::Sfloat),
        ("lfloat", xiao_syntax::ScalarType::Lfloat),
        ("str", xiao_syntax::ScalarType::Str),
        ("bool", xiao_syntax::ScalarType::Bool),
    ];
    for (spelling, expected) in targets {
        let statement = one_statement(&format!("converted = value as {spelling}\n"));
        let Statement::Assignment { value, .. } = statement else {
            panic!("expected cast assignment");
        };
        assert!(matches!(value, Expression::Cast { target, .. } if target == expected));
    }
    let statement = one_statement("converted = tuple(value)\n");
    let Statement::Assignment { value, .. } = statement else {
        panic!("expected constructor-style conversion");
    };
    assert!(matches!(value, Expression::Call { .. }));
}

#[test]
/// 验证路径、多选、范围、边界、随机和步长节点均保留。
fn parses_selector_items_and_paths() {
    let statement = one_statement("picked{step}[0, 1~2, <3, >=4, =, ?count, !?again]\n");
    let Statement::Expression { expression, .. } = statement else {
        panic!("expected expression statement");
    };
    let Expression::Selector {
        source,
        step,
        selector,
        ..
    } = expression
    else {
        panic!("expected selector");
    };
    assert!(matches!(*source, Expression::Name(_)));
    assert!(matches!(step.as_deref(), Some(Expression::Name(_))));
    assert_eq!(selector.items.len(), 7);
    assert!(matches!(selector.items[0], SelectorItem::Exact { .. }));
    assert!(matches!(selector.items[1], SelectorItem::Range { .. }));
    assert!(matches!(
        selector.items[2],
        SelectorItem::OpenRange { end: Some(_), .. }
    ));
    assert!(matches!(
        selector.items[3],
        SelectorItem::OpenRange { start: Some(_), .. }
    ));
    assert!(matches!(selector.items[4], SelectorItem::All { .. }));
    assert!(matches!(
        selector.items[5],
        SelectorItem::Random {
            mode: RandomMode::WithoutReplacement,
            ..
        }
    ));
    assert!(matches!(
        selector.items[6],
        SelectorItem::Random {
            mode: RandomMode::WithReplacement,
            ..
        }
    ));

    let nested = one_statement("value[-1/`键`]\n");
    let Statement::Expression { expression, .. } = nested else {
        panic!("expected expression statement");
    };
    let Expression::Selector { selector, .. } = expression else {
        panic!("expected selector");
    };
    let SelectorItem::Exact { path, .. } = &selector.items[0] else {
        panic!("expected exact path");
    };
    assert_eq!(path.segments.len(), 2);
    assert!(matches!(
        path.segments[0],
        PathSegment::Integer { negative: true, .. }
    ));
    assert!(matches!(path.segments[1], PathSegment::Name(name) if name.backticked));
}

#[test]
/// 验证 P1 节点和每个选择项都保留精确的 UTF-8 字节区间及书写顺序。
fn preserves_expression_and_selector_spans() {
    let source_text = "picked{2}[0, 1~2, >=3]\n";
    let source = SourceFile::from_text(source_text);
    let result = Parser::new(&source).parse();
    assert!(
        result.is_success(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    let statement = result.program.expect("program").statements.remove(0);
    assert_eq!(source.slice(statement.span()), "picked{2}[0, 1~2, >=3]");
    let expression = statement.expression();
    let Expression::Selector { selector, step, .. } = expression else {
        panic!("expected selector");
    };
    assert_eq!(source.slice(selector.span()), "[0, 1~2, >=3]");
    assert_eq!(source.slice(step.as_ref().expect("step").span()), "2");
    assert_eq!(selector.items.len(), 3);
    assert_eq!(source.slice(selector.items[0].span()), "0");
    assert_eq!(source.slice(selector.items[1].span()), "1~2");
    assert_eq!(source.slice(selector.items[2].span()), ">=3");
}

#[test]
/// 验证选择器表达式可以作为复合赋值和普通赋值目标。
fn parses_extended_assignments() {
    let statement = one_statement("values[0] += delta\n");
    let Statement::ExtendedAssignment {
        target,
        operator,
        value,
        ..
    } = statement
    else {
        panic!("expected extended assignment");
    };
    assert_eq!(operator, AssignmentOperator::AddAssign);
    assert!(matches!(target, Expression::Selector { .. }));
    assert!(matches!(value, Expression::Name(_)));

    let member = one_statement("record.field = replacement\n");
    let Statement::ExtendedAssignment {
        target: Expression::Member { .. },
        operator: AssignmentOperator::Assign,
        ..
    } = member
    else {
        panic!("expected member assignment");
    };
}

#[test]
/// 验证全部冻结的复合赋值拼写都映射到对应的 AST 运算符。
fn parses_all_assignment_operators() {
    let cases = [
        ("+=", AssignmentOperator::AddAssign),
        ("-=", AssignmentOperator::SubtractAssign),
        ("*=", AssignmentOperator::MultiplyAssign),
        ("/=", AssignmentOperator::DivideAssign),
        ("//=", AssignmentOperator::FloorDivideAssign),
        ("%=", AssignmentOperator::RemainderAssign),
        ("**=", AssignmentOperator::PowerAssign),
    ];
    for (spelling, expected) in cases {
        let statement = one_statement(&format!("value {spelling} delta\n"));
        assert_eq!(statement.assignment_operator(), Some(expected));
        assert!(matches!(statement, Statement::ExtendedAssignment { .. }));
    }
}

#[test]
/// 验证空选择器、缺失端点和非法转换目标产生稳定诊断。
fn reports_selector_and_cast_errors() {
    let source = SourceFile::from_text("a[]\nb[1~]\nc as tuple\n");
    let result = Parser::new(&source).parse();
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"X01-PARSE-006"));
    assert!(codes.contains(&"X01-PARSE-008"));
}

#[test]
/// 验证缺少一元操作数时只保留最具体的诊断；空元组按 C0 的 Python 语义合法。
fn avoids_duplicate_prefix_diagnostics() {
    let result = Parser::new(&SourceFile::from_text("+\nvalue = -\n()\n")).parse();
    let codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code())
        .collect::<Vec<_>>();
    assert_eq!(codes, vec!["X01-PARSE-009", "X01-PARSE-009",]);
}

#[test]
/// 验证 `new`、`is not`、`not in` 和括号内软换行的结构解析。
fn parses_new_and_keyword_comparisons() {
    let source = SourceFile::from_text(
        "created = new Widget(\n    left,\n    right,\n)\ncheck = left is not none or item not in values\n",
    );
    let result = Parser::new(&source).parse();
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
    let statements = result.program.expect("program").statements;
    assert_eq!(statements.len(), 2);
    let Statement::Assignment { value, .. } = &statements[0] else {
        panic!("expected assignment");
    };
    assert!(matches!(value, Expression::NewCall { arguments, .. } if arguments.len() == 2));
    let Statement::Assignment { value, .. } = &statements[1] else {
        panic!("expected assignment");
    };
    assert!(matches!(
        value,
        Expression::Binary {
            operator: BinaryOperator::Or,
            ..
        }
    ));
    let Statement::Assignment { value, .. } = &statements[1] else {
        panic!("expected assignment");
    };
    let Expression::Binary { left, right, .. } = value else {
        panic!("expected disjunction");
    };
    assert!(matches!(
        **left,
        Expression::Binary {
            operator: BinaryOperator::IsNot,
            ..
        }
    ));
    assert!(matches!(
        **right,
        Expression::Binary {
            operator: BinaryOperator::NotIn,
            ..
        }
    ));

    let chained = one_statement("created = new Widget().field\n");
    let Statement::Assignment { value, .. } = chained else {
        panic!("expected chained new assignment");
    };
    assert!(
        matches!(value, Expression::Member { object, .. } if matches!(
            *object,
            Expression::NewCall { .. }
        ))
    );

    let qualified = one_statement("created = new net.Client(host)\n");
    let Statement::Assignment { value, .. } = qualified else {
        panic!("expected qualified new assignment");
    };
    assert!(
        matches!(value, Expression::NewCall { callee, .. } if matches!(
            *callee,
            Expression::Member { .. }
        ))
    );
}

#[test]
/// 验证选择器和步长中的任意表达式不会被误判为顶层逗号或除法。
fn parses_expression_operands_inside_selector() {
    let statement = one_statement("items{base + 1}[?count + 1, !?pick(2), -1~end]\n");
    let Statement::Expression { expression, .. } = statement else {
        panic!("expected expression statement");
    };
    let Expression::Selector { step, selector, .. } = expression else {
        panic!("expected selector");
    };
    assert!(matches!(
        step.as_deref(),
        Some(Expression::Binary {
            operator: BinaryOperator::Add,
            ..
        })
    ));
    assert!(matches!(
        &selector.items[0],
        SelectorItem::Random {
            count,
            mode: RandomMode::WithoutReplacement,
            ..
        } if matches!(**count, Expression::Binary { operator: BinaryOperator::Add, .. })
    ));
    assert!(matches!(
        &selector.items[1],
        SelectorItem::Random {
            count,
            mode: RandomMode::WithReplacement,
            ..
        } if matches!(**count, Expression::Call { .. })
    ));
}

#[test]
/// 验证 P1 有效源码快照与解析器诊断契约保持一致。
fn matches_valid_spec_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/03-expression/p1-valid.json"
    )));
}

#[test]
/// 验证 P1 非法源码快照中的错误编号保持稳定且顺序明确。
fn matches_error_spec_snapshot() {
    assert_snapshot(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../../tests/spec/03-expression/p1-errors.json"
    )));
}

#[test]
/// 验证常见分隔符组合的错误恢复不会让解析器崩溃或停滞。
fn recovers_from_malformed_delimiter_corpus() {
    let corpus = [
        "(",
        ")",
        "[",
        "]",
        "{",
        "}",
        "a(",
        "a[",
        "a{",
        "a[1~",
        "a[1/",
        "a[?",
        "a[!?",
        "a[<",
        "a[<=",
        "a[>",
        "a[>=",
        "a[,,]",
        "a[0,,1]",
        "a(,)",
        "a(,,)",
        "new",
        "new Widget",
        "a as",
        "a as tuple",
        "a..b",
        "a +",
        "not",
    ];
    for source in corpus {
        let result =
            std::panic::catch_unwind(|| Parser::new(&SourceFile::from_text(source)).parse());
        assert!(
            result.is_ok(),
            "parser panicked for malformed source: {source:?}"
        );
    }
}

#[test]
/// 验证未闭合选择器在换行处恢复时不会吞掉下一条顶层语句。
fn recovers_selector_at_statement_boundary() {
    let source = SourceFile::from_text("bad[1~\nnext = 2\n");
    let result = Parser::new(&source).parse();
    assert!(result.has_errors());
    let program = result.program.as_ref().expect("program");
    assert!(program
        .statements
        .iter()
        .any(|statement| matches!(statement, Statement::Assignment { target, .. } if target.text(&source) == "next")));

    let multiline_close = SourceFile::from_text("bad[1~\n]\nnext = 3\n");
    let result = Parser::new(&multiline_close).parse();
    let program = result.program.expect("program");
    assert!(program
        .statements
        .iter()
        .any(|statement| matches!(statement, Statement::Assignment { target, .. } if target.text(&multiline_close) == "next")));

    let missing_close = SourceFile::from_text("bad[0\nnext = 4\n");
    let result = Parser::new(&missing_close).parse();
    let program = result.program.expect("program");
    assert!(program
        .statements
        .iter()
        .any(|statement| matches!(statement, Statement::Assignment { target, .. } if target.text(&missing_close) == "next")));
}

#[test]
/// 验证随机生成的标点组合也能有限步恢复，避免新增 Pratt 分支引入死循环。
fn fuzzes_parser_progress_on_small_inputs() {
    let alphabet = b"a01[]{}(),~?!=+-*/<>.";
    let mut state = 0xC0DE_1234_u64;
    for _ in 0..2_000 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let length = (state as usize % 24) + 1;
        let mut source = String::with_capacity(length);
        for _ in 0..length {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            source.push(alphabet[state as usize % alphabet.len()] as char);
        }
        let result =
            std::panic::catch_unwind(|| Parser::new(&SourceFile::from_text(&source)).parse());
        assert!(
            result.is_ok(),
            "parser panicked for generated source: {source:?}"
        );
    }
}
