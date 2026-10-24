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
        let result = self.check_function_arguments(&signature, arguments, span);
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
                if let Expression::Name(name) = &argument.value {
                    let key = self.name_key(*name);
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
                    let consumed = handle_kind
                        .zip(handle_operation)
                        .map(|(kind, operation)| {
                            self.concurrency_handle_states
                                .entry(key.clone())
                                .or_insert_with(|| crate::concurrency::HandleConsumption::new(kind))
                                .apply(operation)
                        })
                        .is_some_and(|applied| !applied);
                    if consumed {
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
