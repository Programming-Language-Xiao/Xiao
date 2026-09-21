//! N0-A 类型化 IR 到 LLVM 文本的降低器。
//!
//! 这一层只接受固定宽度标量。局部槽采用显式 `alloca`，因此分支和循环不需要为尚未
//! 接入的 Runtime 值制造 phi 表或隐藏布局；所有可能溢出的算术都先检查，再继续或进入
//! `llvm.trap` 失败块。

use std::collections::{BTreeMap, BTreeSet};

use xiao_ir::{
    IR_VERSION, IrExpression, IrExpressionKind, IrName, IrProgram, IrSpan, IrStatement,
    IrStatementKind, IrType,
};

use crate::CODEGEN_VERSION;
use crate::error::{CodegenError, Result};
use crate::target::TargetDescription;

/// 入口结果的观察策略。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EntryObservation {
    /// 保持 Xiao 脚本成功的零退出码；入口值不改变进程退出码。
    Ignore,
    /// 将入口最后一个整数/布尔值映射到进程退出码；仅用于测试观察。
    ExitCode,
}

/// N0-A 代码生成选项。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodegenOptions {
    /// 规范化目标描述。
    pub target: TargetDescription,
    /// 是否把入口观察值映射到 `main` 的返回码。
    pub entry_observation: EntryObservation,
}

impl CodegenOptions {
    /// 创建指定目标的默认选项。
    #[must_use]
    pub fn for_target(target: TargetDescription) -> Self {
        Self {
            target,
            entry_observation: EntryObservation::Ignore,
        }
    }

    /// 设置入口观察策略。
    #[must_use]
    pub const fn with_entry_observation(mut self, observation: EntryObservation) -> Self {
        self.entry_observation = observation;
        self
    }
}

impl Default for CodegenOptions {
    /// 为宿主目标创建静态标量默认配置。
    fn default() -> Self {
        Self::for_target(TargetDescription::host())
    }
}

/// 一份已经生成但尚未交给外部工具的 LLVM 模块。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlvmModule {
    /// 完整 LLVM IR 文本。
    pub text: String,
    /// 生成时使用的规范化目标。
    pub target: TargetDescription,
    /// 共用的 Xiao 入口符号。
    pub entry_symbol: String,
    /// N0-A 模块是否调用 Runtime ABI；纯静态模块固定为 `false`。
    pub uses_runtime: bool,
    /// 生成模块实际需要的 Runtime 组件名称；静态模块为空。
    pub runtime_components: Vec<String>,
    /// 动态模块要求的 Runtime ABI 编码版本；静态模块没有该依赖。
    pub runtime_abi_version: Option<u64>,
    /// 后端版本和目标字段组成的可追踪指纹（工具版本在构建驱动器中补入）。
    pub codegen_fingerprint: String,
}

/// 验证输入 IR，并返回第一个结构化错误。
pub fn validate_program(program: &IrProgram) -> Result<()> {
    if program.version != IR_VERSION {
        return Err(CodegenError::IrVersion {
            expected: IR_VERSION,
            actual: program.version,
        });
    }
    let validation = program.validate();
    if let Some(error) = validation.errors.first() {
        return Err(CodegenError::InvalidIr {
            message: error.to_string(),
        });
    }
    Ok(())
}

/// 将一份已验证的类型化 IR 降低成 LLVM 文本。
pub(crate) fn lower_static_program(
    program: &IrProgram,
    options: &CodegenOptions,
) -> Result<LlvmModule> {
    validate_program(program)?;
    let mut generator = ModuleGenerator::new(program, options);
    generator.generate()
}

/// N0-A 支持的固定宽度 LLVM 标量类别。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Scalar {
    Int,
    Sint,
    Float,
    Sfloat,
    Bool,
    None,
}

impl Scalar {
    /// 从 IR 类型解析静态标量，并拒绝动态布局。
    fn from_type(ty: &IrType, span: IrSpan) -> Result<Self> {
        match ty {
            IrType::None => Ok(Self::None),
            IrType::Scalar { name } => match name.as_str() {
                "int" => Ok(Self::Int),
                "sint" => Ok(Self::Sint),
                "float" => Ok(Self::Float),
                "sfloat" => Ok(Self::Sfloat),
                "bool" => Ok(Self::Bool),
                "lint" | "lfloat" | "str" => Err(CodegenError::Unsupported {
                    feature: format!("标量类型 {name}"),
                    span: Some(span),
                }),
                other => Err(CodegenError::Unsupported {
                    feature: format!("未知标量类型 {other}"),
                    span: Some(span),
                }),
            },
            IrType::Function { .. } => Err(CodegenError::Unsupported {
                feature: "函数值".to_owned(),
                span: Some(span),
            }),
            _ => Err(CodegenError::Unsupported {
                feature: "动态或容器类型".to_owned(),
                span: Some(span),
            }),
        }
    }

    /// 返回该标量在 LLVM 文本中的类型名称。
    const fn llvm(self) -> &'static str {
        match self {
            Self::Int => "i64",
            Self::Sint => "i32",
            Self::Float => "double",
            Self::Sfloat => "float",
            Self::Bool => "i1",
            Self::None => "void",
        }
    }

    /// 判断该标量是否为整数类别。
    const fn is_integer(self) -> bool {
        matches!(self, Self::Int | Self::Sint)
    }

    /// 判断该标量是否为浮点类别。
    const fn is_float(self) -> bool {
        matches!(self, Self::Float | Self::Sfloat)
    }

    /// 返回该标量的规范位宽。
    const fn width(self) -> u16 {
        match self {
            Self::Int | Self::Float => 64,
            Self::Sint | Self::Sfloat => 32,
            Self::Bool => 1,
            Self::None => 0,
        }
    }
}

/// 已收集的函数 LLVM 签名。
#[derive(Clone, Debug)]
struct FunctionSignature {
    llvm_name: String,
    parameters: Vec<Scalar>,
    return_type: Scalar,
}

/// 一个局部变量在 LLVM 栈上的槽位描述。
#[derive(Clone, Copy, Debug)]
struct Slot {
    name: usize,
    ty: Scalar,
}

/// 收集声明并生成一个完整 LLVM 模块的状态机。
struct ModuleGenerator<'a> {
    program: &'a IrProgram,
    options: &'a CodegenOptions,
    functions: BTreeMap<String, FunctionSignature>,
    declarations: BTreeSet<String>,
    next_temp: usize,
    next_label: usize,
}

impl<'a> ModuleGenerator<'a> {
    /// 为一份已验证 IR 创建模块生成器。
    fn new(program: &'a IrProgram, options: &'a CodegenOptions) -> Self {
        Self {
            program,
            options,
            functions: BTreeMap::new(),
            declarations: BTreeSet::new(),
            next_temp: 0,
            next_label: 0,
        }
    }

