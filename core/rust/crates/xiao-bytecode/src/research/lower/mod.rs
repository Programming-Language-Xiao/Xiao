//! `IrProgram` 到统一三地址模型的单向降低。
//!
//! 降低器只做 1:1 语义展开：不推断类型（只把 `IrExpression.ty` 映射成类别）、
//! 不重算生命周期（只消费既有结论）、不重排释放顺序（逐条重放冻结计划）。
//!
//! **块结构由本模块按语句结构自行重建**，不把 `IrBasicBlock` 当作块划分依据：
//! `IrBasicBlock.statements` 只是源码区间、不带指令，块由生命周期阶段按区间
//! 即兴产生并含合成错误块。控制流图只降级为两件事的来源——语句到所属作用域的
//! 映射，以及某个退出边是否可能发生。

/// 表达式到三地址指令的展开规则。
mod expr;
/// 退出点上的释放计划接线。
mod plan;
/// 语句降低与块结构重建。
mod stmt;

use std::collections::{BTreeMap, HashMap};

use xiao_ir::{
    IrExpression, IrExpressionKind, IrProgram, IrReleasePlan, IrSpan, IrStatement, IrStatementKind,
    IrType,
};
use xiao_lifetime::ReleaseActionKind;
use xiao_syntax::ScalarType;

use crate::research::sig::{CallSig, CallSigTable};
use crate::research::tac::{
    BlockId, CategoryMap, ConstPool, FuncId, RegisterClass, TAC_VERSION, TacAbi, TacBlock,
    TacConstant, TacFunction, TacInstr, TacOp, TacProgram, VReg,
};

/// 当前三地址格式使用的 ABI 版本。
pub const TAC_BYTECODE_ABI_VERSION: u32 = 1;
/// 当前消费的 Runtime ABI 版本。
pub const TAC_RUNTIME_ABI_VERSION: u32 = 1;

/// 一条释放计划的携带形式。
///
/// 三地址程序保留冻结计划的原始动作序列，由解释器在退出点上执行；这样三种
/// 机型共用同一份释放语义，也不会各自复制一份释放顺序。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TacReleasePlan {
    /// 所属作用域。
    pub scope: u32,
    /// 退出边稳定名称。
    pub exit: String,
    /// 按 `order` 排列的释放动作。
    pub actions: Vec<TacReleaseAction>,
    /// 转移出去、不在此处释放的值。
    pub transferred: Vec<u32>,
}

/// 一条释放动作。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TacReleaseAction {
    /// `IrValue.id`。
    pub value: u32,
    /// 计划内的从零开始顺序。
    pub order: usize,
    /// 强释放或弱释放。
    pub kind: ReleaseActionKind,
}

/// 语句到所属静态作用域的索引。
///
/// 来源是控制流基本块的 `statements`（语句源码区间）与 `scope` 的对应关系，
/// 这是数据而不是推断；比按源码区间包含关系反推可靠。
#[derive(Debug, Default)]
struct ScopeIndex {
    by_span: HashMap<(usize, usize), u32>,
}

impl ScopeIndex {
    /// 从控制流图建立语句到作用域的映射；同一区间重复出现时保留首个。
    fn build(program: &IrProgram) -> Self {
        let mut by_span = HashMap::new();
        for block in &program.control_flow.blocks {
            for span in &block.statements {
                by_span.entry((span.start, span.end)).or_insert(block.scope);
            }
        }
        Self { by_span }
    }

    /// 返回语句所属作用域。
    fn of(&self, span: IrSpan) -> Option<u32> {
        self.by_span.get(&(span.start, span.end)).copied()
    }
}

/// 降低一个已验证的 `IrProgram`。
#[must_use]
pub fn lower_program(program: &IrProgram) -> TacProgram {
    let mut lowerer = Lowerer::new(program);
    lowerer.run();
    lowerer.finish()
}

