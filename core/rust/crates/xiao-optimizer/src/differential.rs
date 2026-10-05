//! 未优化与优化结果的统一差分口径。

use serde::{Deserialize, Serialize};

use crate::OptimizationLevel;

/// 一次后端执行的可观察结果。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DifferentialObservation {
    /// 标准输出。
    pub output: String,
    /// 错误摘要；没有错误时为空。
    pub error: Option<String>,
    /// 进程或 VM 退出码。
    pub exit_code: i32,
    /// 按冻结顺序记录的释放事件。
    pub drops: Vec<String>,
}

/// 一项稳定的语义差分。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DifferentialDifference {
    /// 差异字段名：`output`、`error`、`exit_code` 或 `drops`。
    pub field: String,
    /// 未优化基线值。
    pub baseline: String,
    /// 优化结果值。
    pub optimized: String,
}

/// 多路差分中以第一路为基线的一项点名差异。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NamedDifferentialDifference {
    /// 基线一侧名称。
    pub baseline_side: String,
    /// 偏离一侧名称。
    pub compared_side: String,
    /// 差异字段名。
    pub field: String,
    /// 基线字段值。
    pub baseline: String,
    /// 偏离字段值。
    pub compared: String,
}

/// 一个未优化/优化结果对照用例。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DifferentialCase {
    /// 稳定用例名称。
    pub name: String,
    /// 未优化基线观察值。
    pub baseline: DifferentialObservation,
    /// 优化侧观察值；当前 13B 使用 O0 占位管线。
    pub optimized: DifferentialObservation,
}

impl DifferentialCase {
    /// 创建一个差分用例。
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        baseline: DifferentialObservation,
        optimized: DifferentialObservation,
    ) -> Self {
        Self {
            name: name.into(),
            baseline,
            optimized,
        }
    }
}

/// 一个由差分执行器实际消费的稳定输入。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DifferentialInput {
    /// 稳定用例名称。
    pub name: String,
    /// 供两个执行侧消费的规范输入文本。
    pub input: String,
}

impl DifferentialInput {
    /// 创建一个差分输入。
    #[must_use]
    pub fn new(name: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            input: input.into(),
        }
    }
}

/// 一个差分用例的稳定报告。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DifferentialCaseReport {
    /// 用例名称。
    pub name: String,
    /// 逐字段差异。
    pub differences: Vec<DifferentialDifference>,
}

/// 一组差分用例的稳定报告。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DifferentialSuiteReport {
    /// 当前差分侧实际使用的优化级别。
    pub optimization_level: OptimizationLevel,
    /// 按用例名排序的报告。
    pub cases: Vec<DifferentialCaseReport>,
    /// 是否所有用例逐项一致。
    pub passed: bool,
}

/// 三方差分执行对象：源码直跑、未优化字节码和优化字节码。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThreeWayExecutionSide {
    /// 源码直跑。
    Source,
    /// 未优化字节码。
    UnoptimizedBytecode,
    /// 优化字节码。
    OptimizedBytecode,
}

/// 三方差分观察值。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThreeWayObservation {
    /// 标准输出。
    pub output: String,
    /// 错误摘要；没有错误时为空。
    pub error: Option<String>,
    /// 固定随机源产生的可观察序列。
    pub random: Vec<String>,
    /// 按冻结顺序记录的释放事件。
    pub drops: Vec<String>,
}

/// 三方差分的单字段差异。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThreeWayDifference {
    /// 差异字段名：`output`、`error`、`random` 或 `drops`。
    pub field: String,
    /// 源码直跑观察值。
    pub source: String,
    /// 未优化字节码观察值。
    pub unoptimized: String,
    /// 优化字节码观察值。
    pub optimized: String,
}

/// 三方差分用例报告。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThreeWayCaseReport {
    /// 稳定用例名称。
    pub name: String,
    /// 逐字段差异。
    pub differences: Vec<ThreeWayDifference>,
}

/// 三方差分套件报告。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ThreeWaySuiteReport {
    /// 当前优化侧实际使用的级别。
    pub optimization_level: OptimizationLevel,
    /// 按用例名排序的报告。
    pub cases: Vec<ThreeWayCaseReport>,
    /// 是否三个观察对象逐项一致。
    pub passed: bool,
}

