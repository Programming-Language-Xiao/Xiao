//! 18B 归档入口的函数调用闭包。只遍历可达函数，不按整个模块放大能力。

use std::collections::{BTreeMap, BTreeSet};

use xiao_bytecode::{FuncId, TacArgument, TacOp, TacProgram, VReg, lower_program};
use xiao_intrinsics::IntrinsicId;

use crate::frontend::{FrontendArtifact, FrontendCompiler, FrontendContext, FrontendRequest};
use crate::module_loader::DriverModuleLoader;

/// 一个已编译的模块，保留编译请求用于缓存身份及显式调试对象。
pub(crate) struct ClosureModule {
    pub(crate) request: FrontendRequest,
    pub(crate) artifact: FrontendArtifact,
    pub(crate) program: TacProgram,
}

/// 本地对象与官方要求的精确函数闭包。
pub(crate) struct ArchiveClosure {
    pub(crate) modules: BTreeMap<String, ClosureModule>,
    pub(crate) stdlib_modules: BTreeSet<String>,
    pub(crate) intrinsics: BTreeSet<IntrinsicId>,
}

/// 寄存器中与调用图有关的值；标量不需要加入此集合。
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Target {
    Function(String, FuncId),
    Module(String),
    Table(String, u32),
    /// 尚未实现目标传播的值，合流时不得被已知函数吞掉。
    Unknown,
    /// 已知标量或 none，不会引入额外调用。
    NonCallable,
}

/// 一个抽象值可能指向的调用目标。
type Values = BTreeSet<Target>;
/// 模块内唯一的函数身份。
type Function = (String, FuncId);
/// 函数局部寄存器到抽象目标的映射。
type Registers = BTreeMap<VReg, Values>;

/// 跨模块、函数参数和返回值共同收敛的只读调用图分析状态。
struct Analyzer {
    loader: DriverModuleLoader<'static>,
    context: FrontendContext,
    result: ArchiveClosure,
    functions: BTreeSet<Function>,
    parameters: BTreeMap<Function, Vec<Values>>,
    returns: BTreeMap<Function, Values>,
    exports: BTreeMap<(String, String), Values>,
    unresolved: BTreeSet<(String, FuncId, usize, usize)>,
    changed: bool,
}

/// 从入口 0 沿模块初始化、静态/动态调用及函数值传播计算最小不动点。
pub(crate) fn collect_archive_closure(request: FrontendRequest) -> Result<ArchiveClosure, String> {
    let artifact = FrontendCompiler::new()
        .compile(&request)
        .map_err(|error| error.to_string())?;
    let program = lower_program(&artifact.ir);
    xiao_bytecode::verify_for_execution(&artifact.ir, &program)
        .map_err(|error| error.to_string())?;
    let mut analyzer = Analyzer {
        loader: DriverModuleLoader::new(request.context.project_root.as_deref(), None),
        context: request.context.clone(),
        result: ArchiveClosure {
            modules: BTreeMap::from([(
                "entry".to_owned(),
                ClosureModule {
                    request,
                    artifact,
                    program,
                },
            )]),
            stdlib_modules: BTreeSet::new(),
            intrinsics: BTreeSet::new(),
        },
        functions: BTreeSet::from([("entry".to_owned(), FuncId::new(0))]),
        parameters: BTreeMap::new(),
        returns: BTreeMap::new(),
        exports: BTreeMap::new(),
        unresolved: BTreeSet::new(),
        changed: true,
    };
    while analyzer.changed {
        analyzer.changed = false;
        analyzer.unresolved.clear();
        for function in analyzer.functions.clone() {
            analyzer.scan(function)?;
        }
    }
    if !analyzer.unresolved.is_empty() {
        return Err("归档调用图包含无法静态确定的动态调用，不能安全推导能力要求".to_owned());
    }
    Ok(analyzer.result)
}

impl Analyzer {
    /// 与 VM 的模块身份规则一致，官方库内的相对模块仍属于官方命名空间。
    fn identity(current: &str, module: &str) -> String {
        if module == "stdlib" || module.starts_with("stdlib.") || module.starts_with("project:") {
            module.to_owned()
        } else if current.starts_with("stdlib.") {
            format!("stdlib.{module}")
        } else {
            format!("project:{module}")
        }
    }

