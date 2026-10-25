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
    FunctionParameterKind, FunctionTypeAnnotation, Name, Statement, TableKind, TypeTerm,
};

use crate::containers::{ArrayType, DictType};
use crate::diagnostics::{
    DICT_METHOD_RECEIVER_CODE, DICT_METHOD_UNKNOWN_CODE, DICT_METHOD_VALUE_CODE, NON_CONSTANT_CODE,
    SET_METHOD_RECEIVER_CODE, SET_METHOD_UNKNOWN_CODE, SET_METHOD_VALUE_CODE,
    TABLE_CONSTRUCTOR_CODE, TABLE_DECLARATION_CODE, TABLE_INITIALIZER_CODE, TABLE_LIFECYCLE_CODE,
    TABLE_MEMBER_CODE, TABLE_VISIBILITY_CODE,
};
use crate::environment::{Binding, NamespaceOrigin};
use crate::functions::{FunctionParameterSignature, FunctionSignature};
use crate::set_types::SetType;
use crate::tables::{TableMemberKind, TableMemberSignature, TableSignature, TableType};
use crate::types::{Type, TypeScheme};

use super::{FunctionFrame, TypeChecker};

/// `stdlib.set` 对外冻结的方法名称。
const SET_METHOD_NAMES: [&str; 8] = [
    "size",
    "is_empty",
    "add",
    "remove",
    "contains",
    "union",
    "intersection",
    "difference",
];

/// `stdlib.dict` 对外冻结的方法名称。
const DICT_METHOD_NAMES: [&str; 8] = [
    "get", "set", "remove", "contains", "keys", "values", "size", "is_empty",
];

/// 判断名称是否属于集合方法面；反引号名称保持普通成员语义。
fn is_set_method_name(checker: &TypeChecker<'_>, member: Name) -> bool {
    !member.backticked && SET_METHOD_NAMES.contains(&checker.display_name(member).as_str())
}

