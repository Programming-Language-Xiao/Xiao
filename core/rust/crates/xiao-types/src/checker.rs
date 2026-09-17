//! 面向 P2/S0 的静态类型检查器。
//!
//! 检查器只消费 `xiao-syntax` 的公开 AST 和源码区间，输出类型化结果、结构化
//! 诊断及需要后端插入的运行时检查标记；它绝不执行 Xiao 程序或修改原始 AST。

use std::collections::BTreeMap;

use xiao_diagnostics::{Diagnostic, DiagnosticParam, Severity};
use xiao_source::{SourceFile, SourceSpan};
use xiao_syntax::{
    AssignmentOperator, BinaryOperator, CallArgument, DeclaredType, EntryMode, Expression,
    IndexPath, LiteralKind, Program, ScalarType, Statement, UnaryOperator,
};

use crate::containers::ContainerMaterializationPlan;
use crate::conversion::{
    ConversionKind, can_assign, classify_conversion, is_float, is_integer, is_numeric,
};
use crate::diagnostics::*;
use crate::environment::{Binding, EnvironmentError, TypeEnvironment};
use crate::functions::FunctionSignature;
use crate::numeric::{
    ConstantValue, NumericError, binary_scalar_type, boolean_integer_adjust, check_float_range,
    check_float_to_integer_range, check_integer_range, is_decimal_integer, parse_float_literal,
    parse_integer_literal,
};
use crate::types::Type;
use crate::unify::{TypeContext, UnifyError};

/// C0 容器语义的子模块；保持主检查器只负责语句分派和标量规则。
#[path = "container_checker.rs"]
pub(crate) mod container_checker;
/// 04 条件、循环、返回和入口静态检查。
#[path = "control_checker.rs"]
mod control_checker;
/// 04 函数定义、签名占位和调用参数检查。
#[path = "function_checker.rs"]
mod function_checker;
/// C1 有序容器选择、随机种子和选择器左值检查。
#[path = "selector_checker.rs"]
mod selector_checker;
/// C2-A 集合字面量、可哈希检查和成员判断。
#[path = "set_checker.rs"]
mod set_checker;
/// C2-C 集合代数、比较和动态检查计划。
#[path = "set_operations.rs"]
mod set_operations;
/// 05-C 表声明、成员访问和生命周期静态契约。
#[path = "table_checker.rs"]
mod table_checker;

use self::set_operations::should_attempt_set_semantics;
use self::table_checker::TableFrame;

/// 后端需要保留的运行时检查种类。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeCheckKind {
    /// 固定宽度整数或浮点范围检查。
    NumericRange,
    /// `str` 到 `bool` 的四种合法拼写检查。
    StringBoolean,
    /// 动态值转换的运行时类型检查。
    DynamicConversion,
    /// 动态算术的除零、溢出或操作数检查。
    Arithmetic,
    /// 动态选择器索引、范围端点或字符串位置检查。
    SelectorBounds,
    /// 动态选择器步长检查。
    SelectorStep,
    /// 动态随机抽取数量和候选集边界检查。
    RandomCount,
    /// 动态 `random.seed` 非负性和表示范围检查。
    RandomSeed,
    /// 动态集合元素的可哈希性检查。
    SetHashability,
    /// 动态集合成员判断的类型/容器检查。
    SetMembership,
    /// 动态集合代数操作数、结果或形状检查。
    SetOperation,
    /// 动态集合比较的成员/关系检查。
    SetComparison,
    /// 动态值作为 `if`/`while` 条件的布尔检查。
    BooleanCondition,
    /// 动态值参与 `for in` 迭代时的可迭代性检查。
    Iterable,
}

/// 一个带源码区间的运行时检查标记。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RuntimeCheck {
    /// 需要插入检查的源码区间。
    pub span: SourceSpan,
    /// 检查种类。
    pub kind: RuntimeCheckKind,
}

/// 一个类型化 AST 节点的旁路记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedNode {
    /// 节点源码区间。
    pub span: SourceSpan,
    /// 节点推断/检查后的类型。
    pub ty: Type,
}

/// 一次完整 P2 类型检查的结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeCheckResult {
    /// 按访问顺序保存的类型化节点，不改写原始 AST。
    pub nodes: Vec<TypedNode>,
    /// 累积的类型诊断。
    pub diagnostics: Vec<Diagnostic>,
    /// 需要由 Runtime 或后端执行的动态检查。
    pub runtime_checks: Vec<RuntimeCheck>,
    /// 检查结束时的环境快照，供后续 IR 阶段消费。
    pub environment: TypeEnvironment,
    /// 空数组路径声明对应的静态物化计划。
    pub materialization_plans: Vec<ContainerMaterializationPlan>,
    /// 有序容器选择器的规范化计划。
    pub selection_plans: Vec<crate::selection_model::SelectionPlan>,
    /// 选择器左值的事务性标量广播计划。
    pub broadcast_assignment_plans: Vec<crate::selection_model::BroadcastAssignmentPlan>,
    /// `random.seed` 调用的运行上下文种子计划。
    pub random_seed_plans: Vec<crate::selection_model::RandomSeedPlan>,
    /// 顶层程序入口模式。
    pub entry_mode: EntryMode,
    /// 已登记并完成推断的函数签名。
    pub function_signatures: BTreeMap<String, FunctionSignature>,
    /// 已登记并完成检查的表签名。
    pub table_signatures: BTreeMap<String, crate::tables::TableSignature>,
}

impl TypeCheckResult {
    /// 判断是否包含错误诊断。
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    /// 判断检查是否成功且没有错误。
    #[must_use]
    pub fn is_success(&self) -> bool {
        !self.has_errors()
    }

    /// 返回与源码区间完全匹配的最后一个类型记录。
    #[must_use]
    pub fn type_at(&self, span: SourceSpan) -> Option<&Type> {
        self.nodes
            .iter()
            .rev()
            .find(|node| node.span == span)
            .map(|node| &node.ty)
    }

    /// 返回某个规范化名称的最终绑定。
    #[must_use]
    pub fn binding(&self, name: &str) -> Option<&Binding> {
        self.environment
            .lookup(name)
            .or_else(|| self.environment.lookup(&format!("ascii:{name}")))
            .or_else(|| self.environment.lookup(&format!("backtick:{name}")))
    }

    /// 返回类型记录的只读视图。
    #[must_use]
    pub fn nodes(&self) -> &[TypedNode] {
        &self.nodes
    }

    /// 返回运行时检查标记的只读视图。
    #[must_use]
    pub fn runtime_checks(&self) -> &[RuntimeCheck] {
        &self.runtime_checks
    }

    /// 返回诊断的只读视图。
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// 返回容器默认值/形状计划的只读视图。
    #[must_use]
    pub fn materialization_plans(&self) -> &[ContainerMaterializationPlan] {
        &self.materialization_plans
    }

    /// 返回有序容器选择计划的只读视图。
    #[must_use]
    pub fn selection_plans(&self) -> &[crate::selection_model::SelectionPlan] {
        &self.selection_plans
    }

    /// 返回选择器广播赋值计划的只读视图。
    #[must_use]
    pub fn broadcast_assignment_plans(&self) -> &[crate::selection_model::BroadcastAssignmentPlan] {
        &self.broadcast_assignment_plans
    }

    /// 返回 `random.seed` 计划的只读视图。
    #[must_use]
    pub fn random_seed_plans(&self) -> &[crate::selection_model::RandomSeedPlan] {
        &self.random_seed_plans
    }

    /// 返回程序入口模式。
    #[must_use]
    pub const fn entry_mode(&self) -> EntryMode {
        self.entry_mode
    }

    /// 返回函数签名表的只读视图。
    #[must_use]
    pub fn function_signatures(&self) -> &BTreeMap<String, FunctionSignature> {
        &self.function_signatures
    }

