//! 模块身份、初始化与调用上下文切换；不与执行单元共享单线程模块状态。

use super::*;

impl<C: Carrier, S: VmEventSink> Vm<'_, C, S> {
    /// 将模块载入失败转换为带来源上下文的可恢复 VM 错误。
    pub(super) fn module_fault(&self, identity: &str, cause: impl std::fmt::Display) -> Fault {
        let mut package_context = None;
        let label = if let Some(name) = identity.strip_prefix("package:") {
            let root = name.split('.').next().unwrap_or(name);
            let environment = self
                .module_loader
                .and_then(|loader| loader.environment(identity))
                .unwrap_or("未知环境");
            package_context = Some((root.to_owned(), environment.to_owned()));
            format!("包 {root}（来源环境 {environment}）模块 {name}")
        } else {
            if identity == crate::STANDARD_LIBRARY_NAMESPACE
                || identity.starts_with(crate::STANDARD_LIBRARY_PREFIX)
            {
                format!(
                    "官方标准库模块 {}",
                    identity
                        .strip_prefix(crate::STANDARD_LIBRARY_PREFIX)
                        .unwrap_or(identity)
                )
            } else {
                format!(
                    "项目模块 {}",
                    identity.strip_prefix("project:").unwrap_or(identity)
                )
            }
        };
        let mut error = XiaoError::invalid_value(format!("{label} 加载失败: {cause}"));
        if let Some((root, environment)) = package_context {
            error = error
                .with_param("package", DiagnosticParam::Text(root))
                .with_param("environment", DiagnosticParam::Text(environment));
        }
        Fault::Error(error)
    }

    /// 在模块自身的程序、源码和 PC 映射中执行闭包并恢复外层上下文。
    pub(super) fn with_module_program<T>(
        &mut self,
        identity: &str,
        source_name: &str,
        program: Rc<TacProgram>,
        execute: impl FnOnce(&mut Self) -> Result<T, Fault>,
    ) -> Result<T, Fault> {
        let previous_program = self.active_program.replace(program.clone());
        let previous_module = std::mem::replace(&mut self.current_module, identity.to_owned());
        let previous_metadata = std::mem::replace(
            &mut self.metadata,
            VmMetadata::new(identity.to_owned(), source_name.to_owned(), None),
        );
        let previous_map = std::mem::replace(
            &mut self.pc_map,
            build_pc_map(&program, xiao_bytecode::OperandWidth::Leb128).ok(),
        );
        let result = execute(self);
        self.pc_map = previous_map;
        self.metadata = previous_metadata;
        self.current_module = previous_module;
        self.active_program = previous_program;
        result
    }

    /// 确保模块在当前 VM 会话内至多初始化一次。
    pub(super) fn initialize_module(&mut self, identity: &str) -> Result<(), Fault> {
        if self.session.initialized_modules.contains_key(identity) {
            return Ok(());
        }
        if !self.initializing_modules.insert(identity.to_owned()) {
            return Err(self.module_fault(identity, "循环初始化"));
        }
        let result = self.initialize_uncached(identity);
        self.initializing_modules.remove(identity);
        result
    }

    /// 加载并执行尚未缓存的模块入口，成功后保存其导出。
    fn initialize_uncached(&mut self, identity: &str) -> Result<(), Fault> {
        let loader = self
            .module_loader
            .ok_or_else(|| self.module_fault(identity, "未注册模块编译器"))?;
        let compiled = loader
            .load(identity)
            .map_err(|error| self.module_fault(identity, error))?;
        let Some(compiled) = compiled else {
            return Ok(());
        };
        let instance_tables = compiled.instance_tables;
        let program = Rc::new(compiled.program);
        let source_name = compiled.source_name;
        let previous_exports = self.current_exports.take();
        self.current_exports = Some(BTreeMap::new());
        let result = self.with_module_program(identity, &source_name, program.clone(), |vm| {
            vm.execute(FuncId::new(0), &[], None)
        });
        let exports = self.current_exports.take().unwrap_or_default();
        self.current_exports = previous_exports;
        if let Err(Fault::Error(error)) = result {
            let wrapped =
                self.module_fault(identity, format!("{}: {}", error.code(), error.message()));
            if let Fault::Error(wrapper) = wrapped {
                return Err(Fault::Error(wrapper.with_cause(error)));
            }
        } else {
            result?;
        }
        self.session.initialized_modules.insert(
            identity.to_owned(),
            ModuleInstance {
                program,
                source_name,
                exports,
                instance_tables,
            },
        );
        self.sink.record(VmEvent::ModuleLoaded {
            module: identity.to_owned(),
        });
        Ok(())
    }

    /// 解析模块成员；并发原生模块返回仅供 TAC 计划调用的函数句柄。
    pub(super) fn module_member(
        &mut self,
        identity: &str,
        member: &str,
    ) -> Result<RuntimeValue, Fault> {
        if identity == "std.concurrent" || identity == "stdlib.concurrent" {
            // 类型阶段已验证成员并生成 TAC concurrency plan；VM 不重新解析
            // 模块成员名，动态调用只通过调用 span 查找已降低计划。
            return Ok(RuntimeValue::ModuleFunction(identity.to_owned(), 0));
        }
        if let Some(name) = member.strip_prefix("ascii:") {
            let child = format!("{identity}.{name}");
            if self
                .module_loader
                .is_some_and(|loader| loader.contains(&child))
            {
                self.initialize_module(&child)?;
                return Ok(RuntimeValue::Module(child));
            }
        }
        self.initialize_module(identity)?;
        let instance = self.session.initialized_modules.get(identity);
        if let Some(value) = instance.and_then(|module| module.exports.get(member)) {
            return Ok(value.clone());
        }
        if instance.is_some_and(|module| module.instance_tables.contains(member)) {
            return Err(
                self.module_fault(identity, format!("表构造器 {member} 暂不支持跨模块引用"))
            );
        }
        Err(self.module_fault(identity, format!("导出 {member} 不存在")))
    }

    /// 根据当前模块来源解析导入模块的规范身份。
    pub(super) fn import_identity(&self, module: &str) -> String {
        if module == "std.concurrent" || module == "stdlib.concurrent" {
            return module.to_owned();
        }
        if module == crate::STANDARD_LIBRARY_NAMESPACE
            || module.starts_with(crate::STANDARD_LIBRARY_PREFIX)
        {
            return module.to_owned();
        }
        if self.current_module == crate::STANDARD_LIBRARY_NAMESPACE
            || self
                .current_module
                .starts_with(crate::STANDARD_LIBRARY_PREFIX)
        {
            return if module.is_empty() {
                crate::STANDARD_LIBRARY_NAMESPACE.to_owned()
            } else {
                format!("{}.{module}", crate::STANDARD_LIBRARY_NAMESPACE)
            };
        }
        if let Some(path) = self.current_module.strip_prefix("package:") {
            let root = path.split('.').next().unwrap_or(path);
            format!("package:{root}.{module}")
        } else {
            format!("project:{module}")
        }
    }
}