    /// 生成声明、函数、入口和可追踪指纹。
    fn generate(&mut self) -> Result<LlvmModule> {
        self.declaration("declare void @llvm.trap()");
        self.collect_functions()?;
        let mut definitions = Vec::new();
        for statement in &self.program.body {
            if let IrStatementKind::Function {
                name,
                parameters,
                return_type,
                body,
            } = &statement.kind
            {
                definitions.push(self.emit_function(name, parameters, return_type, body)?);
            }
        }
        let entry = self.emit_entry()?;
        let mut text = String::new();
        text.push_str("; Xiao N0-A LLVM module\n");
        text.push_str(&format!("; target = {}\n", self.options.target.triple));
        text.push_str(&format!(
            "target triple = \"{}\"\n\n",
            escape_llvm(&self.options.target.triple)
        ));
        for declaration in &self.declarations {
            text.push_str(declaration);
            text.push('\n');
        }
        if !self.declarations.is_empty() {
            text.push('\n');
        }
        for definition in definitions {
            text.push_str(&definition);
            text.push('\n');
        }
        text.push_str(&entry);
        let fingerprint = format!(
            "xiao-codegen-{CODEGEN_VERSION}-{}",
            stable_hash(
                format!(
                    "{};{}",
                    CODEGEN_VERSION,
                    self.options.target.fingerprint_fields()
                )
                .as_bytes()
            )
        );
        Ok(LlvmModule {
            text,
            target: self.options.target.clone(),
            entry_symbol: "xiao_entry".to_owned(),
            uses_runtime: false,
            runtime_components: Vec::new(),
            runtime_abi_version: None,
            codegen_fingerprint: fingerprint,
        })
    }

    /// 收集函数签名并在发现不支持的参数时提前失败。
    fn collect_functions(&mut self) -> Result<()> {
        for (index, statement) in self.program.body.iter().enumerate() {
            let IrStatementKind::Function {
                name,
                parameters,
                return_type,
                body: _,
            } = &statement.kind
            else {
                continue;
            };
            if self.functions.contains_key(&name.text) {
                return Err(CodegenError::InvalidIr {
                    message: format!("函数 {} 重复定义", name.text),
                });
            }
            if parameters
                .iter()
                .any(|parameter| parameter.kind == "var_args" || parameter.kind == "var_keywords")
            {
                return Err(CodegenError::Unsupported {
                    feature: "*args/**kwargs 形参".to_owned(),
                    span: Some(name.span),
                });
            }
            let parameters = parameters
                .iter()
                .map(|parameter| Scalar::from_type(&parameter.ty, parameter.span))
                .collect::<Result<Vec<_>>>()?;
            let llvm_name = format!("xiao_fn_{index}_{}", sanitize(&name.text));
            let return_type = Scalar::from_type(return_type, statement.span)?;
            self.functions.insert(
                name.text.clone(),
                FunctionSignature {
                    llvm_name,
                    parameters,
                    return_type,
                },
            );
        }
        Ok(())
    }

    /// 发射一个用户函数的 LLVM 定义。
    fn emit_function(
        &mut self,
        name: &IrName,
        parameters: &[xiao_ir::IrParameter],
        return_type: &IrType,
        body: &[IrStatement],
    ) -> Result<String> {
        let signature =
            self.functions
                .get(&name.text)
                .cloned()
                .ok_or_else(|| CodegenError::InvalidIr {
                    message: format!("函数 {} 未登记", name.text),
                })?;
        let ret = Scalar::from_type(return_type, name.span)?;
        let mut slots = BTreeMap::new();
        let mut next_slot = 0;
        for parameter in parameters {
            let ty = Scalar::from_type(&parameter.ty, parameter.span)?;
            insert_slot(&mut slots, &parameter.name.text, ty, &mut next_slot)?;
        }
        collect_slots(body, &mut slots, &mut next_slot)?;
        let args = parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let ty = Scalar::from_type(&parameter.ty, parameter.span)?;
                Ok(format!("{} %arg{index}", ty.llvm()))
            })
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        let mut function = FunctionGenerator::new(self, slots, ret, None);
        function.emit_line(format!(
            "define {} @{}({args}) {{",
            ret.llvm(),
            signature.llvm_name
        ));
        function.emit_label("entry");
        function.emit_slots();
        for (index, parameter) in parameters.iter().enumerate() {
            let slot = function.slot(&parameter.name.text, parameter.span)?;
            let ty = Scalar::from_type(&parameter.ty, parameter.span)?;
            function.emit_line(format!(
                "  store {} %arg{index}, ptr %slot{}",
                ty.llvm(),
                slot.name
            ));
        }
        function.emit_statements(body)?;
        if !function.terminated {
            function.emit_return_default();
        }
        function.emit_line("}");
        Ok(function.lines.join("\n") + "\n")
    }

    /// 发射脚本入口及其 `main` 适配器。
    fn emit_entry(&mut self) -> Result<String> {
        let mut slots = BTreeMap::new();
        let mut next_slot = 0;
        let statements = self
            .program
            .body
            .iter()
            .filter(|statement| !matches!(statement.kind, IrStatementKind::Function { .. }))
            .cloned()
            .collect::<Vec<_>>();
        collect_slots(&statements, &mut slots, &mut next_slot)?;
        let observed = self.options.entry_observation == EntryObservation::ExitCode;
        let observation_slot = observed.then_some(next_slot);
        let entry_return = if observed { Scalar::Int } else { Scalar::None };
        let mut function = FunctionGenerator::new(self, slots, entry_return, observation_slot);
        function.emit_line(format!("define {} @xiao_entry() {{", entry_return.llvm()));
        function.emit_label("entry");
        function.emit_slots();
        if let Some(slot) = observation_slot {
            function.emit_line(format!("  %slot{slot} = alloca i64"));
            function.emit_line(format!("  store i64 0, ptr %slot{slot}"));
        }
        function.emit_statements(&statements)?;
        if !function.terminated {
            if observed {
                let slot = observation_slot.expect("观察模式必须分配观察槽");
                let value = function.module.next_temp();
                function.emit_line(format!("  {value} = load i64, ptr %slot{slot}"));
                function.emit_line(format!("  ret i64 {value}"));
            } else {
                function.emit_line("  ret void");
            }
            function.terminated = true;
        }
        function.emit_line("}");
        let mut output = function.lines.join("\n") + "\n\n";
        if observed {
            output.push_str("define i32 @main() {\nentry:\n  %xiao_exit = call i64 @xiao_entry()\n  %xiao_exit_code = trunc i64 %xiao_exit to i32\n  ret i32 %xiao_exit_code\n}\n");
        } else {
            output.push_str(
                "define i32 @main() {\nentry:\n  call void @xiao_entry()\n  ret i32 0\n}\n",
            );
        }
        Ok(output)
    }

    /// 登记一条去重的 LLVM 声明。
    fn declaration(&mut self, text: impl Into<String>) {
        self.declarations.insert(text.into());
    }

    /// 分配下一个模块级临时值编号。
    fn next_temp(&mut self) -> String {
        let value = format!("%t{}", self.next_temp);
        self.next_temp += 1;
        value
    }

    /// 分配下一个带前缀的基本块标签。
    fn next_label(&mut self, prefix: &str) -> String {
        let value = format!("{prefix}{}", self.next_label);
        self.next_label += 1;
        value
    }
}

/// 负责在单个函数内发射指令和控制流的生成器。
struct FunctionGenerator<'a, 'b> {
    module: &'a mut ModuleGenerator<'b>,
    slots: BTreeMap<String, Slot>,
    return_type: Scalar,
    lines: Vec<String>,
    terminated: bool,
    loops: Vec<(String, String)>,
    observation_slot: Option<usize>,
}

