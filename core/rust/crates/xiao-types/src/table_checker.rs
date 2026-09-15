//! 05-C 表声明、成员访问和构造签名的静态检查。
//!
//! 本模块只建立表的静态接口和生命周期方法契约。它不会创建表实例，
//! 也不会执行 `init`/`drop`；运行时生命周期、逃逸分析和 RAII 由后续
//! Runtime/IR 阶段负责。将这些规则放在独立模块中，避免主检查器与表
//! 成员规则形成高度耦合。

use std::collections::btree_map::Entry;

use xiao_diagnostics::DiagnosticParam;
use xiao_source::SourceSpan;
use xiao_syntax::{
    CallArgument, CallArgumentKind, DeclaredType, Expression, FunctionParameter,
    FunctionParameterKind, FunctionTypeAnnotation, Name, ScalarType, Statement, TableKind,
    TypeTerm,
};

use crate::containers::{ArrayType, DictType};
use crate::diagnostics::{
    TABLE_CONSTRUCTOR_CODE, TABLE_DECLARATION_CODE, TABLE_INITIALIZER_CODE, TABLE_LIFECYCLE_CODE,
    TABLE_MEMBER_CODE, TABLE_VISIBILITY_CODE,
};
use crate::functions::{FunctionParameterSignature, FunctionSignature};
use crate::set_types::SetType;
use crate::tables::{TableMemberKind, TableMemberSignature, TableSignature, TableType};
use crate::types::{Type, TypeScheme};

use super::{FunctionFrame, TypeChecker};

/// 当前正在检查的表上下文；只保存静态名称，不代表运行时对象。
#[derive(Clone, Debug)]
pub(super) struct TableFrame {
    /// 当前表的规范化名称。
    pub(super) name: String,
    /// 当前表的声明形态。
    pub(super) kind: TableKind,
}

impl<'source> TypeChecker<'source> {
    /// 在普通函数登记后预登记所有顶层表及其成员占位签名。
    pub(super) fn register_top_level_tables(&mut self, statements: &[Statement]) {
        for statement in statements {
            let Statement::Table {
                name,
                kind,
                body,
                span,
                ..
            } = statement
            else {
                continue;
            };
            let table_name = name.unquoted_text(self.source).to_owned();
            let table_key = self.name_key(*name);
            if self.table_signatures.contains_key(&table_name)
                || self.environment.contains_current(&table_key)
            {
                self.type_error_with_params(
                    TABLE_DECLARATION_CODE,
                    "x05.type.duplicate_table",
                    name.span,
                    format!("表 {} 已经声明", table_name),
                    [("table".to_owned(), DiagnosticParam::Text(table_name))],
                );
                continue;
            }

            let mut signature = TableSignature::new(table_name.clone(), *kind, *span);
            self.collect_table_member_signatures(&mut signature, body);
            let table_type = match kind {
                TableKind::Singleton => TableType::singleton(table_name.clone()),
                TableKind::Instance => TableType::constructor(table_name.clone()),
            };
            if let Err(error) = self
                .environment
                .declare_constant(table_key, TypeScheme::monomorphic(Type::Table(table_type)))
            {
                self.environment_error(name.span, error);
                continue;
            }
            self.table_signatures.insert(table_name, signature);
        }
    }

