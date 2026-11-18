//! C0/C1 容器与选择器规格快照测试。
//!
//! 这两个快照在 09R2 的跨层审计中被发现**从未被任何测试加载**：它们在
//! `module-registry.json` 里登记为有效契约，却因为缺少 harness 而长期没有执行，
//! 快照与实现的分歧因此被静默掩盖。本文件把契约重新接回执行。
//!
//! 快照只固定静态检查的机器契约：阶段、成功/失败状态和诊断编号。展示文本不
//! 写入快照，避免国际化目录变更破坏语义测试。

use std::collections::BTreeMap;

use serde::Deserialize;
use xiao_source::SourceFile;
use xiao_syntax::Parser;
use xiao_types::TypeChecker;

/// 一条容器或选择器快照用例。
#[derive(Debug, Deserialize)]
struct SnapshotCase {
    /// 用例名称，用于失败定位。
    name: String,
    /// 要解析和检查的 Xiao 源码。
    source: String,
    /// 期望状态；只有 `error` 表示必须产生错误诊断。
    expect: String,
    /// 期望的诊断编号；既接受裸编号，也接受带消息键的完整对象。
    #[serde(default)]
    diagnostics: Vec<DiagnosticExpectation>,
}

/// 一份快照文件。
#[derive(Debug, Deserialize)]
struct SnapshotFile {
    /// 阶段标识。
    stage: String,
    /// 快照状态；C0 快照早于该字段的引入，允许缺省。
    #[serde(default)]
    status: Option<String>,
    /// 用例列表。
    cases: Vec<SnapshotCase>,
}

/// 一条诊断期望；兼容两种历史写法。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DiagnosticExpectation {
    /// 只有稳定编号。
    Code(String),
    /// 带消息键与结构化参数的完整形态。
    Full {
        /// 稳定机器诊断编号。
        code: String,
        /// 稳定消息目录键。
        #[allow(dead_code)]
        message_id: String,
        /// 消息插值参数。
        #[serde(default)]
        #[allow(dead_code)]
        params: BTreeMap<String, serde_json::Value>,
    },
}

impl DiagnosticExpectation {
    /// 返回期望的稳定诊断编号。
    fn code(&self) -> &str {
        match self {
            Self::Code(code) => code,
            Self::Full { code, .. } => code,
        }
    }
}

/// 读取并执行一份快照。
fn assert_snapshot(raw: &str, expected_stage: &str) {
    let snapshot: SnapshotFile = serde_json::from_str(raw).expect("快照 JSON 必须有效");
    assert_eq!(snapshot.stage, expected_stage);
    assert!(!snapshot.cases.is_empty(), "快照不应为空");
    if let Some(status) = snapshot.status {
        assert_eq!(
            status, "verified-static",
            "快照状态必须是被验证过的静态契约"
        );
    }

    for case in snapshot.cases {
        let source = SourceFile::from_text(&case.source);
        let parsed = Parser::new(&source).parse();
        assert!(
            parsed.diagnostics.is_empty(),
            "快照用例 {} 不应包含解析诊断：{:?}",
            case.name,
            parsed.diagnostics
        );
        let program = parsed.program.expect("快照应始终产生程序 AST");
        let result = TypeChecker::check(&source, &program);
        let actual = result
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code().to_owned())
            .collect::<Vec<_>>();
        let expected = case
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            result.has_errors(),
            case.expect == "error",
            "用例 {} 的错误状态与快照不符，实际诊断：{actual:?}",
            case.name
        );
        assert_eq!(actual, expected, "用例 {} 的诊断与快照不符", case.name);
    }
}

#[test]
/// C0 合法容器与精确路径快照。
fn c0_valid_snapshot_is_stable() {
    assert_snapshot(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../tests/spec/05-containers/c0-valid.json"
        )),
        "03A",
    );
}

#[test]
/// C0 容器错误快照。
fn c0_error_snapshot_is_stable() {
    assert_snapshot(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../tests/spec/05-containers/c0-errors.json"
        )),
        "03A",
    );
}

#[test]
/// C1 有序选择器合法快照。
fn c1_valid_snapshot_is_stable() {
    assert_snapshot(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../tests/spec/05-containers/c1-valid.json"
        )),
        "03B",
    );
}

#[test]
/// C1 有序选择器错误快照。
fn c1_error_snapshot_is_stable() {
    assert_snapshot(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../tests/spec/05-containers/c1-errors.json"
        )),
        "03B",
    );
}