/// 15A 四方差分执行对象。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FourWayExecutionSide {
    /// 未优化原生。
    UnoptimizedNative,
    /// 优化原生。
    OptimizedNative,
    /// 未优化字节码。
    UnoptimizedBytecode,
    /// 优化字节码。
    OptimizedBytecode,
}

/// 四方差分观察值；允许布局、体积和时间在观察对象之外独立记录。
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FourWayObservation {
    /// 标准输出。
    pub output: String,
    /// 结构化错误类别。
    pub error: Option<String>,
    /// 进程退出码。
    pub exit_code: i32,
    /// 固定随机源序列。
    pub random: Vec<String>,
    /// 容器遍历顺序。
    pub containers: Vec<String>,
    /// 释放事件顺序。
    pub drops: Vec<String>,
}

/// 四方差分字段差异。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FourWayDifference {
    /// 差异字段。
    pub field: String,
    /// 未优化原生观察值。
    pub unoptimized_native: String,
    /// 优化原生观察值。
    pub optimized_native: String,
    /// 未优化字节码观察值。
    pub unoptimized_bytecode: String,
    /// 优化字节码观察值。
    pub optimized_bytecode: String,
}

/// 四方差分用例报告。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FourWayCaseReport {
    /// 稳定用例名称。
    pub name: String,
    /// 六类不允许差异。
    pub differences: Vec<FourWayDifference>,
}

/// 四方差分套件报告。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FourWaySuiteReport {
    /// 原生/字节码优化侧级别。
    pub optimization_level: OptimizationLevel,
    /// 按名称排序的用例。
    pub cases: Vec<FourWayCaseReport>,
    /// 是否所有对象逐项一致。
    pub passed: bool,
}

/// 比较未优化和优化执行结果，返回稳定排序的差异列表。
#[must_use]
pub fn compare_observations(
    baseline: &DifferentialObservation,
    optimized: &DifferentialObservation,
) -> Vec<DifferentialDifference> {
    let mut differences = Vec::new();
    if baseline.output != optimized.output {
        differences.push(difference("output", &baseline.output, &optimized.output));
    }
    if baseline.error != optimized.error {
        differences.push(difference(
            "error",
            &format!("{:?}", baseline.error),
            &format!("{:?}", optimized.error),
        ));
    }
    if baseline.exit_code != optimized.exit_code {
        differences.push(difference(
            "exit_code",
            &baseline.exit_code.to_string(),
            &optimized.exit_code.to_string(),
        ));
    }
    if baseline.drops != optimized.drops {
        differences.push(difference(
            "drops",
            &format!("{:?}", baseline.drops),
            &format!("{:?}", optimized.drops),
        ));
    }
    differences
}

/// 以第一路为基线比较任意数量的观察对象。
///
/// 所有字段比较仍由 [`compare_observations`] 完成，调用方只负责提供稳定的
/// 侧名称。结果顺序固定为输入顺序、字段顺序，适合跨平台报告直接序列化。
#[must_use]
pub fn compare_named_observations(
    sides: &[(&str, DifferentialObservation)],
) -> Vec<NamedDifferentialDifference> {
    let Some((baseline_side, baseline)) = sides.first() else {
        return Vec::new();
    };
    sides[1..]
        .iter()
        .flat_map(|(compared_side, compared)| {
            compare_observations(baseline, compared).into_iter().map(|difference| {
                NamedDifferentialDifference {
                    baseline_side: (*baseline_side).to_owned(),
                    compared_side: (*compared_side).to_owned(),
                    field: difference.field,
                    baseline: difference.baseline,
                    compared: difference.optimized,
                }
            })
        })
        .collect()
}

/// 执行当前 O0 基线差分套件；用例按名称排序以清除未定义输入顺序。
#[must_use]
pub fn run_o0_differential_suite(mut cases: Vec<DifferentialCase>) -> DifferentialSuiteReport {
    cases.sort_by(|left, right| left.name.cmp(&right.name));
    let cases = cases
        .into_iter()
        .map(|case| DifferentialCaseReport {
            name: case.name,
            differences: compare_observations(&case.baseline, &case.optimized),
        })
        .collect::<Vec<_>>();
    let passed = cases.iter().all(|case| case.differences.is_empty());
    DifferentialSuiteReport {
        optimization_level: OptimizationLevel::O0,
        cases,
        passed,
    }
}