    /// 收集表成员占位签名；真正的字段表达式和方法体在第二遍检查。
    fn collect_table_member_signatures(
        &mut self,
        signature: &mut TableSignature,
        body: &[Statement],
    ) {
        for statement in body {
            let (member_name, member) = match statement {
                Statement::Assignment { target, .. } => {
                    let key = self.name_key(*target);
                    (
                        key,
                        TableMemberSignature::field(
                            self.name_key(*target),
                            Type::Dynamic,
                            target.span,
                        ),
                    )
                }
                Statement::Declaration {
                    target,
                    declared_type,
                    ..
                } => {
                    let key = self.name_key(*target);
                    (
                        key,
                        TableMemberSignature::field(
                            self.name_key(*target),
                            declared_type_to_type(declared_type, &mut self.context),
                            target.span,
                        ),
                    )
                }
                Statement::ConstDeclaration {
                    target,
                    declared_type,
                    ..
                } => {
                    let key = self.name_key(*target);
                    (
                        key,
                        TableMemberSignature::field(
                            self.name_key(*target),
                            declared_type.map(Type::scalar).unwrap_or(Type::Dynamic),
                            target.span,
                        ),
                    )
                }
                Statement::Function {
                    name,
                    parameters,
                    return_type,
                    span,
                    ..
                } => {
                    let key = self.name_key(*name);
                    let function =
                        self.build_method_signature(key.clone(), parameters, *return_type, *span);
                    (key, TableMemberSignature::method(function))
                }
                other => {
                    self.type_error(
                        TABLE_MEMBER_CODE,
                        "x05.type.invalid_table_member",
                        other.span(),
                        "表体只能包含字段或 def 方法".to_owned(),
                    );
                    continue;
                }
            };
            match signature.members.entry(member_name) {
                Entry::Occupied(entry) => {
                    let member_name = entry.key().clone();
                    self.type_error_with_params(
                        TABLE_MEMBER_CODE,
                        "x05.type.duplicate_table_member",
                        member.span,
                        format!("表 {} 的成员 {} 重复", signature.name, member_name),
                        [
                            (
                                "table".to_owned(),
                                DiagnosticParam::Text(signature.name.clone()),
                            ),
                            ("member".to_owned(), DiagnosticParam::Text(member_name)),
                        ],
                    );
                }
                Entry::Vacant(entry) => {
                    entry.insert(member);
                }
            }
        }
    }

