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
    DYNAMIC_CHECK_CODE, FACT_CONFLICT_CODE, INVALID_EDGE_CODE, RUNTIME_BOUNDARY_CODE,
    STRONG_CYCLE_CODE, TASK_BOUNDARY_CODE, THREAD_BOUNDARY_CODE, UNKNOWN_ID_CODE,
};
/// 重导出图错误和所有权图。
pub use graph::{GraphError, OwnershipGraph};
/// 重导出生命周期模型。
pub use model::{
    BasicBlock, BlockId, BoundaryFact, BoundaryFactError, BoundaryFacts, BoundaryKind,
    BoundaryOperation, BoundaryValue, BoundaryValueKind, ControlFlowEdgeKind, ControlFlowGraph,
    CrossThreadFact, DynamicLifetimeCheck, EscapeReason, ExitKind, LifetimeResult, OwnershipEdge,
    OwnershipEdgeReason, OwnershipKind, ReleaseAction, ReleaseActionKind, ReleasePlan, RuntimeId,
    RuntimeIdFact, RuntimeIdFacts, ScopeId, ScopeInfo, ScopeKind, StorageClass, TaskBoundary,
    ThreadBoundary, ValueId, ValueInfo, classify_boundary_type,
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

    /// 使用显式 lowering 事实执行生命周期分析。
    ///
    /// AST 遍历只负责普通生命周期关系；`spawn`/`submit` 等并发操作必须由
    /// 已完成解析和类型判定的 lowering 通过 `BoundaryFacts` 显式提交，分析器
    /// 不会根据名称猜测并发语义。
    #[must_use]
    pub fn analyze_with_boundary_facts(
        &self,
        program: &Program,
        type_result: &TypeCheckResult,
        boundary_facts: BoundaryFacts,
    ) -> LifetimeResult {
        let mut result = self.analyze(program, type_result);
        let _ = result.merge_boundary_facts(boundary_facts);
        result
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
    /// 合并一组显式 lowering 产生的边界事实，并把事实诊断并入结果。
    pub fn merge_boundary_facts(
        &mut self,
        boundary_facts: BoundaryFacts,
    ) -> Result<(), BoundaryFactError> {
        let diagnostics = boundary_facts.diagnostics.clone();
        let result = self.boundary_facts.merge(boundary_facts);
        self.diagnostics.extend(diagnostics);
        if let Err(error) = &result {
            self.diagnostics.push(xiao_diagnostics::Diagnostic::new(
                RUNTIME_BOUNDARY_CODE,
                "x06.lifetime.boundary_merge",
                xiao_diagnostics::Severity::Error,
                None,
                error.to_string(),
            ));
        }
        result
    }

    /// 以显式 lowering 事实替换当前空容器；适合 IR 组装器在分析后调用。
    pub fn with_boundary_facts(mut self, boundary_facts: BoundaryFacts) -> Self {
        let _ = self.merge_boundary_facts(boundary_facts);
        self
    }

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