    /// 编译首次可达的文件模块；目录命名空间不生成对象。
    fn load(&mut self, identity: &str) -> Result<bool, String> {
        if self.result.modules.contains_key(identity) {
            return Ok(true);
        }
        let Some((path, root, exports)) = self.loader.descriptor(identity) else {
            if xiao_vm::ModuleLoader::contains(&self.loader, identity) {
                return Ok(false);
            }
            return Err(format!("归档不支持第三方依赖或缺少本地模块：{identity}"));
        };
        let mut context = self.context.clone();
        context.project_root = Some(root);
        context.module_exports = exports;
        let request = FrontendRequest::from_file(path)
            .map_err(|error| error.to_string())?
            .with_context(context);
        let artifact = FrontendCompiler::new()
            .compile(&request)
            .map_err(|error| error.to_string())?;
        let program = lower_program(&artifact.ir);
        xiao_bytecode::verify_for_execution(&artifact.ir, &program)
            .map_err(|error| error.to_string())?;
        if identity.starts_with("stdlib.") {
            self.result.stdlib_modules.insert(identity.to_owned());
        }
        self.result.modules.insert(
            identity.to_owned(),
            ClosureModule {
                request,
                artifact,
                program,
            },
        );
        self.changed = true;
        Ok(true)
    }

    /// 将模块顶层入口纳入闭包，不执行模块初始化。
    fn initialize(&mut self, identity: &str) -> Result<(), String> {
        if self.load(identity)? {
            self.changed |= self.functions.insert((identity.to_owned(), FuncId::new(0)));
        }
        Ok(())
    }

    /// 按模块导出及表方法解析成员，保留函数值的来源身份。
    fn member(&mut self, targets: Values, member: &str) -> Result<Values, String> {
        let mut result = Values::new();
        for target in targets {
            match target {
                Target::Module(identity) => {
                    let child = format!(
                        "{identity}.{}",
                        member.strip_prefix("ascii:").unwrap_or(member)
                    );
                    if member.starts_with("ascii:")
                        && xiao_vm::ModuleLoader::contains(&self.loader, &child)
                    {
                        self.initialize(&child)?;
                        result.insert(Target::Module(child));
                    } else {
                        self.initialize(&identity)?;
                        result.extend(
                            self.exports
                                .get(&(identity, member.to_owned()))
                                .cloned()
                                .unwrap_or_default(),
                        );
                    }
                }
                Target::Table(identity, table) => {
                    if let Some(callee) = self.result.modules[&identity].program.table_definitions
                        [table as usize]
                        .methods
                        .get(member)
                    {
                        result.insert(Target::Function(identity, *callee));
                    } else {
                        result.insert(Target::Unknown);
                    }
                }
                Target::Unknown => {
                    result.insert(Target::Unknown);
                }
                Target::Function(..) | Target::NonCallable => {
                    result.insert(Target::NonCallable);
                }
            }
        }
        Ok(result)
    }

    /// 传播调用参数和已知返回值，关键字参数按被调方签名绑定。
    fn call(&mut self, targets: Values, arguments: &[(Option<String>, Values)]) -> Values {
        let mut result = Values::new();
        for target in targets {
            let Target::Function(module, callee) = target else {
                continue;
            };
            let program = &self.result.modules[&module].program;
            let declaration = &program.functions[callee.get() as usize];
            let signature = declaration
                .signature
                .and_then(|id| program.signatures.get(id));
            let mut bound = vec![Values::new(); declaration.parameters.len()];
            let mut positional = 0;
            for (name, values) in arguments {
                let slot = if let Some(name) = name {
                    signature.and_then(|signature| {
                        signature
                            .parameter_names
                            .iter()
                            .position(|parameter| parameter == name)
                    })
                } else {
                    let slot = positional;
                    positional += 1;
                    Some(slot)
                };
                if let Some(slot) = slot.and_then(|slot| bound.get_mut(slot)) {
                    slot.extend(values.clone());
                }
            }
            let function = (module, callee);
            self.changed |= self.functions.insert(function.clone());
            let slots = self.parameters.entry(function.clone()).or_default();
            slots.resize_with(slots.len().max(bound.len()), Values::new);
            for (slot, values) in slots.iter_mut().zip(&bound) {
                let count = slot.len();
                slot.extend(values.clone());
                self.changed |= slot.len() != count;
            }
            result.extend(self.returns.get(&function).cloned().unwrap_or_default());
        }
        result
    }