impl<'a, 'b> FunctionGenerator<'a, 'b> {
    /// 创建函数级生成器并接管局部槽表。
    fn new(
        module: &'a mut ModuleGenerator<'b>,
        slots: BTreeMap<String, Slot>,
        return_type: Scalar,
        observation_slot: Option<usize>,
    ) -> Self {
        Self {
            module,
            slots,
            return_type,
            lines: Vec::new(),
            terminated: false,
            loops: Vec::new(),
            observation_slot,
        }
    }

    /// 追加一行 LLVM 文本。
    fn emit_line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    /// 追加基本块标签并标记后续块未终止。
    fn emit_label(&mut self, label: impl Into<String>) {
        self.emit_line(format!("{}:", label.into()));
        self.terminated = false;
    }

    /// 为全部局部槽发射 `alloca`。
    fn emit_slots(&mut self) {
        let mut slots = self.slots.values().copied().collect::<Vec<_>>();
        slots.sort_by_key(|slot| slot.name);
        for slot in slots {
            self.emit_line(format!("  %slot{} = alloca {}", slot.name, slot.ty.llvm()));
        }
    }

    /// 记录入口观察模式下最后一个整数值。
    fn record_observation(&mut self, value: String, ty: Scalar) {
        let Some(slot) = self.observation_slot else {
            return;
        };
        let value = match ty {
            Scalar::Int => value,
            Scalar::Sint | Scalar::Bool => {
                let temp = self.module.next_temp();
                let instruction = if ty == Scalar::Bool { "zext" } else { "sext" };
                self.emit_line(format!(
                    "  {temp} = {instruction} {} {value} to i64",
                    ty.llvm()
                ));
                temp
            }
            // `ExitCode` is a test-only integer observation. Floating and Runtime values
            // remain executable but do not replace the last integer observation.
            Scalar::Float | Scalar::Sfloat | Scalar::None => return,
        };
        self.emit_line(format!("  store i64 {value}, ptr %slot{slot}"));
    }

    /// 按名称查找局部槽并报告缺失名称。
    fn slot(&self, name: &str, span: IrSpan) -> Result<Slot> {
        self.slots
            .get(name)
            .copied()
            .ok_or_else(|| CodegenError::InvalidIr {
                message: format!("名称 {name} 没有局部槽（{}..{}）", span.start, span.end),
            })
    }

    /// 依次发射一组语句，跳过已终止的控制流块。
    fn emit_statements(&mut self, statements: &[IrStatement]) -> Result<()> {
        for statement in statements {
            if self.terminated {
                break;
            }
            self.emit_statement(statement)?;
        }
        Ok(())
    }

    /// 发射单条语句并维护局部控制流状态。
    fn emit_statement(&mut self, statement: &IrStatement) -> Result<()> {
        match &statement.kind {
            IrStatementKind::Expression { value } => {
                let (value, ty) = self.emit_expression(value)?;
                self.record_observation(value, ty);
            }
            IrStatementKind::Assignment { target, value } => {
                let slot = self.slot(&target.text, target.span)?;
                let (value, ty) = self.emit_expression(value)?;
                let value = self.cast_value(value, ty, slot.ty, statement.span)?;
                self.emit_line(format!(
                    "  store {} {value}, ptr %slot{}",
                    slot.ty.llvm(),
                    slot.name
                ));
                self.record_observation(value, slot.ty);
            }
            IrStatementKind::Declaration {
                target,
                declared_type,
                value,
                ..
            } => {
                let slot = self.slot(&target.text, target.span)?;
                if let Some(value) = value {
                    let (value, ty) = self.emit_expression(value)?;
                    let value = self.cast_value(value, ty, slot.ty, target.span)?;
                    self.emit_line(format!(
                        "  store {} {value}, ptr %slot{}",
                        slot.ty.llvm(),
                        slot.name
                    ));
                    self.record_observation(value, slot.ty);
                } else {
                    let ty = Scalar::from_type(declared_type, target.span)?;
                    let value = default_value(ty).to_owned();
                    self.emit_line(format!(
                        "  store {} {value}, ptr %slot{}",
                        ty.llvm(),
                        slot.name
                    ));
                    self.record_observation(value, ty);
                }
            }
            IrStatementKind::ConstDeclaration { target, value, .. } => {
                let slot = self.slot(&target.text, target.span)?;
                let (value, ty) = self.emit_expression(value)?;
                let value = self.cast_value(value, ty, slot.ty, target.span)?;
                self.emit_line(format!(
                    "  store {} {value}, ptr %slot{}",
                    slot.ty.llvm(),
                    slot.name
                ));
                self.record_observation(value, slot.ty);
            }
            IrStatementKind::ExtendedAssignment {
                target,
                operator,
                value,
            } => {
                let IrExpressionKind::Name { name } = &target.kind else {
                    return Err(unsupported("复合赋值目标", Some(target.span)));
                };
                let slot = self.slot(&name.text, name.span)?;
                let left = self.load_slot(slot);
                let (right, right_ty) = self.emit_expression(value)?;
                let (result, result_ty) = self.emit_binary_values(
                    operator.trim_end_matches('='),
                    (left, slot.ty),
                    (right, right_ty),
                    target.span,
                    &IrType::Scalar {
                        name: scalar_name(slot.ty).to_owned(),
                    },
                )?;
                let result = self.cast_value(result, result_ty, slot.ty, target.span)?;
                self.emit_line(format!(
                    "  store {} {result}, ptr %slot{}",
                    slot.ty.llvm(),
                    slot.name
                ));
                self.record_observation(result, slot.ty);
            }
            IrStatementKind::If {
                condition,
                body,
                elif_branches,
                else_body,
            } => self.emit_if(condition, body, elif_branches, else_body.as_deref())?,
            IrStatementKind::While { condition, body } => self.emit_while(condition, body)?,
            IrStatementKind::Return { value } => {
                if let Some(value) = value {
                    let (value, ty) = self.emit_expression(value)?;
                    let value = self.cast_value(value, ty, self.return_type, statement.span)?;
                    self.emit_line(format!("  ret {} {value}", self.return_type.llvm()));
                } else if self.return_type == Scalar::None {
                    self.emit_line("  ret void");
                } else {
                    self.emit_return_default();
                }
                self.terminated = true;
            }
            IrStatementKind::Break => {
                let Some((_, break_label)) = self.loops.last() else {
                    return Err(unsupported("循环外 break", Some(statement.span)));
                };
                self.emit_line(format!("  br label %{break_label}"));
                self.terminated = true;
            }
            IrStatementKind::Continue => {
                let Some((continue_label, _)) = self.loops.last() else {
                    return Err(unsupported("循环外 continue", Some(statement.span)));
                };
                self.emit_line(format!("  br label %{continue_label}"));
                self.terminated = true;
            }
            IrStatementKind::Function { .. } => {
                return Err(unsupported("嵌套函数", Some(statement.span)));
            }
            IrStatementKind::Import { .. } => {
                return Err(unsupported("跨模块导入", Some(statement.span)));
            }
            IrStatementKind::For { .. } => {
                return Err(unsupported("for/容器迭代", Some(statement.span)));
            }
            IrStatementKind::Table { .. } => {
                return Err(unsupported("表", Some(statement.span)));
            }
            IrStatementKind::Try { .. } | IrStatementKind::Raise { .. } => {
                return Err(unsupported("异常控制流", Some(statement.span)));
            }
        }
        Ok(())
    }

