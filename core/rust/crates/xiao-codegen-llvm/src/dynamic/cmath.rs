//! cmath 动态 C ABI 接线；借读输入保持强引用，先释放临时输入再传播状态。
use super::{DynamicGenerator, VALUE_TYPE};
use crate::error::{CodegenError, Result};
use xiao_ir::{IrCallArgument, IrSpan};

impl<'a> DynamicGenerator<'a> {
    /// 求值与绑定分离；已初始化的独立out不与输入重叠，失败不读取out。
    pub(super) fn emit_cmath(
        &mut self,
        declaration: &xiao_intrinsics::IntrinsicDecl,
        arguments: &[IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        let order =
            crate::cmath::argument_order(declaration.id.get(), arguments).ok_or_else(|| {
                CodegenError::Unsupported {
                    feature: format!("{} 参数形态", declaration.public_name),
                    span: Some(span),
                }
            })?;
        let symbol = format!("xiao_runtime_{}", declaration.public_name);
        let types = vec!["ptr"; arguments.len() + 1].join(", ");
        self.declarations
            .insert(format!("declare i32 @{symbol}({types})"));
        let mut slots = Vec::new();
        for _ in arguments {
            let slot = self.next_temp();
            self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
            self.emit(format!("  store {VALUE_TYPE} zeroinitializer, ptr {slot}"));
            slots.push(slot);
        }
        let failed = self.next_label("cmath.arguments.fail");
        let ready = self.next_label("cmath.arguments.ready");
        self.push_error_context(failed.clone());
        for (arg, slot) in arguments.iter().zip(&slots) {
            let value = self.emit_expression(&arg.value)?;
            self.store_call_argument(&arg.value, &arg.value, &value, slot);
        }
        self.pop_error_context();
        self.emit(format!("  br label %{ready}"));
        self.terminated = true;
        self.emit_label(&failed);
        let release = self.value_release_symbol();
        for slot in slots.iter().rev() {
            self.emit(format!("  call void @{release}(ptr {slot})"));
        }
        self.emit(format!("  br label %{}", self.error_target()));
        self.terminated = true;
        self.emit_label(&ready);
        let out = self.next_temp();
        self.emit(format!("  {out} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} zeroinitializer, ptr {out}"));
        let inputs = order
            .iter()
            .map(|&i| format!("ptr {}", slots[i]))
            .collect::<Vec<_>>()
            .join(", ");
        let status = self.next_temp();
        self.emit(format!(
            "  {status} = call i32 @{symbol}({inputs}, ptr {out})"
        ));
        for slot in slots.iter().rev() {
            self.emit(format!("  call void @{release}(ptr {slot})"));
        }
        let ok = self.next_label("cmath.ok");
        let error = self.next_label("cmath.error");
        let fatal = self.next_label("cmath.abi.failure");
        self.emit(format!(
            "  switch i32 {status}, label %{fatal} [ i32 0, label %{ok} i32 6, label %{error} ]"
        ));
        self.terminated = true;
        self.emit_label(&fatal);
        self.emit(format!("  call void @{release}(ptr {out})"));
        self.emit("  call void @xiao_runtime_fatal_abi()".to_owned());
        self.emit("  unreachable".to_owned());
        self.terminated = true;
        self.emit_label(&error);
        self.emit(format!("  call void @{release}(ptr {out})"));
        self.emit(format!(
            "  call i32 @xiao_runtime_error_attach_span(i64 {}, i64 {})",
            span.start, span.end
        ));
        self.emit(format!("  br label %{}", self.error_target()));
        self.terminated = true;
        self.emit_label(&ok);
        let result = self.next_temp();
        self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {out}"));
        Ok(result)
    }
}