/// 判断名称是否属于字典方法面；反引号名称保持普通成员语义。
fn is_dict_method_name(checker: &TypeChecker<'_>, member: Name) -> bool {
    !member.backticked && DICT_METHOD_NAMES.contains(&checker.display_name(member).as_str())
}

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
                            declared_type
                                .as_ref()
                                .map(|declared| declared_type_to_type(declared, &mut self.context))
                                .unwrap_or(Type::Dynamic),
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
                    let function = self.build_method_signature(
                        key.clone(),
                        parameters,
                        return_type.clone(),
                        *span,
                    );
                    let mut member = TableMemberSignature::method(function.clone());
                    member.ty = method_type_without_self(&function);
                    (key, member)
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
                .as_ref()
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
            .map(|annotation| annotation_type(&annotation))
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
        self.push_scope();
        self.declare_table_fields(&signature);

        for statement in body {
            // 方法体从全局签名表查成员；及时发布已检查字段和先前方法，
            // 不能直到整个表结束才让 self 看见新的接口。
            self.table_signatures
                .insert(table_name.clone(), signature.clone());
            match statement {
                Statement::Assignment { target, value, .. } => {
                    self.check_table_field_assignment(&mut signature, *target, value);
                }
                Statement::Declaration {
                    target,
                    declared_type,
                    constraint_path,
                    value,
                    ..
                } => {
                    if matches!(declared_type, DeclaredType::Resource(_)) {
                        self.reject_resource_constraint_path(constraint_path.as_ref());
                    }
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
                    self.check_table_const_field(
                        &mut signature,
                        *target,
                        declared_type.as_ref(),
                        value,
                    );
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
                    return_type.clone(),
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

        self.pop_scope();
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
        if let (DeclaredType::Set(annotation), Type::Set(set)) = (declared_type, &expected) {
            self.check_set_annotation_members(set, annotation.span);
        }
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
        declared_type: Option<&DeclaredType>,
        value: &Expression,
    ) {
        let value_type = self.check_expression(value);
        let field_type = declared_type
            .map(|declared| declared_type_to_type(declared, &mut self.context))
            .unwrap_or_else(|| value_type.clone());
        if value_type.contains_resource_value() || field_type.contains_resource_value() {
            self.type_error(
                NON_CONSTANT_CODE,
                "x02.type.non_constant_initializer",
                value.span(),
                "资源及包含资源的容器不能用于 const 初始化".to_owned(),
            );
        } else {
            self.require_pure_initializer(value);
        }
        if let Some(DeclaredType::Scalar(declared_type)) = declared_type {
            self.check_explicit_target(value, &value_type, *declared_type, false);
        } else if matches!(declared_type, Some(DeclaredType::Set(_))) {
            self.type_error(
                NON_CONSTANT_CODE,
                "x02.type.non_constant_initializer",
                value.span(),
                "集合类型不能用于 const 初始化".to_owned(),
            );
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
        let previous_loop_concurrency_handles = std::mem::take(&mut self.loop_concurrency_handles);
        self.loop_depth = 0;
        self.push_scope();
        for (index, parameter) in local_parameters.iter().enumerate() {
            self.reset_concurrency_binding(&parameter.name);
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
            } else {
                self.bind_concurrency_handle(parameter.name.clone(), &parameter.ty, None);
            }
        }
        self.check_resource_parameter_boundaries(parameters);
        self.register_top_level_functions(body);
        let body_flow = self.check_statement_sequence(body);
        let _ = self.pop_scope_with_concurrency_flow(body_flow);
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
        self.loop_concurrency_handles = previous_loop_concurrency_handles;
        self.loop_depth = previous_loop_depth;
        self.current_function = previous_frame;
        if let Some(member) = signature.members.get_mut(&method_key) {
            member.ty = method_type_without_self(&method_signature);
            member.function = Some(method_signature);
        }
    }

    /// 返回集合方法的官方函数签名。
    fn set_method_signature(&self, member: Name) -> Option<FunctionSignature> {
        if !is_set_method_name(self, member) {
            return None;
        }
        self.module_functions
            .get("stdlib.set")
            .and_then(|members| members.get(&self.name_key(member)))
            .cloned()
    }

    /// 返回字典方法的官方函数签名。
    /// 返回字典方法在官方标准库中的函数签名。
    fn dict_method_signature(&self, member: Name) -> Option<FunctionSignature> {
        if !is_dict_method_name(self, member) {
            return None;
        }
        self.module_functions
            .get("stdlib.dict")
            .and_then(|members| members.get(&self.name_key(member)))
            .cloned()
    }

    /// 判断类型是否为字典表或字典列。
    /// 判断类型是否为固定形状或开放键集字典。
    fn is_dict_type(ty: &Type) -> bool {
        matches!(ty, Type::DictTable(_) | Type::DictColumn(_))
    }

    /// 将开放字典签名的接收者值类型与实际字典值类型统一。
    /// 将开放字典签名的值类型与表/列接收者统一。
    fn unify_dict_receiver(&mut self, expected: &Type, actual: &Type, span: SourceSpan) {
        let Type::DictTable(expected_dictionary) = expected else {
            self.unify_or_report(expected, actual, span, "x04.type.call_argument_type");
            return;
        };
        let Some(expected_value) = expected_dictionary.open_value() else {
            self.unify_or_report(expected, actual, span, "x04.type.call_argument_type");
            return;
        };
        let actual_dictionary = match actual {
            Type::DictTable(dictionary) | Type::DictColumn(dictionary) => dictionary,
            _ => {
                self.unify_or_report(expected, actual, span, "x04.type.call_argument_type");
                return;
            }
        };
        if let Some(actual_value) = actual_dictionary.open_value() {
            self.unify_or_report(
                expected_value,
                actual_value,
                span,
                "x04.type.call_argument_type",
            );
        } else {
            for entry in &actual_dictionary.entries {
                self.unify_or_report(
                    expected_value,
                    &entry.value,
                    span,
                    "x04.type.call_argument_type",
                );
            }
        }
    }

    /// 检查字典方法直接调用，并把接收者绑定为自由函数的第一个参数。
    pub(super) fn check_dict_method_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Option<Type> {
        let Expression::Member { object, member, .. } = callee else {
            return None;
        };
        let checked_object = self.check_expression(object);
        let object_type = self.context.apply(&checked_object);
        if !Self::is_dict_type(&object_type)
            && !matches!(object_type, Type::Dynamic | Type::Variable(_))
        {
            return None;
        }
        let Some(signature) = self.dict_method_signature(*member) else {
            if Self::is_dict_type(&object_type) {
                self.type_error(
                    DICT_METHOD_UNKNOWN_CODE,
                    "x03.type.dict_method_unknown",
                    member.span,
                    format!("字典没有方法 {}", self.display_name(*member)),
                );
                for argument in arguments {
                    self.check_expression(&argument.value);
                }
                return Some(Type::Dynamic);
            }
            return None;
        };
        if matches!(object_type, Type::Dynamic | Type::Variable(_)) {
            self.type_error(
                DICT_METHOD_RECEIVER_CODE,
                "x03.type.dict_method_requires_static_dict",
                object.span(),
                format!(
                    "字典方法 {} 的接收者必须是静态字典，实际为 {}",
                    self.display_name(*member),
                    object_type
                ),
            );
            for argument in arguments {
                self.check_expression(&argument.value);
            }
            return Some(Type::Dynamic);
        }
        let mut signature = self.instantiate_function_signature(&signature);
        if let Some(receiver) = signature
            .parameters
            .first()
            .map(|parameter| parameter.ty.clone())
        {
            self.unify_dict_receiver(&receiver, &object_type, object.span());
            signature.parameters.remove(0);
        }
        Some(self.check_function_arguments(&signature, arguments, span))
    }

    /// 检查集合方法直接调用，并把接收者绑定为自由函数的第一个参数。
    pub(super) fn check_set_method_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Option<Type> {
        let Expression::Member { object, member, .. } = callee else {
            return None;
        };
        let signature = self.set_method_signature(*member)?;
        let checked_object = self.check_expression(object);
        let object_type = self.context.apply(&checked_object);
        let object_span = object.span();
        match object_type {
            Type::Set(_) => {
                let mut signature = self.instantiate_function_signature(&signature);
                let receiver = signature
                    .parameters
                    .first()
                    .map(|parameter| parameter.ty.clone());
                if let Some(receiver) = receiver {
                    self.unify_or_report(
                        &receiver,
                        &object_type,
                        object_span,
                        "x04.type.call_argument_type",
                    );
                    signature.parameters.remove(0);
                }
                Some(self.check_function_arguments(&signature, arguments, span))
            }
            Type::Dynamic | Type::Variable(_) => {
                self.type_error(
                    SET_METHOD_RECEIVER_CODE,
                    "x03.type.set_method_requires_static_set",
                    object_span,
                    format!(
                        "集合方法 {} 的接收者必须是静态 set，实际为 {}",
                        self.display_name(*member),
                        object_type
                    ),
                );
                for argument in arguments {
                    self.check_expression(&argument.value);
                }
                Some(Type::Dynamic)
            }
            // 其它静态对象交回表方法和普通成员路径；表可以合法声明同名方法，
            // 不能因为集合方法探测而提前劫持它的既有诊断和调用语义。
            _ => None,
        }
    }

    /// 检查成员表达式并返回字段或方法类型。
    pub(super) fn check_table_member_expression(
        &mut self,
        object: &Expression,
        member: Name,
        span: SourceSpan,
    ) -> Type {
        if let Some(path) = self.imported_namespace_path(object) {
            self.check_expression(object);
            if let Some(signature) = self
                .module_functions
                .get(&path)
                .and_then(|members| members.get(&self.name_key(member)))
            {
                return Type::Function {
                    parameters: signature
                        .parameters
                        .iter()
                        .map(|parameter| parameter.ty.clone())
                        .collect(),
                    return_type: Box::new(signature.return_type.clone()),
                };
            }
            if self
                .module_functions
                .contains_key(&format!("{path}.{}", self.display_name(member)))
            {
                return Type::Dynamic;
            }
        }
        if let Some(path) = self.external_namespace_path(object) {
            self.check_expression(object);
            let member_name = self.display_name(member);
            if !member.backticked
                && self
                    .external_namespaces
                    .members
                    .get(&path)
                    .is_some_and(|members| members.contains(&member_name))
            {
                return Type::Dynamic;
            }
            self.type_error(
                crate::diagnostics::UNDEFINED_NAME_CODE,
                "x02.type.undefined_name",
                member.span,
                format!("包命名空间 {path} 中未定义名称 {member_name}"),
            );
            return Type::Dynamic;
        }
        let object_type = self.check_expression(object);
        if Self::is_dict_type(&self.context.apply(&object_type)) {
            if is_dict_method_name(self, member) {
                self.type_error(
                    DICT_METHOD_VALUE_CODE,
                    "x03.type.dict_method_value",
                    member.span,
                    format!(
                        "字典方法 {} 只能直接调用，不能作为值使用",
                        self.display_name(member)
                    ),
                );
            } else {
                self.type_error(
                    DICT_METHOD_UNKNOWN_CODE,
                    "x03.type.dict_method_unknown",
                    member.span,
                    format!("字典没有方法 {}", self.display_name(member)),
                );
            }
            return Type::Dynamic;
        }
        if matches!(
            self.context.apply(&object_type),
            Type::Dynamic | Type::Variable(_)
        ) && is_dict_method_name(self, member)
        {
            self.type_error(
                DICT_METHOD_RECEIVER_CODE,
                "x03.type.dict_method_requires_static_dict",
                object.span(),
                format!(
                    "字典方法 {} 的接收者必须是静态字典，实际为 {}",
                    self.display_name(member),
                    self.context.apply(&object_type)
                ),
            );
            return Type::Dynamic;
        }
        if let Type::Set(_) = self.context.apply(&object_type) {
            if is_set_method_name(self, member) {
                self.type_error(
                    SET_METHOD_VALUE_CODE,
                    "x03.type.set_method_value",
                    member.span,
                    format!(
                        "集合方法 {} 只能直接调用，不能作为值使用",
                        self.display_name(member)
                    ),
                );
            } else {
                self.type_error(
                    SET_METHOD_UNKNOWN_CODE,
                    "x03.type.set_method_unknown",
                    member.span,
                    format!("集合没有方法 {}", self.display_name(member)),
                );
            }
            return Type::Dynamic;
        }
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

    /// 从外部包命名空间表达式恢复完整点号路径。
    fn external_namespace_path(&self, expression: &Expression) -> Option<String> {
        match expression {
            Expression::Name(name) if !name.backticked => {
                let root = self.display_name(*name);
                (self.external_namespaces.members.contains_key(&root)
                    && self
                        .environment
                        .lookup(&self.name_key(*name))
                        .and_then(Binding::namespace_origin)
                        == Some(NamespaceOrigin::Package))
                .then_some(root)
            }
            Expression::Member { object, member, .. } if !member.backticked => {
                let path = format!(
                    "{}.{}",
                    self.external_namespace_path(object)?,
                    self.display_name(*member)
                );
                self.external_namespaces
                    .members
                    .contains_key(&path)
                    .then_some(path)
            }
            _ => None,
        }
    }

    /// 方法调用消费完整静态签名，并复用普通函数的缺省/关键字匹配规则。
    pub(super) fn check_table_method_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Option<Type> {
        let Expression::Member { object, member, .. } = callee else {
            return None;
        };
        let callable = self.check_expression(callee);
        let object_type = self
            .nodes
            .iter()
            .rev()
            .find(|node| node.span == object.span())
            .map(|node| self.context.apply(&node.ty))?;
        let Type::Table(table) = object_type else {
            return None;
        };
        let signature = self
            .table_signature(&table.name)?
            .members
            .get(&self.name_key(*member))?
            .function
            .clone()?;
        if !matches!(callable, Type::Function { .. }) {
            return Some(Type::Dynamic);
        }
        let mut bound = signature;
        if !bound.parameters.is_empty() {
            bound.parameters.remove(0);
        }
        Some(self.check_function_arguments(&bound, arguments, span))
    }

    /// 成员写入沿用成员可见性、算子和赋值兼容规则；方法不能被字段写入替换。
    pub(super) fn check_table_member_assignment(
        &mut self,
        target: &Expression,
        operator: xiao_syntax::AssignmentOperator,
        value: &Expression,
    ) {
        let expected = self.check_expression(target);
        if matches!(expected, Type::Function { .. }) {
            self.type_error(
                TABLE_MEMBER_CODE,
                "x05.type.assign_method",
                target.span(),
                "表方法不能被赋值覆盖".to_owned(),
            );
            self.check_expression(value);
            return;
        }
        let actual = if let Some(operator) = super::constant::assignment_binary_operator(operator) {
            self.check_expression(&Expression::Binary {
                operator,
                left: Box::new(target.clone()),
                right: Box::new(value.clone()),
                span: target.span(),
            })
        } else {
            self.check_expression(value)
        };
        if !actual.is_dynamic()
            && !expected.is_dynamic()
            && !self.types_compatible_for_assignment_at(&actual, &expected, target.span())
        {
            self.type_error(
                TABLE_MEMBER_CODE,
                "x05.type.table_field_type_mismatch",
                target.span(),
                format!("字段写入类型 {actual} 不符合 {expected}"),
            );
        }
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
        DeclaredType::Resource(resource) => Type::Resource(*resource),
        DeclaredType::Set(annotation) => {
            let members = annotation
                .members
                .iter()
                .map(|term| match term {
                    TypeTerm::Scalar(scalar) => Type::scalar(*scalar),
                    TypeTerm::None => Type::None,
                    TypeTerm::Resource(resource) => Type::Resource(*resource),
                    TypeTerm::TypeParameter(_) => Type::Dynamic,
                })
                .collect::<Vec<_>>();
            Type::Set(SetType::heterogeneous(members))
        }
        DeclaredType::Thread(element) => Type::thread(annotation_type(element)),
        DeclaredType::Task(element) => Type::task(annotation_type(element)),
        DeclaredType::Runtime => Type::Runtime,
        DeclaredType::Pool => Type::Pool,
        DeclaredType::Shared(element) => Type::shared(annotation_type(element)),
        DeclaredType::SharedMut(element) => Type::shared_mut(annotation_type(element)),
        DeclaredType::LockGuard(element) => Type::lock_guard(annotation_type(element)),
    }
}

/// 将函数类型注解降低为类型层类型。
fn annotation_type(annotation: &FunctionTypeAnnotation) -> Type {
    match annotation {
        FunctionTypeAnnotation::Scalar(scalar) => Type::scalar(*scalar),
        FunctionTypeAnnotation::None => Type::None,
        FunctionTypeAnnotation::Resource(resource) => Type::Resource(*resource),
        FunctionTypeAnnotation::Set(_) => Type::set(Type::Dynamic),
        FunctionTypeAnnotation::Array(element) => {
            Type::Array(ArrayType::homogeneous(annotation_type(element)))
        }
        FunctionTypeAnnotation::Dict(annotation) => {
            Type::DictTable(DictType::open(annotation_type(&annotation.value)))
        }
        FunctionTypeAnnotation::TypeParameter(_) => Type::Dynamic,
        FunctionTypeAnnotation::Thread(element) => Type::thread(annotation_type(element)),
        FunctionTypeAnnotation::Task(element) => Type::task(annotation_type(element)),
        FunctionTypeAnnotation::Runtime => Type::Runtime,
        FunctionTypeAnnotation::Pool => Type::Pool,
        FunctionTypeAnnotation::Shared(element) => Type::shared(annotation_type(element)),
        FunctionTypeAnnotation::SharedMut(element) => Type::shared_mut(annotation_type(element)),
        FunctionTypeAnnotation::LockGuard(element) => Type::lock_guard(annotation_type(element)),
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