/// 执行输入驱动的 O0 差分套件。
///
/// 执行器会收到同一份输入两次，分别代表未优化基线和当前 O0 优化侧；套件本身
/// 只负责稳定排序、四字段比较和报告，不推断后端语义。
pub fn run_o0_differential_suite_with<F>(
    mut inputs: Vec<DifferentialInput>,
    mut execute: F,
) -> DifferentialSuiteReport
where
    F: FnMut(&str, OptimizationLevel) -> DifferentialObservation,
{
    inputs.sort_by(|left, right| left.name.cmp(&right.name));
    let cases = inputs
        .into_iter()
        .map(|input| {
            let baseline = execute(&input.input, OptimizationLevel::O0);
            let optimized = execute(&input.input, OptimizationLevel::O0);
            DifferentialCaseReport {
                name: input.name,
                differences: compare_observations(&baseline, &optimized),
            }
        })
        .collect::<Vec<_>>();
    let passed = cases.iter().all(|case| case.differences.is_empty());
    DifferentialSuiteReport {
        optimization_level: OptimizationLevel::O0,
        cases,
        passed,
    }
}

/// 比较源码、未优化字节码和优化字节码的四类可观察结果。
#[must_use]
pub fn compare_three_observations(
    source: &ThreeWayObservation,
    unoptimized: &ThreeWayObservation,
    optimized: &ThreeWayObservation,
) -> Vec<ThreeWayDifference> {
    let mut differences = Vec::new();
    if !(source.output == unoptimized.output && source.output == optimized.output) {
        differences.push(three_way_difference(
            "output",
            &source.output,
            &unoptimized.output,
            &optimized.output,
        ));
    }
    if !(source.error == unoptimized.error && source.error == optimized.error) {
        differences.push(three_way_difference(
            "error",
            &format!("{:?}", source.error),
            &format!("{:?}", unoptimized.error),
            &format!("{:?}", optimized.error),
        ));
    }
    if !(source.random == unoptimized.random && source.random == optimized.random) {
        differences.push(three_way_difference(
            "random",
            &format!("{:?}", source.random),
            &format!("{:?}", unoptimized.random),
            &format!("{:?}", optimized.random),
        ));
    }
    if !(source.drops == unoptimized.drops && source.drops == optimized.drops) {
        differences.push(three_way_difference(
            "drops",
            &format!("{:?}", source.drops),
            &format!("{:?}", unoptimized.drops),
            &format!("{:?}", optimized.drops),
        ));
    }
    differences
}

/// 执行输入驱动的三方差分套件；输入按名称排序，避免未定义顺序。
#[must_use]
pub fn run_three_way_differential_suite_with<F>(
    inputs: Vec<DifferentialInput>,
    execute: F,
) -> ThreeWaySuiteReport
where
    F: FnMut(&str, ThreeWayExecutionSide) -> ThreeWayObservation,
{
    run_three_way_differential_suite_with_level(inputs, OptimizationLevel::O1, execute)
}

/// 执行三方差分并显式记录优化侧级别。
#[must_use]
pub fn run_three_way_differential_suite_with_level<F>(
    mut inputs: Vec<DifferentialInput>,
    optimization_level: OptimizationLevel,
    mut execute: F,
) -> ThreeWaySuiteReport
where
    F: FnMut(&str, ThreeWayExecutionSide) -> ThreeWayObservation,
{
    inputs.sort_by(|left, right| left.name.cmp(&right.name));
    let cases = inputs
        .into_iter()
        .map(|input| {
            let source = execute(&input.input, ThreeWayExecutionSide::Source);
            let unoptimized = execute(&input.input, ThreeWayExecutionSide::UnoptimizedBytecode);
            let optimized = execute(&input.input, ThreeWayExecutionSide::OptimizedBytecode);
            ThreeWayCaseReport {
                name: input.name,
                differences: compare_three_observations(&source, &unoptimized, &optimized),
            }
        })
        .collect::<Vec<_>>();
    let passed = cases.iter().all(|case| case.differences.is_empty());
    ThreeWaySuiteReport {
        optimization_level,
        cases,
        passed,
    }
}

