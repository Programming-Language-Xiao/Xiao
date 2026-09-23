//! 动态降低器的 Runtime ABI 声明与调用辅助。

use super::predicate::statement_uses_container_abi;
use super::{BYTES_TYPE, DynamicGenerator, VALUE_TYPE};
use crate::target::ObjectFormat;

impl<'a> DynamicGenerator<'a> {
    /// 登记 Runtime ABI 声明，并记录可解释组件清单。
    pub(super) fn declare_runtime(&mut self) {
        self.declarations
            .insert("declare void @llvm.trap()".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_abi_is_compatible(i32, i32)".to_owned());
        self.declarations
            .insert("declare void @xiao_runtime_release(ptr)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_value_copy(ptr, ptr)".to_owned());
        self.declarations
            .insert("declare void @xiao_runtime_value_release(ptr)".to_owned());
        self.declarations
            .insert("declare i32 @xiao_runtime_value_release_weak(ptr)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_none", ""));
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
            .insert(self.value_declaration("xiao_runtime_value_lint", "ptr"));
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_lfloat", "ptr"));
        self.declarations
            .insert("declare i32 @xiao_runtime_array_new(ptr, i64, ptr)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_array", "ptr"));
        self.declarations
            .insert("declare i32 @xiao_runtime_tuple_new(ptr, i64, ptr)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_tuple", "ptr"));
        self.declarations
            .insert("declare i32 @xiao_runtime_dict_new(i32, ptr, ptr, i64, ptr)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_dict", "ptr, i32"));
        self.declarations
            .insert("declare i32 @xiao_runtime_set_new(ptr, i64, ptr)".to_owned());
        self.declarations
            .insert(self.value_declaration("xiao_runtime_value_set", "ptr"));
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

    /// 判断程序是否实际构造容器，而不是仅仅声明了动态值。
    fn program_uses_container_abi(&self) -> bool {
        self.program.body.iter().any(statement_uses_container_abi)
    }
}
