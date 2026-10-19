//! 04-B 函数签名登记、推断和调用参数检查。
//!
//! 函数定义先登记占位签名，再检查函数体，因此递归和同一文件内的前向
//! 引用不会依赖源码出现顺序。这里仅生成静态类型约束；真正的调用栈、
//! 闭包对象和参数展开由后续 Runtime/IR 阶段负责。

use std::collections::{BTreeMap, BTreeSet};

use xiao_diagnostics::DiagnosticParam;
use xiao_source::SourceSpan;
use xiao_syntax::{
    CallArgument, CallArgumentKind, Expression, FunctionParameter, FunctionParameterKind,
    FunctionTypeAnnotation, Name, Statement, TypeTerm,
};

use crate::containers::{ArrayType, DictType};
use crate::diagnostics::*;
use crate::functions::{FunctionParameterSignature, FunctionSignature};
use crate::set_types::{SetType, hashability};
use crate::types::{Type, TypeScheme};

use super::{FunctionFrame, TypeChecker};

impl<'source> TypeChecker<'source> {
    /// 在程序开始时登记所有顶层函数的占位签名。
    pub(super) fn register_top_level_functions(&mut self, statements: &[Statement]) {
        for statement in statements {
            if let Statement::Function {
                name,
                type_parameters,
                parameters,
                return_type,
                span,
                ..
            } = statement
            {
                self.register_function_signature(
                    *name,
                    parameters,
                    return_type.clone(),
                    *span,
                    type_parameters,
                );
            }
        }
    }

