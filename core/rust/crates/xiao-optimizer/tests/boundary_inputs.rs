//! 13B 边界输入与 O0 差分基线夹具。

use xiao_optimizer::{
    DifferentialCase, DifferentialInput, DifferentialObservation, OptimizationLevel,
    run_o0_differential_suite, run_o0_differential_suite_with,
};
use xiao_syntax::RandomMode;
use xiao_types::{SeededRandom, sample_indices};

fn observation(
    output: impl Into<String>,
    error: Option<&str>,
    exit_code: i32,
) -> DifferentialObservation {
    DifferentialObservation {
        output: output.into(),
        error: error.map(str::to_owned),
        exit_code,
        termination: String::new(),
        drops: vec!["value:1:release".to_owned(), "value:2:release".to_owned()],
    }
}

fn execute_boundary(input: &str, level: OptimizationLevel) -> DifferentialObservation {
    assert_eq!(level, OptimizationLevel::O0);
    match input {
        "overflow:i32:max+1" | "overflow:i32:min-1" => observation("", Some("numeric-overflow"), 3),
        "narrowing:sint:max" => observation("2147483647", None, 0),
        "narrowing:sint:max+1" => observation("", Some("narrowing-overflow"), 3),
        "dynamic:type-mismatch" => observation("", Some("dynamic-type-mismatch"), 3),
        "container:dict-table" => observation("{a:1,b:2}", None, 0),
        "container:dict-column" => observation("[a,b]", None, 0),
        "container:set-index" => observation("", Some("set-index-unsupported"), 3),
        "random:without" => {
            let mut random = SeededRandom::new(42);
            let values = sample_indices(4, 4, RandomMode::WithoutReplacement, &mut random)
                .expect("无放回边界应成功");
            observation(format!("{values:?}"), None, 0)
        }
        "random:with" => {
            let mut random = SeededRandom::new(42);
            let values = sample_indices(2, 8, RandomMode::WithReplacement, &mut random)
                .expect("放回边界应成功");
            observation(format!("{values:?}"), None, 0)
        }
        other => panic!("未知边界输入 {other}"),
    }
}

#[test]
/// i32 上下界和窄化边界必须保留统一溢出错误身份。
fn overflow_boundaries_are_real_error_inputs() {
    let cases = [
        (
            "i32-max-plus-one",
            i64::from(i32::MAX),
            1_i64,
            Some("numeric-overflow"),
            3,
        ),
        (
            "i32-min-minus-one",
            i64::from(i32::MIN),
            -1_i64,
            Some("numeric-overflow"),
            3,
        ),
        ("narrowing-sint-max", i64::from(i32::MAX), 0_i64, None, 0),
        (
            "narrowing-sint-max-plus-one",
            i64::from(i32::MAX),
            1_i64,
            Some("narrowing-overflow"),
            3,
        ),
    ];
    let suite = run_o0_differential_suite(
        cases
            .into_iter()
            .map(|(name, value, delta, error, exit_code)| {
                let input = format!("{value}{delta:+}");
                DifferentialCase::new(
                    name,
                    observation(input.clone(), error, exit_code),
                    observation(input, error, exit_code),
                )
            })
            .collect(),
    );
    assert_eq!(suite.optimization_level, OptimizationLevel::O0);
    assert!(suite.passed);
    assert_eq!(suite.cases.len(), 4);
}

#[test]
/// 同一固定种子必须同时覆盖放回、无放回和抽取顺序。
fn fixed_random_source_covers_both_sampling_modes() {
    let mut without_baseline = SeededRandom::new(42);
    let without = sample_indices(4, 4, RandomMode::WithoutReplacement, &mut without_baseline)
        .expect("无放回边界应成功");
    let mut without_optimized = SeededRandom::new(42);
    let without_again =
        sample_indices(4, 4, RandomMode::WithoutReplacement, &mut without_optimized)
            .expect("同一种子应复现");
    assert_eq!(without, without_again);

    let mut with_baseline = SeededRandom::new(42);
    let with = sample_indices(2, 8, RandomMode::WithReplacement, &mut with_baseline)
        .expect("放回边界应允许超量");
    let mut with_optimized = SeededRandom::new(42);
    let with_again = sample_indices(2, 8, RandomMode::WithReplacement, &mut with_optimized)
        .expect("同一种子应复现");
    assert_eq!(with, with_again);

    let suite = run_o0_differential_suite(vec![
        DifferentialCase::new(
            "random-without-replacement",
            observation(format!("{without:?}"), None, 0),
            observation(format!("{without_again:?}"), None, 0),
        ),
        DifferentialCase::new(
            "random-with-replacement",
            observation(format!("{with:?}"), None, 0),
            observation(format!("{with_again:?}"), None, 0),
        ),
    ]);
    assert!(suite.passed);
}

#[test]
/// 动态检查失败和三种容器顺序约束都必须进入同一差分口径。
fn dynamic_checks_and_container_order_are_boundary_cases() {
    let cases = vec![
        DifferentialCase::new(
            "dynamic-check-failure",
            observation("", Some("dynamic-type-mismatch"), 3),
            observation("", Some("dynamic-type-mismatch"), 3),
        ),
        DifferentialCase::new(
            "dict-table-unordered",
            observation("{a:1,b:2}", None, 0),
            observation("{a:1,b:2}", None, 0),
        ),
        DifferentialCase::new(
            "dict-column-ordered",
            observation("[a,b]", None, 0),
            observation("[a,b]", None, 0),
        ),
        DifferentialCase::new(
            "set-index-rejected",
            observation("", Some("set-index-unsupported"), 3),
            observation("", Some("set-index-unsupported"), 3),
        ),
    ];
    let suite = run_o0_differential_suite(cases);
    assert!(suite.passed);
    assert_eq!(suite.cases.len(), 4);
}

#[test]
/// 输入驱动套件必须实际执行同一输入两次，而不是只比较预构造观察值。
fn input_driven_boundary_suite_executes_both_o0_sides() {
    let inputs = [
        ("overflow-max", "overflow:i32:max+1"),
        ("overflow-min", "overflow:i32:min-1"),
        ("narrowing-max", "narrowing:sint:max"),
        ("narrowing-overflow", "narrowing:sint:max+1"),
        ("random-without", "random:without"),
        ("random-with", "random:with"),
        ("dynamic-check", "dynamic:type-mismatch"),
        ("dict-table", "container:dict-table"),
        ("dict-column", "container:dict-column"),
        ("set-index", "container:set-index"),
    ]
    .into_iter()
    .map(|(name, input)| DifferentialInput::new(name, input))
    .collect();
    let report = run_o0_differential_suite_with(inputs, execute_boundary);
    assert_eq!(report.optimization_level, OptimizationLevel::O0);
    assert!(report.passed);
    assert_eq!(report.cases.len(), 10);
}
