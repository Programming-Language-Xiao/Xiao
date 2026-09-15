//! Xiao 06-A 静态生命周期分析的稳定门面。
//!
//! 本 crate 将语法树和类型检查结果转换为作用域、逃逸、所有权图和释放计划。
//! 它不执行 Xiao 代码，也不创建 Runtime 对象；后续 IR、字节码和 LLVM 后端
//! 应消费这里的只读结果，而不是重新实现生命周期规则。

/// 06-A 稳定生命周期诊断编号和构造辅助。
mod diagnostics;
/// AST 控制流与逃逸事实收集器。
mod escape;
/// 强/弱所有权图和拓扑排序。
mod graph;
/// 作用域、值、控制流和结果数据模型。
mod model;
/// 作用域退出边释放计划构建器。
mod release;

/// 重导出生命周期诊断编号。
pub use diagnostics::{
    DYNAMIC_CHECK_CODE, FACT_CONFLICT_CODE, INVALID_EDGE_CODE, STRONG_CYCLE_CODE, UNKNOWN_ID_CODE,
};
/// 重导出图错误和所有权图。
pub use graph::{GraphError, OwnershipGraph};
/// 重导出生命周期模型。
pub use model::{
    BasicBlock, BlockId, ControlFlowEdgeKind, ControlFlowGraph, DynamicLifetimeCheck, EscapeReason,
    ExitKind, LifetimeResult, OwnershipEdge, OwnershipEdgeReason, OwnershipKind, ReleaseAction,
    ReleaseActionKind, ReleasePlan, ScopeId, ScopeInfo, ScopeKind, StorageClass, ValueId,
    ValueInfo,
};

use escape::EscapeAnalyzer;
use xiao_source::SourceFile;
use xiao_syntax::Program;
use xiao_types::TypeCheckResult;

/// 绑定一个不可变源码文件的生命周期分析器。
pub struct LifetimeAnalyzer<'source> {
    source: &'source SourceFile,
}

impl<'source> LifetimeAnalyzer<'source> {
    /// 创建生命周期分析器。
    #[must_use]
    pub const fn new(source: &'source SourceFile) -> Self {
        Self { source }
    }

    /// 返回分析器使用的源码文件。
    #[must_use]
    pub const fn source(&self) -> &'source SourceFile {
        self.source
    }

    /// 对程序和既有类型结果执行完整 06-A 分析。
    ///
    /// 类型结果可以包含错误或缺失节点；分析器不会因此 panic，而是把相关
    /// 值按动态堆强拥有处理，并在结果中留下 `X06-LIFETIME-005` 检查记录。
    #[must_use]
    pub fn analyze(&self, program: &Program, type_result: &TypeCheckResult) -> LifetimeResult {
        let (result, transfers) = EscapeAnalyzer::new(self.source, type_result).run(program);
        release::finalize(result, transfers)
    }

    /// `analyze` 的显式程序别名，便于流水线代码表达阶段名称。
    #[must_use]
    pub fn analyze_program(
        &self,
        program: &Program,
        type_result: &TypeCheckResult,
    ) -> LifetimeResult {
        self.analyze(program, type_result)
    }
}

/// 使用源码、程序和类型结果执行一次生命周期分析的便捷函数。
#[must_use]
pub fn analyze(
    source: &SourceFile,
    program: &Program,
    type_result: &TypeCheckResult,
) -> LifetimeResult {
    LifetimeAnalyzer::new(source).analyze(program, type_result)
}

impl LifetimeResult {
    /// 根据分析结果重建可独立验证的所有权图。
    ///
    /// 该方法会验证所有节点和边身份；若结果由外部手工修改而损坏，则返回
    /// `GraphError`，不会 panic。
    pub fn ownership_graph(&self) -> Result<OwnershipGraph, GraphError> {
        OwnershipGraph::from_edges(
            self.values
                .values()
                .map(|value| (value.id, value.declaration_order)),
            self.ownership_edges().cloned(),
        )
    }
}