    /// 建立一个函数签名并在当前环境绑定函数名称。
    pub(super) fn register_function_signature(
        &mut self,
        name: Name,
        parameters: &[FunctionParameter],
        return_annotation: Option<FunctionTypeAnnotation>,
        span: SourceSpan,
        type_parameters: &[Name],
    ) {
        let key = self.name_key(name);
        if let Some(existing) = self.function_signatures.get(&key) {
            if existing.span != span {
                self.type_error_with_params(
                    FUNCTION_DECLARATION_CODE,
                    "x04.type.duplicate_function",
                    name.span,
                    format!("函数 {} 已经声明", self.display_name(name)),
                    [(
                        "name".to_owned(),
                        DiagnosticParam::Text(self.display_name(name)),
                    )],
                );
            }
            // 同一 AST 节点可能先在预登记阶段出现，再在正式检查阶段
            // 再次经过这里；已有同跨度签名无需重复写入环境。
            return;
        }
        if self.environment.contains_current(&key) {
            self.type_error_with_params(
                FUNCTION_DECLARATION_CODE,
                "x04.type.duplicate_function",
                name.span,
                format!("函数 {} 已经声明", self.display_name(name)),
                [(
                    "name".to_owned(),
                    DiagnosticParam::Text(self.display_name(name)),
                )],
            );
            return;
        }
        for parameter in parameters {
            if let Some(annotation) = parameter.annotation.as_ref() {
                self.check_function_annotation_hashability(annotation);
            }
        }
        if let Some(annotation) = return_annotation.as_ref() {
            self.check_function_annotation_hashability(annotation);
        }
        let mut generic_types = BTreeMap::new();
        let mut generic_ids = Vec::with_capacity(type_parameters.len());
        for type_parameter in type_parameters {
            let id = self.context.fresh_variable();
            generic_ids.push(id);
            generic_types.insert(
                type_parameter.unquoted_text(self.source).to_owned(),
                Type::variable(id),
            );
        }
        let mut parameter_types = Vec::with_capacity(parameters.len());
        let mut signatures = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let ty = self.parameter_type(parameter, &generic_types);
            parameter_types.push(ty.clone());
            signatures.push(FunctionParameterSignature {
                name: self.name_key(parameter.name),
                kind: parameter.kind,
                ty,
                has_default: parameter.default.is_some(),
            });
        }
        let return_type = return_annotation
            .map(|annotation| annotation_type(&annotation, &generic_types))
            .unwrap_or_else(|| self.context.fresh_type());
        let function_type = Type::Function {
            parameters: parameter_types,
            return_type: Box::new(return_type.clone()),
        };
        let signature = FunctionSignature::new(key.clone(), signatures, return_type, span)
            .with_type_parameters(
                type_parameters
                    .iter()
                    .map(|name| name.unquoted_text(self.source).to_owned())
                    .collect::<Vec<_>>(),
            )
            .with_type_parameter_ids(generic_ids.clone());
        if let Err(error) = self.environment.declare_constant(
            key.clone(),
            if generic_ids.is_empty() {
                TypeScheme::monomorphic(function_type)
            } else {
                TypeScheme::quantified(generic_ids, function_type)
            },
        ) {
            self.environment_error(name.span, error);
            return;
        }
        self.function_signatures.insert(key, signature);
    }

    /// 资源形参在被调函数入口验证，覆盖动态模块和函数值调用。
    pub(super) fn check_resource_parameter_boundaries(&mut self, parameters: &[FunctionParameter]) {
        for parameter in parameters {
            let ty = if parameter.kind == FunctionParameterKind::VarKeywords {
                let Some(FunctionTypeAnnotation::Resource(resource)) = parameter.annotation else {
                    continue;
                };
                Type::Resource(resource)
            } else {
                let Some(binding) = self
                    .environment
                    .lookup_current(&self.name_key(parameter.name))
                else {
                    continue;
                };
                binding.scheme().ty.clone()
            };
            self.resource_parameters
                .push((parameter.name.span, parameter.kind, ty));
        }
    }

    /// 推断结束后再生成入口检查，包含由返回注解或调用推断出的资源形参。
    pub(super) fn finalize_resource_parameter_boundaries(&mut self) {
        for (span, parameter_kind, ty) in std::mem::take(&mut self.resource_parameters) {
            let ty = self.context.apply(&ty);
            let resource = match ty {
                Type::Resource(resource) => resource,
                Type::Array(ArrayType::Homogeneous { element, .. })
                    if parameter_kind == FunctionParameterKind::VarArgs =>
                {
                    let Type::Resource(resource) = *element else {
                        continue;
                    };
                    resource
                }
                _ => continue,
            };
            let kind = match parameter_kind {
                FunctionParameterKind::VarArgs => super::RuntimeCheckKind::ResourceArray,
                FunctionParameterKind::VarKeywords => super::RuntimeCheckKind::ResourceDictionary,
                _ => super::RuntimeCheckKind::ResourceType,
            };
            self.push_runtime_check_with_expected(span, kind, Type::Resource(resource));
        }
    }

    /// 根据参数声明建立参数绑定类型；可变参数以容器类型表示。
    fn parameter_type(
        &mut self,
        parameter: &FunctionParameter,
        generic_types: &BTreeMap<String, Type>,
    ) -> Type {
        let base = parameter
            .annotation
            .as_ref()
            .map(|annotation| annotation_type(annotation, generic_types))
            .unwrap_or_else(|| self.context.fresh_type());
        match parameter.kind {
            FunctionParameterKind::VarArgs => Type::Array(ArrayType::homogeneous(base)),
            FunctionParameterKind::VarKeywords => Type::DictTable(DictType::new(Vec::new())),
            FunctionParameterKind::PositionalOnly
            | FunctionParameterKind::PositionalOrKeyword
            | FunctionParameterKind::KeywordOnly => base,
        }
    }

    /// 校验函数头中已经具体写出的集合成员是否满足集合可哈希契约。
    /// 类型参数项保留到调用点实例化，不能在声明阶段武断拒绝。
    fn check_function_annotation_hashability(&mut self, annotation: &FunctionTypeAnnotation) {
        let FunctionTypeAnnotation::Set(set) = annotation else {
            return;
        };
        for term in &set.members {
            let ty = match term {
                TypeTerm::Scalar(scalar) => Type::scalar(*scalar),
                TypeTerm::None => Type::None,
                TypeTerm::Resource(resource) => Type::Resource(*resource),
                TypeTerm::TypeParameter(_) => continue,
            };
            if hashability(&ty) == crate::set_types::Hashability::Unhashable {
                self.type_error_with_params(
                    SET_UNHASHABLE_ELEMENT_CODE,
                    "x03.type.set_unhashable_element",
                    set.span,
                    format!("类型 {} 不能作为集合元素", ty),
                    [(
                        "actual_type".to_owned(),
                        DiagnosticParam::Text(ty.to_string()),
                    )],
                );
            }
        }
    }

    /// 检查一个函数定义的参数、默认值和缩进体，并写回推断签名。
    pub(super) fn check_function_statement(
        &mut self,
        name: Name,
        parameters: &[FunctionParameter],
        return_annotation: Option<FunctionTypeAnnotation>,
        body: &[Statement],
        span: SourceSpan,
        type_parameters: &[Name],
    ) {
        let key = self.name_key(name);
        if !self.function_signatures.contains_key(&key) {
            self.register_function_signature(
                name,
                parameters,
                return_annotation,
                span,
                type_parameters,
            );
        }
        let Some(signature) = self.function_signatures.get(&key).cloned() else {
            return;
        };
        if signature.span != span {
            // 预登记阶段已经报告同名函数冲突；不要把重复定义的函数体
            // 错误地检查成第一份签名的另一份实现。
            return;
        }

        let rigid_type_parameters = signature.type_parameter_ids.clone();
        self.context
            .push_rigid(rigid_type_parameters.iter().copied());
        let previous_function_key = self.current_function_key.replace(key.clone());

        // 默认值在函数定义环境中检查；参数名称的绑定在下面的函数作用域中建立。
        for (parameter, parameter_signature) in parameters.iter().zip(&signature.parameters) {
            if let Some(default) = &parameter.default {
                let default_type = self.check_expression(default);
                self.unify_or_report(
                    &parameter_signature.ty,
                    &default_type,
                    default.span(),
                    "x04.type.default_parameter_type",
                );
            }
        }

        let previous_frame = self.current_function.take();
        let previous_loop_depth = self.loop_depth;
        self.current_function = Some(FunctionFrame {
            return_type: signature.return_type.clone(),
            saw_return: false,
        });
        self.loop_depth = 0;
        self.environment.push_scope();
        for parameter_signature in &signature.parameters {
            if let Err(error) = self.environment.declare_mutable(
                parameter_signature.name.clone(),
                parameter_signature.ty.clone(),
                true,
            ) {
                self.environment_error(name.span, error);
            }
        }
        self.check_resource_parameter_boundaries(parameters);
        self.register_top_level_functions(body);
        for statement in body {
            self.check_statement(statement);
        }

        let frame = self.current_function.take().expect("函数检查上下文应存在");
        if !frame.saw_return {
            self.unify_or_report(
                &frame.return_type,
                &Type::None,
                span,
                "x04.type.implicit_none_return",
            );
        }
        let resolved_return = self.context.apply(&frame.return_type);
        let mut resolved_parameters = Vec::with_capacity(signature.parameters.len());
        for parameter in &signature.parameters {
            let resolved = self.context.apply(&parameter.ty);
            resolved_parameters.push(FunctionParameterSignature {
                name: parameter.name.clone(),
                kind: parameter.kind,
                ty: resolved,
                has_default: parameter.has_default,
            });
        }
        let updated = FunctionSignature::new(
            key.clone(),
            resolved_parameters,
            resolved_return.clone(),
            signature.span,
        )
        .with_type_parameters(signature.type_parameters.clone())
        .with_type_parameter_ids(signature.type_parameter_ids.clone());
        self.function_signatures.insert(key.clone(), updated);
        let function_type = Type::Function {
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| self.context.apply(&parameter.ty))
                .collect(),
            return_type: Box::new(resolved_return),
        };
        // 参数作用域可能遮蔽函数名；先退出局部作用域，再更新外层函数绑定。
        self.environment.pop_scope();
        let scheme = if signature.type_parameter_ids.is_empty() {
            TypeScheme::monomorphic(function_type)
        } else {
            TypeScheme::quantified(signature.type_parameter_ids.clone(), function_type)
        };
        let _ = self.environment.replace_scheme(&key, scheme);
        self.loop_depth = previous_loop_depth;
        self.current_function = previous_frame;
        self.current_function_key = previous_function_key;
        self.context.pop_rigid(rigid_type_parameters);
    }

    /// 在整份程序的约束收集完成后统一报告仍未解析的函数类型变量。
    ///
    /// 不能在单个函数体结束时立即报告：后续定义后的调用、赋值或递归
    /// 约束仍可能为该函数参数提供具体类型。此收尾步骤只生成诊断和
    /// 最新签名快照，不执行任何用户代码。
    pub(super) fn finalize_function_inference(&mut self) {
        let signatures = self
            .function_signatures
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for signature in signatures {
            let quantified = signature
                .type_parameter_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            let resolved_parameters = signature
                .parameters
                .iter()
                .map(|parameter| FunctionParameterSignature {
                    name: parameter.name.clone(),
                    kind: parameter.kind,
                    ty: self.context.apply(&parameter.ty),
                    has_default: parameter.has_default,
                })
                .collect::<Vec<_>>();
            let resolved_return = self.context.apply(&signature.return_type);
            for parameter in &resolved_parameters {
                if contains_unresolved_variable(&parameter.ty, &quantified) {
                    self.type_error_with_params(
                        FUNCTION_INFERENCE_CODE,
                        "x04.type.unresolved_parameter",
                        signature.span,
                        format!(
                            "函数 {} 的参数 {} 无法推断类型",
                            signature.name, parameter.name
                        ),
                        [
                            (
                                "function".to_owned(),
                                DiagnosticParam::Text(signature.name.clone()),
                            ),
                            (
                                "parameter".to_owned(),
                                DiagnosticParam::Text(parameter.name.clone()),
                            ),
                        ],
                    );
                }
            }
            if contains_unresolved_variable(&resolved_return, &quantified) {
                self.type_error_with_params(
                    FUNCTION_INFERENCE_CODE,
                    "x04.type.unresolved_return",
                    signature.span,
                    format!("函数 {} 的返回类型无法推断", signature.name),
                    [(
                        "function".to_owned(),
                        DiagnosticParam::Text(signature.name.clone()),
                    )],
                );
            }
            let updated = FunctionSignature::new(
                signature.name.clone(),
                resolved_parameters,
                resolved_return.clone(),
                signature.span,
            )
            .with_type_parameters(signature.type_parameters.clone())
            .with_type_parameter_ids(signature.type_parameter_ids.clone());
            self.function_signatures
                .insert(signature.name.clone(), updated);
            let function_type = Type::Function {
                parameters: signature
                    .parameters
                    .iter()
                    .map(|parameter| self.context.apply(&parameter.ty))
                    .collect(),
                return_type: Box::new(resolved_return),
            };
            let scheme = if signature.type_parameter_ids.is_empty() {
                TypeScheme::monomorphic(function_type)
            } else {
                TypeScheme::quantified(signature.type_parameter_ids.clone(), function_type)
            };
            let _ = self.environment.replace_scheme(&signature.name, scheme);
        }
    }

    /// 检查已知函数的完整调用参数列表，并返回其返回类型。
    pub(super) fn check_known_function_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Option<Type> {
        // 使用与声明/环境相同的规范化键，反引号名称不能被误当成
        // 普通 ASCII 内建函数，也不能在调用时丢失 Unicode 内容。
        let key = self.function_callee_key(callee)?;
        let signature = self.function_signatures.get(&key).cloned()?;
        let signature = if self.current_function_key.as_deref() == Some(key.as_str()) {
            signature
        } else {
            self.instantiate_function_signature(&signature)
        };
        Some(self.check_function_arguments(&signature, arguments, span))
    }

    /// 为一次调用创建独立的泛型签名实例。
    pub(super) fn instantiate_function_signature(
        &mut self,
        signature: &FunctionSignature,
    ) -> FunctionSignature {
        if signature.type_parameter_ids.is_empty() {
            return signature.clone();
        }
        let function = Type::Function {
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            return_type: Box::new(signature.return_type.clone()),
        };
        let instantiated = self.context.instantiate(&TypeScheme::quantified(
            signature.type_parameter_ids.clone(),
            function,
        ));
        let Type::Function {
            parameters,
            return_type,
        } = instantiated
        else {
            return signature.clone();
        };
        FunctionSignature::new(
            signature.name.clone(),
            signature
                .parameters
                .iter()
                .cloned()
                .zip(parameters)
                .map(|(parameter, ty)| FunctionParameterSignature { ty, ..parameter })
                .collect::<Vec<_>>(),
            *return_type,
            signature.span,
        )
        .with_type_parameters(signature.type_parameters.clone())
    }

    /// 使用同一参数匹配器检查顶层函数和去掉隐式接收者后的表方法。
    pub(super) fn check_function_arguments(
        &mut self,
        signature: &FunctionSignature,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        let mut used = BTreeSet::new();
        let mut positional_index = 0usize;
        let mut saw_keyword = false;
        let mut varargs_index = None;
        let mut varkwargs_index = None;
        for (index, parameter) in signature.parameters.iter().enumerate() {
            match parameter.kind {
                FunctionParameterKind::VarArgs => varargs_index = Some(index),
                FunctionParameterKind::VarKeywords => varkwargs_index = Some(index),
                _ => {}
            }
        }
        for argument in arguments {
            let actual = self.check_expression(&argument.value);
            match argument.kind {
                CallArgumentKind::Positional => {
                    if saw_keyword {
                        self.call_error(argument.span, "位置参数不能位于关键字参数之后");
                        continue;
                    }
                    while positional_index < signature.parameters.len()
                        && !signature.parameters[positional_index]
                            .kind
                            .accepts_positional()
                    {
                        positional_index += 1;
                    }
                    let target = if positional_index < signature.parameters.len()
                        && signature.parameters[positional_index].kind
                            != FunctionParameterKind::VarArgs
                    {
                        let index = positional_index;
                        positional_index += 1;
                        Some(index)
                    } else {
                        varargs_index
                    };
                    if let Some(index) = target {
                        if !used.insert(index) && Some(index) != varargs_index {
                            self.call_error(argument.span, "函数参数被重复传入");
                        } else {
                            let parameter = &signature.parameters[index];
                            // `*args` 在签名中以数组保存，但每个普通位置实参
                            // 只统一到数组的元素类型；否则 `f(1, 2)` 会被
                            // 错误地拿 `int` 与 `[int]` 比较。
                            let expected = if parameter.kind == FunctionParameterKind::VarArgs {
                                vararg_element_type(&parameter.ty)
                            } else {
                                parameter.ty.clone()
                            };
                            self.check_generic_set_hashability(
                                signature,
                                &expected,
                                &actual,
                                argument.span,
                            );
                            self.unify_or_report(
                                &expected,
                                &actual,
                                argument.span,
                                "x04.type.call_argument_type",
                            );
                        }
                    } else {
                        self.call_error(argument.span, "函数接收的参数数量不足以容纳该位置参数");
                    }
                }
                CallArgumentKind::Keyword => {
                    saw_keyword = true;
                    let Some(name) = argument.name else {
                        self.call_error(argument.span, "关键字参数缺少名称");
                        continue;
                    };
                    let parameter_key = self.name_key(name);
                    let target = signature
                        .parameters
                        .iter()
                        .enumerate()
                        .find(|(_, parameter)| parameter.name == parameter_key);
                    if let Some((index, parameter)) = target {
                        if parameter.kind == FunctionParameterKind::PositionalOnly {
                            self.call_error(argument.span, "位置专用参数不能使用关键字传入");
                        } else if parameter.kind == FunctionParameterKind::VarKeywords {
                            // `**kwargs` 的名称是函数体内绑定，不是一个
                            // 必须用同名关键字填充的普通参数；任意未知键
                            // 都由该可变参数接收。
                            used.insert(index);
                        } else if !used.insert(index) {
                            self.call_error(argument.span, "函数参数被重复传入");
                        } else {
                            self.check_generic_set_hashability(
                                signature,
                                &parameter.ty,
                                &actual,
                                argument.span,
                            );
                            self.unify_or_report(
                                &parameter.ty,
                                &actual,
                                argument.span,
                                "x04.type.call_argument_type",
                            );
                        }
                    } else if varkwargs_index.is_none() {
                        self.call_error(argument.span, "函数不存在该关键字参数");
                    }
                }
                CallArgumentKind::Star => {
                    let element = iterable_element_type(&actual);
                    if let Some(index) = varargs_index {
                        if let Some(element) = element {
                            let expected = vararg_element_type(&signature.parameters[index].ty);
                            self.unify_or_report(
                                &expected,
                                &element,
                                argument.span,
                                "x04.type.star_argument_type",
                            );
                        } else {
                            self.call_error(argument.span, "* 参数必须展开已知可迭代容器");
                        }
                    } else {
                        self.call_error(argument.span, "函数没有可变位置参数");
                    }
                }
                CallArgumentKind::DoubleStar => {
                    if !matches!(
                        actual,
                        Type::DictTable(_) | Type::DictColumn(_) | Type::Dynamic
                    ) {
                        self.call_error(argument.span, "** 参数必须展开字典表或字典列");
                    } else if varkwargs_index.is_none() {
                        self.call_error(argument.span, "函数没有可变关键字参数");
                    }
                }
            }
        }
        for (index, parameter) in signature.parameters.iter().enumerate() {
            if matches!(
                parameter.kind,
                FunctionParameterKind::VarArgs | FunctionParameterKind::VarKeywords
            ) || parameter.has_default
            {
                continue;
            }
            if !used.contains(&index) {
                self.call_error(span, "函数缺少必需参数");
            }
        }
        self.context.apply(&signature.return_type)
    }

    /// 在泛型集合调用点检查已知的不可哈希元素类型。
    fn check_generic_set_hashability(
        &mut self,
        signature: &FunctionSignature,
        expected: &Type,
        actual: &Type,
        span: SourceSpan,
    ) {
        if signature.type_parameters.is_empty() {
            return;
        }
        let generic_set_variables = signature
            .parameters
            .iter()
            .filter_map(|parameter| match &parameter.ty {
                Type::Set(set) => Some(set.member_types().flat_map(Type::free_vars)),
                _ => None,
            })
            .flatten()
            .collect::<BTreeSet<_>>();
        let expected_is_generic_set = matches!(expected, Type::Set(set) if set
            .member_types()
            .any(|member| !member.free_vars().is_empty()));
        let expected_is_generic_element = matches!(expected, Type::Variable(variable) if generic_set_variables.contains(variable));
        if !expected_is_generic_set && !expected_is_generic_element {
            return;
        }
        let members = if let Type::Set(set) = actual {
            set.member_types().collect::<Vec<_>>()
        } else if expected_is_generic_element {
            vec![actual]
        } else {
            Vec::new()
        };
        for member in members {
            if crate::set_types::hashability(member) == crate::set_types::Hashability::Unhashable {
                self.type_error(
                    SET_GENERIC_UNHASHABLE_CODE,
                    "x03.type.generic_set_unhashable",
                    span,
                    format!("泛型集合元素类型 {} 不可哈希", member),
                );
            }
        }
    }

    /// 统一两个类型并把失败映射为函数阶段诊断。
    pub(super) fn unify_or_report(
        &mut self,
        expected: &Type,
        actual: &Type,
        span: SourceSpan,
        message_id: &'static str,
    ) {
        // 资源类型采用名义子类型关系：`file`/`directory` 可以传给
        // `resource` 父参数，但不能把这条关系写进通用统一器。
        if matches!(expected, Type::Resource(_))
            && matches!(actual, Type::Resource(_))
            && crate::can_assign(actual, expected)
        {
            return;
        }
        if let Err(error) = self.context.unify(expected, actual) {
            if matches!(error, crate::unify::UnifyError::RigidTypeParameter { .. }) {
                self.type_error(
                    RIGID_TYPE_PARAMETER_CODE,
                    "x04.type.rigid_type_parameter",
                    span,
                    error.to_string(),
                );
            } else {
                self.type_error_with_params(
                    FUNCTION_CALL_CODE,
                    message_id,
                    span,
                    error.to_string(),
                    [
                        (
                            "expected".to_owned(),
                            DiagnosticParam::Text(expected.to_string()),
                        ),
                        (
                            "actual".to_owned(),
                            DiagnosticParam::Text(actual.to_string()),
                        ),
                    ],
                );
            }
        }
    }

    /// 追加一个调用参数诊断。
    fn call_error(&mut self, span: SourceSpan, message: &str) {
        self.type_error(
            FUNCTION_CALL_CODE,
            "x04.type.call_argument",
            span,
            message.to_string(),
        );
    }
}

