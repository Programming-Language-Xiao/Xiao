//! N0-B 动态值降低器。
//!
//! 本模块与 N0-A 的静态标量降低器分开维护。它只在前端 IR 明确包含字符串、容器或表
//! 时启用，并通过 `xiao-runtime-abi` 的固定 `%xiao.value` 布局传递值。静态程序仍由
//! [`crate::ir::lower_static_program`] 生成原生 `i64`/`f64`/`i1`，不会因为本模块存在
//! 而链接 Runtime。

use std::collections::{BTreeMap, BTreeSet};

use xiao_ir::{IrExpression, IrProgram, IrSpan, IrStatement};
use xiao_optimizer::OptimizationReport;
use xiao_runtime_abi::ABI_ENCODED_VERSION;

use crate::CODEGEN_VERSION;
use crate::error::{CodegenError, Result};
use crate::ir::{CodegenOptions, LlvmModule, source_map_for_program, validate_program};
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

/// 动态值的固定 `{ i32, i64 }` LLVM 结构名；它对应 C ABI 的 tag 和 union payload 两个字段。
const VALUE_TYPE: &str = "%xiao.value";
/// 不拥有输入内存的 UTF-8 字节视图结构名。
const BYTES_TYPE: &str = "%xiao.bytes";
/// 表字段描述符的 LLVM 结构名。
const TABLE_FIELD_TYPE: &str = "%xiao.table.field";
/// 表描述符的 LLVM 结构名。
const TABLE_DESCRIPTOR_TYPE: &str = "%xiao.table.descriptor";
/// 错误位置的 LLVM 固定布局；实际传递使用指针，避免目标 ABI 聚合参数差异。
const ERROR_LOCATION_TYPE: &str = "%xiao.error.location";

/// 动态原生错误处理上下文的最小标签。
#[derive(Clone, Debug)]
struct ErrorContext {
    /// 处理当前错误的 LLVM 基本块。
    dispatch: String,
}

/// 当前非局部退出需要经过的作用域清理区域。
#[derive(Clone, Debug)]
struct CleanupContext<'a> {
    /// 该区域的 `finally` 主体；`None` 表示只需要释放作用域。
    finally_body: Option<&'a [IrStatement]>,
    /// `finally` 作用域编号。
    finally_scope: Option<u32>,
    /// 受保护主体或 `catch` 作用域编号。
    protected_scope: Option<u32>,
    /// 清理失败后的外层错误派发目标。
    failure_target: String,
}

