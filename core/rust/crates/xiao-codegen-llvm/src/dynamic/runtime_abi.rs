//! 动态降低器的 Runtime ABI 声明与调用辅助。

use super::predicate::statement_uses_container_abi;
use super::{BYTES_TYPE, DynamicGenerator, ERROR_LOCATION_TYPE, VALUE_TYPE};
use crate::target::ObjectFormat;
use xiao_ir::IrSpan;

/// 检查从程序 main 入口可达的通用复制调用。
///
/// 只分析本降低器发射的直接调用与 `br` 边，不折叠运行时条件。
/// 未调用的函数和无前驱的 finally 后续块均不构成链接依赖；
/// 函数与块共同作为访问键，使同名标签、循环及递归保持独立且可终止。
pub(super) fn has_reachable_value_copy(text: &str) -> bool {
    use std::collections::{BTreeMap, BTreeSet};

    let mut blocks: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    let mut function = None;
    let mut current = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with("define ") {
            function = line
                .split_once('@')
                .and_then(|(_, rest)| rest.split_once('('))
                .map(|(name, _)| name);
            current = None;
        } else if line == "}" {
            function = None;
            current = None;
        } else if let Some(name) = function {
            if let Some(label) = line.strip_suffix(':') {
                current = Some(label);
                blocks.entry((name, label)).or_default();
            } else if let Some(label) = current {
                blocks.entry((name, label)).or_default().push(line);
            }
        }
    }
    let mut pending = vec![("main", "entry")];
    let mut visited = BTreeSet::new();
    while let Some(key) = pending.pop() {
        if !visited.insert(key) {
            continue;
        }
        let Some(lines) = blocks.get(&key) else {
            continue;
        };
        for line in lines {
            if line.contains("call i32 @xiao_runtime_value_copy(") {
                return true;
            }
            if line.starts_with("br ") {
                for target in line.split("label %").skip(1) {
                    pending.push((key.0, target.split(',').next().unwrap_or(target).trim()));
                }
            } else if (line.starts_with("call ") || line.contains(" = call "))
                && let Some((_, callee)) = line.split_once('@')
                && let Some((name, _)) = callee.split_once('(')
            {
                pending.push((name, "entry"));
            }
        }
    }
    false
}