/// 将函数注解转换为类型层标量/空值类型。
fn annotation_type(
    annotation: &FunctionTypeAnnotation,
    generic_types: &BTreeMap<String, Type>,
) -> Type {
    match annotation {
        FunctionTypeAnnotation::Scalar(scalar) => Type::scalar(*scalar),
        FunctionTypeAnnotation::None => Type::None,
        FunctionTypeAnnotation::Resource(resource) => Type::Resource(*resource),
        FunctionTypeAnnotation::Set(annotation) => Type::Set(SetType::heterogeneous(
            annotation
                .members
                .iter()
                .map(|term| match term {
                    TypeTerm::Scalar(scalar) => Type::scalar(*scalar),
                    TypeTerm::None => Type::None,
                    TypeTerm::Resource(resource) => Type::Resource(*resource),
                    TypeTerm::TypeParameter(name) => {
                        generic_types.get(name).cloned().unwrap_or(Type::Dynamic)
                    }
                })
                .collect::<Vec<_>>(),
        )),
        FunctionTypeAnnotation::TypeParameter(name) => {
            generic_types.get(name).cloned().unwrap_or(Type::Dynamic)
        }
    }
}

/// 判断类型中是否仍有未统一变量。
fn contains_unresolved_variable(ty: &Type, quantified: &BTreeSet<crate::types::TypeVarId>) -> bool {
    match ty {
        Type::Variable(variable) => !quantified.contains(variable),
        Type::Function {
            parameters,
            return_type,
        } => {
            parameters
                .iter()
                .any(|parameter| contains_unresolved_variable(parameter, quantified))
                || contains_unresolved_variable(return_type, quantified)
        }
        Type::Tuple(items) => items
            .iter()
            .any(|item| contains_unresolved_variable(item, quantified)),
        Type::Array(array) => match array {
            ArrayType::Homogeneous { element, .. } => {
                contains_unresolved_variable(element, quantified)
            }
            ArrayType::Heterogeneous { elements } => elements
                .iter()
                .any(|element| contains_unresolved_variable(element, quantified)),
            ArrayType::Unknown => false,
        },
        Type::DictTable(dictionary) | Type::DictColumn(dictionary) => dictionary
            .entries
            .iter()
            .any(|entry| contains_unresolved_variable(&entry.value, quantified)),
        Type::Set(set) => set
            .member_types()
            .any(|member| contains_unresolved_variable(member, quantified)),
        Type::Table(_) | Type::Resource(_) | Type::Scalar(_) | Type::None | Type::Dynamic => false,
    }
}