    /// 返回表签名表的只读视图。
    #[must_use]
    pub fn table_signatures(&self) -> &BTreeMap<String, crate::tables::TableSignature> {
        &self.table_signatures
    }
}

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

    /// 检查一条顶层语句并更新名称环境。
    fn check_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Expression { expression, .. } => {
                self.check_expression(expression);
            }
            Statement::Assignment { target, value, .. } => {
                self.check_simple_assignment(*target, value);
            }
            Statement::ExtendedAssignment {
                target,
                operator,
                value,
                ..
            } => self.check_extended_assignment(target, *operator, value),
            Statement::Declaration {
                target,
                declared_type,
                constraint_path,
                value,
                ..
            } => self.check_declaration(
                *target,
                declared_type,
                constraint_path.as_ref(),
                value.as_ref(),
            ),
            Statement::ConstDeclaration {
                target,
                declared_type,
                value,
                ..
            } => self.check_const_declaration(*target, *declared_type, value),
            // 05-A/B 的导入解析由 `xiao-modules` 负责；当前类型检查器只保留
            // 语句位置，不把跨文件名称错误地当成本地动态值。
            Statement::Import { .. } => {}
            Statement::Function {
                name,
                parameters,
                return_type,
                body,
                span,
                ..
            } => self.check_function_statement(*name, parameters, *return_type, body, *span),
            Statement::Table {
                name,
                kind,
                body,
                span,
                ..
            } => self.check_table_statement(*name, *kind, body, *span),
            Statement::If {
                condition,
                body,
                elif_branches,
                else_body,
                ..
            } => self.check_if_statement(condition, body, elif_branches, else_body.as_deref()),
            Statement::For {
                target,
                iterable,
                body,
                ..
            } => self.check_for_statement(*target, iterable, body),
            Statement::While {
                condition, body, ..
            } => self.check_while_statement(condition, body),
            Statement::Return { value, span, .. } => {
                self.check_return_statement(value.as_ref(), *span)
            }
            Statement::Break { span, .. } => self.check_break_statement(*span),
            Statement::Continue { span, .. } => self.check_continue_statement(*span),
            Statement::Try {
                body,
                catches,
                finally_body,
                span,
                ..
            } => self.check_try_statement(body, catches, finally_body.as_deref(), *span),
            Statement::Raise { value, span, .. } => self.check_raise_statement(value, *span),
        }
    }

    /// 检查普通名称赋值，首次出现时建立单态绑定。
    fn check_simple_assignment(&mut self, target: xiao_syntax::Name, value: &Expression) {
        let value_type = self.check_expression(value);
        let key = self.name_key(target);
        if let Some(binding) = self.environment.lookup(&key).cloned() {
            let existing_type = self.context.instantiate(binding.scheme());
            if value_type.is_dynamic() {
                self.push_runtime_check(value.span(), RuntimeCheckKind::DynamicConversion);
            }
            if let (Type::Set(source), Type::Set(_target)) = (&value_type, &existing_type)
                && source.allows_dynamic()
            {
                // 集合静态成员已经在类型层验证；动态尾标只能由 Runtime
                // 完成成员类型/哈希检查，不能静默当作完全兼容。
                self.push_runtime_check(value.span(), RuntimeCheckKind::SetMembership);
            }
            if !self.types_compatible_for_assignment(&value_type, &existing_type) {
                self.type_error(
                    ASSIGNMENT_TYPE_MISMATCH_CODE,
                    "x02.type.assignment_mismatch",
                    target.span,
                    format!("不能把 {} 赋给已锁定的 {}", value_type, existing_type),
                );
            } else {
                if value_type.is_container() && !binding.container_constraints.is_empty() {
                    self.check_container_assignment_constraints(
                        &value_type,
                        &binding.container_constraints,
                        value.span(),
                    );
                }
                if let Err(error) = self.environment.assign(&key) {
                    self.environment_error(target.span, error);
                }
            }
            return;
        }
        if let Err(error) = self.environment.declare_mutable(key, value_type, true) {
            self.environment_error(target.span, error);
        }
    }

    /// 检查复合赋值并验证结果仍能写回左值类型槽。
    fn check_extended_assignment(
        &mut self,
        target: &Expression,
        operator: AssignmentOperator,
        value: &Expression,
    ) {
        if matches!(target, Expression::Selector { .. }) {
            self.check_selector_assignment(target, operator, value);
            return;
        }
        let right_type = self.check_expression(value);
        let Expression::Name(name) = target else {
            self.check_expression(target);
            self.type_error(
                INVALID_OPERANDS_CODE,
                "x02.type.non_name_assignment_target",
                target.span(),
                "P2 只支持标量名称的复合赋值".to_string(),
            );
            return;
        };
        let key = self.name_key(*name);
        let Some(binding) = self.environment.lookup(&key).cloned() else {
            self.undefined_name(*name);
            return;
        };
        let initialized = binding.initialized;
        if !initialized {
            self.type_error(
                UNINITIALIZED_READ_CODE,
                "x02.type.uninitialized_read",
                name.span,
                format!("名称 {} 在复合赋值前不能读取", self.display_name(*name)),
            );
        }
        let left_type = self.context.instantiate(binding.scheme());
        let binary = assignment_binary_operator(operator);
        let mut operation_valid = initialized;
        let mut dynamic_operation = false;
        let mut set_dynamic_boundary = false;
        let result_type = if let Some(binary) = binary {
            if should_attempt_set_semantics(binary, &left_type, &right_type) {
                let analysis =
                    self.check_set_operation_types(binary, &left_type, &right_type, target.span());
                operation_valid &= analysis.valid;
                dynamic_operation = analysis.result.is_dynamic();
                set_dynamic_boundary = analysis.has_dynamic_boundary;
                analysis.result
            } else {
                match (&left_type, &right_type) {
                    (Type::Dynamic, _) | (_, Type::Dynamic) => {
                        dynamic_operation = true;
                        self.push_runtime_check(target.span(), RuntimeCheckKind::Arithmetic);
                        Type::Dynamic
                    }
                    (Type::Scalar(left), Type::Scalar(right)) => {
                        match binary_scalar_type(binary, *left, *right) {
                            Ok(scalar) => Type::scalar(scalar),
                            Err(error) => {
                                operation_valid = false;
                                self.numeric_error(target.span(), binary, error);
                                Type::Dynamic
                            }
                        }
                    }
                    _ => {
                        operation_valid = false;
                        self.type_error(
                            INVALID_OPERANDS_CODE,
                            "x02.type.invalid_compound_operands",
                            target.span(),
                            format!("复合赋值不能作用于 {} 和 {}", left_type, right_type),
                        );
                        Type::Dynamic
                    }
                }
            }
        } else {
            operation_valid = false;
            self.type_error(
                INVALID_OPERANDS_CODE,
                "x02.type.invalid_compound_operands",
                target.span(),
                format!("复合赋值不能作用于 {} 和 {}", left_type, right_type),
            );
            Type::Dynamic
        };
        let constant_known = if operation_valid && !dynamic_operation {
            binary.is_some_and(|binary| {
                self.check_constant_binary_result(
                    binary,
                    target.span(),
                    target,
                    value,
                    &result_type,
                )
            })
        } else {
            false
        };
        if operation_valid
            && !dynamic_operation
            && !constant_known
            && let (Type::Scalar(left), Type::Scalar(right), Type::Scalar(result)) =
                (&left_type, &right_type, &result_type)
            && let Some(binary) = binary
            && binary_requires_runtime_check(binary, *left, *right, *result)
        {
            self.push_runtime_check(
                target.span(),
                if binary == BinaryOperator::Divide
                    || binary == BinaryOperator::FloorDivide
                    || binary == BinaryOperator::Remainder
                {
                    RuntimeCheckKind::Arithmetic
                } else {
                    RuntimeCheckKind::NumericRange
                },
            );
        }
        if operation_valid && set_dynamic_boundary && left_type.is_set() {
            // 集合运算的动态尾标必须在写回锁定左值时再次验证成员类型；
            // `SetOperation` 只描述运算形状本身。
            self.push_runtime_check(target.span(), RuntimeCheckKind::SetMembership);
        }
        if operation_valid
            && !dynamic_operation
            && !self.types_compatible_for_assignment(&result_type, &left_type)
        {
            self.type_error(
                ASSIGNMENT_TYPE_MISMATCH_CODE,
                "x02.type.compound_result_mismatch",
                target.span(),
                format!("复合赋值结果 {} 不符合 {}", result_type, left_type),
            );
        } else if operation_valid {
            if let Err(error) = self.environment.assign(&key) {
                self.environment_error(name.span, error);
            }
        }
    }

    /// 检查带显式标量类型的声明和可选初值。
    fn check_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: &DeclaredType,
        constraint_path: Option<&IndexPath>,
        value: Option<&Expression>,
    ) {
        match declared_type {
            DeclaredType::Set(annotation) => {
                self.check_set_declaration(target, annotation, constraint_path, value);
            }
            DeclaredType::Scalar(scalar) => {
                if !self.try_check_container_declaration(target, *scalar, constraint_path, value) {
                    self.check_scalar_declaration(target, *scalar, value);
                }
            }
        }
    }

    /// 检查带显式标量类型的声明和可选初值。
    fn check_scalar_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: ScalarType,
        value: Option<&Expression>,
    ) {
        let key = self.name_key(target);
        let value_type = value.map(|expression| self.check_expression(expression));
        if let Some(value) = value {
            if let Some(value_type) = &value_type {
                self.check_explicit_target(value, value_type, declared_type, true);
            }
        }
        if self.environment.contains_current(&key) {
            self.type_error(
                DUPLICATE_DECLARATION_CODE,
                "x02.type.duplicate_declaration",
                target.span,
                format!("名称 {} 在当前作用域中已经声明", self.display_name(target)),
            );
            return;
        }
        if let Err(error) =
            self.environment
                .declare_mutable(key, Type::scalar(declared_type), value.is_some())
        {
            self.environment_error(target.span, error);
        }
    }

    /// 检查编译期常量声明、求值和泛化方案。
    fn check_const_declaration(
        &mut self,
        target: xiao_syntax::Name,
        declared_type: Option<ScalarType>,
        value: &Expression,
    ) {
        let key = self.name_key(target);
        let diagnostics_before_value = self.diagnostics.len();
        let value_type = self.check_expression(value);
        if let Some(declared_type) = declared_type {
            // `const` 不能把运行时检查当成合法初始化；即使源值是
            // Dynamic，也只保留后面的 NON_CONSTANT 诊断。
            self.check_explicit_target(value, &value_type, declared_type, false);
        }
        let constant = self.eval_const(value);
        if let Some(constant) = constant {
            let stored = if let Some(target) = declared_type {
                convert_constant(constant.clone(), target)
            } else {
                Some(constant)
            };
            if let Some(stored) = stored {
                self.constant_values.insert(key.clone(), stored);
            }
        } else if !self.diagnostics[diagnostics_before_value..]
            .iter()
            .any(|diagnostic| {
                diagnostic.code() == ARITHMETIC_ERROR_CODE
                    || diagnostic.code() == INVALID_CONVERSION_CODE
            })
        {
            self.type_error(
                NON_CONSTANT_CODE,
                "x02.type.non_constant_initializer",
                value.span(),
                "const 的初始化表达式必须能在编译期求值".to_string(),
            );
        }
        if self.environment.contains_current(&key) {
            self.type_error(
                DUPLICATE_DECLARATION_CODE,
                "x02.type.duplicate_declaration",
                target.span,
                format!("名称 {} 在当前作用域中已经声明", self.display_name(target)),
            );
            return;
        }
        // 显式类型必须成为常量绑定的锁定类型，不能因为初始化表达式
        // 的推断类型更宽而丢失声明者的约束（例如 `const sint n = 1`）。
        let binding_type = declared_type
            .map(Type::scalar)
            .unwrap_or_else(|| value_type.clone());
        let scheme = self.context.generalize(&self.environment, &binding_type);
        if let Err(error) = self.environment.declare_constant(key, scheme) {
            self.environment_error(target.span, error);
        }
    }

    /// 检查表达式写入显式类型槽时的兼容性和范围。
    fn check_explicit_target(
        &mut self,
        expression: &Expression,
        source_type: &Type,
        target: ScalarType,
        allow_runtime_checks: bool,
    ) {
        if source_type.is_dynamic() {
            if allow_runtime_checks {
                self.push_runtime_check(expression.span(), RuntimeCheckKind::DynamicConversion);
            }
            return;
        }
        let target_type = Type::scalar(target);
        let constant = self.eval_const(expression);

        // 恒等和安全加宽可以直接写入目标槽；若目标是较窄的浮点，仍
        // 需要检查已知常量是否会变成无穷大。
        if can_assign(source_type, &target_type) {
            if let (Type::Scalar(source), Some(value)) = (source_type, constant.as_ref())
                && is_numeric(*source)
                && (is_integer(target) || is_float(target))
                && let Err(error) = self.check_constant_target(value, target)
            {
                self.numeric_error(expression.span(), BinaryOperator::Add, error);
            }
            return;
        }

        let conversion = match classify_conversion(source_type, target) {
            Ok(conversion) => conversion,
            Err(error) => {
                self.type_error(
                    INVALID_CONVERSION_CODE,
                    "x02.type.invalid_conversion",
                    expression.span(),
                    error.to_string(),
                );
                return;
            }
        };

        // 常量数值可以安全落入较窄的整数/浮点槽（但不能借此
        // 偷渡隐式的浮点到整数截断）。这条例外只适用于声明初值，
        // 普通变量表达式仍必须显式写出转换。
        if matches!(conversion.kind, ConversionKind::NumericNarrowing)
            && let Some(value) = constant.as_ref()
        {
            match self.check_constant_target(value, target) {
                Ok(()) => return,
                Err(error) => {
                    self.numeric_error(expression.span(), BinaryOperator::Add, error);
                    return;
                }
            }
        }

        // `str -> bool` 的内容检查只有在已经写出显式转换时才会发生；
        // 直接声明仍是非法隐式转换。已知非法拼写应报告转换错误，
        // 而不是同时制造一个无关的运行时检查。
        if conversion.kind == ConversionKind::StrToBool
            && constant.is_some()
            && !self.static_conversion_value_is_valid(expression, target)
        {
            return;
        }

        self.type_error(
            ASSIGNMENT_TYPE_MISMATCH_CODE,
            "x02.type.implicit_conversion",
            expression.span(),
            format!("不能隐式把 {} 转换为 {}", source_type, target.as_str()),
        );
    }

    /// 递归检查表达式并记录其旁路类型。
    fn check_expression(&mut self, expression: &Expression) -> Type {
        let ty = match expression {
            Expression::Literal { kind, span } => self.literal_type(*kind, *span),
            Expression::Name(name) => self.check_name(*name),
            Expression::ArrayLiteral { .. }
            | Expression::TupleLiteral { .. }
            | Expression::DictTableLiteral { .. }
            | Expression::DictColumnLiteral { .. }
            | Expression::SetLiteral { .. } => self.check_container_expression(expression),
            Expression::Group { expression, .. } => self.check_expression(expression),
            Expression::Unary {
                operator,
                operand,
                span,
            } => self.check_unary(*operator, operand, *span),
            Expression::Binary {
                operator,
                left,
                right,
                span,
            } => self.check_binary(*operator, left, right, *span),
            Expression::Call {
                callee,
                arguments,
                span,
            } => self.check_call(callee, arguments, *span),
            Expression::NewCall {
                callee,
                arguments,
                span,
            } => self.check_new_call(callee, arguments, *span),
            Expression::Member {
                object,
                member,
                span,
            } => self.check_table_member_expression(object, *member, *span),
            Expression::Cast {
                expression,
                target,
                span,
            } => self.check_cast(expression, *target, *span),
            Expression::Selector {
                source,
                step,
                selector,
                span,
            } => self.check_container_selector(source, step.as_deref(), selector, *span),
        };
        self.nodes.push(TypedNode {
            span: expression.span(),
            ty: ty.clone(),
        });
        ty
    }

    /// 将字面量类别和文本映射为静态标量类型。
    fn literal_type(&mut self, kind: LiteralKind, span: SourceSpan) -> Type {
        match kind {
            LiteralKind::Integer => {
                let text = self.source.slice(span);
                match parse_integer_literal(text) {
                    Ok(value) if (i64::MIN as i128..=i64::MAX as i128).contains(&value) => {
                        Type::scalar(ScalarType::Int)
                    }
                    Ok(_) => Type::scalar(ScalarType::Lint),
                    Err(_error) if is_decimal_integer(text) => Type::scalar(ScalarType::Lint),
                    Err(error) => {
                        self.numeric_error(span, BinaryOperator::Add, error);
                        Type::Dynamic
                    }
                }
            }
            LiteralKind::Float => match parse_float_literal(self.source.slice(span)) {
                Ok(_) => Type::scalar(ScalarType::Float),
                Err(error) => {
                    self.numeric_error(span, BinaryOperator::Add, error);
                    Type::Dynamic
                }
            },
            LiteralKind::String => Type::scalar(ScalarType::Str),
            LiteralKind::Boolean => Type::scalar(ScalarType::Bool),
            LiteralKind::None => Type::None,
        }
    }

    /// 查找名称、检查初始化状态并实例化其类型方案。
    fn check_name(&mut self, name: xiao_syntax::Name) -> Type {
        let key = self.name_key(name);
        let Some(binding) = self.environment.lookup(&key).cloned() else {
            self.undefined_name(name);
            return Type::Dynamic;
        };
        if !binding.initialized {
            self.type_error(
                UNINITIALIZED_READ_CODE,
                "x02.type.uninitialized_read",
                name.span,
                format!("名称 {} 在赋值前不能读取", self.display_name(name)),
            );
        }
        let instantiated = self.context.instantiate(binding.scheme());
        self.context.apply(&instantiated)
    }

    /// 检查一元运算的操作数和结果类型。
    fn check_unary(
        &mut self,
        operator: UnaryOperator,
        operand: &Expression,
        span: SourceSpan,
    ) -> Type {
        let operand_type = self.check_expression(operand);
        if operand_type.is_dynamic() {
            self.push_runtime_check(span, RuntimeCheckKind::Arithmetic);
            return if operator == UnaryOperator::Not {
                Type::scalar(ScalarType::Bool)
            } else {
                Type::Dynamic
            };
        }
        if let Type::Variable(variable) = &operand_type {
            if operator == UnaryOperator::Not {
                let boolean = Type::scalar(ScalarType::Bool);
                if self
                    .context
                    .unify(&Type::Variable(*variable), &boolean)
                    .is_ok()
                {
                    return boolean;
                }
            } else {
                // 一元正负只能约束“数值族”；具体宽度由返回值、赋值或
                // 调用点继续统一。暂不把未知参数错误地降成 dynamic。
                return Type::Variable(*variable);
            }
        }
        match operator {
            UnaryOperator::Not if operand_type.is_bool() => Type::scalar(ScalarType::Bool),
            UnaryOperator::Plus | UnaryOperator::Minus if operand_type.is_numeric() => {
                let result = operand_type.clone();
                if let Type::Scalar(scalar) = result {
                    if let Some(value) = self.eval_const(operand) {
                        match eval_const_unary(operator, value) {
                            Some(Err(error)) => {
                                self.numeric_error(span, BinaryOperator::Add, error)
                            }
                            Some(Ok(value)) => {
                                if let Err(error) = self.check_constant_target(&value, scalar) {
                                    self.numeric_error(span, BinaryOperator::Add, error);
                                }
                            }
                            None => {}
                        }
                    } else if matches!(
                        scalar,
                        ScalarType::Sint | ScalarType::Int | ScalarType::Sfloat | ScalarType::Float
                    ) {
                        self.push_runtime_check(span, RuntimeCheckKind::NumericRange);
                    }
                }
                result
            }
            _ => {
                self.type_error(
                    INVALID_OPERANDS_CODE,
                    "x02.type.invalid_unary_operand",
                    span,
                    format!("一元运算 {} 不能作用于 {}", operator.as_str(), operand_type),
                );
                Type::Dynamic
            }
        }
    }

    /// 检查二元运算、数值提升和编译期算术错误。
    fn check_binary(
        &mut self,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
        span: SourceSpan,
    ) -> Type {
        if matches!(operator, BinaryOperator::In | BinaryOperator::NotIn) {
            return self.check_set_membership(operator, left, right, span);
        }
        let left_type = self.check_expression(left);
        let right_type = self.check_expression(right);
        if should_attempt_set_semantics(operator, &left_type, &right_type) {
            return self
                .check_set_operation_types(operator, &left_type, &right_type, span)
                .result;
        }
        if let Some(result) = self.infer_binary_with_variables(operator, &left_type, &right_type) {
            return result;
        }
        let mut operation_valid = false;
        let result = match (&left_type, &right_type) {
            (Type::Dynamic, _) | (_, Type::Dynamic) => {
                self.push_runtime_check(span, RuntimeCheckKind::Arithmetic);
                Type::Dynamic
            }
            (Type::Scalar(left), Type::Scalar(right)) => {
                match binary_scalar_type(operator, *left, *right) {
                    Ok(result) => {
                        operation_valid = true;
                        Type::scalar(result)
                    }
                    Err(error) => {
                        self.numeric_error(span, operator, error);
                        Type::Dynamic
                    }
                }
            }
            _ => {
                self.type_error(
                    INVALID_OPERANDS_CODE,
                    "x02.type.invalid_binary_operands",
                    span,
                    format!(
                        "运算 {} 不能作用于 {} 和 {}",
                        operator.as_str(),
                        left_type,
                        right_type
                    ),
                );
                Type::Dynamic
            }
        };
        let constant_known = if operation_valid {
            self.check_constant_binary_result(operator, span, left, right, &result)
        } else {
            false
        };
        if operation_valid
            && !constant_known
            && let (Type::Scalar(left), Type::Scalar(right), Type::Scalar(result)) =
                (&left_type, &right_type, &result)
            && binary_requires_runtime_check(operator, *left, *right, *result)
        {
            self.push_runtime_check(
                span,
                if operator == BinaryOperator::Divide
                    || operator == BinaryOperator::FloorDivide
                    || operator == BinaryOperator::Remainder
                {
                    RuntimeCheckKind::Arithmetic
                } else {
                    RuntimeCheckKind::NumericRange
                },
            );
        }
        result
    }

    /// 在函数签名仍含类型变量时建立最小的二元运算约束。
    ///
    /// 函数体通常先于调用点检查；如果这里把未知参数立即当成非法
    /// 操作数，合法的 `def f(x) -> int` 也无法由返回值或后续调用完成
    /// 推断。该辅助只处理可局部确定的标量约束，其余情况仍交给统一器
    /// 或最终的“无法推断”诊断，不把未知值静默改成动态类型。
    fn infer_binary_with_variables(
        &mut self,
        operator: BinaryOperator,
        left: &Type,
        right: &Type,
    ) -> Option<Type> {
        let left = self.context.apply(left);
        let right = self.context.apply(right);
        let left_variable = matches!(left, Type::Variable(_));
        let right_variable = matches!(right, Type::Variable(_));
        if !left_variable && !right_variable {
            return None;
        }

        let is_comparison = matches!(
            operator,
            BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::Less
                | BinaryOperator::LessEqual
                | BinaryOperator::Greater
                | BinaryOperator::GreaterEqual
                | BinaryOperator::Is
                | BinaryOperator::IsNot
        );
        if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
            let boolean = Type::scalar(ScalarType::Bool);
            let valid_left = !left_variable && left == boolean;
            let valid_right = !right_variable && right == boolean;
            if (valid_left || left_variable) && (valid_right || right_variable) {
                if left_variable {
                    self.context.unify(&left, &boolean).ok()?;
                }
                if right_variable {
                    self.context.unify(&right, &boolean).ok()?;
                }
                return Some(boolean);
            }
            return None;
        }

        if left_variable && right_variable {
            if is_comparison {
                self.context.unify(&left, &right).ok()?;
                return Some(Type::scalar(ScalarType::Bool));
            }
            if matches!(
                operator,
                BinaryOperator::Add
                    | BinaryOperator::Subtract
                    | BinaryOperator::Multiply
                    | BinaryOperator::Divide
                    | BinaryOperator::FloorDivide
                    | BinaryOperator::Remainder
                    | BinaryOperator::Power
            ) {
                self.context.unify(&left, &right).ok()?;
                return Some(self.context.apply(&left));
            }
            return None;
        }

        let (variable, known, variable_on_left) = if left_variable {
            (&left, &right, true)
        } else {
            (&right, &left, false)
        };
        let Type::Scalar(known_scalar) = known else {
            return is_comparison.then_some(Type::scalar(ScalarType::Bool));
        };
        let candidate = match operator {
            BinaryOperator::Add => {
                if *known_scalar == ScalarType::Str
                    || is_numeric(*known_scalar)
                    || (!variable_on_left && *known_scalar == ScalarType::Bool)
                {
                    if *known_scalar == ScalarType::Bool && variable_on_left {
                        None
                    } else if *known_scalar == ScalarType::Bool {
                        Some(ScalarType::Int)
                    } else {
                        Some(*known_scalar)
                    }
                } else {
                    None
                }
            }
            BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::FloorDivide
            | BinaryOperator::Remainder
            | BinaryOperator::Power => {
                if is_numeric(*known_scalar) {
                    Some(*known_scalar)
                } else if !variable_on_left
                    && *known_scalar == ScalarType::Bool
                    && matches!(operator, BinaryOperator::Subtract)
                {
                    Some(ScalarType::Int)
                } else {
                    None
                }
            }
            _ if is_comparison => Some(*known_scalar),
            _ => None,
        }?;

        let candidate_type = Type::scalar(candidate);
        self.context.unify(variable, &candidate_type).ok()?;
        let resolved_left = self.context.apply(&left);
        let resolved_right = self.context.apply(&right);
        if is_comparison {
            return Some(Type::scalar(ScalarType::Bool));
        }
        match (&resolved_left, &resolved_right) {
            (Type::Scalar(left), Type::Scalar(right)) => {
                binary_scalar_type(operator, *left, *right)
                    .ok()
                    .map(Type::scalar)
            }
            _ => Some(self.context.apply(&candidate_type)),
        }
    }

    /// 检查赋值兼容性；含 HM 类型变量时先统一，再使用固定转换矩阵。
    fn types_compatible_for_assignment(&mut self, source: &Type, target: &Type) -> bool {
        let source = self.context.apply(source);
        let target = self.context.apply(target);
        if !source.free_vars().is_empty() || !target.free_vars().is_empty() {
            self.context.unify(&source, &target).is_ok()
        } else {
            can_assign(&source, &target)
        }
    }

    /// 检查常量二元运算的求值错误及结果宽度；动态表达式留给后端
    /// 的运行时检查，不在这里假装已经完成求值。
    fn check_constant_binary_result(
        &mut self,
        operator: BinaryOperator,
        span: SourceSpan,
        left: &Expression,
        right: &Expression,
        result_type: &Type,
    ) -> bool {
        let (Some(left), Some(right)) = (self.eval_const(left), self.eval_const(right)) else {
            return false;
        };
        let Some(result) = eval_const_binary(operator, left, right) else {
            return false;
        };
        match result {
            Err(error) => {
                self.numeric_error(span, operator, error);
                true
            }
            Ok(value) => {
                if let Type::Scalar(target) = result_type
                    && let Err(error) = self.check_constant_target(&value, *target)
                {
                    self.numeric_error(span, operator, error);
                }
                true
            }
        }
    }

    /// 检查标量转换、预留内建函数和 HM 函数调用。
    fn check_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        if is_random_seed_callee(callee, self.source) {
            return self.check_random_seed_call(arguments, span);
        }
        if self.is_set_constructor(callee) {
            return self.check_set_constructor(arguments, span);
        }
        if self
            .function_callee_key(callee)
            .is_some_and(|key| self.function_signatures.contains_key(&key))
        {
            self.check_expression(callee);
            return self
                .check_known_function_call(callee, arguments, span)
                .unwrap_or(Type::Dynamic);
        }
        if let Some(target) = self.scalar_callee(callee) {
            if arguments.len() != 1 {
                self.type_error(
                    INVALID_OPERANDS_CODE,
                    "x02.type.conversion_arity",
                    span,
                    "标量转换函数必须接收一个参数".to_string(),
                );
                return Type::Dynamic;
            }
            let source_diagnostics = self.diagnostics.len();
            let source_type = self.check_expression(&arguments[0].value);
            if !self.static_conversion_value_is_valid(&arguments[0].value, target) {
                return Type::Dynamic;
            }
            if !self.static_target_range_is_valid(&arguments[0].value, target) {
                return Type::Dynamic;
            }
            let constant_known = self.eval_const(&arguments[0].value).is_some();
            match classify_conversion(&source_type, target) {
                Ok(conversion) => {
                    if conversion.requires_runtime_check
                        && !constant_known
                        && !self.has_errors_since(source_diagnostics)
                    {
                        self.push_runtime_check(
                            arguments[0].value.span(),
                            match conversion.kind {
                                ConversionKind::StrToBool => RuntimeCheckKind::StringBoolean,
                                ConversionKind::RuntimeChecked => {
                                    RuntimeCheckKind::DynamicConversion
                                }
                                _ => RuntimeCheckKind::NumericRange,
                            },
                        );
                    }
                    Type::scalar(target)
                }
                Err(error) => {
                    self.type_error(
                        INVALID_CONVERSION_CODE,
                        "x02.type.invalid_conversion",
                        arguments[0].value.span(),
                        error.to_string(),
                    );
                    Type::Dynamic
                }
            }
        } else if self.simple_callee_name(callee).as_deref() == Some("input") {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            Type::scalar(ScalarType::Str)
        } else if self.simple_callee_name(callee).as_deref() == Some("print") {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            Type::None
        } else {
            let callee_type = self.check_expression(callee);
            let argument_types = arguments
                .iter()
                .map(|argument| self.check_expression(&argument.value))
                .collect::<Vec<_>>();
            if let Type::Function {
                parameters,
                return_type,
            } = callee_type
            {
                if parameters.len() != argument_types.len() {
                    self.type_error(
                        INVALID_OPERANDS_CODE,
                        "x02.type.call_arity",
                        span,
                        format!(
                            "函数需要 {} 个参数，实际得到 {}",
                            parameters.len(),
                            argument_types.len()
                        ),
                    );
                } else {
                    for (expected, actual) in parameters.iter().zip(argument_types.iter()) {
                        if let Err(error) = self.context.unify(expected, actual) {
                            self.unification_error(span, error);
                        }
                    }
                }
                *return_type
            } else {
                Type::Dynamic
            }
        }
    }

    /// 检查构造调用；P2 暂只报告尚未开放的表构造语义。
    fn check_new_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        self.check_table_new_call(callee, arguments, span)
    }

    /// 检查 `as` 显式转换并记录必要的运行时检查。
    fn check_cast(
        &mut self,
        expression: &Expression,
        target: ScalarType,
        span: SourceSpan,
    ) -> Type {
        let source_diagnostics = self.diagnostics.len();
        let source_type = self.check_expression(expression);
        if !self.static_conversion_value_is_valid(expression, target) {
            return Type::Dynamic;
        }
        if !self.static_target_range_is_valid(expression, target) {
            return Type::Dynamic;
        }
        let constant_known = self.eval_const(expression).is_some();
        match classify_conversion(&source_type, target) {
            Ok(conversion) => {
                if conversion.requires_runtime_check
                    && !constant_known
                    && !self.has_errors_since(source_diagnostics)
                {
                    self.push_runtime_check(
                        span,
                        match conversion.kind {
                            ConversionKind::StrToBool => RuntimeCheckKind::StringBoolean,
                            ConversionKind::RuntimeChecked => RuntimeCheckKind::DynamicConversion,
                            _ => RuntimeCheckKind::NumericRange,
                        },
                    );
                }
                Type::scalar(target)
            }
            Err(error) => {
                self.type_error(
                    INVALID_CONVERSION_CODE,
                    "x02.type.invalid_conversion",
                    span,
                    error.to_string(),
                );
                Type::Dynamic
            }
        }
    }

    /// 识别未被反引号包裹的标量转换构造器。
    fn scalar_callee(&self, callee: &Expression) -> Option<ScalarType> {
        let name = self.simple_callee_name(callee)?;
        ScalarType::from_name(&name)
    }

    /// 检查已知常量是否满足转换的内容约束；动态值留给运行时检查。
    fn static_conversion_value_is_valid(
        &mut self,
        expression: &Expression,
        target: ScalarType,
    ) -> bool {
        let Some(constant) = self.eval_const(expression) else {
            return true;
        };
        if target == ScalarType::Bool {
            if let ConstantValue::String(value) = &constant {
                if !matches!(value.as_str(), "true" | "True" | "false" | "False") {
                    self.type_error(
                        INVALID_CONVERSION_CODE,
                        "x02.type.invalid_string_boolean",
                        expression.span(),
                        "只有 true/True/false/False 可以转换为 bool".to_string(),
                    );
                    return false;
                }
            }
        }
        true
    }

    /// 检查编译期已知数值是否超出显式转换的目标范围。
    fn static_target_range_is_valid(
        &mut self,
        expression: &Expression,
        target: ScalarType,
    ) -> bool {
        let Some(constant) = self.eval_const(expression) else {
            return true;
        };
        let numeric_source = matches!(
            &constant,
            ConstantValue::Integer(_) | ConstantValue::BigInteger(_) | ConstantValue::Float(_)
        );
        if numeric_source && (is_integer(target) || is_float(target)) {
            if let Err(error) = self.check_explicit_constant_target(&constant, target) {
                self.numeric_error(expression.span(), BinaryOperator::Add, error);
                return false;
            }
        }
        true
    }

    /// 验证显式数值转换的常量边界；与隐式初始化不同，浮点到整数
    /// 在这里允许向零截断，只要截断后的结果仍在目标范围内。
    fn check_explicit_constant_target(
        &self,
        value: &ConstantValue,
        target: ScalarType,
    ) -> Result<(), NumericError> {
        match value {
            ConstantValue::Float(value) if is_integer(target) => {
                check_float_to_integer_range(*value, target)
            }
            _ => self.check_constant_target(value, target),
        }
    }

    /// 读取普通名称调用者文本；反引号名称不视为内建函数。
    fn simple_callee_name(&self, callee: &Expression) -> Option<String> {
        let Expression::Name(name) = callee else {
            return None;
        };
        if name.backticked {
            return None;
        }
        Some(name.unquoted_text(self.source).to_owned())
    }

    /// 返回任意名称调用者的规范化环境键；内建函数仍由
    /// [`Self::simple_callee_name`] 单独限制为普通 ASCII 名称。
    fn function_callee_key(&self, callee: &Expression) -> Option<String> {
        let Expression::Name(name) = callee else {
            return None;
        };
        Some(self.name_key(*name))
    }

    /// 纯递归求值一个已知编译期表达式，动态输入返回 `None`。
    fn eval_const(&self, expression: &Expression) -> Option<ConstantValue> {
        match expression {
            Expression::Literal { kind, span } => match kind {
                LiteralKind::Integer => {
                    let text = self.source.slice(*span);
                    parse_integer_literal(text)
                        .map(ConstantValue::Integer)
                        .ok()
                        .or_else(|| {
                            is_decimal_integer(text)
                                .then(|| ConstantValue::BigInteger(text.to_owned()))
                        })
                }
                LiteralKind::Float => parse_float_literal(self.source.slice(*span))
                    .ok()
                    .map(ConstantValue::Float),
                LiteralKind::String => {
                    decode_string(self.source.slice(*span)).map(ConstantValue::String)
                }
                LiteralKind::Boolean => {
                    Some(ConstantValue::Boolean(self.source.slice(*span) == "true"))
                }
                LiteralKind::None => Some(ConstantValue::None),
            },
            Expression::Name(name) => self.constant_values.get(&self.name_key(*name)).cloned(),
            Expression::Group { expression, .. } => self.eval_const(expression),
            Expression::Unary {
                operator, operand, ..
            } => {
                let value = self.eval_const(operand)?;
                match eval_const_unary(*operator, value) {
                    Some(Ok(value)) => Some(value),
                    Some(Err(_)) | None => None,
                }
            }
            Expression::Binary {
                operator,
                left,
                right,
                ..
            } => {
                let left = self.eval_const(left)?;
                let right = self.eval_const(right)?;
                match eval_const_binary(*operator, left, right) {
                    Some(Ok(value)) => Some(value),
                    Some(Err(_)) | None => None,
                }
            }
            Expression::Cast {
                expression, target, ..
            } => {
                let value = self.eval_const(expression)?;
                convert_constant(value, *target)
            }
            Expression::Call {
                callee, arguments, ..
            } => {
                let target = self.scalar_callee(callee)?;
                let value = self.eval_const(&arguments.first()?.value)?;
                convert_constant(value, target)
            }
            _ => None,
        }
    }

    /// 验证常量值能否落入指定标量类型的表示范围。
    fn check_constant_target(
        &self,
        value: &ConstantValue,
        target: ScalarType,
    ) -> Result<(), NumericError> {
        match value {
            ConstantValue::Integer(value) => {
                if is_integer(target) {
                    check_integer_range(*value, target)
                } else if is_float(target) {
                    check_float_range(*value as f64, target)
                } else {
                    Ok(())
                }
            }
            ConstantValue::BigInteger(_) => {
                if target == ScalarType::Lint {
                    Ok(())
                } else {
                    Err(NumericError::Overflow(target))
                }
            }
            ConstantValue::Float(value) => {
                if is_float(target) {
                    check_float_range(*value, target)
                } else if is_integer(target) {
                    Err(NumericError::InvalidLiteral(
                        "浮点值不能隐式转换为整数".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            ConstantValue::Boolean(_) if target == ScalarType::Bool => Ok(()),
            ConstantValue::String(_) if target == ScalarType::Str => Ok(()),
            _ => Ok(()),
        }
    }

    /// 生成区分普通/反引号名称的环境键。
    fn name_key(&self, name: xiao_syntax::Name) -> String {
        let prefix = if name.backticked {
            "backtick:"
        } else {
            "ascii:"
        };
        format!("{prefix}{}", name.unquoted_text(self.source))
    }

    /// 读取名称的原始源码文本用于诊断。
    fn display_name(&self, name: xiao_syntax::Name) -> String {
        self.source.slice(name.span).to_owned()
    }

    /// 追加未定义名称诊断。
    fn undefined_name(&mut self, name: xiao_syntax::Name) {
        self.type_error(
            UNDEFINED_NAME_CODE,
            "x02.type.undefined_name",
            name.span,
            format!("未定义名称 {}", self.display_name(name)),
        );
    }

    /// 将环境操作错误映射为稳定类型诊断。
    fn environment_error(&mut self, span: SourceSpan, error: EnvironmentError) {
        let (code, message_id) = match error {
            EnvironmentError::Duplicate(_) => {
                (DUPLICATE_DECLARATION_CODE, "x02.type.duplicate_declaration")
            }
            EnvironmentError::Unknown(_) => (UNDEFINED_NAME_CODE, "x02.type.undefined_name"),
            EnvironmentError::Immutable(_) => {
                (ASSIGNMENT_TYPE_MISMATCH_CODE, "x02.type.assign_immutable")
            }
        };
        self.type_error(code, message_id, span, error.to_string());
    }

    /// 将数值错误按操作数/算术类别映射为稳定诊断。
    fn numeric_error(&mut self, span: SourceSpan, operator: BinaryOperator, error: NumericError) {
        let code = if matches!(&error, NumericError::InvalidOperands { .. }) {
            INVALID_OPERANDS_CODE
        } else {
            ARITHMETIC_ERROR_CODE
        };
        self.type_error(
            code,
            "x02.type.arithmetic_error",
            span,
            format!("运算 {}：{}", operator.as_str(), error),
        );
    }

    /// 将 HM 统一失败映射为带源码区间的诊断。
    fn unification_error(&mut self, span: SourceSpan, error: UnifyError) {
        self.type_error(
            UNIFICATION_ERROR_CODE,
            "x02.type.unification_error",
            span,
            error.to_string(),
        );
    }

    /// 追加一条错误级别的结构化类型诊断。
    fn type_error(
        &mut self,
        code: &'static str,
        message_id: &'static str,
        span: SourceSpan,
        message: String,
    ) {
        self.type_error_with_params(code, message_id, span, message, []);
    }

    /// 保存新增诊断的稳定参数，展示文本仅作为当前阶段的预览。
    fn type_error_with_params(
        &mut self,
        code: &'static str,
        message_id: &'static str,
        span: SourceSpan,
        message: String,
        params: impl IntoIterator<Item = (String, DiagnosticParam)>,
    ) {
        self.diagnostics.push(
            Diagnostic::new(code, message_id, Severity::Error, Some(span), message)
                .with_params(params),
        );
    }

    /// 追加一个去重前的运行时检查标记。
    fn push_runtime_check(&mut self, span: SourceSpan, kind: RuntimeCheckKind) {
        self.runtime_checks.push(RuntimeCheck { span, kind });
    }

    /// 判断指定诊断索引之后是否已经出现错误；错误恢复表达式不再
    /// 额外注入一个看似可执行的运行时检查。
    fn has_errors_since(&self, start: usize) -> bool {
        self.diagnostics
            .get(start..)
            .is_some_and(|diagnostics| diagnostics.iter().any(Diagnostic::is_error))
    }
}

/// 判断调用者是否是内建的 `random.seed` 成员。
fn is_random_seed_callee(callee: &Expression, source: &SourceFile) -> bool {
    let Expression::Member { object, member, .. } = callee else {
        return false;
    };
    let Expression::Name(module) = object.as_ref() else {
        return false;
    };
    !module.backticked
        && !member.backticked
        && module.unquoted_text(source) == "random"
        && member.unquoted_text(source) == "seed"
}

/// 检查一个已解析程序的便捷函数。
#[must_use]
pub fn check(source: &SourceFile, program: &Program) -> TypeCheckResult {
    TypeChecker::check(source, program)
}

/// 将复合赋值运算符映射为普通二元运算符。
fn assignment_binary_operator(operator: AssignmentOperator) -> Option<BinaryOperator> {
    Some(match operator {
        AssignmentOperator::AddAssign => BinaryOperator::Add,
        AssignmentOperator::SubtractAssign => BinaryOperator::Subtract,
        AssignmentOperator::IntersectAssign => BinaryOperator::Intersect,
        AssignmentOperator::SymmetricDifferenceAssign => BinaryOperator::SymmetricDifference,
        AssignmentOperator::MultiplyAssign => BinaryOperator::Multiply,
        AssignmentOperator::DivideAssign => BinaryOperator::Divide,
        AssignmentOperator::FloorDivideAssign => BinaryOperator::FloorDivide,
        AssignmentOperator::RemainderAssign => BinaryOperator::Remainder,
        AssignmentOperator::PowerAssign => BinaryOperator::Power,
        AssignmentOperator::Assign => return None,
    })
}

/// 判断一个已经通过静态操作数检查的二元运算是否仍可能在运行时
/// 触发固定宽度溢出、非有限结果或除零。
fn binary_requires_runtime_check(
    operator: BinaryOperator,
    left: ScalarType,
    right: ScalarType,
    result: ScalarType,
) -> bool {
    if !matches!(
        operator,
        BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::FloorDivide
            | BinaryOperator::Remainder
            | BinaryOperator::Power
    ) {
        return false;
    }
    // 布尔奇偶加减和字符串拼接没有数值范围检查；除法族无论
    // 宽度如何都必须检查零除数。
    if left == ScalarType::Bool && is_integer(right) {
        return false;
    }
    if left == ScalarType::Str && right == ScalarType::Str {
        return false;
    }
    matches!(
        operator,
        BinaryOperator::Divide | BinaryOperator::FloorDivide | BinaryOperator::Remainder
    ) || matches!(
        result,
        ScalarType::Sint | ScalarType::Int | ScalarType::Sfloat | ScalarType::Float
    )
}

/// 解码 P2 支持的短字符串转义。
fn decode_string(text: &str) -> Option<String> {
    let mut characters = text.chars();
    let quote = characters.next()?;
    if quote != '\'' && quote != '"' || characters.next_back()? != quote {
        return None;
    }
    let inner = &text[quote.len_utf8()..text.len() - quote.len_utf8()];
    let mut result = String::new();
    let mut escaped = false;
    for character in inner.chars() {
        if escaped {
            result.push(match character {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '\'' => '\'',
                '"' => '"',
                _ => return None,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            result.push(character);
        }
    }
    (!escaped).then_some(result)
}

/// 将已知常量转换为目标标量，失败时返回 `None`。
fn convert_constant(value: ConstantValue, target: ScalarType) -> Option<ConstantValue> {
    match (value, target) {
        (ConstantValue::Boolean(value), ScalarType::Str) => Some(ConstantValue::String(
            if value { "true" } else { "false" }.to_owned(),
        )),
        (ConstantValue::String(value), ScalarType::Bool) => match value.as_str() {
            "true" | "True" => Some(ConstantValue::Boolean(true)),
            "false" | "False" => Some(ConstantValue::Boolean(false)),
            _ => None,
        },
        (ConstantValue::Integer(value), target) if is_integer(target) => {
            check_integer_range(value, target).ok()?;
            Some(ConstantValue::Integer(value))
        }
        (ConstantValue::BigInteger(value), ScalarType::Lint) => {
            Some(ConstantValue::BigInteger(value))
        }
        (ConstantValue::Integer(value), target) if is_float(target) => {
            let value = value as f64;
            check_float_range(value, target).ok()?;
            Some(ConstantValue::Float(value))
        }
        (ConstantValue::Float(value), target) if is_integer(target) => {
            check_float_to_integer_range(value, target).ok()?;
            // `trunc` 已在范围检查中完成语义判定；`as i128` 在这里
            // 只作为暂存形式，最终的 lint 高精度表示仍由 Runtime 接管。
            Some(ConstantValue::Integer(value.trunc() as i128))
        }
        (ConstantValue::Float(value), target) if is_float(target) => {
            check_float_range(value, target).ok()?;
            Some(ConstantValue::Float(value))
        }
        (value, target) => {
            let matching = matches!(
                (&value, target),
                (ConstantValue::Boolean(_), ScalarType::Bool)
                    | (ConstantValue::String(_), ScalarType::Str)
            );
            matching.then_some(value)
        }
    }
}

/// 纯求值一个常量二元运算并报告溢出/除零。
fn eval_const_binary(
    operator: BinaryOperator,
    left: ConstantValue,
    right: ConstantValue,
) -> Option<Result<ConstantValue, NumericError>> {
    use std::cmp::Ordering;

    match operator {
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::FloorDivide
        | BinaryOperator::Remainder
        | BinaryOperator::Power => eval_numeric_binary(operator, left, right),
        BinaryOperator::Equal | BinaryOperator::NotEqual => {
            let equal = constants_equal(&left, &right)?;
            Some(Ok(ConstantValue::Boolean(
                if operator == BinaryOperator::Equal {
                    equal
                } else {
                    !equal
                },
            )))
        }
        BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => {
            let ordering = constants_ordering(&left, &right)?;
            let result = match operator {
                BinaryOperator::Less => ordering == Ordering::Less,
                BinaryOperator::LessEqual => ordering != Ordering::Greater,
                BinaryOperator::Greater => ordering == Ordering::Greater,
                BinaryOperator::GreaterEqual => ordering != Ordering::Less,
                _ => unreachable!("已限定为比较运算"),
            };
            Some(Ok(ConstantValue::Boolean(result)))
        }
        BinaryOperator::Is | BinaryOperator::IsNot => {
            let equal = constants_equal(&left, &right)?;
            Some(Ok(ConstantValue::Boolean(
                if operator == BinaryOperator::Is {
                    equal
                } else {
                    !equal
                },
            )))
        }
        BinaryOperator::And | BinaryOperator::Or => {
            let (ConstantValue::Boolean(left), ConstantValue::Boolean(right)) = (&left, &right)
            else {
                return None;
            };
            Some(Ok(ConstantValue::Boolean(
                if operator == BinaryOperator::And {
                    *left && *right
                } else {
                    *left || *right
                },
            )))
        }
        BinaryOperator::In | BinaryOperator::NotIn => None,
        BinaryOperator::Intersect | BinaryOperator::SymmetricDifference => None,
    }
}

/// 求值数值和字符串加法等算术运算；不支持的常量组合返回 `None`。
fn eval_numeric_binary(
    operator: BinaryOperator,
    left: ConstantValue,
    right: ConstantValue,
) -> Option<Result<ConstantValue, NumericError>> {
    if let (
        BinaryOperator::Add | BinaryOperator::Subtract,
        ConstantValue::Boolean(value),
        ConstantValue::Integer(amount),
    ) = (operator, &left, &right)
    {
        return Some(Ok(ConstantValue::Boolean(boolean_integer_adjust(
            *value, *amount,
        ))));
    }
    if operator == BinaryOperator::Add {
        if let (ConstantValue::String(left), ConstantValue::String(right)) = (&left, &right) {
            return Some(Ok(ConstantValue::String(format!("{left}{right}"))));
        }
    }

    if let (ConstantValue::Integer(left), ConstantValue::Integer(right)) = (&left, &right) {
        return Some(match operator {
            BinaryOperator::Add => left
                .checked_add(*right)
                .map(ConstantValue::Integer)
                .ok_or(NumericError::Overflow(ScalarType::Lint)),
            BinaryOperator::Subtract => left
                .checked_sub(*right)
                .map(ConstantValue::Integer)
                .ok_or(NumericError::Overflow(ScalarType::Lint)),
            BinaryOperator::Multiply => left
                .checked_mul(*right)
                .map(ConstantValue::Integer)
                .ok_or(NumericError::Overflow(ScalarType::Lint)),
            BinaryOperator::FloorDivide => {
                if *right == 0 {
                    Err(NumericError::DivisionByZero)
                } else {
                    left.checked_div(*right)
                        .map(ConstantValue::Integer)
                        .ok_or(NumericError::Overflow(ScalarType::Lint))
                }
            }
            BinaryOperator::Remainder => {
                if *right == 0 {
                    Err(NumericError::DivisionByZero)
                } else {
                    left.checked_rem(*right)
                        .map(ConstantValue::Integer)
                        .ok_or(NumericError::Overflow(ScalarType::Lint))
                }
            }
            BinaryOperator::Power if *right >= 0 && *right <= u32::MAX as i128 => left
                .checked_pow(*right as u32)
                .map(ConstantValue::Integer)
                .ok_or(NumericError::Overflow(ScalarType::Lint)),
            BinaryOperator::Power => return None,
            BinaryOperator::Divide => {
                if *right == 0 {
                    Err(NumericError::DivisionByZero)
                } else {
                    finite_float(*left as f64 / *right as f64)
                }
            }
            _ => return None,
        });
    }

    let left_float = constant_as_f64(&left)?;
    let right_float = constant_as_f64(&right)?;
    let result = match operator {
        BinaryOperator::Add => left_float + right_float,
        BinaryOperator::Subtract => left_float - right_float,
        BinaryOperator::Multiply => left_float * right_float,
        BinaryOperator::Divide => {
            if right_float == 0.0 {
                return Some(Err(NumericError::DivisionByZero));
            }
            left_float / right_float
        }
        BinaryOperator::Power => left_float.powf(right_float),
        // 这两个运算在类型检查阶段只接受整数；混合浮点常量不可达。
        BinaryOperator::FloorDivide | BinaryOperator::Remainder => return None,
        _ => return None,
    };
    Some(finite_float(result))
}

/// 纯求值一个常量一元运算，并保留溢出错误。
fn eval_const_unary(
    operator: UnaryOperator,
    value: ConstantValue,
) -> Option<Result<ConstantValue, NumericError>> {
    Some(match (operator, value) {
        (UnaryOperator::Plus, value @ ConstantValue::Integer(_))
        | (UnaryOperator::Plus, value @ ConstantValue::Float(_)) => Ok(value),
        (UnaryOperator::Minus, ConstantValue::Integer(value)) => value
            .checked_neg()
            .map(ConstantValue::Integer)
            .ok_or(NumericError::Overflow(ScalarType::Lint)),
        (UnaryOperator::Minus, ConstantValue::Float(value)) => finite_float(-value),
        (UnaryOperator::Not, ConstantValue::Boolean(value)) => Ok(ConstantValue::Boolean(!value)),
        _ => return None,
    })
}

/// 将有限的整数/浮点常量转换为双精度暂存值。
fn constant_as_f64(value: &ConstantValue) -> Option<f64> {
    let result = match value {
        ConstantValue::Integer(value) => *value as f64,
        ConstantValue::Float(value) => *value,
        _ => return None,
    };
    result.is_finite().then_some(result)
}

/// 判断两个常量是否相等；数值族按数值而不是枚举变体比较。
fn constants_equal(left: &ConstantValue, right: &ConstantValue) -> Option<bool> {
    if let (Some(left), Some(right)) = (constant_as_f64(left), constant_as_f64(right)) {
        return Some(left == right);
    }
    match (left, right) {
        (ConstantValue::BigInteger(left), ConstantValue::BigInteger(right)) => {
            Some(normalize_decimal(left) == normalize_decimal(right))
        }
        (ConstantValue::BigInteger(left), ConstantValue::Integer(right))
        | (ConstantValue::Integer(right), ConstantValue::BigInteger(left)) => {
            let normalized = normalize_decimal(left);
            Some(normalized.parse::<i128>().ok() == Some(*right))
        }
        (ConstantValue::Integer(_), ConstantValue::Float(_))
        | (ConstantValue::Float(_), ConstantValue::Integer(_)) => Some(false),
        (ConstantValue::Boolean(left), ConstantValue::Boolean(right)) => Some(left == right),
        (ConstantValue::String(left), ConstantValue::String(right)) => Some(left == right),
        (ConstantValue::None, ConstantValue::None) => Some(true),
        _ => None,
    }
}

/// 求两个可比较常量的顺序。
fn constants_ordering(left: &ConstantValue, right: &ConstantValue) -> Option<std::cmp::Ordering> {
    if let (Some(left), Some(right)) = (constant_as_f64(left), constant_as_f64(right)) {
        return left.partial_cmp(&right);
    }
    match (left, right) {
        (ConstantValue::String(left), ConstantValue::String(right)) => Some(left.cmp(right)),
        (ConstantValue::Boolean(left), ConstantValue::Boolean(right)) => Some(left.cmp(right)),
        _ => None,
    }
}

/// 去除十进制大整数的无意义前导零，便于常量相等比较。
fn normalize_decimal(value: &str) -> &str {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let trimmed = unsigned.trim_start_matches('0');
    if trimmed.is_empty() {
        "0"
    } else if value.starts_with('-') {
        // 当前字面量扫描只产生无符号大整数；该分支为后续负大整数
        // 常量保留语义位置，暂不分配新的字符串。
        value
    } else {
        trimmed
    }
}

/// 确保浮点结果有限。
fn finite_float(value: f64) -> Result<ConstantValue, NumericError> {
    value
        .is_finite()
        .then_some(ConstantValue::Float(value))
        .ok_or(NumericError::NonFinite)
}