impl<'a> DynamicGenerator<'a> {
    /// 登记 Runtime ABI 声明，并记录可解释组件清单。
    pub(super) fn declare_runtime(&mut self) {
        self.declarations
            .insert("declare void @llvm.trap()".to_owned());
        self.declarations
            .insert("declare ptr @llvm.stacksave()".to_owned());
        self.declarations
            .insert("declare void @llvm.stackrestore(ptr)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_abi_is_compatible(i32, i32)".to_owned());
        self.declarations
            .insert("declare void @xiao_runtime_error_clear()".to_owned());
        self.declarations.insert(format!(
            "declare i32 @xiao_runtime_language_context_set({})",
            self.bytes_parameter_type()
        ));
        self.declarations
            .insert("declare void @xiao_runtime_release_trace_begin()".to_owned());
        self.declarations
            .insert("declare void @xiao_runtime_release_trace_flush()".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_error_class()".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_error_attach_span(i64, i64)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_error_exit_code()".to_owned());
        self.declarations
            .insert("declare void @xiao_runtime_error_report()".to_owned());
        if self.options.debug_startup.is_some() {
            self.declarations
                .insert("declare i32 @xiao_runtime_diagnostic_prepare()".to_owned());
            self.declarations
                .insert("declare i32 @xiao_runtime_diagnostic_ready()".to_owned());
            self.declarations
                .insert("declare void @xiao_runtime_diagnostic_finish()".to_owned());
        }
        self.declarations
            .insert("declare void @xiao_runtime_fatal_abi()".to_owned());
        self.declarations.insert(format!(
            "declare i32 @xiao_runtime_error_matches({})",
            self.bytes_parameter_type()
        ));
        self.declarations
            .insert("declare i32 @xiao_runtime_error_take(ptr)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_error_raise_value(ptr, ptr)".to_owned());
        self.declarations
            .insert("declare void @xiao_runtime_release(ptr)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_value_copy(ptr, ptr)".to_owned());
        self.declarations
            .insert("declare void @xiao_runtime_value_release_strong(ptr)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_value_release_weak(ptr)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_print_values(ptr, i64)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_input", "ptr, i8"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_none", ""));
        self.declarations.insert(format!(
            "declare i32 @xiao_runtime_value_binary({}, ptr, ptr, ptr)",
            self.bytes_parameter_type()
        ));
        self.declarations.insert(format!(
            "declare i32 @xiao_runtime_value_unary({}, ptr, ptr)",
            self.bytes_parameter_type()
        ));
        self.declarations.insert(format!(
            "declare i32 @xiao_runtime_value_cast({}, ptr, ptr)",
            self.bytes_parameter_type()
        ));
        self.declarations.insert(format!(
            "declare i32 @xiao_runtime_dynamic_check({}, ptr)",
            self.bytes_parameter_type()
        ));
        self.declarations
            .insert("declare i32 @xiao_runtime_value_iter_len(ptr, ptr)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_value_iter_get(ptr, i64, ptr)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_value_select(ptr, i64, ptr, ptr)".to_owned());
        self.declarations.insert(self.value_declaration(
            "xiao_runtime_error_new",
            &format!(
                "{}, {}, {}, ptr",
                self.bytes_parameter_type(),
                self.bytes_parameter_type(),
                self.bytes_parameter_type()
            ),
        ));
        self.declarations.insert(self.value_declaration(
            "xiao_runtime_error_new_values",
            &format!("{}, ptr, ptr, ptr", self.bytes_parameter_type()),
        ));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_int", "i64"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_sint", "i32"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_float", "double"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_sfloat", "float"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_bool", "i8"));
        self.declarations.insert(format!(
            "declare i32 @xiao_runtime_string_new({}, ptr)",
            self.bytes_parameter_type()
        ));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_str", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_str_owned", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_lint", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_lint_owned", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_lfloat", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_lfloat_owned", "ptr"));
        self.declarations
            .insert("declare i32 @xiao_runtime_array_new(ptr, i64, ptr)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_array", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_array_owned", "ptr"));
        self.declarations
            .insert("declare i32 @xiao_runtime_tuple_new(ptr, i64, ptr)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_tuple", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_tuple_owned", "ptr"));
        self.declarations
            .insert("declare i32 @xiao_runtime_dict_new(i32, ptr, ptr, i64, ptr)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_dict", "ptr, i32"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_dict_owned", "ptr, i32"));
        self.declarations
            .insert("declare i32 @xiao_runtime_set_new(ptr, i64, ptr)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_set", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_set_owned", "ptr"));
        self.declarations
            .insert("declare i32 @xiao_runtime_table_new(ptr, ptr)".to_owned());
        self.declarations.insert(format!(
            "declare i32 @xiao_runtime_table_get(ptr, {}, ptr)",
            self.bytes_parameter_type()
        ));
        self.declarations.insert(format!(
            "declare i32 @xiao_runtime_table_set(ptr, {}, ptr)",
            self.bytes_parameter_type()
        ));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_table", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_table_owned", "ptr"));
        self.declared_runtime_components.insert("value".to_owned());
        self.declared_runtime_components.insert("rc".to_owned());
        if self.program_uses_container_abi() {
            self.declared_runtime_components
                .insert("containers".to_owned());
        }
        if self
            .program
            .ownership
            .release_plans
            .iter()
            .flat_map(|plan| plan.actions.iter())
            .any(|action| action.kind == "weak")
        {
            self.declared_runtime_components.insert("weak".to_owned());
        }
        if !self.program.table_signatures.is_empty() {
            self.declared_runtime_components.insert("tables".to_owned());
        }
    }

    /// 判断目标 C ABI 是否把 16 字节 `XiaoValue` 通过隐藏返回槽传递。
    ///
    /// Windows x64 的 MSVC/COFF ABI 对该布局使用 `sret`；SysV/Clang 的 ELF 与 Mach-O
    /// 目标则按两个寄存器直接返回。Runtime 的 Rust `extern "C"` 符号必须与目标侧采用
    /// 同一调用约定，否则函数虽然能链接，返回时会破坏调用方栈帧。
    fn value_return_is_indirect(&self) -> bool {
        matches!(self.options.target.object_format, ObjectFormat::Coff)
    }

    /// 返回 Runtime 对聚合字节视图参数采用的 LLVM 类型。
    fn bytes_parameter_type(&self) -> &'static str {
        if self.value_return_is_indirect() {
            "ptr"
        } else {
            BYTES_TYPE
        }
    }