    /// 发射 `if`、`elif` 和 `else` 分支。
    fn emit_if(
        &mut self,
        condition: &IrExpression,
        body: &[IrStatement],
        elif_branches: &[xiao_ir::IrElifBranch],
        else_body: Option<&[IrStatement]>,
    ) -> Result<()> {
        let condition_span = condition.span;
        let (condition, ty) = self.emit_expression(condition)?;
        if ty != Scalar::Bool {
            return Err(unsupported("非 bool 条件", Some(condition_span)));
        }
        let then_label = self.module.next_label("if.then");
        let else_label = self.module.next_label("if.next");
        let merge_label = self.module.next_label("if.merge");
        self.emit_line(format!(
            "  br i1 {condition}, label %{then_label}, label %{else_label}"
        ));
        self.terminated = true;
        self.emit_label(&then_label);
        self.emit_statements(body)?;
        if !self.terminated {
            self.emit_line(format!("  br label %{merge_label}"));
            self.terminated = true;
        }
        self.emit_label(&else_label);
        if elif_branches.is_empty() {
            if let Some(else_body) = else_body {
                self.emit_statements(else_body)?;
            }
        } else {
            self.emit_elif_chain(elif_branches, else_body, merge_label.clone())?;
        }
        if !self.terminated {
            self.emit_line(format!("  br label %{merge_label}"));
            self.terminated = true;
        }
        self.emit_label(merge_label);
        Ok(())
    }

    /// 递归发射一条 `elif` 链并汇合到统一出口。
    fn emit_elif_chain(
        &mut self,
        branches: &[xiao_ir::IrElifBranch],
        else_body: Option<&[IrStatement]>,
        merge_label: String,
    ) -> Result<()> {
        let branch = &branches[0];
        let (condition, ty) = self.emit_expression(&branch.condition)?;
        if ty != Scalar::Bool {
            return Err(unsupported("非 bool elif 条件", Some(branch.span)));
        }
        let then_label = self.module.next_label("elif.then");
        let next_label = self.module.next_label("elif.next");
        self.emit_line(format!(
            "  br i1 {condition}, label %{then_label}, label %{next_label}"
        ));
        self.terminated = true;
        self.emit_label(then_label);
        self.emit_statements(&branch.body)?;
        if !self.terminated {
            self.emit_line(format!("  br label %{merge_label}"));
            self.terminated = true;
        }
        self.emit_label(next_label);
        if branches.len() > 1 {
            self.emit_elif_chain(&branches[1..], else_body, merge_label)?;
        } else {
            if let Some(else_body) = else_body {
                self.emit_statements(else_body)?;
            }
            // 没有 else 时，最后一个条件为假仍然必须离开当前基本块；否则
            // 会留下一个没有终结指令的 `elif.next`，llvm-as 会拒绝整份模块。
            if !self.terminated {
                self.emit_line(format!("  br label %{merge_label}"));
                self.terminated = true;
            }
        }
        Ok(())
    }

    /// 发射带显式条件块和循环回边的 `while`。
    fn emit_while(&mut self, condition: &IrExpression, body: &[IrStatement]) -> Result<()> {
        let condition_label = self.module.next_label("while.cond");
        let body_label = self.module.next_label("while.body");
        let end_label = self.module.next_label("while.end");
        self.emit_line(format!("  br label %{condition_label}"));
        self.terminated = true;
        self.emit_label(&condition_label);
        let (condition, ty) = self.emit_expression(condition)?;
        if ty != Scalar::Bool {
            return Err(unsupported("非 bool while 条件", None));
        }
        self.emit_line(format!(
            "  br i1 {condition}, label %{body_label}, label %{end_label}"
        ));
        self.terminated = true;
        self.emit_label(&body_label);
        self.loops
            .push((condition_label.clone(), end_label.clone()));
        self.emit_statements(body)?;
        self.loops.pop();
        if !self.terminated {
            self.emit_line(format!("  br label %{condition_label}"));
            self.terminated = true;
        }
        self.emit_label(end_label);
        Ok(())
    }

    /// 将一个 IR 表达式发射为 LLVM 值和静态标量类别。
    fn emit_expression(&mut self, expression: &IrExpression) -> Result<(String, Scalar)> {
        let expected = Scalar::from_type(&expression.ty, expression.span)?;
        match &expression.kind {
            IrExpressionKind::Literal { literal, text } => {
                if literal == "str" {
                    return Err(unsupported("str 字面量", Some(expression.span)));
                }
                Ok((literal_value(expected, text, expression.span)?, expected))
            }
            IrExpressionKind::Name { name } => {
                let slot = self.slot(&name.text, name.span)?;
                Ok((self.load_slot(slot), slot.ty))
            }
            IrExpressionKind::Group { expression } => self.emit_expression(expression),
            IrExpressionKind::Unary { operator, operand } => {
                let (value, ty) = self.emit_expression(operand)?;
                self.emit_unary(operator, value, ty, expected, expression.span)
            }
            IrExpressionKind::Binary {
                operator,
                left,
                right,
            } => {
                // 09-B0 的生产降低器尚未给逻辑/身份运算分配冻结 TAC 语义；原生侧
                // 必须保持同一接受边界，不能用 eager LLVM 指令悄悄扩大语言子集。
                if matches!(operator.as_str(), "and" | "or" | "is" | "is not") {
                    return Err(unsupported(
                        format!("运算 {operator}（生产字节码降低尚未接通）"),
                        Some(expression.span),
                    ));
                }
                let (left, left_ty) = self.emit_expression(left)?;
                let (right, right_ty) = self.emit_expression(right)?;
                self.emit_binary_values(
                    operator,
                    (left, left_ty),
                    (right, right_ty),
                    expression.span,
                    &expression.ty,
                )
            }
            IrExpressionKind::Call { callee, arguments } => {
                self.emit_call(callee, arguments, expected, expression.span)
            }
            IrExpressionKind::Cast {
                expression: inner,
                target,
            } => {
                let (value, source) = self.emit_expression(inner)?;
                let target = scalar_name_type(target, expression.span)?;
                let value = self.cast_value(value, source, target, expression.span)?;
                Ok((value, target))
            }
            IrExpressionKind::Array { .. }
            | IrExpressionKind::Tuple { .. }
            | IrExpressionKind::DictTable { .. }
            | IrExpressionKind::Set { .. }
            | IrExpressionKind::DictColumn { .. }
            | IrExpressionKind::NewCall { .. }
            | IrExpressionKind::Member { .. }
            | IrExpressionKind::Selector { .. } => {
                Err(unsupported("动态值或容器表达式", Some(expression.span)))
            }
        }
    }