/// 降低过程中的全部可变状态。
struct Lowerer<'ir> {
    program: &'ir IrProgram,
    scopes: ScopeIndex,
    constants: ConstPool,
    signatures: CallSigTable,
    functions: Vec<TacFunction>,
    categories: CategoryMap,
    plans: Vec<TacReleasePlan>,
    /// `IrValue.id` 到源码区间的映射，用于把释放动作还原成寄存器。
    value_spans: HashMap<u32, IrSpan>,
    /// `IrValue.id` 到存储类别的映射。
    value_storage: HashMap<u32, String>,
    /// `IrValue.id` 到所属作用域的映射，用于按作用域消歧同名绑定。
    value_scopes: HashMap<u32, u32>,
    /// 顶层函数名到函数索引的映射。
    ///
    /// 脚本入口固定占用索引 0，命名函数从 1 开始，因此这里存的是「入口之后的
    /// 序号」；调用点必须拿到与 `TacProgram.functions` 对齐的索引。
    named_functions: BTreeMap<String, FuncId>,
    /// 当前的函数构建状态。
    frame: Frame,
    /// 顶层函数名到调用签名的映射。
    function_signatures: BTreeMap<String, crate::research::tac::SigId>,
    /// 本批次尚未降低的构造；由验证器转成诊断，不静默跳过。
    unsupported: Vec<String>,
}

/// 单个函数的构建状态。
#[derive(Debug, Default)]
struct Frame {
    blocks: Vec<TacBlock>,
    current: Option<BlockId>,
    next_vreg: u32,
    value_regs: BTreeMap<u32, VReg>,
    locals: Vec<VReg>,
    parameters: Vec<VReg>,
    used_scopes: Vec<u32>,
    scope_stack: Vec<u32>,
    /// 活动循环的 (循环体入口, 循环出口) 栈。
    loops: Vec<(BlockId, BlockId)>,
    /// 当前语句产生的、需要在消费后释放的临时堆值寄存器。
    pending_temporaries: Vec<VReg>,
}

impl<'ir> Lowerer<'ir> {
    /// 建立降低器并预处理所有权与签名信息。
    fn new(program: &'ir IrProgram) -> Self {
        let value_spans = program
            .ownership
            .values
            .iter()
            .map(|value| (value.id, value.span))
            .collect();
        let value_storage = program
            .ownership
            .values
            .iter()
            .map(|value| (value.id, value.storage.clone()))
            .collect();
        let plans = program
            .ownership
            .release_plans
            .iter()
            .map(convert_plan)
            .collect();
        Self {
            program,
            scopes: ScopeIndex::build(program),
            constants: ConstPool::new(),
            signatures: CallSigTable::new(),
            functions: Vec::new(),
            categories: CategoryMap::new(),
            plans,
            value_spans,
            value_storage,
            value_scopes: program
                .ownership
                .values
                .iter()
                .map(|value| (value.id, value.scope))
                .collect(),
            named_functions: BTreeMap::new(),
            frame: Frame::default(),
            function_signatures: BTreeMap::new(),
            unsupported: Vec::new(),
        }
    }

