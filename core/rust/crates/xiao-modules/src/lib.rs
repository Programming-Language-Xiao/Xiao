//! Xiao 文件模块、导入图和导出边界的 crate 入口。
//!
//! 本 crate 只负责本地 `.xiao` 文件的发现、导入语句解析和静态名称/依赖
//! 关系；不执行模块初始化，不下载包，也不依赖类型检查、Runtime 或 CLI。

/// 05 阶段模块分析所使用的稳定诊断编号和消息标识。
mod diagnostics;
/// 项目根目录扫描、源码读取和本地模块候选建立。
mod discovery;
/// 跨模块名称、符号、绑定和依赖图的公开数据模型。
mod model;
/// 导入目标、词法作用域、再导出和循环依赖解析。
mod resolver;

/// 重导出模块分析诊断编号，供 CLI 和后续国际化层消费。
pub use diagnostics::*;
/// 暴露本地项目模块分析入口。
pub use discovery::analyze_project;
/// 暴露模块图、绑定和符号模型，作为后续 Runtime/IR 阶段的只读契约。
pub use model::{
    BindingKind, ExportOrigin, ImportEdge, ImportEdgeKind, ImportPathName, LexicalScopeId,
    ModuleGraph, ModuleKind, ModuleName, ModuleRecord, ModuleSymbol, ModuleSymbolKind,
    NamespaceRecord, ProjectModuleResult, ResolvedBinding,
};
