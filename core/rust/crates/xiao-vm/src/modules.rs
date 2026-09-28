//! 单次 VM 执行的模块编译回调；编译器由驱动器提供，VM 决定何时调用。

use xiao_bytecode::TacProgram;
use xiao_ir::IrProgram;

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

/// 只读模块视图及首次引用时调用的统一编译入口。
pub trait ModuleLoader: std::fmt::Debug {
    /// 是否有该文件模块或目录命名空间。
    fn contains(&self, identity: &str) -> bool;
    /// 只在 VM 真正引用模块时编译；目录命名空间返回 `None`。
    fn compile(&self, identity: &str) -> Result<Option<CompiledModule>, String>;
    /// 包加载失败时携带的来源环境；项目模块返回 `None`。
    fn environment(&self, identity: &str) -> Option<&str>;
}