    /// 依次降低入口脚本与全部顶层函数。
    fn run(&mut self) {
        let functions = self
            .program
            .body
            .iter()
            .filter_map(|statement| match &statement.kind {
                IrStatementKind::Function { name, .. } => Some((name.text.clone(), statement)),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (index, (name, _)) in functions.iter().enumerate() {
            self.named_functions
                .insert(name.clone(), FuncId::new(index as u32 + 1));
            if let IrStatementKind::Function {
                parameters,
                return_type,
                ..
            } = &functions[index].1.kind
            {
                let parameters = parameters.clone();
                let return_type = return_type.clone();
                let signature = self.signature_for(&parameters, &return_type);
                self.function_signatures.insert(name.clone(), signature);
            }
        }
        let script = self
            .program
            .body
            .iter()
            .filter(|statement| !matches!(statement.kind, IrStatementKind::Function { .. }))
            .cloned()
            .collect::<Vec<_>>();
        self.lower_function("", &script, &[], self.program.span, None);
        for (name, statement) in functions {
            let IrStatementKind::Function {
                parameters,
                return_type,
                body,
                ..
            } = &statement.kind
            else {
                continue;
            };
            let signature = self
                .function_signatures
                .get(&name)
                .copied()
                .unwrap_or_else(|| self.signature_for(parameters, return_type));
            let parameters = parameters.clone();
            self.lower_function(&name, body, &parameters, statement.span, Some(signature));
        }
    }

    /// 汇总降低结果。
    fn finish(self) -> TacProgram {
        TacProgram {
            version: TAC_VERSION,
            abi: TacAbi {
                bytecode_abi_version: TAC_BYTECODE_ABI_VERSION,
                runtime_abi_version: TAC_RUNTIME_ABI_VERSION,
                ir_version: self.program.version,
                language_version: self.program.language_version.clone(),
                target: self.program.target.clone(),
            },
            constants: self.constants,
            signatures: self.signatures,
            functions: self.functions,
            categories: self.categories,
            plans: self.plans,
            unsupported: self.unsupported,
        }
    }

    /// 合成一条调用签名。
    fn signature_for(
        &mut self,
        parameters: &[xiao_ir::IrParameter],
        return_type: &IrType,
    ) -> crate::research::tac::SigId {
        use crate::research::sig::ParamKind;

        let signature = if parameters.is_empty() && return_type == &IrType::Dynamic {
            CallSig::dynamic()
        } else {
            let mut signature = CallSig::plain(
                parameters.iter().map(|item| item.ty.clone()).collect(),
                return_type.clone(),
            );
            signature.parameter_names = parameters
                .iter()
                .map(|item| item.name.text.clone())
                .collect();
            signature.parameter_kinds = parameters
                .iter()
                .map(|item| {
                    ParamKind::from_name(&item.kind).unwrap_or(ParamKind::PositionalOrKeyword)
                })
                .collect();
            signature.has_defaults = parameters
                .iter()
                .map(|item| item.default.is_some())
                .collect();
            for (index, parameter) in parameters.iter().enumerate() {
                match ParamKind::from_name(&parameter.kind) {
                    Some(ParamKind::VarArgs) => signature.var_args_slot = Some(index),
                    Some(ParamKind::VarKeywords) => signature.kw_args_slot = Some(index),
                    _ => {}
                }
            }
            signature
        };
        self.signatures.intern(signature)
    }

    /// 降低一个函数体。
    fn lower_function(
        &mut self,
        name: &str,
        body: &[IrStatement],
        parameters: &[xiao_ir::IrParameter],
        span: IrSpan,
        signature: Option<crate::research::tac::SigId>,
    ) {
        self.frame = Frame::default();
        let entry = self.new_block(span);
        self.switch_to(entry);
        let program_scope = self
            .program
            .ownership
            .scopes
            .iter()
            .find(|scope| scope.kind == "program")
            .map_or(0, |scope| scope.id);
        self.enter_scope(program_scope, span);
        let parameter_registers = self.declare_parameters(parameters);
        let function_scope = self
            .program
            .ownership
            .scopes
            .iter()
            .find(|scope| scope.kind == "function" && contains(scope.span, span))
            .map(|scope| scope.id);
        if let Some(scope) = function_scope {
            self.enter_scope(scope, span);
        }
        self.lower_statements(body, "normal");
        if let Some(scope) = function_scope {
            self.exit_scope(scope, "normal");
        }
        self.exit_scope(program_scope, "normal");
        self.emit(TacInstr::new(TacOp::Return { value: None }, span));
        let blocks = std::mem::take(&mut self.frame.blocks);
        let locals = std::mem::take(&mut self.frame.locals);
        let value_registers = std::mem::take(&mut self.frame.value_regs);
        let used_scopes = std::mem::take(&mut self.frame.used_scopes);
        self.functions.push(TacFunction {
            name: name.to_owned(),
            signature,
            entry,
            blocks,
            parameters: parameter_registers,
            locals,
            scopes: used_scopes,
            handlers: Vec::new(),
            value_registers,
            span,
        });
    }

    /// 为形参分配寄存器，并把它们与对应的 `IrValue` 绑定。
    ///
    /// 绑定是必需的：函数体里按名引用形参会走 `register_of`，如果不先登记，
    /// 它会为同一个值再分配一个新寄存器，读到的就是从未写入的槽位。
    fn declare_parameters(&mut self, parameters: &[xiao_ir::IrParameter]) -> Vec<VReg> {
        let mut registers = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let class = Lowerer::class_of_type(&parameter.ty);
            let register = self.new_binding_register(class, parameter.span);
            self.frame.parameters.push(register);
            self.frame.locals.push(register);
            if let Some(value) = self.value_of_name_at(&parameter.name.text, parameter.span) {
                self.frame.value_regs.insert(value, register);
            }
            registers.push(register);
        }
        registers
    }

    /// 分配一个新的虚拟寄存器并登记类别。
    fn new_register(&mut self, class: RegisterClass, _span: IrSpan) -> VReg {
        let register = VReg::new(self.frame.next_vreg);
        self.frame.next_vreg = self.frame.next_vreg.saturating_add(1);
        self.categories.insert(register, class);
        // 新建的值寄存器承载一条指令刚产生的值，语句结束时要释放。
        // 具名绑定的寄存器走 `new_binding_register`，它们由释放计划负责。
        if matches!(class, RegisterClass::ObjHandle) {
            self.note_temporary(register);
        }
        register
    }

    /// 新建一个承载具名绑定的寄存器。
    ///
    /// 绑定寄存器**不登记为临时值**：它的生命周期由冻结释放计划决定，
    /// 再发一次 `Release` 会重复释放。
    fn new_binding_register(&mut self, class: RegisterClass, span: IrSpan) -> VReg {
        let register = VReg::new(self.frame.next_vreg);
        self.frame.next_vreg = self.frame.next_vreg.saturating_add(1);
        self.categories.insert(register, class);
        let _ = span;
        register
    }

    /// 判断一个寄存器当前是否承载待释放的临时值。
    fn is_pending_temporary(&self, register: VReg) -> bool {
        self.frame.pending_temporaries.contains(&register)
    }

    /// 新建一个基本块，**不切换当前块**。
    ///
    /// 分配与切换刻意分开：调用方通常要先把终止跳转发进前驱块，再切到新块，
    /// 否则跳转会落进自己块里形成自环。
    fn new_block(&mut self, span: IrSpan) -> BlockId {
        let scope = self.innermost_scope().unwrap_or(0);
        let id = BlockId::new(self.frame.blocks.len() as u32);
        self.frame.blocks.push(TacBlock {
            id,
            scope,
            instructions: Vec::new(),
        });
        if !self.frame.used_scopes.contains(&scope) {
            self.frame.used_scopes.push(scope);
        }
        let _ = span;
        id
    }

    /// 返回当前最内层作用域。
    fn innermost_scope(&self) -> Option<u32> {
        self.frame.scope_stack.last().copied()
    }

    /// 返回语句所属的静态作用域。
    fn scope_of_statement(&self, span: IrSpan) -> Option<u32> {
        self.scopes.of(span)
    }

    /// 按名称与源码区间查找绑定的值编号。
    ///
    /// 名称优先；同名遮蔽时用源码区间消歧，因为绑定值的区间就是其名称区间。
    fn value_of_name_at(&self, name: &str, span: IrSpan) -> Option<u32> {
        let prefixed = [format!("ascii:{name}"), format!("backtick:{name}")];
        self.program
            .ownership
            .values
            .iter()
            .find(|value| {
                value
                    .name
                    .as_deref()
                    .is_some_and(|item| prefixed.iter().any(|p| p == item))
                    && value.span == span
            })
            .or_else(|| {
                self.program.ownership.values.iter().find(|value| {
                    value
                        .name
                        .as_deref()
                        .is_some_and(|item| prefixed.iter().any(|p| p == item))
                })
            })
            .map(|value| value.id)
    }

    /// 压入一个活动循环。
    fn push_loop(&mut self, body: BlockId, exit: BlockId) {
        self.frame.loops.push((body, exit));
    }

    /// 弹出一个活动循环。
    fn pop_loop(&mut self) {
        self.frame.loops.pop();
    }

    /// 返回当前循环的跳转目标：`break` 去出口，`continue` 回循环体。
    fn loop_target(&self, exit: &str) -> Option<&BlockId> {
        let (body, leave) = self.frame.loops.last()?;
        Some(if exit == "break" { leave } else { body })
    }

    /// 切换到指定块。
    fn switch_to(&mut self, block: BlockId) {
        self.frame.current = Some(block);
    }

    /// 向当前块追加一条指令；没有当前块时静默丢弃。
    fn emit(&mut self, instruction: TacInstr) {
        let Some(current) = self.frame.current else {
            return;
        };
        if let Some(block) = self.frame.blocks.get_mut(current.get() as usize) {
            block.instructions.push(instruction);
        }
    }

    /// 进入一个静态作用域。
    fn enter_scope(&mut self, scope: u32, span: IrSpan) {
        if self.frame.scope_stack.last() == Some(&scope) {
            return;
        }
        if self.frame.scope_stack.contains(&scope) {
            return;
        }
        self.frame.scope_stack.push(scope);
        if !self.frame.used_scopes.contains(&scope) {
            self.frame.used_scopes.push(scope);
        }
        self.emit(TacInstr::new(TacOp::EnterScope(scope), span));
    }

    /// 取得或创建某个 `IrValue` 对应的寄存器。
    fn register_of(&mut self, value: u32) -> VReg {
        if let Some(register) = self.frame.value_regs.get(&value) {
            return *register;
        }
        let span = self
            .value_spans
            .get(&value)
            .copied()
            .unwrap_or_else(|| IrSpan::new(0, 0));
        let class = self.class_for_value(value);
        let register = self.new_binding_register(class, span);
        self.frame.value_regs.insert(value, register);
        self.frame.locals.push(register);
        register
    }

    /// 返回某个 `IrValue` 的寄存器类别。
    fn class_for_value(&self, value: u32) -> RegisterClass {
        match self.value_storage.get(&value).map(String::as_str) {
            Some("stack") => RegisterClass::Int,
            Some("heap_weak" | "heap_strong") => RegisterClass::ObjHandle,
            _ => RegisterClass::Poly,
        }
    }

    /// 按标量类型返回寄存器类别。
    fn class_of_type(ty: &IrType) -> RegisterClass {
        match ty {
            IrType::Scalar { name } => match ScalarType::from_name(name) {
                Some(ScalarType::Int | ScalarType::Sint) => RegisterClass::Int,
                Some(ScalarType::Float | ScalarType::Sfloat) => RegisterClass::Float,
                Some(ScalarType::Bool) => RegisterClass::Bool,
                Some(ScalarType::Str | ScalarType::Lint | ScalarType::Lfloat) => {
                    RegisterClass::ObjHandle
                }
                None => RegisterClass::Poly,
            },
            IrType::None => RegisterClass::None,
            IrType::Dynamic | IrType::Variable { .. } => RegisterClass::Dynamic,
            _ => RegisterClass::ObjHandle,
        }
    }

    /// 把常量登记进常量池并发出加载指令。
    fn emit_constant(&mut self, constant: TacConstant, span: IrSpan) -> VReg {
        let class = constant.register_class();
        let id = self.constants.intern(constant);
        let register = self.new_register(class, span);
        self.emit(TacInstr::with_dst(TacOp::LoadConst(id), register, span));
        register
    }

    /// 降低一个表达式并返回其寄存器。
    fn lower_expression(&mut self, expression: &IrExpression) -> VReg {
        expr::lower(self, expression)
    }

    /// 按语句结构重建块并降低语句列表；结束时按 `exit` 离开当前作用域。
    fn lower_statements(&mut self, statements: &[IrStatement], exit: &str) {
        stmt::lower_statements(self, statements, exit);
    }

    /// 按源码名称查找绑定的值编号；名称前缀由生命周期阶段添加，这里剥掉。
    ///
    /// 必须按作用域消歧：不同函数可以各有一个同名形参，全局取首个会串到别的
    /// 函数的绑定上，读到本帧从未写入的寄存器。优先取当前作用域栈里最内层的
    /// 那个候选，找不到活动候选时才退回首个。
    fn value_of_name(&self, name: &str) -> Option<u32> {
        let candidates = self.candidates_of_name(name);
        let active = candidates
            .iter()
            .filter(|(_, scope)| self.frame.scope_stack.contains(scope))
            .max_by_key(|(_, scope)| {
                self.frame
                    .scope_stack
                    .iter()
                    .position(|item| item == scope)
                    .unwrap_or(0)
            })
            .map(|(value, _)| *value);
        active.or_else(|| candidates.first().map(|(value, _)| *value))
    }

    /// 列出同名绑定的 `(值编号, 所属作用域)` 候选。
    fn candidates_of_name(&self, name: &str) -> Vec<(u32, u32)> {
        let ascii = format!("ascii:{name}");
        let backtick = format!("backtick:{name}");
        self.program
            .ownership
            .values
            .iter()
            .filter(|value| {
                value
                    .name
                    .as_deref()
                    .is_some_and(|item| item == ascii || item == backtick)
            })
            .map(|value| {
                (
                    value.id,
                    self.value_scopes.get(&value.id).copied().unwrap_or(0),
                )
            })
            .collect()
    }

    /// 按名称查找顶层函数索引。
    fn function_index(&self, name: &str) -> Option<FuncId> {
        self.named_functions.get(name).copied()
    }

    /// 按名称解析调用目标；不是已知顶层函数时返回 `None`。
    fn function_index_of_expression(&self, callee: &IrExpression) -> Option<FuncId> {
        let IrExpressionKind::Name { name } = &callee.kind else {
            return None;
        };
        self.named_functions.get(&name.text).copied()
    }

    /// 返回某个函数的调用签名。
    fn signature_of_function(&self, target: FuncId) -> Option<crate::research::tac::SigId> {
        let name = self
            .program
            .body
            .iter()
            .filter_map(|statement| match &statement.kind {
                IrStatementKind::Function { name, .. } => Some(name.text.clone()),
                _ => None,
            })
            .nth(target.get().saturating_sub(1) as usize)?;
        self.function_signatures.get(&name).copied()
    }

    /// 登记一个新建的临时堆值寄存器，供语句结束时释放。
    ///
    /// 临时值不进释放计划（生命周期阶段按 `temporary` 过滤），所以必须由降低器
    /// 在消费点之后显式释放，否则字面量产生的堆值会一直漏。
    fn note_temporary(&mut self, register: VReg) {
        if !self.frame.pending_temporaries.contains(&register) {
            self.frame.pending_temporaries.push(register);
        }
    }

    /// 取出并清空当前待释放的临时寄存器。
    fn take_pending_temporaries(&mut self) -> Vec<VReg> {
        std::mem::take(&mut self.frame.pending_temporaries)
    }

    /// 在当前位置发出释放临时值的指令。
    ///
    /// 被 `Move` 搬进绑定的临时值此时寄存器已空，释放是空操作，不会重复释放。
    fn flush_temporaries(&mut self, span: IrSpan) {
        for register in self.take_pending_temporaries() {
            self.emit(TacInstr::new(
                TacOp::Release {
                    value: register,
                    kind: ReleaseActionKind::Strong,
                },
                span,
            ));
        }
    }

    /// 记录一个本批次尚未降低的构造。
    fn record_unsupported(&mut self, note: String) {
        if !self.unsupported.contains(&note) {
            self.unsupported.push(note);
        }
    }

    /// 列出从最内层到 `stop_kind` 的作用域链。
    fn scope_chain_to(&self, stop_kind: &str) -> Vec<u32> {
        let mut chain = Vec::new();
        for scope in self.frame.scope_stack.iter().rev() {
            chain.push(*scope);
            let kind = self
                .program
                .ownership
                .scopes
                .iter()
                .find(|item| item.id == *scope)
                .map_or("", |item| item.kind.as_str());
            if kind == stop_kind {
                break;
            }
        }
        chain
    }

    /// 为一个作用域发出退出计划并离开它。
    fn exit_scope(&mut self, scope: u32, exit: &str) {
        let span = self
            .program
            .ownership
            .scopes
            .iter()
            .find(|item| item.id == scope)
            .map_or_else(|| IrSpan::new(0, 0), |item| item.span);
        self.run_plan(scope, exit, span);
        self.emit(TacInstr::new(
            TacOp::ExitScope {
                scope,
                exit: exit.to_owned(),
            },
            span,
        ));
        if let Some(index) = self
            .frame
            .scope_stack
            .iter()
            .rposition(|item| *item == scope)
        {
            self.frame.scope_stack.truncate(index);
        }
    }
}

/// 把 IR 释放计划转换为三地址携带形式。
fn convert_plan(plan: &IrReleasePlan) -> TacReleasePlan {
    TacReleasePlan {
        scope: plan.scope,
        exit: plan.exit.clone(),
        actions: plan
            .actions
            .iter()
            .map(|action| TacReleaseAction {
                value: action.value,
                order: action.order,
                // 反向解析必须走 `from_name`：自己写 match 会让未知拼写静默
                // 退化成强释放，而拼写表本来就有单一来源。
                kind: ReleaseActionKind::from_name(&action.kind)
                    .unwrap_or(ReleaseActionKind::Strong),
            })
            .collect(),
        transferred: plan.transferred.clone(),
    }
}

/// 判断外层区间是否包含内层区间。
fn contains(outer: IrSpan, inner: IrSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}