    /// 创建与普通函数一致的表方法签名。
    fn build_method_signature(
        &mut self,
        name: String,
        parameters: &[FunctionParameter],
        return_annotation: Option<FunctionTypeAnnotation>,
        span: SourceSpan,
    ) -> FunctionSignature {
        let mut signatures = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let base = parameter
                .annotation
                .map(annotation_type)
                .unwrap_or_else(|| self.context.fresh_type());
            let ty = match parameter.kind {
                FunctionParameterKind::VarArgs => Type::Array(ArrayType::homogeneous(base)),
                FunctionParameterKind::VarKeywords => Type::DictTable(DictType::new(Vec::new())),
                FunctionParameterKind::PositionalOnly
                | FunctionParameterKind::PositionalOrKeyword
                | FunctionParameterKind::KeywordOnly => base,
            };
            signatures.push(FunctionParameterSignature {
                name: self.name_key(parameter.name),
                kind: parameter.kind,
                ty,
                has_default: parameter.default.is_some(),
            });
        }
        let return_type = return_annotation
            .map(annotation_type)
            .unwrap_or_else(|| self.context.fresh_type());
        FunctionSignature::new(name, signatures, return_type, span)
    }

    /// 检查一个表的字段、方法和静态生命周期契约。
    pub(super) fn check_table_statement(
        &mut self,
        name: Name,
        kind: TableKind,
        body: &[Statement],
        _span: SourceSpan,
    ) {
        let table_name = name.unquoted_text(self.source).to_owned();
        let Some(mut signature) = self.table_signatures.get(&table_name).cloned() else {
            return;
        };
        let previous_table = self.current_table.replace(TableFrame {
            name: table_name.clone(),
            kind,
        });
        self.environment.push_scope();
        self.declare_table_fields(&signature);

        for statement in body {
            match statement {
                Statement::Assignment { target, value, .. } => {
                    self.check_table_field_assignment(&mut signature, *target, value);
                }
                Statement::Declaration {
                    target,
                    declared_type,
                    value,
                    ..
                } => {
                    self.check_table_field_declaration(
                        &mut signature,
                        *target,
                        declared_type,
                        value.as_ref(),
                    );
                }
                Statement::ConstDeclaration {
                    target,
                    declared_type,
                    value,
                    ..
                } => {
                    self.check_table_const_field(&mut signature, *target, *declared_type, value);
                }
                Statement::Function {
                    name,
                    parameters,
                    return_type,
                    body,
                    span,
                    ..
                } => self.check_table_method(
                    &mut signature,
                    *name,
                    parameters,
                    *return_type,
                    body,
                    *span,
                ),
                other => self.type_error(
                    TABLE_MEMBER_CODE,
                    "x05.type.invalid_table_member",
                    other.span(),
                    "表体只能包含字段或 def 方法".to_owned(),
                ),
            }
        }

        self.environment.pop_scope();
        self.current_table = previous_table;
        self.table_signatures.insert(table_name, signature);
    }

    /// 为表字段预先建立局部绑定，支持方法体中的静态查找。
    fn declare_table_fields(&mut self, signature: &TableSignature) {
        for member in signature.members.values() {
            if member.kind != TableMemberKind::Field {
                continue;
            }
            let declaration = self.name_key_from_member(&member.name);
            if let Err(error) = self.environment.declare_mutable(
                declaration,
                member.ty.clone(),
                !member.ty.is_dynamic(),
            ) {
                self.environment_error(member.span, error);
            }
        }
    }

    /// 检查无类型字段赋值，并把推导出的类型写回成员签名。
    fn check_table_field_assignment(
        &mut self,
        signature: &mut TableSignature,
        target: Name,
        value: &Expression,
    ) {
        let value_type = self.check_expression(value);
        self.require_pure_initializer(value);
        let key = self.name_key(target);
        if let Some(member) = signature.members.get_mut(&key) {
            member.ty = value_type.clone();
        }
        let _ = self
            .environment
            .replace_scheme(&key, TypeScheme::monomorphic(value_type));
        let _ = self.environment.mark_initialized(&key);
    }

    /// 检查带显式类型的字段声明。
    fn check_table_field_declaration(
        &mut self,
        signature: &mut TableSignature,
        target: Name,
        declared_type: &DeclaredType,
        value: Option<&Expression>,
    ) {
        let expected = declared_type_to_type(declared_type, &mut self.context);
        if let Some(value) = value {
            let actual = self.check_expression(value);
            self.require_pure_initializer(value);
            if !crate::conversion::can_assign(&actual, &expected)
                && !actual.is_dynamic()
                && !expected.is_dynamic()
            {
                self.type_error_with_params(
                    TABLE_MEMBER_CODE,
                    "x05.type.table_field_type_mismatch",
                    value.span(),
                    format!(
                        "字段 {} 的初始化类型 {} 不符合 {}",
                        self.display_name(target),
                        actual,
                        expected
                    ),
                    [
                        (
                            "member".to_owned(),
                            DiagnosticParam::Text(self.display_name(target)),
                        ),
                        (
                            "actual_type".to_owned(),
                            DiagnosticParam::Text(actual.to_string()),
                        ),
                        (
                            "expected_type".to_owned(),
                            DiagnosticParam::Text(expected.to_string()),
                        ),
                    ],
                );
            }
        }
        let key = self.name_key(target);
        if let Some(member) = signature.members.get_mut(&key) {
            member.ty = expected.clone();
        }
        let _ = self
            .environment
            .replace_scheme(&key, TypeScheme::monomorphic(expected));
        if value.is_some() {
            let _ = self.environment.mark_initialized(&key);
        }
    }

    /// 检查表内编译期常量字段。
    fn check_table_const_field(
        &mut self,
        signature: &mut TableSignature,
        target: Name,
        declared_type: Option<ScalarType>,
        value: &Expression,
    ) {
        let value_type = self.check_expression(value);
        self.require_pure_initializer(value);
        let field_type = declared_type
            .map(Type::scalar)
            .unwrap_or_else(|| value_type.clone());
        if let Some(declared_type) = declared_type {
            self.check_explicit_target(value, &value_type, declared_type, false);
        }
        let key = self.name_key(target);
        if let Some(member) = signature.members.get_mut(&key) {
            member.ty = field_type.clone();
        }
        let _ = self
            .environment
            .replace_scheme(&key, TypeScheme::monomorphic(field_type));
    }

    /// 检查表方法的 `self`、参数、返回值和 `init`/`drop` 契约。
    fn check_table_method(
        &mut self,
        signature: &mut TableSignature,
        name: Name,
        parameters: &[FunctionParameter],
        return_annotation: Option<FunctionTypeAnnotation>,
        body: &[Statement],
        span: SourceSpan,
    ) {
        let method_key = self.name_key(name);
        let Some(member) = signature.members.get(&method_key).cloned() else {
            return;
        };
        let Some(mut method_signature) = member.function else {
            return;
        };
        let method_name = name.unquoted_text(self.source);
        let is_init = method_name == "init";
        let is_drop = method_name == "drop";
        let valid_self = parameters.first().is_some_and(|parameter| {
            !parameter.name.backticked && parameter.name.unquoted_text(self.source) == "self"
        });
        if !valid_self {
            self.type_error(
                TABLE_LIFECYCLE_CODE,
                "x05.type.method_requires_self",
                name.span,
                format!("表方法 {} 的第一个参数必须是 self", method_name),
            );
        }
        if is_drop && parameters.len() != 1 {
            self.type_error(
                TABLE_LIFECYCLE_CODE,
                "x05.type.drop_arity",
                span,
                "drop(self) 不能接收除 self 外的参数".to_owned(),
            );
        }
        if (is_init || is_drop)
            && return_annotation
                .is_some_and(|annotation| annotation != FunctionTypeAnnotation::None)
        {
            self.type_error(
                TABLE_LIFECYCLE_CODE,
                "x05.type.lifecycle_return",
                span,
                format!("{} 方法的返回类型必须是 none", method_name),
            );
        }

        let previous_frame = self.current_function.take();
        let previous_loop_depth = self.loop_depth;
        let table_type = self
            .current_table
            .as_ref()
            .map(|frame| {
                Type::Table(if frame.kind == TableKind::Instance {
                    TableType::instance(frame.name.clone())
                } else {
                    TableType::singleton(frame.name.clone())
                })
            })
            .unwrap_or(Type::Dynamic);

        let mut local_parameters = method_signature.parameters.clone();
        if let Some(first) = local_parameters.first_mut() {
            first.ty = table_type.clone();
        }
        for (parameter, parameter_signature) in parameters.iter().zip(&local_parameters) {
            if let Some(default) = &parameter.default {
                let actual = self.check_expression(default);
                self.unify_or_report(
                    &parameter_signature.ty,
                    &actual,
                    default.span(),
                    "x05.type.method_default_parameter",
                );
            }
        }

        let return_type = if is_init || is_drop {
            Type::None
        } else {
            method_signature.return_type.clone()
        };
        self.current_function = Some(FunctionFrame {
            return_type: return_type.clone(),
            saw_return: false,
        });
        self.loop_depth = 0;
        self.environment.push_scope();
        for (index, parameter) in local_parameters.iter().enumerate() {
            let result = if index == 0 && valid_self {
                self.environment.declare_constant(
                    parameter.name.clone(),
                    TypeScheme::monomorphic(table_type.clone()),
                )
            } else {
                self.environment
                    .declare_mutable(parameter.name.clone(), parameter.ty.clone(), true)
            };
            if let Err(error) = result {
                self.environment_error(method_signature.span, error);
            }
        }
        self.register_top_level_functions(body);
        for statement in body {
            self.check_statement(statement);
        }
        let frame = self.current_function.take().expect("方法检查上下文应存在");
        if !frame.saw_return {
            self.unify_or_report(
                &frame.return_type,
                &Type::None,
                span,
                "x05.type.method_implicit_none_return",
            );
        }
        let resolved_return = self.context.apply(&frame.return_type);
        let resolved_parameters = local_parameters
            .iter()
            .map(|parameter| FunctionParameterSignature {
                name: parameter.name.clone(),
                kind: parameter.kind,
                ty: self.context.apply(&parameter.ty),
                has_default: parameter.has_default,
            })
            .collect::<Vec<_>>();
        method_signature = FunctionSignature::new(
            method_signature.name.clone(),
            resolved_parameters,
            resolved_return.clone(),
            method_signature.span,
        );
        self.environment.pop_scope();
        self.loop_depth = previous_loop_depth;
        self.current_function = previous_frame;
        if let Some(member) = signature.members.get_mut(&method_key) {
            member.ty = method_type_without_self(&method_signature);
            member.function = Some(method_signature);
        }
    }

    /// 检查成员表达式并返回字段或方法类型。
    pub(super) fn check_table_member_expression(
        &mut self,
        object: &Expression,
        member: Name,
        span: SourceSpan,
    ) -> Type {
        let object_type = self.check_expression(object);
        let Type::Table(table_type) = self.context.apply(&object_type) else {
            if object_type.is_dynamic() || matches!(object_type, Type::Variable(_)) {
                return Type::Dynamic;
            }
            self.type_error(
                crate::diagnostics::INVALID_OPERANDS_CODE,
                "x02.type.member_not_scalar",
                span,
                format!(
                    "类型 {} 没有成员 {}",
                    object_type,
                    self.display_name(member)
                ),
            );
            return Type::Dynamic;
        };
        let Some(signature) = self.table_signature(&table_type.name) else {
            self.type_error(
                TABLE_MEMBER_CODE,
                "x05.type.unknown_table",
                span,
                format!("未知表 {}", table_type.name),
            );
            return Type::Dynamic;
        };
        let member_key = self.name_key(member);
        let Some(member_signature) = signature.members.get(&member_key) else {
            self.type_error_with_params(
                TABLE_MEMBER_CODE,
                "x05.type.unknown_table_member",
                member.span,
                format!(
                    "表 {} 没有成员 {}",
                    table_type.name,
                    self.display_name(member)
                ),
                [
                    (
                        "table".to_owned(),
                        DiagnosticParam::Text(table_type.name.clone()),
                    ),
                    (
                        "member".to_owned(),
                        DiagnosticParam::Text(self.display_name(member)),
                    ),
                ],
            );
            return Type::Dynamic;
        };
        let inside = self.current_table.as_ref().is_some_and(|frame| {
            frame.name == table_type.name
                && match object {
                    Expression::Name(name) => {
                        !name.backticked && name.unquoted_text(self.source) == "self"
                    }
                    _ => true,
                }
        });
        if !member_signature.is_public() && !inside {
            self.type_error(
                TABLE_VISIBILITY_CODE,
                "x05.type.private_table_member",
                member.span,
                format!("表成员 {} 只能在表内部访问", self.display_name(member)),
            );
            return Type::Dynamic;
        }
        member_signature.ty.clone()
    }

    /// 检查 `new Table(...)` 并返回实例类型。
    pub(super) fn check_table_new_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        let Expression::Name(name) = callee else {
            self.check_expression(callee);
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            self.type_error(
                TABLE_CONSTRUCTOR_CODE,
                "x05.type.new_requires_table",
                span,
                "new 的目标必须是可实例化表".to_owned(),
            );
            return Type::Dynamic;
        };
        let table_name = name.unquoted_text(self.source).to_owned();
        let Some(signature) = self.table_signature(&table_name).cloned() else {
            self.check_expression(callee);
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            self.type_error(
                TABLE_CONSTRUCTOR_CODE,
                "x05.type.new_requires_table",
                span,
                format!("{} 不是可实例化表", table_name),
            );
            return Type::Dynamic;
        };
        self.check_expression(callee);
        if !signature.is_instantiable() {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            self.type_error(
                TABLE_CONSTRUCTOR_CODE,
                "x05.type.singleton_not_constructible",
                span,
                format!("单例表 {} 不能使用 new", table_name),
            );
            return Type::Dynamic;
        }
        let init = signature
            .members
            .values()
            .find(|member| {
                member.is_method()
                    && member
                        .function
                        .as_ref()
                        .is_some_and(|function| method_name(&function.name) == "init")
            })
            .and_then(|member| member.function.as_ref());
        if let Some(init) = init {
            self.check_constructor_arguments(init, arguments, span);
        } else {
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            if !arguments.is_empty() {
                self.type_error(
                    TABLE_CONSTRUCTOR_CODE,
                    "x05.type.constructor_arity",
                    span,
                    format!("表 {} 没有 init，只能无参构造", table_name),
                );
            }
        }
        Type::Table(TableType::instance(table_name))
    }

    /// 按位置/关键字检查 `init` 的构造参数，并验证必需参数是否齐全。
    fn check_constructor_arguments(
        &mut self,
        signature: &FunctionSignature,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) {
        let parameters = signature.parameters.get(1..).unwrap_or(&[]);
        let mut used = vec![false; parameters.len()];
        let mut positional = 0usize;
        let mut saw_keyword = false;
        for argument in arguments {
            let actual = self.check_expression(&argument.value);
            match argument.kind {
                CallArgumentKind::Positional => {
                    if saw_keyword {
                        self.constructor_error(argument.span, "位置参数不能位于关键字参数之后");
                        continue;
                    }
                    if positional >= parameters.len() {
                        self.constructor_error(argument.span, "init 接收的参数数量过多");
                        continue;
                    }
                    let parameter = &parameters[positional];
                    positional += 1;
                    used[positional - 1] = true;
                    self.unify_or_report(
                        &method_argument_type(parameter),
                        &actual,
                        argument.span,
                        "x05.type.constructor_argument_type",
                    );
                }
                CallArgumentKind::Keyword => {
                    saw_keyword = true;
                    let Some(name) = argument.name else {
                        self.constructor_error(argument.span, "构造关键字参数缺少名称");
                        continue;
                    };
                    let key = self.name_key(name);
                    let Some((index, parameter)) = parameters
                        .iter()
                        .enumerate()
                        .find(|(_, parameter)| parameter.name == key)
                    else {
                        self.constructor_error(argument.span, "init 不存在该关键字参数");
                        continue;
                    };
                    if used[index] {
                        self.constructor_error(argument.span, "构造参数被重复传入");
                        continue;
                    }
                    used[index] = true;
                    self.unify_or_report(
                        &method_argument_type(parameter),
                        &actual,
                        argument.span,
                        "x05.type.constructor_argument_type",
                    );
                }
                CallArgumentKind::Star | CallArgumentKind::DoubleStar => {
                    self.constructor_error(argument.span, "new 暂不支持展开构造参数");
                }
            }
        }
        for (index, parameter) in parameters.iter().enumerate() {
            if !used[index]
                && !parameter.has_default
                && !matches!(
                    parameter.kind,
                    FunctionParameterKind::VarArgs | FunctionParameterKind::VarKeywords
                )
            {
                self.constructor_error(span, "init 缺少必需构造参数");
            }
        }
    }

    /// 判断并报告字段初始化器是否为静态纯表达式。
    fn require_pure_initializer(&mut self, expression: &Expression) {
        if !self.is_pure_initializer(expression) {
            self.type_error(
                TABLE_INITIALIZER_CODE,
                "x05.type.dynamic_table_initializer",
                expression.span(),
                "表字段初始化器必须是静态纯表达式".to_owned(),
            );
        }
    }

    /// 递归判断表字段初始化器；允许字面量、常量、纯运算和容器。
    fn is_pure_initializer(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Literal { .. } => true,
            Expression::Name(name) => self
                .environment
                .lookup(&self.name_key(*name))
                .is_some_and(|binding| binding.constant),
            Expression::ArrayLiteral { elements, .. }
            | Expression::TupleLiteral { elements, .. }
            | Expression::SetLiteral { elements, .. } => elements
                .iter()
                .all(|element| self.is_pure_initializer(element)),
            Expression::DictTableLiteral { entries, .. }
            | Expression::DictColumnLiteral { entries, .. } => entries
                .iter()
                .all(|entry| self.is_pure_initializer(&entry.value)),
            Expression::Group { expression, .. }
            | Expression::Unary {
                operand: expression,
                ..
            }
            | Expression::Cast { expression, .. } => self.is_pure_initializer(expression),
            Expression::Binary { left, right, .. } => {
                self.is_pure_initializer(left) && self.is_pure_initializer(right)
            }
            Expression::Call {
                callee, arguments, ..
            } => {
                self.scalar_callee(callee).is_some()
                    && arguments.len() == 1
                    && self.is_pure_initializer(&arguments[0].value)
            }
            Expression::NewCall { .. }
            | Expression::Member { .. }
            | Expression::Selector { .. } => false,
        }
    }

    /// 按名称读取表签名，兼容调用方传入 `ascii:` 前缀。
    pub(super) fn table_signature(&self, name: &str) -> Option<&TableSignature> {
        self.table_signatures.get(name).or_else(|| {
            self.table_signatures
                .get(name.strip_prefix("ascii:").unwrap_or(name))
        })
    }

    /// 从成员签名名称恢复环境键。
    fn name_key_from_member(&self, name: &str) -> String {
        if name.starts_with("ascii:") || name.starts_with("backtick:") {
            name.to_owned()
        } else {
            format!("ascii:{name}")
        }
    }

    /// 构造错误参数统一入口。
    fn constructor_error(&mut self, span: SourceSpan, message: &str) {
        self.type_error(
            TABLE_CONSTRUCTOR_CODE,
            "x05.type.constructor_argument",
            span,
            message.to_owned(),
        );
    }
}

