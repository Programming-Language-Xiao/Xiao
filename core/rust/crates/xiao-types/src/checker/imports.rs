//! 把已声明的模块接口接到作用域绑定；资源类型和位置专用参数不会在别名处丢失。

use xiao_source::SourceSpan;
use xiao_syntax::{CallArgument, Expression};

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
            return binding.with_imported_function(signature.clone());
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
        let signature = self.instantiate_function_signature(&signature);
        self.check_expression(callee);
        Some(self.check_function_arguments(&signature, arguments, span))
    }
}