/// 比较四个执行对象的六类语言可观察结果。
#[must_use]
pub fn compare_four_observations(
    unoptimized_native: &FourWayObservation,
    optimized_native: &FourWayObservation,
    unoptimized_bytecode: &FourWayObservation,
    optimized_bytecode: &FourWayObservation,
) -> Vec<FourWayDifference> {
    let values = |field: &str| -> Option<FourWayDifference> {
        let (a, b, c, d) = match field {
            "output" => (
                unoptimized_native.output.clone(),
                optimized_native.output.clone(),
                unoptimized_bytecode.output.clone(),
                optimized_bytecode.output.clone(),
            ),
            "error" => (
                format!("{:?}", unoptimized_native.error),
                format!("{:?}", optimized_native.error),
                format!("{:?}", unoptimized_bytecode.error),
                format!("{:?}", optimized_bytecode.error),
            ),
            "exit_code" => (
                unoptimized_native.exit_code.to_string(),
                optimized_native.exit_code.to_string(),
                unoptimized_bytecode.exit_code.to_string(),
                optimized_bytecode.exit_code.to_string(),
            ),
            "random" => (
                format!("{:?}", unoptimized_native.random),
                format!("{:?}", optimized_native.random),
                format!("{:?}", unoptimized_bytecode.random),
                format!("{:?}", optimized_bytecode.random),
            ),
            "containers" => (
                format!("{:?}", unoptimized_native.containers),
                format!("{:?}", optimized_native.containers),
                format!("{:?}", unoptimized_bytecode.containers),
                format!("{:?}", optimized_bytecode.containers),
            ),
            "drops" => (
                format!("{:?}", unoptimized_native.drops),
                format!("{:?}", optimized_native.drops),
                format!("{:?}", unoptimized_bytecode.drops),
                format!("{:?}", optimized_bytecode.drops),
            ),
            _ => return None,
        };
        if a == b && a == c && a == d {
            None
        } else {
            Some(FourWayDifference {
                field: field.to_owned(),
                unoptimized_native: a,
                optimized_native: b,
                unoptimized_bytecode: c,
                optimized_bytecode: d,
            })
        }
    };
    [
        "output",
        "error",
        "exit_code",
        "random",
        "containers",
        "drops",
    ]
    .into_iter()
    .filter_map(values)
    .collect()
}

/// 执行输入驱动的四方差分套件。
#[must_use]
pub fn run_four_way_differential_suite_with<F>(
    mut inputs: Vec<DifferentialInput>,
    optimization_level: OptimizationLevel,
    mut execute: F,
) -> FourWaySuiteReport
where
    F: FnMut(&str, FourWayExecutionSide) -> FourWayObservation,
{
    inputs.sort_by(|left, right| left.name.cmp(&right.name));
    let cases = inputs
        .into_iter()
        .map(|input| {
            let unoptimized_native = execute(&input.input, FourWayExecutionSide::UnoptimizedNative);
            let optimized_native = execute(&input.input, FourWayExecutionSide::OptimizedNative);
            let unoptimized_bytecode =
                execute(&input.input, FourWayExecutionSide::UnoptimizedBytecode);
            let optimized_bytecode = execute(&input.input, FourWayExecutionSide::OptimizedBytecode);
            FourWayCaseReport {
                name: input.name,
                differences: compare_four_observations(
                    &unoptimized_native,
                    &optimized_native,
                    &unoptimized_bytecode,
                    &optimized_bytecode,
                ),
            }
        })
        .collect::<Vec<_>>();
    let passed = cases.iter().all(|case| case.differences.is_empty());
    FourWaySuiteReport {
        optimization_level,
        cases,
        passed,
    }
}

fn difference(field: &str, baseline: &str, optimized: &str) -> DifferentialDifference {
    DifferentialDifference {
        field: field.to_owned(),
        baseline: baseline.to_owned(),
        optimized: optimized.to_owned(),
    }
}

