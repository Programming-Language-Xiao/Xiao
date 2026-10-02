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

fn difference(field: &str, baseline: &str, optimized: &str) -> DifferentialDifference {
    DifferentialDifference {
        field: field.to_owned(),
        baseline: baseline.to_owned(),
        optimized: optimized.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DifferentialCase, DifferentialInput, DifferentialObservation, compare_observations,
        run_o0_differential_suite, run_o0_differential_suite_with,
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
}