    /// 把字节视图物化为目标 ABI 所需的参数形态。
    pub(super) fn emit_bytes_argument(&mut self, value: &str) -> String {
        if self.value_return_is_indirect() {
            let output = self.next_temp();
            self.emit(format!("  {output} = alloca {BYTES_TYPE}"));
            self.emit(format!("  store {BYTES_TYPE} {value}, ptr {output}"));
            format!("ptr {output}")
        } else {
            format!("{BYTES_TYPE} {value}")
        }
    }

    /// 生成一个固定布局的源码位置指针。
    pub(super) fn emit_error_location(&mut self, span: IrSpan) -> String {
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {ERROR_LOCATION_TYPE}"));
        let start = self.next_temp();
        self.emit(format!(
            "  {start} = insertvalue {ERROR_LOCATION_TYPE} zeroinitializer, i64 {}, 0",
            span.start
        ));
        let end = self.next_temp();
        self.emit(format!(
            "  {end} = insertvalue {ERROR_LOCATION_TYPE} {start}, i64 {}, 1",
            span.end
        ));
        let present = self.next_temp();
        self.emit(format!(
            "  {present} = insertvalue {ERROR_LOCATION_TYPE} {end}, i8 1, 2"
        ));
        self.emit(format!(
            "  store {ERROR_LOCATION_TYPE} {present}, ptr {output}"
        ));
        output
    }

    /// 检查一个值构造器是否在 Runtime 中留下了挂起错误。
    pub(super) fn check_pending_error_at(&mut self, span: IrSpan) {
        let class = self.next_temp();
        self.emit(format!("  {class} = call i32 @xiao_runtime_error_class()"));
        let ok = self.next_temp();
        let ok_label = self.next_label("abi.error.ok");
        let failed_label = self.next_label("abi.error.fail");
        self.emit(format!("  {ok} = icmp eq i32 {class}, 0"));
        self.emit(format!(
            "  br i1 {ok}, label %{ok_label}, label %{failed_label}"
        ));
        self.terminated = true;
        self.emit_label(&failed_label);
        self.emit(format!(
            "  call i32 @xiao_runtime_error_attach_span(i64 {}, i64 {})",
            span.start, span.end
        ));
        self.emit(format!("  br label %{}", self.error_target()));
        self.terminated = true;
        self.emit_label(&ok_label);
    }

    /// 生成一个 Runtime ABI 值返回函数的 LLVM 声明。
    fn value_declaration(&self, name: &str, arguments: &str) -> String {
        if self.value_return_is_indirect() {
            let prefix = format!("declare void @{name}(ptr sret({VALUE_TYPE}) align 8");
            if arguments.is_empty() {
                format!("{prefix})")
            } else {
                format!("{prefix}, {arguments})")
            }
        } else if arguments.is_empty() {
            format!("declare {VALUE_TYPE} @{name}()")
        } else {
            format!("declare {VALUE_TYPE} @{name}({arguments})")
        }
    }

    /// 发射一个 Runtime ABI 值返回调用，并屏蔽目标 C ABI 的返回槽差异。
    pub(super) fn emit_value_call(&mut self, name: &str, arguments: &str) -> String {
        let value = self.next_temp();
        if self.value_return_is_indirect() {
            let output = self.next_temp();
            self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
            self.emit(format!(
                "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
            ));
            if arguments.is_empty() {
                self.emit(format!(
                    "  call void @{name}(ptr sret({VALUE_TYPE}) {output})"
                ));
            } else {
                self.emit(format!(
                    "  call void @{name}(ptr sret({VALUE_TYPE}) {output}, {arguments})"
                ));
            }
            self.emit(format!("  {value} = load {VALUE_TYPE}, ptr {output}"));
        } else if arguments.is_empty() {
            self.emit(format!("  {value} = call {VALUE_TYPE} @{name}()"));
        } else {
            self.emit(format!(
                "  {value} = call {VALUE_TYPE} @{name}({arguments})"
            ));
        }
        value
    }

    /// 发射一个稳定 ASCII 名称的字节视图，并转换为目标 ABI 的参数形态。
    pub(super) fn emit_operation_argument(&mut self, name: &str) -> String {
        let bytes = self.emit_bytes_value(name.as_bytes());
        self.emit_bytes_argument(&bytes)
    }