    /// 遍历一个可达函数的控制流及异常边，不遍历未调用函数。
    fn scan(&mut self, key: Function) -> Result<(), String> {
        let (identity, function_id) = &key;
        let program = self.result.modules[identity].program.clone();
        let function = &program.functions[function_id.get() as usize];
        let mut initial = Registers::new();
        for (register, values) in function
            .parameters
            .iter()
            .zip(self.parameters.get(&key).into_iter().flatten())
        {
            initial.insert(*register, values.clone());
        }
        let mut states = BTreeMap::from([(function.entry, initial)]);
        let mut pending = BTreeSet::from([function.entry]);
        while let Some(block_id) = pending.pop_first() {
            let mut registers = states[&block_id].clone();
            let block = function.block(block_id).ok_or("调用图基本块不存在")?;
            let mut successors = BTreeSet::new();
            for instruction in &block.instructions {
                let values = |register: &VReg| registers.get(register).cloned().unwrap_or_default();
                let args = |arguments: &[TacArgument]| {
                    arguments
                        .iter()
                        .map(|arg| (arg.name.clone(), values(&arg.value)))
                        .collect::<Vec<_>>()
                };
                let result = match &instruction.op {
                    TacOp::LoadConst(_) | TacOp::LoadNone => Values::from([Target::NonCallable]),
                    TacOp::LoadFunc(id) => Values::from([Target::Function(identity.clone(), *id)]),
                    TacOp::Copy(value)
                    | TacOp::Move(value)
                    | TacOp::Box(value)
                    | TacOp::Unbox(value) => values(value),
                    TacOp::ImportModule {
                        module,
                        binding_module,
                    } => {
                        let module = Self::identity(identity, module);
                        self.initialize(&module)?;
                        Values::from([Target::Module(Self::identity(identity, binding_module))])
                    }
                    TacOp::PackageRoot(_) => return Err("归档首版不支持第三方包依赖".to_owned()),
                    TacOp::MemberGet { object, member } => self.member(values(object), member)?,
                    TacOp::ExportValue { name, value } => {
                        let export = self
                            .exports
                            .entry((identity.clone(), name.clone()))
                            .or_default();
                        let count = export.len();
                        export.extend(values(value));
                        self.changed |= export.len() != count;
                        Values::new()
                    }
                    TacOp::Call {
                        callee,
                        signature,
                        arguments,
                    } => {
                        if let Some(id) = program
                            .signatures
                            .get(*signature)
                            .and_then(|sig| sig.intrinsic_id())
                        {
                            self.result
                                .intrinsics
                                .insert(IntrinsicId::new(id).ok_or("无效 intrinsic ID")?);
                            Values::from([Target::Unknown])
                        } else {
                            self.call(
                                Values::from([Target::Function(identity.clone(), *callee)]),
                                &args(arguments),
                            )
                        }
                    }
                    TacOp::CallDynamic { callee, arguments } => {
                        let targets = values(callee);
                        if targets.is_empty() || targets.contains(&Target::Unknown) {
                            self.unresolved.insert((
                                identity.clone(),
                                *function_id,
                                instruction.span.start,
                                instruction.span.end,
                            ));
                        }
                        self.call(targets, &args(arguments))
                    }
                    TacOp::LoadTable {
                        table,
                        construct,
                        arguments,
                    } => {
                        let definition = &program.table_definitions[*table as usize];
                        if *construct {
                            let mut calls = Values::from([Target::Function(
                                identity.clone(),
                                definition.fields,
                            )]);
                            for name in ["ascii:init", "ascii:drop"] {
                                if let Some(id) = definition.methods.get(name) {
                                    calls.insert(Target::Function(identity.clone(), *id));
                                }
                            }
                            self.call(calls, &args(arguments));
                        }
                        Values::from([Target::Table(identity.clone(), *table)])
                    }
                    TacOp::Return { value } => {
                        let returned = self.returns.entry(key.clone()).or_default();
                        let count = returned.len();
                        returned.extend(
                            value
                                .as_ref()
                                .map(values)
                                .unwrap_or_else(|| Values::from([Target::NonCallable])),
                        );
                        self.changed |= returned.len() != count;
                        Values::new()
                    }
                    TacOp::Jump(target) | TacOp::CallSub { sub: target } => {
                        successors.insert(*target);
                        Values::new()
                    }
                    TacOp::BranchIf {
                        if_true, if_false, ..
                    } => {
                        successors.extend([*if_true, *if_false]);
                        Values::new()
                    }
                    TacOp::Check { on_failure, .. } => {
                        successors.insert(*on_failure);
                        Values::new()
                    }
                    _ if instruction.dst.is_some() => Values::from([Target::Unknown]),
                    _ => Values::new(),
                };
                if let Some(dst) = instruction.dst {
                    registers.insert(dst, result);
                }
            }
            for handler in &function.handlers {
                if block_id >= handler.protected.0 && block_id < handler.protected.1 {
                    successors.insert(handler.handler);
                }
            }
            for successor in successors {
                let fresh = !states.contains_key(&successor);
                let state = states.entry(successor).or_default();
                let mut changed = fresh;
                for (register, values) in &registers {
                    let slot = state.entry(*register).or_default();
                    let count = slot.len();
                    slot.extend(values.clone());
                    changed |= slot.len() != count;
                }
                if changed {
                    pending.insert(successor);
                }
            }
        }
        Ok(())
    }
}