    /// 发射静态函数调用并检查参数和返回类型。
    fn emit_call(
        &mut self,
        callee: &IrExpression,
        arguments: &[xiao_ir::IrCallArgument],
        expected: Scalar,
        span: IrSpan,
    ) -> Result<(String, Scalar)> {
        let IrExpressionKind::Name { name } = &callee.kind else {
            return Err(unsupported("动态派发调用", Some(span)));
        };
        let signature = self
            .module
            .functions
            .get(&name.text)
            .cloned()
            .ok_or_else(|| unsupported(format!("未知函数 {}", name.text), Some(name.span)))?;
        if arguments.len() != signature.parameters.len() {
            return Err(unsupported("函数实参数量不匹配", Some(span)));
        }
        let mut values = Vec::with_capacity(arguments.len());
        for (argument, parameter_ty) in arguments.iter().zip(signature.parameters.iter().copied()) {
            if argument.kind != "positional" {
                return Err(unsupported("关键字或展开实参", Some(argument.span)));
            }
            let (value, ty) = self.emit_expression(&argument.value)?;
            values.push(format!(
                "{} {}",
                parameter_ty.llvm(),
                self.cast_value(value, ty, parameter_ty, argument.span)?
            ));
        }
        if signature.return_type == Scalar::None {
            self.emit_line(format!(
                "  call void @{}({})",
                signature.llvm_name,
                values.join(", ")
            ));
            Ok((default_value(expected).to_owned(), expected))
        } else {
            let temp = self.module.next_temp();
            self.emit_line(format!(
                "  {temp} = call {} @{}({})",
                signature.return_type.llvm(),
                signature.llvm_name,
                values.join(", ")
            ));
            let value = self.cast_value(temp, signature.return_type, expected, span)?;
            Ok((value, expected))
        }
    }

    /// 发射一元运算并保持固定宽度语义。
    fn emit_unary(
        &mut self,
        operator: &str,
        value: String,
        ty: Scalar,
        expected: Scalar,
        span: IrSpan,
    ) -> Result<(String, Scalar)> {
        match operator {
            "+" => Ok((self.cast_value(value, ty, expected, span)?, expected)),
            "not" => {
                if ty != Scalar::Bool {
                    return Err(unsupported("非 bool not", Some(span)));
                }
                let temp = self.module.next_temp();
                self.emit_line(format!("  {temp} = xor i1 {value}, true"));
                Ok((temp, Scalar::Bool))
            }
            "-" => {
                if ty.is_integer() {
                    let zero = default_value(ty);
                    let value =
                        self.checked_integer_bin("sub", ty, zero.to_owned(), value, span)?;
                    Ok((self.cast_value(value, ty, expected, span)?, expected))
                } else if ty.is_float() {
                    let temp = self.module.next_temp();
                    self.emit_line(format!("  {temp} = fneg {} {value}", ty.llvm()));
                    self.check_finite(temp.clone(), ty, span)?;
                    Ok((self.cast_value(temp, ty, expected, span)?, expected))
                } else {
                    Err(unsupported("非数值一元负号", Some(span)))
                }
            }
            _ => Err(unsupported(format!("一元运算 {operator}"), Some(span))),
        }
    }

    /// 发射二元算术、比较或逻辑运算。
    fn emit_binary_values(
        &mut self,
        operator: &str,
        left_input: (String, Scalar),
        right_input: (String, Scalar),
        span: IrSpan,
        result_type: &IrType,
    ) -> Result<(String, Scalar)> {
        let (left, left_ty) = left_input;
        let (right, right_ty) = right_input;
        if matches!(operator, "==" | "!=" | "<" | "<=" | ">" | ">=") {
            if left_ty == Scalar::Bool && right_ty == Scalar::Bool {
                let predicate = if operator == "==" {
                    "eq"
                } else if operator == "!=" {
                    "ne"
                } else {
                    return Err(unsupported("布尔排序比较", Some(span)));
                };
                let temp = self.module.next_temp();
                self.emit_line(format!("  {temp} = icmp {predicate} i1 {left}, {right}"));
                return Ok((temp, Scalar::Bool));
            }
            let common = common_numeric(left_ty, right_ty)
                .ok_or_else(|| unsupported("比较类型", Some(span)))?;
            let left = self.cast_value(left, left_ty, common, span)?;
            let right = self.cast_value(right, right_ty, common, span)?;
            let predicate = match operator {
                "==" => "eq",
                "!=" => {
                    if common.is_float() {
                        "une"
                    } else {
                        "ne"
                    }
                }
                "<" => {
                    if common.is_float() {
                        "olt"
                    } else {
                        "slt"
                    }
                }
                "<=" => {
                    if common.is_float() {
                        "ole"
                    } else {
                        "sle"
                    }
                }
                ">" => {
                    if common.is_float() {
                        "ogt"
                    } else {
                        "sgt"
                    }
                }
                ">=" => {
                    if common.is_float() {
                        "oge"
                    } else {
                        "sge"
                    }
                }
                _ => unreachable!(),
            };
            let temp = self.module.next_temp();
            let instruction = if common.is_float() { "fcmp" } else { "icmp" };
            self.emit_line(format!(
                "  {temp} = {instruction} {predicate} {} {left}, {right}",
                common.llvm()
            ));
            return Ok((temp, Scalar::Bool));
        }
        if (operator == "+" || operator == "-") && left_ty == Scalar::Bool && right_ty.is_integer()
        {
            let right = self.cast_value(right, right_ty, Scalar::Int, span)?;
            let left_i = self.module.next_temp();
            self.emit_line(format!("  {left_i} = zext i1 {left} to i64"));
            let parity = self.module.next_temp();
            self.emit_line(format!("  {parity} = and i64 {right}, 1"));
            let combined = self.module.next_temp();
            self.emit_line(format!("  {combined} = xor i64 {left_i}, {parity}"));
            let value = self.module.next_temp();
            self.emit_line(format!("  {value} = trunc i64 {combined} to i1"));
            return Ok((value, Scalar::Bool));
        }
        let result = Scalar::from_type(result_type, span)?;
        let common = if operator == "/" {
            common_division(left_ty, right_ty)
        } else {
            common_numeric(left_ty, right_ty)
        }
        .ok_or_else(|| unsupported("算术类型", Some(span)))?;
        let left = self.cast_value(left, left_ty, common, span)?;
        let right = self.cast_value(right, right_ty, common, span)?;
        let value = match operator {
            "+" => {
                if common.is_integer() {
                    self.checked_integer_bin("add", common, left, right, span)?
                } else {
                    self.float_bin("fadd", common, left, right, span)?
                }
            }
            "-" => {
                if common.is_integer() {
                    self.checked_integer_bin("sub", common, left, right, span)?
                } else {
                    self.float_bin("fsub", common, left, right, span)?
                }
            }
            "*" => {
                if common.is_integer() {
                    self.checked_integer_bin("mul", common, left, right, span)?
                } else {
                    self.float_bin("fmul", common, left, right, span)?
                }
            }
            "/" => self.float_bin("fdiv", common, left, right, span)?,
            "//" => self.floor_div(common, left, right, span)?,
            "%" => self.floor_rem(common, left, right, span)?,
            "**" => self.power(common, left, right, result, span)?,
            _ => return Err(unsupported(format!("二元运算 {operator}"), Some(span))),
        };
        Ok((self.cast_value(value, common, result, span)?, result))
    }