/// 将一份含动态值的 IR 降低为调用 Runtime ABI 的 LLVM 文本。
pub(crate) fn lower_program(program: &IrProgram, options: &CodegenOptions) -> Result<LlvmModule> {
    validate_program(program)?;
    if !program_uses_runtime(program) {
        return Err(CodegenError::InvalidIr {
            message: "动态降低器收到纯静态程序".to_owned(),
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
    cleanup_depth: usize,
}

/// 动态 LLVM 文本生成器。
struct DynamicGenerator<'a> {
    program: &'a IrProgram,
    options: &'a CodegenOptions,
    /// 当前生成上下文的语句体；入口使用 `program.body`，函数使用自身 body。
    body: &'a [IrStatement],
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
    error_stack: Vec<ErrorContext>,
    error_terminal_label: String,
    cleanup_stack: Vec<CleanupContext<'a>>,
    finally_stack_saves: Vec<String>,
    /// 当前程序中可调用的顶层函数定义。
    function_definitions: BTreeMap<String, &'a IrStatement>,
    /// 生成后追加到模块文本的函数定义。
    function_texts: Vec<String>,
    /// 是否正在生成一个动态函数体。
    function_mode: bool,
    /// 动态函数的返回槽和成功/错误出口。
    function_return_slot: Option<String>,
    function_return_label: Option<String>,
    /// 当前函数所属生命周期作用域；隔离不同函数的同名值。
    function_scope: Option<u32>,
    /// 返回展开先运行所有外层 finally，再按内到外顺序执行挂起的释放计划。
    pending_return_releases: Vec<(Option<u32>, String)>,
    /// 已在当前入口初始化的模块，保证多次 import 只执行一次。
    initialized_modules: BTreeSet<String>,
}

impl<'a> DynamicGenerator<'a> {
    /// 为一份已验证动态 IR 创建生成器。
    fn new(program: &'a IrProgram, options: &'a CodegenOptions) -> Self {
        Self {
            program,
            options,
            body: &program.body,
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
            error_stack: Vec::new(),
            error_terminal_label: "xiao.error.terminal".to_owned(),
            cleanup_stack: Vec::new(),
            finally_stack_saves: Vec::new(),
            function_definitions: BTreeMap::new(),
            function_texts: Vec::new(),
            function_mode: false,
            function_return_slot: None,
            function_return_label: None,
            function_scope: None,
            pending_return_releases: Vec::new(),
            initialized_modules: BTreeSet::new(),
        }
    }

    /// 生成 ABI 类型、声明、入口和释放序列。
    fn generate(mut self) -> Result<LlvmModule> {
        self.collect_table_initializers()?;
        self.collect_slots(self.body)?;
        for module in &self.program.modules {
            self.collect_slots(&module.body)?;
        }
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
        self.function_definitions = self
            .program
            .body
            .iter()
            .filter_map(|statement| match &statement.kind {
                xiao_ir::IrStatementKind::Function { name, .. } => {
                    Some((name.text.clone(), statement))
                }
                _ => None,
            })
            .collect();
        self.emit_entry()?;
        let functions = self
            .function_definitions
            .values()
            .copied()
            .collect::<Vec<_>>();
        for statement in functions {
            self.emit_function_definition(statement)?;
        }
        let mut text = String::new();
        text.push_str("; Xiao N0-B LLVM dynamic module\n");
        text.push_str(&format!("; target = {}\n", self.options.target.triple));
        text.push_str(&format!(
            "target triple = \"{}\"\n\n",
            escape_llvm(&self.options.target.triple)
        ));
        text.push_str(&format!("{VALUE_TYPE} = type {{ i32, i64 }}\n"));
        text.push_str(&format!("{BYTES_TYPE} = type {{ ptr, i64 }}\n\n"));
        text.push_str(&format!(
            "{TABLE_FIELD_TYPE} = type {{ {BYTES_TYPE}, i32, i8 }}\n"
        ));
        text.push_str(&format!(
            "{TABLE_DESCRIPTOR_TYPE} = type {{ {BYTES_TYPE}, i32, ptr, i64 }}\n\n"
        ));
        text.push_str(&format!(
            "{ERROR_LOCATION_TYPE} = type {{ i64, i64, i8, [7 x i8] }}\n\n"
        ));
        for entry in source_map_for_program(self.program) {
            text.push_str(&format!(
                "; xiao.source-map {} {}..{}\n",
                entry.label, entry.span.start, entry.span.end
            ));
        }
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
        for function in &self.function_texts {
            text.push_str(function);
            text.push('\n');
        }
        // finally 总是抛错时，正常返回块仍可能被发射，但链接前会被 LLVM 删除。
        // 传递依赖必须按入口可达调用登记，不能把死块中的复制算作产物依赖。
        if runtime_abi::has_reachable_value_copy(&text) {
            self.declared_runtime_components.insert("weak".to_owned());
        }
        let components = self
            .declared_runtime_components
            .into_iter()
            .collect::<Vec<_>>();
        let fingerprint = format!(
            "xiao-codegen-{CODEGEN_VERSION}-{}",
            stable_hash(
                format!(
                    "{CODEGEN_VERSION};dynamic;optimization={};abi={ABI_ENCODED_VERSION};locale={};{}",
                    self.options.optimization_level,
                    self.options.locale,
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
            source_map: source_map_for_program(self.program),
            optimization_level: self.options.optimization_level,
            optimization_report: OptimizationReport::empty(),
            native_optimization_report: crate::optimization::LlvmOptimizationReport::empty(
                &self.options.target,
                self.options.optimization_level,
            ),
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

    /// 返回当前错误应跳转到的 LLVM 基本块。
    fn error_target(&self) -> String {
        self.error_stack
            .last()
            .map(|context| context.dispatch.clone())
            .unwrap_or_else(|| self.error_terminal_label.clone())
    }

    /// 检查一个 Runtime C ABI 状态码；失败边进入统一错误路由。
    fn check_status(&mut self, status: &str) {
        self.check_status_at(status, self.program.span);
    }

    /// 检查一个 Runtime C ABI 状态码并保留触发调用的源码位置。
    fn check_status_at(&mut self, status: &str, span: IrSpan) {
        let ok = self.next_temp();
        let label = format!("abi.ok{}", self.next_label);
        self.next_label += 1;
        let failed = format!("abi.status.fail{}", self.next_label);
        self.next_label += 1;
        self.emit(format!("  {ok} = icmp eq i32 {status}, 0"));
        self.emit(format!("  br i1 {ok}, label %{label}, label %{failed}"));
        self.terminated = true;
        self.emit_label(&failed);
        self.emit(format!(
            "  call i32 @xiao_runtime_error_attach_span(i64 {}, i64 {})",
            span.start, span.end
        ));
        self.emit(format!("  br label %{}", self.error_target()));
        self.terminated = true;
        self.emit_label(&label);
    }

    /// 发射一个返回状态码的 Runtime 调用并在继续前检查结果。
    fn checked_status_call(&mut self, call: String) -> String {
        self.checked_status_call_at(call, self.program.span)
    }

    /// 发射一个带源码位置的 Runtime 状态调用。
    fn checked_status_call_at(&mut self, call: String, span: IrSpan) -> String {
        let status = self.next_temp();
        self.emit(format!("  {status} = call i32 {call}"));
        self.check_status_at(&status, span);
        status
    }

    /// 记录一个原生错误处理上下文。
    fn push_error_context(&mut self, dispatch: String) {
        self.error_stack.push(ErrorContext { dispatch });
    }

    /// 离开当前原生错误处理上下文。
    fn pop_error_context(&mut self) {
        let _ = self.error_stack.pop();
    }

    /// 为一次 `finally` 发射栈帧保存点，避免循环体内的临时 `alloca` 累积。
    fn begin_finally_stack_frame(&mut self) -> String {
        let save = self.next_temp();
        self.emit(format!("  {save} = call ptr @llvm.stacksave()"));
        self.finally_stack_saves.push(save.clone());
        save
    }

    /// 在当前控制流边恢复最近的 `finally` 栈帧。
    fn restore_finally_stack_frame(&mut self) {
        if let Some(save) = self.finally_stack_saves.last().cloned() {
            self.emit(format!("  call void @llvm.stackrestore(ptr {save})"));
        }
    }

    /// 结束当前 `finally` 的栈帧跟踪；各控制流边已经分别发射恢复指令。
    fn finish_finally_stack_frame(&mut self, save: &str) {
        let current = self.finally_stack_saves.pop();
        debug_assert_eq!(current.as_deref(), Some(save));
    }

    /// 为一个顶层函数生成统一 `%xiao.value` 指针调用约定的 LLVM 定义。
    fn emit_function_definition(&mut self, statement: &'a IrStatement) -> Result<()> {
        let xiao_ir::IrStatementKind::Function {
            name,
            parameters,
            body,
            ..
        } = &statement.kind
        else {
            return Ok(());
        };
        let symbol = self.function_symbol(&name.text);
        let mut generator = Self::new(self.program, self.options);
        generator.body = body;
        generator.next_global = self.next_global;
        generator.function_mode = true;
        generator.function_scope = generator
            .program
            .ownership
            .scopes
            .iter()
            .find(|scope| scope.kind == "function" && scope.span == statement.span)
            .map(|scope| scope.id);
        generator.function_definitions = self.function_definitions.clone();
        generator.error_terminal_label = "xiao.fn.error".to_owned();
        generator.collect_slots(body)?;
        for parameter in parameters {
            let key = crate::dynamic::predicate::name_key(&parameter.name);
            let index = generator.slots.len();
            generator.slots.entry(key).or_insert(Slot { index });
        }
        generator.collect_value_slots();
        generator.function_return_slot = Some("%xiao.function.result".to_owned());
        generator.function_return_label = Some("xiao.fn.return".to_owned());
        generator.declare_runtime();
        let mut signature = format!("define void @{symbol}(ptr %xiao.function.result");
        for index in 0..parameters.len() {
            signature.push_str(&format!(", ptr %xiao.arg{index}"));
        }
        signature.push_str(") {");
        generator.emit(signature);
        generator.emit("entry:".to_owned());
        generator.emit("  %xiao.return.temporary = alloca i1".to_owned());
        generator.emit("  store i1 false, ptr %xiao.return.temporary".to_owned());
        let function_slots = generator.slots.values().copied().collect::<Vec<_>>();
        for slot in function_slots {
            generator.emit(format!("  %slot{} = alloca {VALUE_TYPE}", slot.index));
            generator.emit(format!(
                "  store {VALUE_TYPE} zeroinitializer, ptr %slot{}",
                slot.index
            ));
        }
        for (index, parameter) in parameters.iter().enumerate() {
            let key = crate::dynamic::predicate::name_key(&parameter.name);
            let slot = generator.slots[&key];
            generator.checked_status_call(format!(
                "@xiao_runtime_value_copy(ptr %xiao.arg{index}, ptr %slot{})",
                slot.index
            ));
        }
        generator.emit_statements(body)?;
        if !generator.terminated {
            let none = generator.none_value();
            generator.emit(format!(
                "  store {VALUE_TYPE} {none}, ptr %xiao.function.result"
            ));
            generator.emit("  br label %xiao.fn.return".to_owned());
            generator.terminated = true;
        }
        generator.emit("xiao.fn.return:".to_owned());
        let temporary = generator.next_temp();
        generator.emit(format!(
            "  {temporary} = load i1, ptr %xiao.return.temporary"
        ));
        generator.emit(format!(
            "  br i1 {temporary}, label %xiao.fn.return.copy, label %xiao.fn.return.cleanup"
        ));
        generator.emit_label("xiao.fn.return.copy");
        let copied = generator.next_temp();
        generator.emit(format!("  {copied} = alloca {VALUE_TYPE}"));
        generator.emit(format!(
            "  store {VALUE_TYPE} zeroinitializer, ptr {copied}"
        ));
        generator.checked_status_call(format!(
            "@xiao_runtime_value_copy(ptr %xiao.function.result, ptr {copied})"
        ));
        generator.emit(
            "  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)".to_owned(),
        );
        let value = generator.next_temp();
        generator.emit(format!("  {value} = load {VALUE_TYPE}, ptr {copied}"));
        generator.emit(format!(
            "  store {VALUE_TYPE} {value}, ptr %xiao.function.result"
        ));
        generator.emit("  br label %xiao.fn.return.cleanup".to_owned());
        generator.emit_label("xiao.fn.return.cleanup");
        generator.release_function_scopes("return")?;
        generator.emit("  ret void".to_owned());
        generator.emit("xiao.fn.error:".to_owned());
        let class = generator.next_temp();
        generator.emit(format!("  {class} = call i32 @xiao_runtime_error_class()"));
        let fatal = generator.next_temp();
        generator.emit(format!("  {fatal} = icmp eq i32 {class}, 2"));
        generator.emit(format!(
            "  br i1 {fatal}, label %xiao.fn.error.return, label %xiao.fn.error.cleanup"
        ));
        generator.emit_label("xiao.fn.error.cleanup");
        generator.release_function_scopes("unmatched_error")?;
        generator.emit(
            "  call void @xiao_runtime_value_release_strong(ptr %xiao.function.result)".to_owned(),
        );
        generator.emit("  br label %xiao.fn.error.return".to_owned());
        generator.emit_label("xiao.fn.error.return");
        generator.emit("  store %xiao.value zeroinitializer, ptr %xiao.function.result".to_owned());
        generator.emit("  ret void".to_owned());
        generator.emit("}".to_owned());
        generator.emit(String::new());
        self.next_global = generator.next_global;
        self.globals.extend(generator.globals);
        self.declared_runtime_components
            .extend(generator.declared_runtime_components);
        self.function_texts.push(generator.lines.join("\n"));
        Ok(())
    }

    /// 为顶层函数生成稳定的 LLVM 符号名。
    fn function_symbol(&self, name: &str) -> String {
        let index = self
            .function_definitions
            .keys()
            .position(|candidate| candidate == name)
            .unwrap_or(0);
        format!("xiao.fn.{index}.{}", stable_hash(name.as_bytes()))
    }
}
