//! 未优化与优化结果的统一差分口径。

use serde::{Deserialize, Serialize};

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

fn difference(field: &str, baseline: &str, optimized: &str) -> DifferentialDifference {
    DifferentialDifference {
        field: field.to_owned(),
        baseline: baseline.to_owned(),
        optimized: optimized.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{DifferentialObservation, compare_observations};

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
}