    /// 发射带溢出检查的整数二元运算。
    fn checked_integer_bin(
        &mut self,
        op: &str,
        ty: Scalar,
        left: String,
        right: String,
        span: IrSpan,
    ) -> Result<String> {
        let intrinsic = match (op, ty) {
            ("add", Scalar::Int) => "llvm.sadd.with.overflow.i64",
            ("sub", Scalar::Int) => "llvm.ssub.with.overflow.i64",
            ("mul", Scalar::Int) => "llvm.smul.with.overflow.i64",
            ("add", Scalar::Sint) => "llvm.sadd.with.overflow.i32",
            ("sub", Scalar::Sint) => "llvm.ssub.with.overflow.i32",
            ("mul", Scalar::Sint) => "llvm.smul.with.overflow.i32",
            _ => return Err(unsupported("整数算术宽度", Some(span))),
        };
        self.module.declaration(format!(
            "declare {{{}, i1}} @{intrinsic}({}, {})",
            ty.llvm(),
            ty.llvm(),
            ty.llvm()
        ));
        let pair = self.module.next_temp();
        self.emit_line(format!(
            "  {pair} = call {{{}, i1}} @{intrinsic}({} {left}, {} {right})",
            ty.llvm(),
            ty.llvm(),
            ty.llvm()
        ));
        let value = self.module.next_temp();
        let overflow = self.module.next_temp();
        self.emit_line(format!(
            "  {value} = extractvalue {{{}, i1}} {pair}, 0",
            ty.llvm()
        ));
        self.emit_line(format!(
            "  {overflow} = extractvalue {{{}, i1}} {pair}, 1",
            ty.llvm()
        ));
        self.branch_on_trap(overflow, span);
        Ok(value)
    }

    /// 发射浮点二元运算并检查有限性。
    fn float_bin(
        &mut self,
        op: &str,
        ty: Scalar,
        left: String,
        right: String,
        span: IrSpan,
    ) -> Result<String> {
        let value = self.module.next_temp();
        self.emit_line(format!("  {value} = {op} {} {left}, {right}", ty.llvm()));
        self.check_finite(value.clone(), ty, span)?;
        Ok(value)
    }

    /// 对浮点结果发射 NaN/无穷检查。
    fn check_finite(&mut self, value: String, ty: Scalar, span: IrSpan) -> Result<()> {
        let max = match ty {
            Scalar::Float => "1.7976931348623157e+308",
            Scalar::Sfloat => "3.4028234663852886e+38",
            _ => return Ok(()),
        };
        let ordered = self.module.next_temp();
        let lower = self.module.next_temp();
        let upper = self.module.next_temp();
        let bounded = self.module.next_temp();
        self.emit_line(format!(
            "  {ordered} = fcmp ord {} {value}, {value}",
            ty.llvm()
        ));
        self.emit_line(format!(
            "  {lower} = fcmp oge {} {value}, -{max}",
            ty.llvm()
        ));
        self.emit_line(format!("  {upper} = fcmp ole {} {value}, {max}", ty.llvm()));
        self.emit_line(format!("  {bounded} = and i1 {lower}, {upper}"));
        let ok = self.module.next_label("finite.ok");
        let trap = self.module.next_label("finite.trap");
        self.emit_line(format!("  br i1 {ordered}, label %{ok}, label %{trap}"));
        self.terminated = true;
        self.emit_label(trap);
        self.emit_line("  call void @llvm.trap()");
        self.emit_line("  unreachable");
        self.terminated = true;
        self.emit_label(&ok);
        let finite_ok = self.module.next_label("finite.continue");
        let range_trap = self.module.next_label("finite.range_trap");
        self.emit_line(format!(
            "  br i1 {bounded}, label %{finite_ok}, label %{range_trap}"
        ));
        self.terminated = true;
        self.emit_label(&range_trap);
        self.emit_line("  call void @llvm.trap()");
        self.emit_line("  unreachable");
        self.terminated = true;
        self.emit_label(finite_ok);
        let _ = span;
        Ok(())
    }

    /// 发射 Xiao 语义的整数向下除法。
    fn floor_div(
        &mut self,
        ty: Scalar,
        left: String,
        right: String,
        span: IrSpan,
    ) -> Result<String> {
        if !ty.is_integer() {
            return Err(unsupported("浮点整除", Some(span)));
        }
        self.check_divisor(ty, left.clone(), right.clone(), span)?;
        let quotient = self.module.next_temp();
        let remainder = self.module.next_temp();
        self.emit_line(format!("  {quotient} = sdiv {} {left}, {right}", ty.llvm()));
        self.emit_line(format!(
            "  {remainder} = srem {} {left}, {right}",
            ty.llvm()
        ));
        let rem_zero = self.module.next_temp();
        let left_sign = self.module.next_temp();
        let right_sign = self.module.next_temp();
        let signs_differ = self.module.next_temp();
        let needs_adjust = self.module.next_temp();
        self.emit_line(format!(
            "  {rem_zero} = icmp eq {} {remainder}, 0",
            ty.llvm()
        ));
        self.emit_line(format!("  {left_sign} = icmp slt {} {left}, 0", ty.llvm()));
        self.emit_line(format!(
            "  {right_sign} = icmp slt {} {right}, 0",
            ty.llvm()
        ));
        self.emit_line(format!(
            "  {signs_differ} = xor i1 {left_sign}, {right_sign}"
        ));
        let not_zero = self.module.next_temp();
        self.emit_line(format!("  {not_zero} = xor i1 {rem_zero}, true"));
        self.emit_line(format!(
            "  {needs_adjust} = and i1 {not_zero}, {signs_differ}"
        ));
        let adjusted = self.module.next_temp();
        let merge = self.module.next_label("floordiv.merge");
        let adjust = self.module.next_label("floordiv.adjust");
        let keep = self.module.next_label("floordiv.keep");
        self.emit_line(format!(
            "  br i1 {needs_adjust}, label %{adjust}, label %{keep}"
        ));
        self.terminated = true;
        self.emit_label(&adjust);
        let one = "1";
        self.emit_line(format!(
            "  {adjusted} = sub {} {quotient}, {one}",
            ty.llvm()
        ));
        self.emit_line(format!("  br label %{merge}"));
        self.terminated = true;
        self.emit_label(&keep);
        self.emit_line(format!("  br label %{merge}"));
        self.terminated = true;
        self.emit_label(merge);
        let result = self.module.next_temp();
        self.emit_line(format!(
            "  {result} = phi {} [{adjusted}, %{adjust}], [{quotient}, %{keep}]",
            ty.llvm()
        ));
        Ok(result)
    }

    /// 发射与向下除法配套的整数余数。
    fn floor_rem(
        &mut self,
        ty: Scalar,
        left: String,
        right: String,
        span: IrSpan,
    ) -> Result<String> {
        if !ty.is_integer() {
            return Err(unsupported("浮点取模", Some(span)));
        }
        let quotient = self.floor_div(ty, left.clone(), right.clone(), span)?;
        let product = self.module.next_temp();
        self.emit_line(format!(
            "  {product} = mul {} {quotient}, {right}",
            ty.llvm()
        ));
        let result = self.module.next_temp();
        self.emit_line(format!("  {result} = sub {} {left}, {product}", ty.llvm()));
        Ok(result)
    }

    /// 检查除数为零及最小值除以负一的溢出组合。
    fn check_divisor(
        &mut self,
        ty: Scalar,
        left: String,
        right: String,
        span: IrSpan,
    ) -> Result<()> {
        let zero = self.module.next_temp();
        self.emit_line(format!("  {zero} = icmp eq {} {right}, 0", ty.llvm()));
        self.branch_on_trap(zero, span);
        let min = if ty == Scalar::Int {
            i64::MIN.to_string()
        } else {
            i32::MIN.to_string()
        };
        let min_cmp = self.module.next_temp();
        let neg_one = self.module.next_temp();
        self.emit_line(format!("  {min_cmp} = icmp eq {} {left}, {min}", ty.llvm()));
        self.emit_line(format!("  {neg_one} = icmp eq {} {right}, -1", ty.llvm()));
        let invalid = self.module.next_temp();
        self.emit_line(format!("  {invalid} = and i1 {min_cmp}, {neg_one}"));
        self.branch_on_trap(invalid, span);
        Ok(())
    }

