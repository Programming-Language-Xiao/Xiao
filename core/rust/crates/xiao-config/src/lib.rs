//! `config.xiao` 声明式配置模型的 crate 入口。
//!
//! 本 crate 只读取不可执行的配置子集。它复用 Xiao 的词法器和源码区间，
//! 但转换为独立的配置树，绝不调用普通解析器、模块解析器或 Runtime。

/// 规范化配置树中的依赖声明提取。
pub mod dependencies;
/// 配置诊断编号和诊断结果别名。
pub mod diagnostics;
/// 不可执行配置树及其访问辅助。
pub mod model;
/// 从 Xiao Token 流读取 `config.xiao` 的解析器。
pub mod parser;
/// 配置表白名单、字段类型和路径约束。
pub mod validation;

/// 重导出依赖声明及其运行时/开发期分类。
pub use dependencies::{
    DependencyDeclaration, DependencyKind, GitDependencyDeclaration, RemoteDependencyDeclaration,
    dependency_declarations, git_dependency_declarations, remote_dependency_declarations,
};
/// 重导出配置诊断编号、诊断类型和列表辅助，供上层保持统一错误接口。
pub use diagnostics::*;
/// 重导出不可执行配置文档、表、条目和值模型。
pub use model::{
    ConfigDocument, ConfigEntry, ConfigTable, ConfigValue, ConfigValueKind, NormalizedConfig,
};
/// 重导出配置解析入口和带诊断的结果类型。
pub use parser::{
    ConfigParseResult, parse, parse_config, parse_config_document, parse_config_project,
    parse_config_text,
};
/// 重导出通用配置和项目配置校验入口。
pub use validation::{validate_config, validate_project_config};
