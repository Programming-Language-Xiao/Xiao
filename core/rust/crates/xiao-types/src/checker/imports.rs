//! 把已声明的模块接口接到作用域绑定；资源类型和位置专用参数不会在别名处丢失。

use xiao_source::SourceSpan;
use xiao_syntax::{CallArgument, Expression};

use crate::concurrency::check_boundary_argument_count;
use crate::concurrency::{ConcurrentHandleKind, HandleOperation};
use crate::diagnostics::{
    CONCURRENCY_HANDLE_MISUSE_CODE, CONCURRENCY_NOT_SEND_CODE, CONCURRENCY_ORDINARY_HANDLE_CODE,
    FUNCTION_CALL_CODE,
};
use crate::environment::Binding;
use crate::functions::FunctionSignature;
use crate::types::Type;

use super::TypeChecker;

impl TypeChecker<'_> {
    /// 为 from 导入补上真实函数接口或子模块身份。
    pub(super) fn typed_import_binding(
        &self,
        module: &str,
        member: &str,
        binding: Binding,
    ) -> Binding {
        if let Some(signature) = self
            .module_functions
            .get(module)
            .and_then(|members| members.get(member))
        {
            return binding
                .with_imported_function(signature.clone())
                .with_imported_function_module(module.to_owned());
        }
        let name = member.strip_prefix("ascii:").unwrap_or(member);
        let child = format!("{module}.{name}");
        if self.module_functions.contains_key(&child) {
            return binding.with_imported_module(child);
        }
        binding
    }

    /// 从当前可见绑定推导模块路径；局部遮蔽与赋值之后不再使用旧导入身份。
    pub(super) fn imported_namespace_path(&self, expression: &Expression) -> Option<String> {
        match expression {
            Expression::Name(name) => self
                .environment
                .lookup(&self.name_key(*name))?
                .imported_module()
                .map(str::to_owned),
            Expression::Member { object, member, .. } if !member.backticked => {
                let path = format!(
                    "{}.{}",
                    self.imported_namespace_path(object)?,
                    self.display_name(*member)
                );
                self.module_functions.contains_key(&path).then_some(path)
            }
            Expression::Group { expression, .. } => self.imported_namespace_path(expression),
            _ => None,
        }
    }

    /// 解析导入函数的完整签名，保留参数名称、种类及资源父子类型。
    pub(super) fn imported_function_signature(
        &self,
        expression: &Expression,
    ) -> Option<FunctionSignature> {
        match expression {
            Expression::Name(name) => self
                .environment
                .lookup(&self.name_key(*name))?
                .imported_function()
                .cloned(),
            Expression::Member { object, member, .. } => self
                .module_functions
                .get(&self.imported_namespace_path(object)?)?
                .get(&self.name_key(*member))
                .cloned(),
            Expression::Group { expression, .. } => self.imported_function_signature(expression),
            _ => None,
        }
    }

    /// 导入函数调用复用既有参数匹配器，同时记录 callee 的类型节点供降低器使用。
    pub(super) fn check_imported_function_call(
        &mut self,
        callee: &Expression,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Option<Type> {
        let signature = self.imported_function_signature(callee)?;
        let concurrent_module = self.imported_function_module(callee);
        let signature = self.instantiate_function_signature(&signature);
        self.check_expression(callee);
        let operation = signature
            .name
            .strip_prefix("ascii:")
            .unwrap_or(signature.name.as_str());
        let is_std_concurrent = matches!(
            concurrent_module.as_deref(),
            Some("std.concurrent" | "stdlib.concurrent")
        );
        let result = if is_std_concurrent && matches!(operation, "spawn" | "submit") {
            self.check_spawn_like_arguments(&signature, arguments, span)
        } else {
            self.check_function_arguments(&signature, arguments, span)
        };
        let result = self.check_concurrency_contract(
            &signature,
            arguments,
            span,
            result,
            concurrent_module.as_deref(),
        );
        if signature.name == "ascii:remove" || signature.name == "remove" {
            if let Some(argument) = arguments.first() {
                self.invalidate_dictionary_shape_after_mutation(&argument.value);
            }
        }
        Some(result)
    }

    /// 检查 `spawn`/`submit` 的真实函数值与边界参数。
    ///
    /// 签名表保留一个泛型返回占位，入口函数的参数列表必须从第二个实参
    /// 的实际 `Type::Function` 读取；尾部额外值仍按闭包边界值执行 Send 检查。
    fn check_spawn_like_arguments(
        &mut self,
        signature: &FunctionSignature,
        arguments: &[CallArgument],
        span: SourceSpan,
    ) -> Type {
        let mut actuals = Vec::with_capacity(arguments.len());
        for argument in arguments {
            actuals.push(self.check_expression(&argument.value));
            if argument.kind != xiao_syntax::CallArgumentKind::Positional {
                self.call_error(argument.span, "并发入口暂只接受位置参数");
            }
        }
        for (index, expected) in signature.parameters.iter().take(1).enumerate() {
            if let Some(actual) = actuals.get(index) {
                self.unify_or_report(
                    &expected.ty,
                    actual,
                    arguments[index].span,
                    "x04.type.call_argument_type",
                );
            } else {
                self.call_error(span, "并发入口缺少 Runtime/pool 或函数参数");
            }
        }
        let function_type = actuals.get(1).cloned().unwrap_or(Type::Dynamic);
        if let Type::Function {
            parameters,
            return_type,
        } = function_type
        {
            if let Some(expected_return) =
                signature
                    .parameters
                    .get(1)
                    .and_then(|parameter| match &parameter.ty {
                        Type::Function { return_type, .. } => Some(return_type.as_ref()),
                        _ => None,
                    })
            {
                self.unify_or_report(
                    expected_return,
                    &return_type,
                    arguments.get(1).map_or(span, |argument| argument.span),
                    "x04.type.call_argument_type",
                );
            }
            let boundary = actuals.iter().skip(2).collect::<Vec<_>>();
            if boundary.len() != parameters.len() {
                self.call_error(
                    span,
                    "并发入口的边界实参数量必须与函数形参数量完全一致；额外边界值不表示隐式闭包捕获",
                );
            }
            for (index, expected) in parameters.iter().enumerate() {
                if let Some(actual) = boundary.get(index) {
                    let argument_span = arguments
                        .get(index + 2)
                        .map_or(span, |argument| argument.span);
                    self.unify_or_report(
                        expected,
                        actual,
                        argument_span,
                        "x04.type.call_argument_type",
                    );
                }
            }
        } else {
            self.type_error(
                FUNCTION_CALL_CODE,
                "x04.type.call_argument_type",
                arguments.get(1).map_or(span, |argument| argument.span),
                "spawn/submit 的第二个参数必须是函数值".to_owned(),
            );
        }
        self.context.apply(&signature.return_type)
    }

    fn imported_function_module(&self, expression: &Expression) -> Option<String> {
        match expression {
            Expression::Name(name) => self
                .environment
                .lookup(&self.name_key(*name))?
                .imported_function_module()
                .map(str::to_owned),
            Expression::Member { object, .. } => self.imported_namespace_path(object),
            Expression::Group { expression, .. } => self.imported_function_module(expression),
            _ => None,
        }
    }

    /// 在真实导入调用点消费 22B 并发签名、能力表、参数上限和句柄状态。
    fn check_concurrency_contract(
        &mut self,
        signature: &FunctionSignature,
        arguments: &[CallArgument],
        span: SourceSpan,
        result: Type,
        concurrent_module: Option<&str>,
    ) -> Type {
        let is_std_concurrent = matches!(
            concurrent_module,
            Some("std.concurrent" | "stdlib.concurrent")
        );
        if !is_std_concurrent {
            return result;
        }
        let operation = signature
            .name
            .strip_prefix("ascii:")
            .unwrap_or(signature.name.as_str());
        let boundary_start = matches!(operation, "spawn" | "submit").then_some(2);
        if let Some(start) = boundary_start {
            let count = arguments.len().saturating_sub(start);
            if let Err(error) = check_boundary_argument_count(count) {
                self.type_error(
                    FUNCTION_CALL_CODE,
                    "x04.type.call_arity",
                    span,
                    format!(
                        "并发边界参数最多 {} 个，实际收到 {} 个",
                        error.limit, error.count
                    ),
                );
            }
            for argument in arguments.get(start..).unwrap_or_default() {
                let ty = self
                    .known_expression_type(&argument.value)
                    .unwrap_or(Type::Dynamic);
                if matches!(
                    ty,
                    Type::OrdinaryHandle
                        | Type::Runtime
                        | Type::Pool
                        | Type::Thread(_)
                        | Type::Task(_)
                        | Type::LockGuard(_)
                ) {
                    self.type_error(
                        CONCURRENCY_ORDINARY_HANDLE_CODE,
                        "x03.type.concurrency_ordinary_handle",
                        argument.value.span(),
                        "普通 Runtime 句柄不能跨并发边界传递".to_owned(),
                    );
                } else if !(crate::is_send(&ty)
                    || matches!(ty, Type::SharedMut(_)) && crate::can_share_mutably(&ty))
                {
                    self.type_error(
                        CONCURRENCY_NOT_SEND_CODE,
                        "x03.type.concurrency_value_not_send",
                        argument.value.span(),
                        format!("值类型 {} 不满足候选 A 的 Send 能力", ty),
                    );
                }
            }
        }

        if matches!(operation, "shared_read" | "shared_mut")
            && let Some(argument) = arguments.first()
        {
            let ty = self
                .known_expression_type(&argument.value)
                .unwrap_or(Type::Dynamic);
            if !(crate::is_send(&ty) && crate::is_sync(&ty)) {
                self.type_error(
                    CONCURRENCY_NOT_SEND_CODE,
                    "x03.type.concurrency_value_not_send",
                    argument.value.span(),
                    format!("共享句柄的值类型 {} 不满足候选 A 的 Send/Sync 能力", ty),
                );
            }
        }

        if matches!(operation, "join" | "detach" | "wait" | "cancel") {
            if let Some(argument) = arguments.first() {
                let ty = self
                    .known_expression_type(&argument.value)
                    .unwrap_or(Type::Dynamic);
                let valid = match operation {
                    "join" | "detach" => matches!(ty, Type::Thread(_)),
                    "wait" => matches!(ty, Type::Task(_)),
                    "cancel" => matches!(ty, Type::Thread(_) | Type::Task(_)),
                    _ => false,
                };
                if !valid {
                    self.type_error(
                        CONCURRENCY_HANDLE_MISUSE_CODE,
                        "x03.type.concurrency_handle_identity",
                        argument.value.span(),
                        format!("{} 的句柄类型不正确：{}", operation, ty),
                    );
                }
                if let Some(identities) = self.concurrency_handle_identity(&argument.value) {
                    let handle_kind = match ty {
                        Type::Thread(_) => Some(ConcurrentHandleKind::Thread),
                        Type::Task(_) => Some(ConcurrentHandleKind::Task),
                        _ => None,
                    };
                    let handle_operation = match operation {
                        "join" => Some(HandleOperation::Join),
                        "detach" => Some(HandleOperation::Detach),
                        "wait" => Some(HandleOperation::Wait),
                        "cancel" => Some(HandleOperation::Cancel),
                        _ => None,
                    };
                    let consumes = !matches!(operation, "cancel");
                    let loop_carried = !self.is_checking_unreachable()
                        && consumes
                        && self.is_loop_carried_concurrency_handle(&identities);
                    if loop_carried {
                        self.type_error(
                            FUNCTION_CALL_CODE,
                            "x04.type.call_argument",
                            argument.value.span(),
                            "循环可能重复消费同一个并发句柄；请在每次迭代内创建新句柄".to_owned(),
                        );
                    }
                    let consumed = if self.is_checking_unreachable() {
                        false
                    } else {
                        handle_kind
                            .zip(handle_operation)
                            .map(|(kind, handle_operation)| {
                                identities.iter().all(|identity| {
                                    self.concurrency_handle_states
                                        .entry(identity.clone())
                                        .or_insert_with(|| {
                                            crate::concurrency::HandleConsumption::new(kind)
                                        })
                                        .apply(handle_operation)
                                })
                            })
                            .is_some_and(|applied| !applied)
                    };
                    if consumed && !loop_carried {
                        self.type_error(
                            FUNCTION_CALL_CODE,
                            "x04.type.call_argument",
                            argument.value.span(),
                            "并发句柄已经被消费，不能再次使用".to_owned(),
                        );
                    }
                }
            }
        }
        if !matches!(operation, "join" | "detach" | "wait" | "cancel") {
            for &index in &signature.consumed_parameters {
                let Some(argument) = arguments.get(index) else {
                    continue;
                };
                let Some(key) = self.concurrency_binding_key(&argument.value) else {
                    continue;
                };
                let identities = self
                    .concurrency_handle_identity(&argument.value)
                    .unwrap_or_default();
                let already_consumed = !self.is_checking_unreachable()
                    && (self.consumed_concurrency_bindings.contains(&key)
                        || identities
                            .iter()
                            .any(|identity| self.consumed_concurrency_values.contains(identity)));
                let loop_carried = !self.is_checking_unreachable()
                    && self.is_loop_carried_concurrency_handle(&identities);
                if already_consumed || loop_carried {
                    self.type_error(
                        FUNCTION_CALL_CODE,
                        "x04.type.call_argument",
                        argument.value.span(),
                        if loop_carried {
                            "循环可能重复消费同一个并发值；请在每次迭代内创建新值".to_owned()
                        } else {
                            "并发值已经被消费，不能再次使用".to_owned()
                        },
                    );
                }
                if !self.is_checking_unreachable() {
                    self.mark_concurrency_binding_consumed(key);
                }
            }
        }
        if is_std_concurrent {
            self.concurrency_calls.push(crate::ConcurrencyCall {
                operation: operation.to_owned(),
                span,
                argument_spans: arguments
                    .iter()
                    .map(|argument| argument.value.span())
                    .collect(),
            });
        }
        result
    }
}