fn three_way_difference(
    field: &str,
    source: &str,
    unoptimized: &str,
    optimized: &str,
) -> ThreeWayDifference {
    ThreeWayDifference {
        field: field.to_owned(),
        source: source.to_owned(),
        unoptimized: unoptimized.to_owned(),
        optimized: optimized.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DifferentialCase, DifferentialInput, DifferentialObservation, ThreeWayExecutionSide,
        ThreeWayObservation, compare_named_observations, compare_observations,
        compare_three_observations,
        run_o0_differential_suite, run_o0_differential_suite_with,
        run_three_way_differential_suite_with,
    };
    use crate::OptimizationLevel;

    #[test]
    fn compares_output_error_exit_code_and_drop_order() {
        let baseline = DifferentialObservation {
            output: "a".to_owned(),
            error: None,
            exit_code: 0,
            drops: vec!["v1".to_owned(), "v2".to_owned()],
        };
        let optimized = DifferentialObservation {
            output: "b".to_owned(),
            error: Some("E".to_owned()),
            exit_code: 3,
            drops: vec!["v2".to_owned(), "v1".to_owned()],
        };
        let fields = compare_observations(&baseline, &optimized)
            .into_iter()
            .map(|difference| difference.field)
            .collect::<Vec<_>>();
        assert_eq!(fields, vec!["output", "error", "exit_code", "drops"]);
    }

    #[test]
    fn equal_observations_have_no_difference() {
        let observation = DifferentialObservation::default();
        assert!(compare_observations(&observation, &observation).is_empty());
    }

    #[test]
    fn named_comparison_reuses_the_same_field_order_and_names_sides() {
        let baseline = DifferentialObservation::default();
        let changed = DifferentialObservation {
            output: "changed".to_owned(),
            ..DifferentialObservation::default()
        };
        let differences = compare_named_observations(&[
            ("Windows", baseline),
            ("Linux", changed),
        ]);
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].baseline_side, "Windows");
        assert_eq!(differences[0].compared_side, "Linux");
        assert_eq!(differences[0].field, "output");
    }

    #[test]
    fn suite_sorts_cases_and_reports_o0_honestly() {
        let observation = DifferentialObservation::default();
        let report = run_o0_differential_suite(vec![
            DifferentialCase::new("z-case", observation.clone(), observation.clone()),
            DifferentialCase::new("a-case", observation.clone(), observation),
        ]);
        assert_eq!(report.optimization_level.as_str(), "O0");
        assert_eq!(
            report
                .cases
                .iter()
                .map(|case| case.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a-case", "z-case"]
        );
        assert!(report.passed);
    }

    #[test]
    fn input_suite_executes_each_input_for_both_o0_sides() {
        let mut calls = Vec::new();
        let report = run_o0_differential_suite_with(
            vec![DifferentialInput::new("case", "fixture")],
            |input, level| {
                calls.push((input.to_owned(), level));
                DifferentialObservation {
                    output: input.to_owned(),
                    ..DifferentialObservation::default()
                }
            },
        );
        assert!(report.passed);
        assert_eq!(calls.len(), 2);
        assert!(
            calls
                .iter()
                .all(|(input, level)| input == "fixture" && *level == OptimizationLevel::O0)
        );
    }

    #[test]
    fn three_way_suite_compares_random_sequence_and_all_sides() {
        let observation = ThreeWayObservation {
            output: "ok".to_owned(),
            error: None,
            random: vec!["2".to_owned(), "1".to_owned()],
            drops: vec!["v1".to_owned()],
        };
        assert!(compare_three_observations(&observation, &observation, &observation).is_empty());
        let mut calls = Vec::new();
        let report = run_three_way_differential_suite_with(
            vec![DifferentialInput::new("three", "fixture")],
            |input, side| {
                calls.push((input.to_owned(), side));
                observation.clone()
            },
        );
        assert!(report.passed);
        assert_eq!(calls.len(), 3);
        assert!(
            calls
                .iter()
                .any(|(_, side)| *side == ThreeWayExecutionSide::Source)
        );
        assert!(
            calls
                .iter()
                .any(|(_, side)| *side == ThreeWayExecutionSide::UnoptimizedBytecode)
        );
        assert!(
            calls
                .iter()
                .any(|(_, side)| *side == ThreeWayExecutionSide::OptimizedBytecode)
        );
    }

    #[test]
    fn four_way_suite_keeps_six_language_fields_and_ignores_cost_fields() {
        let observation = super::FourWayObservation {
            output: "ok".to_owned(),
            error: None,
            exit_code: 0,
            random: vec!["1".to_owned()],
            containers: vec!["a".to_owned(), "b".to_owned()],
            drops: vec!["v1".to_owned()],
        };
        assert!(
            super::compare_four_observations(
                &observation,
                &observation,
                &observation,
                &observation
            )
            .is_empty()
        );
        let report = super::run_four_way_differential_suite_with(
            vec![DifferentialInput::new("four", "fixture")],
            OptimizationLevel::O2,
            |_, _| observation.clone(),
        );
        assert!(report.passed);
        assert_eq!(report.optimization_level, OptimizationLevel::O2);
    }
}
