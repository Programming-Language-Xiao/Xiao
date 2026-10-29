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
use crate::native_concurrency::NativeConcurrencyPlan;
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
#[path = "dynamic/methods.rs"]
/// 表方法注册、字段辅助函数与指针回调适配。
mod methods;
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
/// ABI 1.8 方法元数据的固定 C 布局。
const TABLE_METHOD_TYPE: &str = "%xiao.table.method";
/// ABI 1.8 独立表描述符，保留旧字段描述符作为内嵌字段。
const TABLE_DESCRIPTOR_V2_TYPE: &str = "%xiao.table.descriptor.v2";
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

/// 使用同源 IR 并发计划降低动态程序；计划由外层入口先建立并保持只读。
pub(crate) fn lower_program_with_concurrency(
    program: &IrProgram,
    options: &CodegenOptions,
    concurrency_plan: &NativeConcurrencyPlan,
) -> Result<LlvmModule> {
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
    DynamicGenerator::new(program, options, concurrency_plan).generate()
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
    /// 同一份 IR 建立的只读并发计划。
    concurrency_plan: &'a NativeConcurrencyPlan,
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
    error_stack: Vec<ErrorContext>,
    error_terminal_label: String,
    cleanup_stack: Vec<CleanupContext<'a>>,
    finally_stack_saves: Vec<String>,
    /// 当前程序中可调用的顶层函数定义。
    function_definitions: BTreeMap<String, &'a IrStatement>,
    /// 按标准库模块和导出名称保存函数定义，避免不同模块的同名导出互相覆盖。
    module_function_definitions: BTreeMap<(String, String), &'a IrStatement>,
    /// 表名与规范成员名共同定位方法，隔离不同表的同名函数。
    method_definitions: BTreeMap<(String, String), &'a IrStatement>,
    /// 当前回调上下文；字段初始化使用合成帧，其余方法消费原函数释放计划。
    callback: Option<methods::CallbackContext>,
    /// 生成后追加到模块文本的函数定义。
    function_texts: Vec<String>,
    /// 已生成的并发 trampoline 符号，避免同一函数被重复定义。
    concurrency_trampolines: BTreeSet<String>,
    /// 是否正在生成一个动态函数体。
    function_mode: bool,
    /// 动态函数的返回槽和成功/错误出口。
    function_return_slot: Option<String>,
    function_return_label: Option<String>,
    /// 每个 return 表达式的帧内拥有槽，直到函数退出才归还被 finally 覆盖的临时值。
    return_sites: Vec<(*const IrExpression, String)>,
    /// 返回值和语句临时对象共享帧内创建顺序，错误出口也可归还。
    frame_temporaries: Vec<String>,
    /// 语句临时槽按表达式节点复用，循环不累计拥有引用。
    statement_sites: Vec<((*const IrExpression, *const IrExpression), String)>,
    /// 当前语句的临时对象，嵌套 finally 语句使用独立列表。
    statement_temporaries: Vec<String>,
    /// 当前函数所属生命周期作用域；隔离不同函数的同名值。
    function_scope: Option<u32>,
    /// 返回展开先运行所有外层 finally，再按内到外顺序执行挂起的释放计划。
    pending_return_releases: Vec<(Option<u32>, String)>,
    /// 已在当前入口初始化的模块，保证多次 import 只执行一次。
    initialized_modules: BTreeSet<String>,
}

impl<'a> DynamicGenerator<'a> {
    /// 为一份已验证动态 IR 创建生成器。
    fn new(
        program: &'a IrProgram,
        options: &'a CodegenOptions,
        concurrency_plan: &'a NativeConcurrencyPlan,
    ) -> Self {
        Self {
            program,
            options,
            concurrency_plan,
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
            error_stack: Vec::new(),
            error_terminal_label: "xiao.error.terminal".to_owned(),
            cleanup_stack: Vec::new(),
            finally_stack_saves: Vec::new(),
            function_definitions: BTreeMap::new(),
            module_function_definitions: BTreeMap::new(),
            method_definitions: BTreeMap::new(),
            callback: None,
            function_texts: Vec::new(),
            concurrency_trampolines: BTreeSet::new(),
            function_mode: false,
            function_return_slot: None,
            function_return_label: None,
            return_sites: Vec::new(),
            frame_temporaries: Vec::new(),
            statement_sites: Vec::new(),
            statement_temporaries: Vec::new(),
            function_scope: None,
            pending_return_releases: Vec::new(),
            initialized_modules: BTreeSet::new(),
        }
    }