    /// 发射 Runtime 二元运算并读取拥有的结果值。
    pub(super) fn emit_binary_runtime(
        &mut self,
        operation: &str,
        left: &str,
        right: &str,
        span: IrSpan,
    ) -> String {
        let operation = self.emit_operation_argument(operation);
        let left_slot = self.next_temp();
        self.emit(format!("  {left_slot} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {left}, ptr {left_slot}"));
        let right_slot = self.next_temp();
        self.emit(format!("  {right_slot} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {right}, ptr {right_slot}"));
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
        ));
        self.checked_status_call_at(
            format!("@xiao_runtime_value_binary({operation}, ptr {left_slot}, ptr {right_slot}, ptr {output})"),
            span,
        );
        let value = self.next_temp();
        self.emit(format!("  {value} = load {VALUE_TYPE}, ptr {output}"));
        value
    }

    /// 发射 Runtime 一元运算并读取拥有的结果值。
    pub(super) fn emit_unary_runtime(
        &mut self,
        operation: &str,
        value: &str,
        span: IrSpan,
    ) -> String {
        let operation = self.emit_operation_argument(operation);
        let input = self.next_temp();
        self.emit(format!("  {input} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {input}"));
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
        ));
        self.checked_status_call_at(
            format!("@xiao_runtime_value_unary({operation}, ptr {input}, ptr {output})"),
            span,
        );
        let result = self.next_temp();
        self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {output}"));
        result
    }

    /// 发射 Runtime 显式转换并读取拥有的结果值。
    pub(super) fn emit_cast_runtime(&mut self, target: &str, value: &str, span: IrSpan) -> String {
        let target = self.emit_operation_argument(target);
        let input = self.next_temp();
        self.emit(format!("  {input} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {input}"));
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
        ));
        self.checked_status_call_at(
            format!("@xiao_runtime_value_cast({target}, ptr {input}, ptr {output})"),
            span,
        );
        let result = self.next_temp();
        self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {output}"));
        result
    }

    /// 发射一个动态检查并把失败路由到当前错误上下文。
    pub(super) fn emit_dynamic_check(&mut self, kind: &str, value: &str, span: IrSpan) {
        let kind = self.emit_operation_argument(kind);
        let input = self.next_temp();
        self.emit(format!("  {input} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {input}"));
        self.checked_status_call_at(
            format!("@xiao_runtime_dynamic_check({kind}, ptr {input})"),
            span,
        );
    }

    /// 发射动态迭代长度读取。
    pub(super) fn emit_iter_len(&mut self, value: &str, span: IrSpan) -> String {
        let input = self.next_temp();
        self.emit(format!("  {input} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {input}"));
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca i64"));
        self.emit(format!("  store i64 0, ptr {output}"));
        self.checked_status_call_at(
            format!("@xiao_runtime_value_iter_len(ptr {input}, ptr {output})"),
            span,
        );
        let result = self.next_temp();
        self.emit(format!("  {result} = load i64, ptr {output}"));
        result
    }

    /// 发射动态迭代元素读取。
    pub(super) fn emit_iter_get(&mut self, value: &str, index: &str, span: IrSpan) -> String {
        let input = self.next_temp();
        self.emit(format!("  {input} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {value}, ptr {input}"));
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
        ));
        self.checked_status_call_at(
            format!("@xiao_runtime_value_iter_get(ptr {input}, i64 {index}, ptr {output})"),
            span,
        );
        let result = self.next_temp();
        self.emit(format!("  {result} = load {VALUE_TYPE}, ptr {output}"));
        result
    }

    /// 调用 Runtime 执行确定性随机选择，并返回拥有的数组值。
    pub(super) fn emit_random_select(
        &mut self,
        source: &str,
        count: &str,
        mode: &str,
        span: IrSpan,
    ) -> String {
        let input = self.next_temp();
        self.emit(format!("  {input} = alloca {VALUE_TYPE}"));
        self.emit(format!("  store {VALUE_TYPE} {source}, ptr {input}"));
        let output = self.next_temp();
        self.emit(format!("  {output} = alloca {VALUE_TYPE}"));
        self.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {output}"
        ));
        let mode = self.emit_operation_argument(mode);
        self.checked_status_call_at(
            format!("@xiao_runtime_value_select(ptr {input}, i64 {count}, {mode}, ptr {output})"),
            span,
        );
        let value = self.next_temp();
        self.emit(format!("  {value} = load {VALUE_TYPE}, ptr {output}"));
        value
    }

    /// 判断程序是否实际构造容器，而不是仅仅声明了动态值。
    fn program_uses_container_abi(&self) -> bool {
        self.program.body.iter().any(statement_uses_container_abi)
    }
}