/// 从可迭代静态类型提取元素类型。
fn iterable_element_type(ty: &Type) -> Option<Type> {
    match ty {
        Type::Scalar(xiao_syntax::ScalarType::Str) => {
            Some(Type::scalar(xiao_syntax::ScalarType::Str))
        }
        Type::Array(ArrayType::Homogeneous { element, .. }) => Some((**element).clone()),
        Type::Array(ArrayType::Heterogeneous { elements }) => {
            Some(common_type(elements).unwrap_or(Type::Dynamic))
        }
        Type::Array(ArrayType::Unknown) => Some(Type::Dynamic),
        Type::Tuple(elements) => Some(common_type(elements).unwrap_or(Type::Dynamic)),
        Type::Set(set) => Some(common_type(&set.to_member_types()).unwrap_or(Type::Dynamic)),
        Type::DictTable(dictionary) | Type::DictColumn(dictionary) => Some(
            common_type(
                &dictionary
                    .entries
                    .iter()
                    .map(|entry| (*entry.value).clone())
                    .collect::<Vec<_>>(),
            )
            .unwrap_or(Type::Dynamic),
        ),
        Type::Dynamic => Some(Type::Dynamic),
        _ => None,
    }
}

/// 从一组元素类型中提取统一类型；异构集合只能退化为动态边界。
fn common_type(types: &[Type]) -> Option<Type> {
    let first = types.first()?.clone();
    types.iter().all(|ty| *ty == first).then_some(first)
}

/// 读取 `*args` 参数的元素类型。
fn vararg_element_type(ty: &Type) -> Type {
    match ty {
        Type::Array(ArrayType::Homogeneous { element, .. }) => (**element).clone(),
        _ => Type::Dynamic,
    }
}
