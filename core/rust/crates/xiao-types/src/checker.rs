//! 面向 P2/S0 的静态类型检查器。
//!
//! 检查器只消费 `xiao-syntax` 的公开 AST 和源码区间，输出类型化结果、结构化
//! 诊断及需要后端插入的运行时检查标记；它绝不执行 Xiao 程序或修改原始 AST。

use std::collections::{BTreeMap, BTreeSet};

use xiao_diagnostics::Diagnostic;
use xiao_source::SourceFile;
use xiao_syntax::Program;

use crate::containers::ContainerMaterializationPlan;
use crate::environment::Binding;
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
/// 已声明模块接口的绑定和调用检查。
#[path = "checker/imports.rs"]
mod imports;
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

/// 检查器输出、类型化节点和后端运行时检查标记的稳定公开模型。
pub use self::result::{
    ConcurrencyCall, RuntimeCheck, RuntimeCheckKind, TypeCheckResult, TypedNode,
};
use self::table_checker::TableFrame;

/// 包路径到可读取成员的静态、无副作用接口；键是完整点号路径。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExternalNamespaces {
    /// 根名称、目录命名空间及文件模块各自的公开成员。
    pub members: BTreeMap<String, BTreeSet<String>>,
}

/// P2 静态检查器；生命周期只借用不可变源码。
pub struct TypeChecker<'source> {
    source: &'source SourceFile,
    context: TypeContext,
    environment: TypeEnvironment,
    external_namespaces: ExternalNamespaces,
    module_functions: BTreeMap<String, BTreeMap<String, FunctionSignature>>,
    resource_parameters: Vec<(
        xiao_source::SourceSpan,
        xiao_syntax::FunctionParameterKind,
        Type,
    )>,
    diagnostics: Vec<Diagnostic>,
    nodes: Vec<TypedNode>,
    runtime_checks: Vec<RuntimeCheck>,
    intrinsic_calls: BTreeMap<(usize, usize), xiao_intrinsics::IntrinsicId>,
    materialization_plans: Vec<ContainerMaterializationPlan>,
    selection_plans: Vec<crate::selection_model::SelectionPlan>,
    broadcast_assignment_plans: Vec<crate::selection_model::BroadcastAssignmentPlan>,
    random_seed_plans: Vec<crate::selection_model::RandomSeedPlan>,
    constant_values: BTreeMap<String, ConstantValue>,
    /// 已知数组别名组；原地追加时一起擦除固定长度。
    array_aliases: BTreeMap<String, BTreeSet<String>>,
    function_signatures: BTreeMap<String, FunctionSignature>,
    table_signatures: BTreeMap<String, crate::tables::TableSignature>,
    current_table: Option<TableFrame>,
    current_function: Option<FunctionFrame>,
    /// 当前正在检查的函数名称，用于让直接递归调用复用刚性签名。
    current_function_key: Option<String>,
    /// 命名并发句柄的静态消费状态；仅用于调用检查。
    concurrency_handle_states: BTreeMap<String, crate::concurrency::HandleConsumption>,
    /// 已由 `std.concurrent` 导入绑定解析出的调用点。
    concurrency_calls: Vec<ConcurrencyCall>,
    /// 函数体中直接执行数组变更的函数摘要。
    mutating_functions: BTreeSet<String>,
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
            external_namespaces: ExternalNamespaces::default(),
            module_functions: BTreeMap::new(),
            resource_parameters: Vec::new(),
            diagnostics: Vec::new(),
            nodes: Vec::new(),
            runtime_checks: Vec::new(),
            intrinsic_calls: BTreeMap::new(),
            materialization_plans: Vec::new(),
            selection_plans: Vec::new(),
            broadcast_assignment_plans: Vec::new(),
            random_seed_plans: Vec::new(),
            constant_values: BTreeMap::new(),
            array_aliases: BTreeMap::new(),
            function_signatures: BTreeMap::new(),
            table_signatures: BTreeMap::new(),
            current_table: None,
            current_function: None,
            current_function_key: None,
            concurrency_handle_states: BTreeMap::new(),
            concurrency_calls: Vec::new(),
            mutating_functions: BTreeSet::new(),
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
        self.collect_mutating_functions(&program.statements);
        self.register_top_level_functions(&program.statements);
        self.register_top_level_tables(&program.statements);
        for statement in &program.statements {
            self.check_statement(statement);
        }
        self.finalize_function_inference();
        self.finalize_resource_parameter_boundaries();
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
            intrinsic_calls: self.intrinsic_calls,
            environment: self.environment,
            materialization_plans: self.materialization_plans,
            selection_plans: self.selection_plans,
            broadcast_assignment_plans: self.broadcast_assignment_plans,
            random_seed_plans: self.random_seed_plans,
            entry_mode: program.entry_mode,
            function_signatures: self.function_signatures,
            table_signatures: self.table_signatures,
            concurrency_calls: self.concurrency_calls,
        }
    }

    /// 预扫描直接调用 `array_push` 的函数，供调用点精确擦除数组长度。
    fn collect_mutating_functions(&mut self, statements: &[xiao_syntax::Statement]) {
        /// 判断表达式树是否直接或递归包含数组变更调用。
        fn expression_mutates(
            expression: &xiao_syntax::Expression,
            source: &xiao_source::SourceFile,
        ) -> bool {
            use xiao_syntax::Expression;
            match expression {
                Expression::Call {
                    callee, arguments, ..
                } => {
                    matches!(callee.as_ref(), Expression::Name(name) if name.unquoted_text(source) == "array_push")
                        || arguments
                            .iter()
                            .any(|argument| expression_mutates(&argument.value, source))
                }
                Expression::ArrayLiteral { elements, .. }
                | Expression::TupleLiteral { elements, .. }
                | Expression::SetLiteral { elements, .. } => elements
                    .iter()
                    .any(|element| expression_mutates(element, source)),
                Expression::DictTableLiteral { entries, .. }
                | Expression::DictColumnLiteral { entries, .. } => entries
                    .iter()
                    .any(|entry| expression_mutates(&entry.value, source)),
                Expression::Group { expression, .. }
                | Expression::Unary {
                    operand: expression,
                    ..
                }
                | Expression::Cast { expression, .. } => expression_mutates(expression, source),
                Expression::Binary { left, right, .. } => {
                    expression_mutates(left, source) || expression_mutates(right, source)
                }
                Expression::Member { object, .. } => expression_mutates(object, source),
                Expression::Selector {
                    source: source_expression,
                    step,
                    ..
                } => {
                    expression_mutates(source_expression, source)
                        || step
                            .as_deref()
                            .is_some_and(|step| expression_mutates(step, source))
                }
                Expression::Literal { .. } | Expression::Name(_) => false,
                Expression::NewCall { arguments, .. } => arguments
                    .iter()
                    .any(|argument| expression_mutates(&argument.value, source)),
            }
        }
        /// 判断语句列表是否包含会改变数组共享句柄的操作。
        fn statements_mutate(
            statements: &[xiao_syntax::Statement],
            source: &xiao_source::SourceFile,
        ) -> bool {
            use xiao_syntax::Statement;
            statements.iter().any(|statement| match statement {
                Statement::Expression { expression, .. } => expression_mutates(expression, source),
                Statement::Assignment { value, .. } | Statement::ConstDeclaration { value, .. } => {
                    expression_mutates(value, source)
                }
                Statement::Declaration { value, .. } => value
                    .as_ref()
                    .is_some_and(|value| expression_mutates(value, source)),
                Statement::ExtendedAssignment { target, value, .. } => {
                    expression_mutates(target, source) || expression_mutates(value, source)
                }
                Statement::If {
                    condition,
                    body,
                    elif_branches,
                    else_body,
                    ..
                } => {
                    expression_mutates(condition, source)
                        || statements_mutate(body, source)
                        || elif_branches.iter().any(|branch| {
                            expression_mutates(&branch.condition, source)
                                || statements_mutate(&branch.body, source)
                        })
                        || else_body
                            .as_deref()
                            .is_some_and(|body| statements_mutate(body, source))
                }
                Statement::For { iterable, body, .. }
                | Statement::While {
                    condition: iterable,
                    body,
                    ..
                } => expression_mutates(iterable, source) || statements_mutate(body, source),
                Statement::Return {
                    value: Some(value), ..
                }
                | Statement::Raise { value, .. } => expression_mutates(value, source),
                Statement::Try {
                    body,
                    catches,
                    finally_body,
                    ..
                } => {
                    statements_mutate(body, source)
                        || catches
                            .iter()
                            .any(|catch| statements_mutate(&catch.body, source))
                        || finally_body
                            .as_deref()
                            .is_some_and(|body| statements_mutate(body, source))
                }
                Statement::Function { .. }
                | Statement::Import { .. }
                | Statement::Return { value: None, .. }
                | Statement::Break { .. }
                | Statement::Continue { .. }
                | Statement::Table { .. } => false,
            })
        }
        fn expression_calls(
            expression: &xiao_syntax::Expression,
            source: &xiao_source::SourceFile,
            calls: &mut BTreeSet<String>,
        ) {
            use xiao_syntax::Expression;
            match expression {
                Expression::Call {
                    callee, arguments, ..
                } => {
                    if let Expression::Name(name) = callee.as_ref() {
                        calls.insert(if name.backticked {
                            format!("backtick:{}", name.unquoted_text(source))
                        } else {
                            format!("ascii:{}", name.unquoted_text(source))
                        });
                    }
                    expression_calls(callee, source, calls);
                    for argument in arguments {
                        expression_calls(&argument.value, source, calls);
                    }
                }
                Expression::ArrayLiteral { elements, .. }
                | Expression::TupleLiteral { elements, .. }
                | Expression::SetLiteral { elements, .. } => {
                    for element in elements {
                        expression_calls(element, source, calls);
                    }
                }
                Expression::DictTableLiteral { entries, .. }
                | Expression::DictColumnLiteral { entries, .. } => {
                    for entry in entries {
                        expression_calls(&entry.value, source, calls);
                    }
                }
                Expression::Group { expression, .. }
                | Expression::Unary {
                    operand: expression,
                    ..
                }
                | Expression::Cast { expression, .. } => {
                    expression_calls(expression, source, calls)
                }
                Expression::Binary { left, right, .. } => {
                    expression_calls(left, source, calls);
                    expression_calls(right, source, calls);
                }
                Expression::Member { object, .. } => expression_calls(object, source, calls),
                Expression::Selector {
                    source: source_expression,
                    step,
                    ..
                } => {
                    expression_calls(source_expression, source, calls);
                    if let Some(step) = step {
                        expression_calls(step, source, calls);
                    }
                }
                Expression::Literal { .. } | Expression::Name(_) => {}
                Expression::NewCall {
                    callee, arguments, ..
                } => {
                    expression_calls(callee, source, calls);
                    for argument in arguments {
                        expression_calls(&argument.value, source, calls);
                    }
                }
            }
        }
        fn statements_calls(
            statements: &[xiao_syntax::Statement],
            source: &xiao_source::SourceFile,
            calls: &mut BTreeSet<String>,
        ) {
            use xiao_syntax::Statement;
            for statement in statements {
                match statement {
                    Statement::Expression { expression, .. }
                    | Statement::Assignment {
                        value: expression, ..
                    }
                    | Statement::ConstDeclaration {
                        value: expression, ..
                    } => expression_calls(expression, source, calls),
                    Statement::Declaration { value, .. } => {
                        if let Some(value) = value {
                            expression_calls(value, source, calls);
                        }
                    }
                    Statement::ExtendedAssignment { target, value, .. } => {
                        expression_calls(target, source, calls);
                        expression_calls(value, source, calls);
                    }
                    Statement::If {
                        condition,
                        body,
                        elif_branches,
                        else_body,
                        ..
                    } => {
                        expression_calls(condition, source, calls);
                        statements_calls(body, source, calls);
                        for branch in elif_branches {
                            expression_calls(&branch.condition, source, calls);
                            statements_calls(&branch.body, source, calls);
                        }
                        if let Some(body) = else_body {
                            statements_calls(body, source, calls);
                        }
                    }
                    Statement::For { iterable, body, .. }
                    | Statement::While {
                        condition: iterable,
                        body,
                        ..
                    } => {
                        expression_calls(iterable, source, calls);
                        statements_calls(body, source, calls);
                    }
                    Statement::Return { value, .. } => {
                        if let Some(value) = value {
                            expression_calls(value, source, calls);
                        }
                    }
                    Statement::Raise { value, .. } => expression_calls(value, source, calls),
                    Statement::Try {
                        body,
                        catches,
                        finally_body,
                        ..
                    } => {
                        statements_calls(body, source, calls);
                        for catch in catches {
                            statements_calls(&catch.body, source, calls);
                        }
                        if let Some(body) = finally_body {
                            statements_calls(body, source, calls);
                        }
                    }
                    Statement::Function { .. }
                    | Statement::Import { .. }
                    | Statement::Break { .. }
                    | Statement::Continue { .. }
                    | Statement::Table { .. } => {}
                }
            }
        }
        let mut calls = BTreeMap::<String, BTreeSet<String>>::new();
        for statement in statements {
            if let xiao_syntax::Statement::Function { name, body, .. } = statement {
                let key = self.name_key(*name);
                if statements_mutate(body, self.source) {
                    self.mutating_functions.insert(key.clone());
                }
                let mut called = BTreeSet::new();
                statements_calls(body, self.source, &mut called);
                calls.insert(key, called);
            }
        }
        loop {
            let before = self.mutating_functions.len();
            for (function, called) in &calls {
                if called
                    .iter()
                    .any(|callee| self.mutating_functions.contains(callee))
                {
                    self.mutating_functions.insert(function.clone());
                }
            }
            if self.mutating_functions.len() == before {
                break;
            }
        }
    }

    /// 返回检查器当前使用的源码。
    #[must_use]
    pub const fn source(&self) -> &'source SourceFile {
        self.source
    }

    /// 只登记外部包接口；此步骤不读取或执行包模块源码。
    #[must_use]
    pub fn with_external_namespaces(mut self, namespaces: ExternalNamespaces) -> Self {
        for root in namespaces.members.keys().filter(|path| !path.contains('.')) {
            let _ = self
                .environment
                .declare(format!("ascii:{root}"), Binding::package_root());
        }
        self.external_namespaces = namespaces;
        self
    }

    /// 注入已声明的模块函数接口；不执行导入、不改变后端的模块调用支持范围。
    #[must_use]
    pub fn with_module_function_signatures(
        mut self,
        functions: BTreeMap<String, BTreeMap<String, FunctionSignature>>,
    ) -> Self {
        self.module_functions = functions;
        self
    }
}

/// 检查一个已解析程序的便捷函数。
#[must_use]
pub fn check(source: &SourceFile, program: &Program) -> TypeCheckResult {
    TypeChecker::check(source, program)
}
