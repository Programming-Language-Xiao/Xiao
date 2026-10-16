//! 20C 第九批：已知未实现降低身份与高精度一元负号回归。

use std::fs;
use std::path::{Path, PathBuf};

use xiao_bytecode::lower_program;
use xiao_driver::{DriverRequest, FrontendCompiler, FrontendRequest};
use xiao_optimizer::OptimizationLevel;
use xiao_vm::VmEvent;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("xiao-driver crate 应位于仓库 core/rust/crates 下")
        .to_path_buf()
}

fn fixture(name: &str) -> (String, FrontendRequest) {
    let path = repository_root().join("tests/fixtures/20c").join(name);
    let source = fs::read_to_string(&path).expect("读取第九批夹具");
    (source.clone(), FrontendRequest::from_text_at(&source, path))
}

fn output(outcome: &xiao_driver::DriverOutcome) -> String {
    outcome
        .as_executed()
        .into_iter()
        .flat_map(|execution| execution.events())
        .filter_map(|event| match event {
            VmEvent::IntrinsicOutput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
/// 可达的已知未实现构造统一使用 `X09-BYTECODE-003`，而不是内部缺口身份。
fn known_unsupported_boundaries_use_the_new_bytecode_code() {
    for name in [
        "ninth-method-value.xiao",
        "ninth-expanded-argument.xiao",
        "ninth-table-expanded-argument.xiao",
    ] {
        let (_source, request) = fixture(name);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .unwrap_or_else(|error| panic!("第九批夹具应通过前端：{name} {error:?}"));
        let tac = lower_program(artifact.ir());
        assert!(
            tac.unsupported
                .iter()
                .any(|note| note.starts_with("known-unsupported:")),
            "夹具没有记录已知未实现构造：{name} {:?}",
            tac.unsupported
        );
        for level in [
            OptimizationLevel::O0,
            OptimizationLevel::O1,
            OptimizationLevel::O2,
            OptimizationLevel::O3,
        ] {
            let outcome = xiao_driver::FrontendVmDriver::new().run_artifact(
                &artifact,
                &DriverRequest::new(request.clone()).with_optimization_level(level),
            );
            assert_eq!(
                outcome.code(),
                Some("X09-BYTECODE-003"),
                "已知未实现构造身份错误：{name} {level:?} {outcome:?}"
            );
        }
    }
}

#[test]
/// VM 的高精度取负覆盖零值归一化、已有负号、精度文本和负零。
fn high_precision_negation_preserves_text_semantics() {
    let (source, request) = fixture("ninth-high-precision-negation.xiao");
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .unwrap_or_else(|error| panic!("高精度取负夹具不应前端拒绝：{error:?}"));
    let outcome =
        xiao_driver::FrontendVmDriver::new().run_artifact(&artifact, &DriverRequest::new(request));
    assert!(outcome.is_success(), "高精度取负夹具失败：{outcome:?}");
    assert_eq!(
        output(&outcome),
        "-42\n0\n-1.234567890123456789\n-0.0\n",
        "高精度取负文本语义漂移：{source}"
    );
}
