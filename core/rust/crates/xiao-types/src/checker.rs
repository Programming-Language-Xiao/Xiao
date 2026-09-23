//! 面向 P2/S0 的静态类型检查器。
//!
//! 检查器只消费 `xiao-syntax` 的公开 AST 和源码区间，输出类型化结果、结构化
//! 诊断及需要后端插入的运行时检查标记；它绝不执行 Xiao 程序或修改原始 AST。

use std::collections::BTreeMap;

use xiao_diagnostics::Diagnostic;
use xiao_source::SourceFile;
use xiao_syntax::{BinaryOperator, Expression, LiteralKind, Program, ScalarType};

use crate::containers::ContainerMaterializationPlan;
use crate::conversion::{is_float, is_integer};
use crate::diagnostics::*;
use crate::environment::TypeEnvironment;
use crate::functions::FunctionSignature;
use crate::numeric::{
    ConstantValue, NumericError, check_float_range, check_float_to_integer_range,
    check_integer_range, is_decimal_integer, parse_float_literal, parse_integer_literal,
};
use crate::types::Type;
use crate::unify::TypeContext;

/// 常量求值和跨职责共享的无状态辅助。
#[path = "checker/constant.rs"]
mod constant;
/// C0 容器语义的子模块；保持主检查器只负责语句分派和标量规则。
#[path = "container_checker.rs"]
pub(crate) mod container_checker;
/// 04 条件、循环、返回和入口静态检查。
#[path = "control_checker.rs"]
mod control_checker;
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

use self::constant::{convert_constant, decode_string, eval_const_binary, eval_const_unary};

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
}

/// 检查一个已解析程序的便捷函数。
#[must_use]
pub fn check(source: &SourceFile, program: &Program) -> TypeCheckResult {
    TypeChecker::check(source, program)
}
