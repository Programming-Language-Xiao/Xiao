//! 静态浮点调用共享C ABI；标量SSA仅在调用边界装入XiaoValue，不含数学实现。
use super::{FunctionGenerator, Scalar, unsupported};
use crate::error::Result;
use xiao_ir::{IrCallArgument, IrSpan};

/// 静态桥独立命名的C值布局，避免依赖动态生成器。
const VALUE: &str = "%xiao.cmath.value";

impl<'a, 'b> FunctionGenerator<'a, 'b> {
    /// 按源码求值再排列静态float实参；不新增数值提升或重载。
    pub(super) fn emit_cmath_intrinsic(
        &mut self,
        declaration: &xiao_intrinsics::IntrinsicDecl,
        arguments: &[IrCallArgument],
        expected: Scalar,
        span: IrSpan,
    ) -> Result<(String, Scalar)> {
        let order = crate::cmath::argument_order(declaration.id.get(), arguments)
            .ok_or_else(|| unsupported("cmath 参数形态", Some(span)))?;
        if expected != Scalar::Float {
            return Err(unsupported("cmath 返回类型必须是float", Some(span)));
        }
        let mut values = Vec::new();
        for argument in arguments {
            let (value, ty) = self.emit_expression(&argument.value)?;
            if ty != Scalar::Float {
                return Err(unsupported("cmath 参数必须是float", Some(argument.span)));
            }
            values.push(value);
        }
        let ordered = order.iter().map(|&i| values[i].clone()).collect::<Vec<_>>();
        Ok((
            self.emit_cmath_scalar_call(declaration.public_name, &ordered, span)?,
            Scalar::Float,
        ))
    }

    /// 为一个已求值的一元/二元float调用构造独立out，按0/6/Fatal分支。
    pub(super) fn emit_cmath_scalar_call(
        &mut self,
        name: &str,
        arguments: &[String],
        span: IrSpan,
    ) -> Result<String> {
        self.module.declare_static_error_runtime();
        self.module
            .declaration(format!("{VALUE} = type {{ i32, i64 }}"));
        self.module.declaration("declare ptr @llvm.stacksave()");
        self.module
            .declaration("declare void @llvm.stackrestore(ptr)");
        self.module
            .declaration("declare i32 @xiao_runtime_error_attach_span(i64, i64)");
        let symbol = format!("xiao_runtime_{name}");
        let types = vec!["ptr"; arguments.len() + 1].join(", ");
        self.module
            .declaration(format!("declare i32 @{symbol}({types})"));
        // 循环中的ABI scratch每次调用后恢复，返回float SSA不依赖这些栈槽。
        let save = self.module.next_temp();
        self.emit_line(format!("  {save} = call ptr @llvm.stacksave()"));
        let mut pointers = Vec::new();
        for value in arguments {
            let bits = self.module.next_temp();
            self.emit_line(format!("  {bits} = bitcast double {value} to i64"));
            let tagged = self.module.next_temp();
            self.emit_line(format!(
                "  {tagged} = insertvalue {VALUE} zeroinitializer, i32 {}, 0",
                xiao_runtime_abi::XiaoValueTag::Float.raw()
            ));
            let boxed = self.module.next_temp();
            self.emit_line(format!(
                "  {boxed} = insertvalue {VALUE} {tagged}, i64 {bits}, 1"
            ));
            let ptr = self.module.next_temp();
            self.emit_line(format!("  {ptr} = alloca {VALUE}, align 8"));
            self.emit_line(format!("  store {VALUE} {boxed}, ptr {ptr}"));
            pointers.push(format!("ptr {ptr}"));
        }
        let out = self.module.next_temp();
        self.emit_line(format!("  {out} = alloca {VALUE}, align 8"));
        self.emit_line(format!("  store {VALUE} zeroinitializer, ptr {out}"));
        let status = self.module.next_temp();
        self.emit_line(format!(
            "  {status} = call i32 @{symbol}({}, ptr {out})",
            pointers.join(", ")
        ));
        let ok = self.module.next_label("cmath.ok");
        let error = self.module.next_label("cmath.error");
        let fatal = self.module.next_label("cmath.abi.failure");
        self.emit_line(format!(
            "  switch i32 {status}, label %{fatal} [ i32 0, label %{ok} i32 6, label %{error} ]"
        ));
        self.terminated = true;
        self.emit_label(&fatal);
        self.emit_line(format!("  call void @llvm.stackrestore(ptr {save})"));
        self.emit_line("  call void @xiao_runtime_fatal_abi()");
        self.emit_line("  unreachable");
        self.terminated = true;
        self.emit_label(&error);
        self.emit_line(format!("  call void @llvm.stackrestore(ptr {save})"));
        self.emit_line(format!(
            "  call i32 @xiao_runtime_error_attach_span(i64 {}, i64 {})",
            span.start, span.end
        ));
        self.emit_line("  call void @xiao_runtime_error_report()");
        let code = self.module.next_temp();
        self.emit_line(format!(
            "  {code} = call i32 @xiao_runtime_error_exit_code()"
        ));
        self.emit_line(format!("  call void @exit(i32 {code})"));
        self.emit_line("  unreachable");
        self.terminated = true;
        self.emit_label(&ok);
        let value = self.module.next_temp();
        let bits = self.module.next_temp();
        let result = self.module.next_temp();
        self.emit_line(format!("  {value} = load {VALUE}, ptr {out}"));
        self.emit_line(format!("  {bits} = extractvalue {VALUE} {value}, 1"));
        self.emit_line(format!("  {result} = bitcast i64 {bits} to double"));
        self.emit_line(format!("  call void @llvm.stackrestore(ptr {save})"));
        Ok(result)
    }
}
