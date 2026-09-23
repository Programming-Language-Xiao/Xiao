//! N0-B 动态值降低器。
//!
//! 本模块与 N0-A 的静态标量降低器分开维护。它只在前端 IR 明确包含字符串、容器或表
//! 时启用，并通过 `xiao-runtime-abi` 的固定 `%xiao.value` 布局传递值。静态程序仍由
//! [`crate::ir::lower_static_program`] 生成原生 `i64`/`f64`/`i1`，不会因为本模块存在
//! 而链接 Runtime。

use std::collections::{BTreeMap, BTreeSet};

use xiao_ir::{IrExpression, IrProgram};
use xiao_runtime_abi::ABI_ENCODED_VERSION;

use crate::CODEGEN_VERSION;
use crate::error::{CodegenError, Result};
use crate::ir::{CodegenOptions, LlvmModule, validate_program};
use crate::text::{escape_llvm, stable_hash};

#[cfg(test)]
#[path = "dynamic_architecture_tests.rs"]
/// 锁定动态降低器门面、静态边界和职责子模块的依赖方向。
mod architecture_tests;
#[path = "dynamic/container.rs"]
/// 数组、字典、集合、表构造与表描述符发射。
mod container;
#[path = "dynamic/control.rs"]
/// 动态语句、条件和循环控制流发射。
mod control;
#[path = "dynamic/entry.rs"]
/// 动态程序入口、C `main` 适配器和观察值发射。
mod entry;
#[path = "dynamic/expression.rs"]
/// 动态表达式、字面量和表字段访问发射。
mod expression;
#[path = "dynamic/predicate.rs"]
/// 纯 IR Runtime/容器谓词和稳定名称辅助。
mod predicate;
#[path = "dynamic/release.rs"]
/// 临时值与所有权退出计划的释放发射。
mod release;
#[path = "dynamic/runtime_abi.rs"]
/// Runtime 声明、目标 ABI 适配和 ABI 调用原语。
mod runtime_abi;
#[path = "dynamic/slot.rs"]
/// 动态槽收集、边界校验和槽读写。
mod slot;
#[path = "dynamic/text.rs"]
/// 动态路径文本解析、转义和 Cast 安全辅助。
mod text;
pub(crate) use self::predicate::program_uses_runtime;

/// 动态值的固定 `{ i32, i32, i64 }` LLVM 结构名。
const VALUE_TYPE: &str = "%xiao.value";
/// 不拥有输入内存的 UTF-8 字节视图结构名。
const BYTES_TYPE: &str = "%xiao.bytes";
/// 表字段描述符的 LLVM 结构名。
const TABLE_FIELD_TYPE: &str = "%xiao.table.field";
/// 表描述符的 LLVM 结构名。
const TABLE_DESCRIPTOR_TYPE: &str = "%xiao.table.descriptor";

/// 将一份含动态值的 IR 降低为调用 Runtime ABI 的 LLVM 文本。
pub(crate) fn lower_program(program: &IrProgram, options: &CodegenOptions) -> Result<LlvmModule> {
    validate_program(program)?;
    if !program_uses_runtime(program) {
        return Err(CodegenError::InvalidIr {
            message: "动态降低器收到纯静态程序".to_owned(),
        });
    }
    if let Some(check) = program.runtime_checks.first() {
        return Err(CodegenError::Unsupported {
            feature: format!("动态运行时检查 {}（原生降低尚未接通）", check.kind),
            span: Some(check.span),
        });
    }
    if options.target.pointer_width != 64 {
        return Err(CodegenError::Unsupported {
            feature: "N0-B 当前只接受 64 位 C ABI 目标".to_owned(),
            span: Some(program.span),
        });
    }
    DynamicGenerator::new(program, options).generate()
}

/// 动态值局部槽。
#[derive(Clone, Copy, Debug)]
struct Slot {
    index: usize,
}

/// 当前动态 `while` 的条件与出口块。
#[derive(Clone, Debug)]
struct LoopLabels {
    condition: String,
    end: String,
}

/// 动态 LLVM 文本生成器。
struct DynamicGenerator<'a> {
    program: &'a IrProgram,
    options: &'a CodegenOptions,
    slots: BTreeMap<String, Slot>,
    value_slots: BTreeMap<u32, Slot>,
    declarations: BTreeSet<String>,
    globals: Vec<String>,
    lines: Vec<String>,
    next_temp: usize,
    next_global: usize,
    next_label: usize,
    terminated: bool,
    loop_stack: Vec<LoopLabels>,
    table_initializers: BTreeMap<String, Vec<(String, IrExpression)>>,
    observation_slot: Option<usize>,
    declared_runtime_components: BTreeSet<String>,
}