    /// 生成 ABI 类型、声明、入口和释放序列。
    fn generate(mut self) -> Result<LlvmModule> {
        self.collect_table_initializers()?;
        self.collect_table_methods();
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
        // 官方 Xiao 模块仍由前端的固定命名空间提供。LLVM 产物是静态链接形态，
        // 因此把其纯 Xiao 顶层函数加入同一符号表，使 `from stdlib.x import f`
        // 能降低为直接函数调用；模块导入语句仍负责模块初始化与导出语义。
        for module in self
            .program
            .modules
            .iter()
            .filter(|module| module.name.starts_with("stdlib.") && module.kind == "file")
        {
            for statement in &module.body {
                let xiao_ir::IrStatementKind::Function { name, .. } = &statement.kind else {
                    continue;
                };
                self.function_definitions
                    .entry(name.text.clone())
                    .or_insert(statement);
                self.module_function_definitions
                    .insert((module.name.clone(), name.text.clone()), statement);
            }
        }
        self.emit_entry()?;
        // 导入语句在入口发射期间才会把别名函数登记到
        // `function_definitions`。trampoline 必须在这之后准备，否则函数表的
        // BTreeMap 位置会因别名插入而改变，生成的目标符号会与实际定义脱节。
        self.prepare_concurrency_trampolines()?;
        let mut seen_functions = BTreeSet::new();
        let functions = self
            .function_definitions
            .values()
            .copied()
            .filter(|statement| {
                // 不用源码区间去重：不同模块可以合法拥有相同的局部 span，
                // 而并发 trampoline 需要为每个真实函数节点保留定义。
                seen_functions.insert(*statement as *const IrStatement as usize)
            })
            .collect::<Vec<_>>();
        for statement in functions {
            self.emit_function_definition(statement)?;
        }
        self.emit_table_callbacks()?;
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
            "{TABLE_METHOD_TYPE} = type {{ {BYTES_TYPE}, i64, ptr, i64, i32, i8, ptr }}\n"
        ));
        text.push_str(&format!("{TABLE_DESCRIPTOR_V2_TYPE} = type {{ i32, i32, {TABLE_DESCRIPTOR_TYPE}, ptr, i64, ptr, ptr, ptr }}\n\n"));
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
        let mut components = runtime_abi::reachable_runtime_components(&text);
        if !self.concurrency_plan.is_empty()
            && !components
                .iter()
                .any(|component| component == "concurrency")
        {
            // 并发调用可能只出现在尚未从 main 调用的函数中；组件报告仍须
            // 反映该模块包含的并发 ABI 依赖，不能由入口可达性静默丢弃。
            components.push("concurrency".to_owned());
            components.sort();
        }
        let fingerprint = format!(
            "xiao-codegen-{CODEGEN_VERSION}-{}",
            stable_hash(
                format!(
                    "{CODEGEN_VERSION};dynamic;optimization={};abi={ABI_ENCODED_VERSION};capabilities={:016x};locale={};{}",
                    self.options.optimization_level,
                    self.options.capabilities,
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
    fn emit_function_definition(&mut self, statement: &IrStatement) -> Result<()> {
        self.emit_function_with_callback(statement, None)
    }

    /// 顶层函数和表回调共用控制流、值槽与清理逻辑，仅调整 C 入口/出口形态。
    fn emit_function_with_callback(
        &mut self,
        statement: &IrStatement,
        callback: Option<methods::CallbackContext>,
    ) -> Result<()> {
        let xiao_ir::IrStatementKind::Function {
            parameters, body, ..
        } = &statement.kind
        else {
            return Ok(());
        };
        let symbol = callback.as_ref().map_or_else(
            || self.function_symbol_for_statement(statement),
            |context| context.symbol.clone(),
        );
        let mut generator =
            DynamicGenerator::new(self.program, self.options, self.concurrency_plan);
        generator.callback = callback;
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
        generator.module_function_definitions = self.module_function_definitions.clone();
        generator.method_definitions = self.method_definitions.clone();
        generator.table_initializers = self.table_initializers.clone();
        generator.concurrency_trampolines = self.concurrency_trampolines.clone();
        generator.error_terminal_label = "xiao.fn.error".to_owned();
        generator.collect_slots(body)?;
        for parameter in parameters {
            let key = crate::dynamic::predicate::name_key(&parameter.name);
            let index = generator.slots.len();
            generator.slots.entry(key).or_insert(Slot { index });
        }
        if !generator
            .callback
            .as_ref()
            .is_some_and(|context| context.fields)
        {
            generator.collect_value_slots();
        }
        generator.function_return_slot = Some("%xiao.function.result".to_owned());
        generator.function_return_label = Some("xiao.fn.return".to_owned());
        generator.declare_runtime();
        let signature = if generator.callback.is_some() {
            format!(
                "define i32 @{symbol}(ptr %xiao.receiver, ptr %xiao.arguments, i64 %xiao.argument.count, ptr %xiao.function.result) {{"
            )
        } else {
            let mut signature = format!("define void @{symbol}(ptr %xiao.function.result");
            for index in 0..parameters.len() {
                signature.push_str(&format!(", ptr %xiao.arg{index}"));
            }
            signature.push_str(") {");
            signature
        };
        generator.emit(signature);
        generator.emit("entry:".to_owned());
        generator.emit("  %xiao.return.source = alloca ptr".to_owned());
        generator.emit("  store ptr null, ptr %xiao.return.source".to_owned());
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
            let argument = if generator.callback.is_none() {
                format!("%xiao.arg{index}")
            } else if index == 0 {
                "%xiao.receiver".to_owned()
            } else {
                let argument = generator.next_temp();
                generator.emit(format!(
                    "  {argument} = getelementptr {VALUE_TYPE}, ptr %xiao.arguments, i64 {}",
                    index - 1
                ));
                argument
            };
            generator.checked_status_call(format!(
                "@xiao_runtime_value_copy(ptr {argument}, ptr %slot{})",
                slot.index
            ));
        }
        for parameter in parameters {
            let slot = generator.slots[&crate::dynamic::predicate::name_key(&parameter.name)];
            let value = generator.next_temp();
            generator.emit(format!(
                "  {value} = load {VALUE_TYPE}, ptr %slot{}",
                slot.index
            ));
            generator.emit_registered_checks_at(parameter.name.span, &value);
        }
        generator.emit_statements(body)?;
        if !generator.terminated {
            generator.emit("  store ptr null, ptr %xiao.return.source".to_owned());
            let none = generator.none_value();
            generator.emit(format!(
                "  store {VALUE_TYPE} {none}, ptr %xiao.function.result"
            ));
            generator.emit("  br label %xiao.fn.return".to_owned());
            generator.terminated = true;
        }
        generator.emit_label("xiao.fn.return");
        generator.release_function_scopes("return")?;
        let scope_status = generator.next_temp();
        generator.emit(format!(
            "  {scope_status} = call i32 @xiao_runtime_error_class()"
        ));
        generator.check_status_at(&scope_status, statement.span);
        if generator.return_sites.is_empty() {
            generator.emit("  br label %xiao.fn.return.cleanup".to_owned());
        } else {
            let source = generator.next_temp();
            generator.emit(format!("  {source} = load ptr, ptr %xiao.return.source"));
            let present = generator.next_temp();
            generator.emit(format!("  {present} = icmp ne ptr {source}, null"));
            generator.emit(format!(
                "  br i1 {present}, label %xiao.fn.return.value, label %xiao.fn.return.cleanup"
            ));
            generator.emit_label("xiao.fn.return.value");
            let temporary = generator.next_temp();
            generator.emit(format!(
                "  {temporary} = load i1, ptr %xiao.return.temporary"
            ));
            generator.emit(format!(
                "  br i1 {temporary}, label %xiao.fn.return.copy, label %xiao.fn.return.move"
            ));
            generator.emit_label("xiao.fn.return.copy");
            generator.checked_status_call(format!(
                "@xiao_runtime_value_copy(ptr {source}, ptr %xiao.function.result)"
            ));
            generator.emit("  br label %xiao.fn.return.cleanup".to_owned());
            generator.emit_label("xiao.fn.return.move");
            let value = generator.next_temp();
            generator.emit(format!("  {value} = load {VALUE_TYPE}, ptr {source}"));
            generator.emit(format!(
                "  store {VALUE_TYPE} {value}, ptr %xiao.function.result"
            ));
            generator.emit(format!(
                "  store {VALUE_TYPE} zeroinitializer, ptr {source}"
            ));
            generator.emit("  br label %xiao.fn.return.cleanup".to_owned());
        }
        generator.emit_label("xiao.fn.return.cleanup");
        generator.release_frame_temporaries();
        let cleanup_status = generator.next_temp();
        generator.emit(format!(
            "  {cleanup_status} = call i32 @xiao_runtime_error_class()"
        ));
        generator.check_status_at(&cleanup_status, statement.span);
        generator.emit(
            if generator.callback.is_some() {
                "  ret i32 0"
            } else {
                "  ret void"
            }
            .to_owned(),
        );
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
        generator.release_frame_temporaries();
        let release = generator.value_release_symbol();
        generator.emit(format!("  call void @{release}(ptr %xiao.function.result)"));
        generator.emit("  br label %xiao.fn.error.return".to_owned());
        generator.emit_label("xiao.fn.error.return");
        generator.emit("  store %xiao.value zeroinitializer, ptr %xiao.function.result".to_owned());
        if generator.callback.is_some() {
            let status = generator.next_temp();
            generator.emit(format!("  {status} = call i32 @xiao_runtime_error_class()"));
            generator.emit(format!("  ret i32 {status}"));
        } else {
            generator.emit("  ret void".to_owned());
        }
        generator.emit("}".to_owned());
        generator.emit(String::new());
        generator.initialize_frame_temporaries();
        self.next_global = generator.next_global;
        self.globals.extend(generator.globals);
        self.function_texts.push(generator.lines.join("\n"));
        Ok(())
    }

    /// 为顶层函数生成稳定的 LLVM 符号名。
    fn function_symbol(&self, name: &str) -> String {
        let Some(statement) = self.function_definitions.get(name).copied() else {
            return format!("xiao.fn.0.{}", stable_hash(name.as_bytes()));
        };
        self.function_symbol_for_statement(statement)
    }

    /// 为一份具体函数定义生成稳定且不冲突的 LLVM 符号名。
    ///
    /// 别名导入可能让两个不同标准库模块的函数共享 Xiao 源码名称；此时不能再
    /// 仅按函数声明中的名称查找索引，必须按定义指针回到函数表中的唯一键。
    fn function_symbol_for_statement(&self, statement: &IrStatement) -> String {
        let key = self
            .function_definitions
            .iter()
            .find(|(_, candidate)| {
                std::ptr::eq(
                    **candidate as *const IrStatement,
                    statement as *const IrStatement,
                )
            })
            .map_or_else(
                || match &statement.kind {
                    xiao_ir::IrStatementKind::Function { name, .. } => name.text.clone(),
                    _ => String::new(),
                },
                |(key, _)| key.clone(),
            );
        let index = self
            .function_definitions
            .keys()
            .position(|candidate| candidate == &key)
            .unwrap_or(0);
        format!("xiao.fn.{index}.{}", stable_hash(key.as_bytes()))
    }

    /// 为动态函数预生成统一的并发 trampoline。
    ///
    /// 入口只接收不透明上下文、值数组和结果槽，再转调既有 Xiao 函数调用约定；
    /// Runtime 因此拥有线程创建/取消/等待的完整协调权，LLVM 不复制调度器。
    fn prepare_concurrency_trampolines(&mut self) -> Result<()> {
        if self.concurrency_plan.is_empty() {
            return Ok(());
        }
        let definitions = self
            .function_definitions
            .iter()
            .map(|(name, statement)| (name.clone(), *statement))
            .collect::<Vec<_>>();
        for (name, statement) in definitions {
            let xiao_ir::IrStatementKind::Function { parameters, .. } = &statement.kind else {
                continue;
            };
            let symbol = self.function_symbol(&name);
            let trampoline = format!(
                "xiao.concurrent.trampoline.{}",
                stable_hash(name.as_bytes())
            );
            if !self.concurrency_trampolines.insert(trampoline.clone()) {
                continue;
            }
            let mut lines = Vec::new();
            lines.push(format!(
                "define i32 @{trampoline}(ptr %xiao.context, ptr %xiao.arguments, i64 %xiao.argument.count, ptr %xiao.result) {{"
            ));
            lines.push("entry:".to_owned());
            lines.push(format!(
                "  %xiao.count.ok = icmp eq i64 %xiao.argument.count, {}",
                parameters.len()
            ));
            lines
                .push("  br i1 %xiao.count.ok, label %xiao.invoke, label %xiao.invalid".to_owned());
            lines.push("xiao.invalid:".to_owned());
            lines.push("  ret i32 2".to_owned());
            lines.push("xiao.invoke:".to_owned());
            let mut call = format!("  call void @{symbol}(ptr %xiao.result");
            for index in 0..parameters.len() {
                let slot = format!("%xiao.arg.{index}");
                lines.push(format!(
                    "  {slot} = getelementptr inbounds %xiao.value, ptr %xiao.arguments, i64 {index}"
                ));
                call.push_str(&format!(", ptr {slot}"));
            }
            call.push(')');
            lines.push(call);
            lines.push("  %xiao.error.class = call i32 @xiao_runtime_error_class()".to_owned());
            lines.push("  ret i32 %xiao.error.class".to_owned());
            lines.push("}".to_owned());
            lines.push(String::new());
            self.function_texts.push(lines.join("\n"));
        }
        Ok(())
    }
}