/// 将语法层声明降低为类型层类型。
fn declared_type_to_type(declared: &DeclaredType, _context: &mut super::TypeContext) -> Type {
    match declared {
        DeclaredType::Scalar(scalar) => Type::scalar(*scalar),
        DeclaredType::Set(annotation) => {
            let members = annotation
                .members
                .iter()
                .map(|term| match term {
                    TypeTerm::Scalar(scalar) => Type::scalar(*scalar),
                    TypeTerm::None => Type::None,
                })
                .collect::<Vec<_>>();
            Type::Set(SetType::heterogeneous(members))
        }
    }
}

/// 将函数类型注解降低为类型层类型。
fn annotation_type(annotation: FunctionTypeAnnotation) -> Type {
    match annotation {
        FunctionTypeAnnotation::Scalar(scalar) => Type::scalar(scalar),
        FunctionTypeAnnotation::None => Type::None,
    }
}

/// 返回方法的展示名称，忽略环境键前缀。
fn method_name(name: &str) -> &str {
    name.strip_prefix("ascii:")
        .or_else(|| name.strip_prefix("backtick:"))
        .unwrap_or(name)
}

/// 构造不包含隐式 `self` 的外部方法函数类型。
fn method_type_without_self(signature: &FunctionSignature) -> Type {
    let parameters = signature
        .parameters
        .get(1..)
        .unwrap_or(&[])
        .iter()
        .map(|parameter| parameter.ty.clone())
        .collect();
    Type::Function {
        parameters,
        return_type: Box::new(signature.return_type.clone()),
    }
}

/// 取方法参数的真实调用类型；可变参数只暴露元素类型。
fn method_argument_type(parameter: &FunctionParameterSignature) -> Type {
    match parameter.kind {
        FunctionParameterKind::VarArgs => match &parameter.ty {
            Type::Array(ArrayType::Homogeneous { element, .. }) => (**element).clone(),
            _ => Type::Dynamic,
        },
        _ => parameter.ty.clone(),
    }
}