    /// 发射浮点幂运算并拒绝尚未接入的动态整数幂。
    fn power(
        &mut self,
        ty: Scalar,
        left: String,
        right: String,
        result: Scalar,
        span: IrSpan,
    ) -> Result<String> {
        if ty.is_float() {
            let intrinsic = if ty == Scalar::Float {
                "llvm.pow.f64"
            } else {
                "llvm.pow.f32"
            };
            self.module.declaration(format!(
                "declare {} @{intrinsic}({}, {})",
                ty.llvm(),
                ty.llvm(),
                ty.llvm()
            ));
            let value = self.module.next_temp();
            self.emit_line(format!(
                "  {value} = call {} @{intrinsic}({} {left}, {} {right})",
                ty.llvm(),
                ty.llvm(),
                ty.llvm()
            ));
            self.check_finite(value.clone(), ty, span)?;
            return Ok(value);
        }
        // Dynamic integer exponents need a dedicated loop and are deliberately deferred to the
        // Runtime batch. Constant exponents still exercise checked integer multiplication.
        let _ = result;
        Err(unsupported("动态整数幂", Some(span)))
    }

    /// 发射固定宽度标量转换并检查窄化边界。
    fn cast_value(
        &mut self,
        value: String,
        source: Scalar,
        target: Scalar,
        span: IrSpan,
    ) -> Result<String> {
        if source == target || target == Scalar::None {
            return Ok(value);
        }
        if source == Scalar::Bool && target.is_integer() {
            let temp = self.module.next_temp();
            self.emit_line(format!("  {temp} = zext i1 {value} to {}", target.llvm()));
            return Ok(temp);
        }
        if source.is_integer() && target == Scalar::Bool {
            let temp = self.module.next_temp();
            self.emit_line(format!("  {temp} = icmp ne {} {value}, 0", source.llvm()));
            return Ok(temp);
        }
        if source.is_integer() && target.is_integer() {
            if source.width() < target.width() {
                let temp = self.module.next_temp();
                self.emit_line(format!(
                    "  {temp} = sext {} {value} to {}",
                    source.llvm(),
                    target.llvm()
                ));
                return Ok(temp);
            }
            if source.width() > target.width() {
                let max = if target == Scalar::Sint {
                    i32::MAX.to_string()
                } else {
                    i64::MAX.to_string()
                };
                let min = if target == Scalar::Sint {
                    i32::MIN.to_string()
                } else {
                    i64::MIN.to_string()
                };
                let hi = self.module.next_temp();
                let lo = self.module.next_temp();
                let bad = self.module.next_temp();
                self.emit_line(format!(
                    "  {hi} = icmp sgt {} {value}, {max}",
                    source.llvm()
                ));
                self.emit_line(format!(
                    "  {lo} = icmp slt {} {value}, {min}",
                    source.llvm()
                ));
                self.emit_line(format!("  {bad} = or i1 {hi}, {lo}"));
                self.branch_on_trap(bad, span);
                let temp = self.module.next_temp();
                self.emit_line(format!(
                    "  {temp} = trunc {} {value} to {}",
                    source.llvm(),
                    target.llvm()
                ));
                return Ok(temp);
            }
        }
        if source.is_integer() && target.is_float() {
            let temp = self.module.next_temp();
            self.emit_line(format!(
                "  {temp} = sitofp {} {value} to {}",
                source.llvm(),
                target.llvm()
            ));
            return Ok(temp);
        }
        if source.is_float() && target.is_float() {
            let temp = self.module.next_temp();
            if source.width() < target.width() {
                self.emit_line(format!(
                    "  {temp} = fpext {} {value} to {}",
                    source.llvm(),
                    target.llvm()
                ));
            } else {
                self.emit_line(format!(
                    "  {temp} = fptrunc {} {value} to {}",
                    source.llvm(),
                    target.llvm()
                ));
                self.check_finite(temp.clone(), target, span)?;
            }
            return Ok(temp);
        }
        if source.is_float() && target.is_integer() {
            // `fptosi` 的合法区间是 [MIN, 2^width)，不能把 MAX 写成
            // `2^width - 1`：对 double 来说 i64::MAX 会舍入到 2^63，边界值
            // 若被放行会把未定义的 LLVM 转换带进生成程序。
            let upper_exclusive = if target == Scalar::Int {
                "9223372036854775808.0"
            } else {
                "2147483648.0"
            };
            let min = if target == Scalar::Int {
                "-9223372036854775808.0"
            } else {
                "-2147483648.0"
            };
            let hi = self.module.next_temp();
            let lo = self.module.next_temp();
            let nan = self.module.next_temp();
            let bad = self.module.next_temp();
            self.emit_line(format!(
                "  {hi} = fcmp oge {} {value}, {upper_exclusive}",
                source.llvm()
            ));
            self.emit_line(format!(
                "  {lo} = fcmp olt {} {value}, {min}",
                source.llvm()
            ));
            self.emit_line(format!(
                "  {nan} = fcmp uno {} {value}, {value}",
                source.llvm()
            ));
            self.emit_line(format!("  {bad} = or i1 {hi}, {lo}"));
            let bad2 = self.module.next_temp();
            self.emit_line(format!("  {bad2} = or i1 {bad}, {nan}"));
            self.branch_on_trap(bad2, span);
            let temp = self.module.next_temp();
            self.emit_line(format!(
                "  {temp} = fptosi {} {value} to {}",
                source.llvm(),
                target.llvm()
            ));
            return Ok(temp);
        }
        Err(unsupported("标量转换", Some(span)))
    }

    /// 从局部槽加载一个值并分配临时编号。
    fn load_slot(&mut self, slot: Slot) -> String {
        let temp = self.module.next_temp();
        self.emit_line(format!(
            "  {temp} = load {}, ptr %slot{}",
            slot.ty.llvm(),
            slot.name
        ));
        temp
    }

    /// 把失败条件分支到 `llvm.trap`，并继续生成成功块。
    fn branch_on_trap(&mut self, condition: String, _span: IrSpan) {
        let trap = self.module.next_label("xiao.trap");
        let continue_label = self.module.next_label("xiao.continue");
        self.emit_line(format!(
            "  br i1 {condition}, label %{trap}, label %{continue_label}"
        ));
        self.terminated = true;
        self.emit_label(trap);
        self.emit_line("  call void @llvm.trap()");
        self.emit_line("  unreachable");
        self.terminated = true;
        self.emit_label(continue_label);
    }

    /// 为尚未显式返回的函数发射默认返回值。
    fn emit_return_default(&mut self) {
        if self.return_type == Scalar::None {
            self.emit_line("  ret void");
        } else {
            self.emit_line(format!(
                "  ret {} {}",
                self.return_type.llvm(),
                default_value(self.return_type)
            ));
        }
        self.terminated = true;
    }
}

