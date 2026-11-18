//! 单次 VM 执行的模块编译回调；编译器由驱动器提供，VM 决定何时调用。

use xiao_bytecode::TacProgram;
use xiao_ir::IrProgram;

/// 重导出官方标准库的权威命名空间常量，供 VM 与驱动器共用。
pub use xiao_standard_library::{STANDARD_LIBRARY_NAMESPACE, STANDARD_LIBRARY_PREFIX};

/// 由统一前端和字节码流水线编译的一份文件模块。
#[derive(Debug)]
pub struct CompiledModule {
    /// 已验证 IR。
    pub ir: IrProgram,
    /// 可执行字节码。
    pub program: TacProgram,
    /// 诊断中使用的源码文件身份。
    pub source_name: String,
}

/// 已验证模块的执行视图；归档模块无需携带编译期 IR 或源码正文。
#[derive(Debug)]
pub struct ExecutableModule {
    /// 已通过权威字节码校验的程序。
    pub program: TacProgram,
    /// 诊断使用的逻辑源码身份。
    pub source_name: String,
    /// 不允许以函数值跨模块引用的实例表构造器键。
    pub instance_tables: std::collections::BTreeSet<String>,
}

/// 只读模块视图及首次引用时调用的统一编译入口。
pub trait ModuleLoader: std::fmt::Debug {
    /// 是否有该文件模块或目录命名空间。
    fn contains(&self, identity: &str) -> bool;
    /// 只在 VM 真正引用模块时编译；目录命名空间返回 `None`。
    fn compile(&self, identity: &str) -> Result<Option<CompiledModule>, String>;
    /// 默认消费统一前端结果；归档加载器可直接提供已验证的字节码。
    fn load(&self, identity: &str) -> Result<Option<ExecutableModule>, String> {
        self.compile(identity).map(|compiled| {
            compiled.map(|module| ExecutableModule {
                instance_tables: module
                    .ir
                    .body
                    .iter()
                    .filter_map(|statement| {
                        if let xiao_ir::IrStatementKind::Table {
                            name, table_kind, ..
                        } = &statement.kind
                            && table_kind == "instance"
                        {
                            Some(format!(
                                "{}:{}",
                                if name.backticked { "backtick" } else { "ascii" },
                                name.text
                            ))
                        } else {
                            None
                        }
                    })
                    .collect(),
                program: module.program,
                source_name: module.source_name,
            })
        })
    }
    /// 包加载失败时携带的来源环境；项目模块返回 `None`。
    fn environment(&self, identity: &str) -> Option<&str>;
}
