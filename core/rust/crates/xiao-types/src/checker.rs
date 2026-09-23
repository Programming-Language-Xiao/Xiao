//! 面向 P2/S0 的静态类型检查器。
//!
//! 检查器只消费 `xiao-syntax` 的公开 AST 和源码区间，输出类型化结果、结构化
//! 诊断及需要后端插入的运行时检查标记；它绝不执行 Xiao 程序或修改原始 AST。

use std::collections::BTreeMap;

use xiao_diagnostics::Diagnostic;
use xiao_source::SourceFile;
use xiao_syntax::Program;

use crate::containers::ContainerMaterializationPlan;
use crate::environment::TypeEnvironment;
use crate::functions::FunctionSignature;
use crate::numeric::ConstantValue;
use crate::types::Type;
use crate::unify::TypeContext;

#[cfg(test)]
#[path = "checker_architecture_tests.rs"]
/// 锁定检查器子模块的源码级依赖方向。
mod architecture_tests;
/// 常量求值和跨职责共享的无状态辅助。
#[path = "checker/constant.rs"]
mod constant;
/// C0 容器语义的子模块；保持主检查器只负责语句分派和标量规则。
#[path = "container_checker.rs"]
pub(crate) mod container_checker;
/// 04 条件、循环、返回和入口静态检查。
#[path = "control_checker.rs"]
mod control_checker;
/// 显式转换、常量目标校验和编译期常量求值。
#[path = "checker/conversion.rs"]
mod conversion;
/// 类型诊断与运行时检查标记上报。
#[path = "checker/diagnostic.rs"]
mod diagnostic;
/// 表达式类型检查、运算推导和调用检查。
#[path = "checker/expression.rs"]
mod expression;
/// 04 函数定义、签名占位和调用参数检查。
#[path = "function_checker.rs"]
mod function_checker;
/// 类型检查结果和运行时检查标记的公开模型。
#[path = "checker/result.rs"]
mod result;
/// C1 有序容器选择、随机种子和选择器左值检查。
#[path = "selector_checker.rs"]
mod selector_checker;
/// C2-A 集合字面量、可哈希检查和成员判断。
#[path = "set_checker.rs"]
mod set_checker;
/// C2-C 集合代数、比较和动态检查计划。
#[path = "set_operations.rs"]
mod set_operations;
/// 语句分派、赋值和声明检查。
#[path = "checker/statement.rs"]
mod statement;
/// 05-C 表声明、成员访问和生命周期静态契约。
#[path = "table_checker.rs"]
mod table_checker;

pub use self::result::{RuntimeCheck, RuntimeCheckKind, TypeCheckResult, TypedNode};
use self::table_checker::TableFrame;

/// P2 静态检查器；生命周期只借用不可变源码。
pub struct TypeChecker<'source> {
    source: &'source SourceFile,
    context: TypeContext,
    environment: TypeEnvironment,
    diagnostics: Vec<Diagnostic>,
    nodes: Vec<TypedNode>,
    runtime_checks: Vec<RuntimeCheck>,
    materialization_plans: Vec<ContainerMaterializationPlan>,
    selection_plans: Vec<crate::selection_model::SelectionPlan>,
    broadcast_assignment_plans: Vec<crate::selection_model::BroadcastAssignmentPlan>,
    random_seed_plans: Vec<crate::selection_model::RandomSeedPlan>,
    constant_values: BTreeMap<String, ConstantValue>,
    function_signatures: BTreeMap<String, FunctionSignature>,
    table_signatures: BTreeMap<String, crate::tables::TableSignature>,
    current_table: Option<TableFrame>,
    current_function: Option<FunctionFrame>,
    loop_depth: usize,
}

/// 正在检查的函数上下文；只存静态返回约束，不持有运行时栈。
#[derive(Clone, Debug)]
struct FunctionFrame {
    /// 返回类型约束。
    return_type: Type,
    /// 是否已经遇到返回语句。
    saw_return: bool,
}

impl<'source> TypeChecker<'source> {
    /// 创建使用全局空作用域的检查器。
    #[must_use]
    pub fn new(source: &'source SourceFile) -> Self {
        Self {
            source,
            context: TypeContext::new(),
            environment: TypeEnvironment::new(),
            diagnostics: Vec::new(),
            nodes: Vec::new(),
            runtime_checks: Vec::new(),
            materialization_plans: Vec::new(),
            selection_plans: Vec::new(),
            broadcast_assignment_plans: Vec::new(),
            random_seed_plans: Vec::new(),
            constant_values: BTreeMap::new(),
            function_signatures: BTreeMap::new(),
            table_signatures: BTreeMap::new(),
            current_table: None,
            current_function: None,
            loop_depth: 0,
        }
    }

    /// 直接检查一个程序；这是不需要保留检查器状态时的便捷入口。
    #[must_use]
    pub fn check(source: &'source SourceFile, program: &Program) -> TypeCheckResult {
        Self::new(source).check_program(program)
    }

    /// 使用当前环境检查整个程序并返回结果。
    #[must_use]
    pub fn check_program(mut self, program: &Program) -> TypeCheckResult {
        self.register_top_level_functions(&program.statements);
        self.register_top_level_tables(&program.statements);
        for statement in &program.statements {
            self.check_statement(statement);
        }
        self.finalize_function_inference();
        // 表达式节点是在约束收集期间记录的；函数调用或函数体返回值可能在
        // 后续语句才把类型变量绑定到具体标量。对外发布结果前统一应用最终
        // 替换，确保 IR 消费者看到的节点类型与函数签名使用同一份结论。
        for node in &mut self.nodes {
            node.ty = self.context.apply(&node.ty);
        }
        TypeCheckResult {
            nodes: self.nodes,
            diagnostics: self.diagnostics,
            runtime_checks: self.runtime_checks,
            environment: self.environment,
            materialization_plans: self.materialization_plans,
            selection_plans: self.selection_plans,
            broadcast_assignment_plans: self.broadcast_assignment_plans,
            random_seed_plans: self.random_seed_plans,
            entry_mode: program.entry_mode,
            function_signatures: self.function_signatures,
            table_signatures: self.table_signatures,
        }
    }

    /// 返回检查器当前使用的源码。
    #[must_use]
    pub const fn source(&self) -> &'source SourceFile {
        self.source
    }
}

/// 检查一个已解析程序的便捷函数。
#[must_use]
pub fn check(source: &SourceFile, program: &Program) -> TypeCheckResult {
    TypeChecker::check(source, program)
}