/// 递归收集语句树中的局部槽并保持声明类型。
fn collect_slots(
    statements: &[IrStatement],
    slots: &mut BTreeMap<String, Slot>,
    next_slot: &mut usize,
) -> Result<()> {
    for statement in statements {
        match &statement.kind {
            IrStatementKind::Assignment { target, value } => {
                let ty = Scalar::from_type(&value.ty, value.span)?;
                // 已有声明/参数的槽位拥有稳定目标宽度；后续赋值交给发射阶段
                // 做受检转换，不因来源表达式宽度不同而重复建槽或误报冲突。
                if !slots.contains_key(&target.text) {
                    insert_slot(slots, &target.text, ty, next_slot)?;
                }
            }
            IrStatementKind::Declaration {
                target,
                declared_type,
                value: _,
                ..
            } => {
                // 显式声明类型是槽位的契约；初始化表达式只提供待转换的来源
                // 类型。若反过来以 value.ty 建槽，`sint x = 1` 会错误生成 i64。
                let ty = Scalar::from_type(declared_type, target.span)?;
                insert_slot(slots, &target.text, ty, next_slot)?;
            }
            IrStatementKind::ConstDeclaration {
                target,
                declared_type,
                value,
            } => {
                let ty = match declared_type.as_deref() {
                    Some(name) => scalar_name_type(name, target.span)?,
                    None => Scalar::from_type(&value.ty, value.span)?,
                };
                insert_slot(slots, &target.text, ty, next_slot)?;
            }
            IrStatementKind::ExtendedAssignment { target, .. } => {
                if let IrExpressionKind::Name { name } = &target.kind {
                    if !slots.contains_key(&name.text) {
                        return Err(unsupported("未声明的复合赋值目标", Some(name.span)));
                    }
                } else {
                    return Err(unsupported("复杂复合赋值目标", Some(target.span)));
                }
            }
            IrStatementKind::If {
                body,
                elif_branches,
                else_body,
                ..
            } => {
                collect_slots(body, slots, next_slot)?;
                for branch in elif_branches {
                    collect_slots(&branch.body, slots, next_slot)?;
                }
                if let Some(body) = else_body {
                    collect_slots(body, slots, next_slot)?;
                }
            }
            IrStatementKind::While { body, .. } | IrStatementKind::For { body, .. } => {
                collect_slots(body, slots, next_slot)?;
            }
            IrStatementKind::Try {
                body,
                catches,
                finally_body,
            } => {
                collect_slots(body, slots, next_slot)?;
                for clause in catches {
                    collect_slots(&clause.body, slots, next_slot)?;
                }
                if let Some(body) = finally_body {
                    collect_slots(body, slots, next_slot)?;
                }
            }
            IrStatementKind::Function { .. }
            | IrStatementKind::Expression { .. }
            | IrStatementKind::Import { .. }
            | IrStatementKind::Return { .. }
            | IrStatementKind::Break
            | IrStatementKind::Continue
            | IrStatementKind::Table { .. }
            | IrStatementKind::Raise { .. } => {}
        }
    }
    Ok(())
}

/// 插入一个局部槽，并拒绝同名不兼容类型。
fn insert_slot(
    slots: &mut BTreeMap<String, Slot>,
    name: &str,
    ty: Scalar,
    next_slot: &mut usize,
) -> Result<()> {
    if let Some(existing) = slots.get(name) {
        if existing.ty != ty && existing.ty != Scalar::None && ty != Scalar::None {
            return Err(CodegenError::InvalidIr {
                message: format!("名称 {name} 的类型前后不一致"),
            });
        }
        return Ok(());
    }
    slots.insert(
        name.to_owned(),
        Slot {
            name: *next_slot,
            ty,
        },
    );
    *next_slot += 1;
    Ok(())
}

/// 将文本标量名称解析为内部类别。
fn scalar_name_type(name: &str, span: IrSpan) -> Result<Scalar> {
    Scalar::from_type(
        &IrType::Scalar {
            name: name.to_owned(),
        },
        span,
    )
}

/// 返回内部标量类别的 Xiao 名称。
fn scalar_name(scalar: Scalar) -> &'static str {
    match scalar {
        Scalar::Int => "int",
        Scalar::Sint => "sint",
        Scalar::Float => "float",
        Scalar::Sfloat => "sfloat",
        Scalar::Bool => "bool",
        Scalar::None => "none",
    }
}

/// 计算两个标量的公共数值类型。
fn common_numeric(left: Scalar, right: Scalar) -> Option<Scalar> {
    if left == right {
        return Some(left);
    }
    if left == Scalar::Bool
        || right == Scalar::Bool
        || left == Scalar::None
        || right == Scalar::None
    {
        return None;
    }
    if left.is_float() || right.is_float() {
        return Some(if left == Scalar::Float || right == Scalar::Float {
            Scalar::Float
        } else {
            Scalar::Sfloat
        });
    }
    Some(Scalar::Int)
}

/// 计算除法的公共类型，并把整数除法提升为浮点。
fn common_division(left: Scalar, right: Scalar) -> Option<Scalar> {
    let common = common_numeric(left, right)?;
    Some(match common {
        Scalar::Int => Scalar::Float,
        Scalar::Sint => Scalar::Sfloat,
        other => other,
    })
}

/// 校验字面量并转换为 LLVM 接受的文本。
fn literal_value(ty: Scalar, text: &str, span: IrSpan) -> Result<String> {
    let text = text.replace('_', "");
    match ty {
        Scalar::Int => text
            .parse::<i64>()
            .map(|value| value.to_string())
            .map_err(|_| invalid_literal(text, span)),
        Scalar::Sint => text
            .parse::<i32>()
            .map(|value| value.to_string())
            .map_err(|_| invalid_literal(text, span)),
        Scalar::Bool => match text.as_str() {
            "true" => Ok("true".to_owned()),
            "false" => Ok("false".to_owned()),
            _ => Err(invalid_literal(text, span)),
        },
        Scalar::Float => match text.parse::<f64>() {
            Ok(value) if value.is_finite() => Ok(format_float(value)),
            _ => Err(invalid_literal(text, span)),
        },
        Scalar::Sfloat => match text.parse::<f32>() {
            Ok(value) if value.is_finite() => Ok(format_float(value as f64)),
            _ => Err(invalid_literal(text, span)),
        },
        Scalar::None => Ok("".to_owned()),
    }
}

/// 构造带源码区间的非法字面量错误。
fn invalid_literal(text: String, span: IrSpan) -> CodegenError {
    CodegenError::InvalidIr {
        message: format!("字面量 {text:?} 无法编码（{}..{}）", span.start, span.end),
    }
}

/// 以稳定科学计数法格式化浮点常量。
fn format_float(value: f64) -> String {
    let text = format!("{value:.17e}");
    text
}

/// 返回 LLVM 文本中的标量默认值。
fn default_value(ty: Scalar) -> &'static str {
    match ty {
        Scalar::Int | Scalar::Sint | Scalar::Bool => "0",
        Scalar::Float | Scalar::Sfloat => "0.0",
        Scalar::None => "",
    }
}

/// 构造带可选源码位置的不支持错误。
fn unsupported(feature: impl Into<String>, span: Option<IrSpan>) -> CodegenError {
    CodegenError::Unsupported {
        feature: feature.into(),
        span,
    }
}

/// 把 Xiao 名称转换为 LLVM 符号安全文本。
fn sanitize(name: &str) -> String {
    let mut output = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "anonymous".to_owned()
    } else {
        output
    }
}

/// 转义 LLVM 字符串字面量中的反斜杠和引号。
fn escape_llvm(text: &str) -> String {
    text.replace('\\', "\\5C").replace('"', "\\22")
}

/// 计算用于构建指纹的稳定 FNV-1a 文本。
fn stable_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