impl<'a> DynamicGenerator<'a> {
    /// 为一份已验证动态 IR 创建生成器。
    fn new(program: &'a IrProgram, options: &'a CodegenOptions) -> Self {
        Self {
            program,
            options,
            slots: BTreeMap::new(),
            value_slots: BTreeMap::new(),
            declarations: BTreeSet::new(),
            globals: Vec::new(),
            lines: Vec::new(),
            next_temp: 0,
            next_global: 0,
            next_label: 0,
            terminated: false,
            loop_stack: Vec::new(),
            table_initializers: BTreeMap::new(),
            observation_slot: None,
            declared_runtime_components: BTreeSet::new(),
        }
    }

    /// 生成 ABI 类型、声明、入口和释放序列。
    fn generate(mut self) -> Result<LlvmModule> {
        self.collect_table_initializers()?;
        self.collect_slots(self.program.body.as_slice())?;
        self.collect_value_slots();
        self.validate_release_scope_boundary()?;
        if self.options.entry_observation == crate::ir::EntryObservation::ExitCode {
            self.observation_slot = Some(self.slots.len());
        }
        self.declare_runtime();
        if self.options.debug_startup.is_some() {
            self.declarations
                .insert("declare i32 @xiao_native_debug_start()".to_owned());
        }
        self.emit_entry()?;
        let mut text = String::new();
        text.push_str("; Xiao N0-B LLVM dynamic module\n");
        text.push_str(&format!("; target = {}\n", self.options.target.triple));
        text.push_str(&format!(
            "target triple = \"{}\"\n\n",
            escape_llvm(&self.options.target.triple)
        ));
        text.push_str(&format!("{VALUE_TYPE} = type {{ i32, i32, i64 }}\n"));
        text.push_str(&format!("{BYTES_TYPE} = type {{ ptr, i64 }}\n\n"));
        text.push_str(&format!(
            "{TABLE_FIELD_TYPE} = type {{ {BYTES_TYPE}, i32, i8 }}\n"
        ));
        text.push_str(&format!(
            "{TABLE_DESCRIPTOR_TYPE} = type {{ {BYTES_TYPE}, i32, ptr, i64 }}\n\n"
        ));
        for global in &self.globals {
            text.push_str(global);
            text.push('\n');
        }
        if !self.globals.is_empty() {
            text.push('\n');
        }
        for declaration in self.declarations {
            text.push_str(&declaration);
            text.push('\n');
        }
        text.push('\n');
        text.push_str(&self.lines.join("\n"));
        text.push('\n');
        let components = self
            .declared_runtime_components
            .into_iter()
            .collect::<Vec<_>>();
        let fingerprint = format!(
            "xiao-codegen-{CODEGEN_VERSION}-{}",
            stable_hash(
                format!(
                    "{CODEGEN_VERSION};dynamic;abi={ABI_ENCODED_VERSION};{}",
                    self.options.target.fingerprint_fields()
                )
                .as_bytes()
            )
        );
        Ok(LlvmModule {
            text,
            target: self.options.target.clone(),
            entry_symbol: "xiao_entry".to_owned(),
            uses_runtime: true,
            runtime_components: components,
            runtime_abi_version: Some(ABI_ENCODED_VERSION),
            codegen_fingerprint: fingerprint,
        })
    }

    /// 追加一行 LLVM 文本。
    fn emit(&mut self, line: String) {
        self.lines.push(line);
    }

    /// 追加一个基本块标签，并把后续指令标记为可达。
    fn emit_label(&mut self, label: &str) {
        self.emit(format!("{label}:"));
        self.terminated = false;
    }

    /// 分配一个临时 SSA 名称。
    fn next_temp(&mut self) -> String {
        let temp = format!("%t{}", self.next_temp);
        self.next_temp += 1;
        temp
    }

    /// 分配一个带稳定前缀的基本块标签。
    fn next_label(&mut self, prefix: &str) -> String {
        let label = format!("{prefix}{}", self.next_label);
        self.next_label += 1;
        label
    }

    /// 检查一个 Runtime C ABI 状态码；失败边统一进入不可恢复 trap。
    fn check_status(&mut self, status: &str) {
        let ok = self.next_temp();
        let label = format!("abi.ok{}", self.next_label);
        self.next_label += 1;
        self.emit(format!("  {ok} = icmp eq i32 {status}, 0"));
        self.emit(format!("  br i1 {ok}, label %{label}, label %abi.fail"));
        self.emit(format!("{label}:"));
    }

    /// 发射一个返回状态码的 Runtime 调用并在继续前检查结果。
    fn checked_status_call(&mut self, call: String) -> String {
        let status = self.next_temp();
        self.emit(format!("  {status} = call i32 {call}"));
        self.check_status(&status);
        status
    }
}
