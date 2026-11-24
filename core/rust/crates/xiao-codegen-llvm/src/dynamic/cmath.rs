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
        self.emit_borrowed_intrinsic(&symbol, "cmath", arguments, order, span, true, None)
    }

    pub(super) fn emit_time(
        &mut self,
        declaration: &xiao_intrinsics::IntrinsicDecl,
        arguments: &[IrCallArgument],
        span: IrSpan,
    ) -> Result<String> {
        let parameters = xiao_intrinsics::time_parameters(declaration.id)
            .expect("活动 time intrinsic 具有固定形参");
        let order =
            crate::cmath::intrinsic_argument_order(parameters, arguments).ok_or_else(|| {
                CodegenError::Unsupported {
                    feature: format!("{} 参数形态", declaration.public_name),
                    span: Some(span),
                }
            })?;
        let symbol = declaration
            .abi_binding
            .time_symbol()
            .expect("time C ABI 绑定");
        let has_out = declaration.signature.return_type != xiao_intrinsics::ValueType::None;
        self.emit_borrowed_intrinsic(
            symbol,
            "time",
            arguments,
            order,
            span,
            has_out,
            Some(self.options.capabilities),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_borrowed_intrinsic(
        &mut self,
        symbol: &str,
        label: &str,
        arguments: &[IrCallArgument],
        order: Vec<usize>,
        span: IrSpan,
        has_out: bool,
        capabilities: Option<u64>,
    ) -> Result<String> {
        let mut types = vec!["ptr"; arguments.len() + usize::from(has_out)];
        if capabilities.is_some() {
            types.push("i64");
        }
        let types = types.join(", ");
        self.declarations
            .insert(format!("declare i32 @{symbol}({types})"));
        let mut slots = Vec::new();
        for _ in arguments {
            let slot = self.next_temp();
            self.emit(format!("  {slot} = alloca {VALUE_TYPE}"));
            self.emit(format!("  store {VALUE_TYPE} zeroinitializer, ptr {slot}"));
            slots.push(slot);
        }
        let failed = self.next_label(&format!("{label}.arguments.fail"));
        let ready = self.next_label(&format!("{label}.arguments.ready"));
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
        let out = has_out.then(|| {
            let out = self.next_temp();
            self.emit(format!("  {out} = alloca {VALUE_TYPE}"));
            self.emit(format!("  store {VALUE_TYPE} zeroinitializer, ptr {out}"));
            out
        });
        let mut inputs = order
            .iter()
            .map(|&i| format!("ptr {}", slots[i]))
            .collect::<Vec<_>>();
        if let Some(out) = &out {
            inputs.push(format!("ptr {out}"));
        }
        if let Some(capabilities) = capabilities {
            inputs.push(format!("i64 {capabilities}"));
        }
        let inputs = inputs.join(", ");
        let status = self.next_temp();
        self.emit(format!("  {status} = call i32 @{symbol}({inputs})"));
        for slot in slots.iter().rev() {
            self.emit(format!("  call void @{release}(ptr {slot})"));
        }
        let ok = self.next_label(&format!("{label}.ok"));
        let error = self.next_label(&format!("{label}.error"));
        let fatal = self.next_label(&format!("{label}.abi.failure"));
        self.emit(format!(
            "  switch i32 {status}, label %{fatal} [ i32 0, label %{ok} i32 6, label %{error} ]"
        ));
        self.terminated = true;
        self.emit_label(&fatal);
        if let Some(out) = &out {
            self.emit(format!("  call void @{release}(ptr {out})"));
        }
        self.emit("  call void @xiao_runtime_fatal_abi()".to_owned());
        self.emit("  unreachable".to_owned());
        self.terminated = true;
        self.emit_label(&error);
        if let Some(out) = &out {
            self.emit(format!("  call void @{release}(ptr {out})"));
        }
        self.emit(format!(
            "  call i32 @xiao_runtime_error_attach_span(i64 {}, i64 {})",
            span.start, span.end
        ));
        self.emit(format!("  br label %{}", self.error_target()));
        self.terminated = true;
        self.emit_label(&ok);
        match out {
            Some(out) => {
                let result = self.next_temp();
                self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {out}"));
                Ok(result)
            }
            None => Ok("zeroinitializer".to_owned()),
        }
    }
}
