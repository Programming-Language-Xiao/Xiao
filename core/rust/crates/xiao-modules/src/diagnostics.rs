//! 05-B 模块发现和名称解析诊断编号。

/// 文件读取、UTF-8 或项目根失败。
pub const MODULE_IO_CODE: &str = "X05-MODULE-001";
/// 文件或目录无法映射为模块名称。
pub const INVALID_MODULE_PATH_CODE: &str = "X05-MODULE-002";
/// 模块/命名空间名称冲突。
pub const MODULE_PATH_CONFLICT_CODE: &str = "X05-MODULE-003";
/// 导入目标不存在。
pub const MISSING_IMPORT_TARGET_CODE: &str = "X05-MODULE-004";
/// 选择导入的符号不存在。
pub const MISSING_IMPORT_SYMBOL_CODE: &str = "X05-MODULE-005";
/// 同一词法作用域中的导入名称冲突。
pub const IMPORT_BINDING_CONFLICT_CODE: &str = "X05-MODULE-006";
/// 文件模块依赖形成循环。
pub const MODULE_CYCLE_CODE: &str = "X05-MODULE-007";
/// 模块/命名空间限定符使用方式非法。
pub const INVALID_QUALIFIER_USE_CODE: &str = "X05-MODULE-008";
