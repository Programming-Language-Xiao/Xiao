//! `config.xiao` 的稳定诊断编号。
//!
//! 编号属于配置读取边界，消息文本只作当前语言预览；上层国际化可以根据
//! `message_id` 和结构化参数重新渲染，不应依赖中文句子判断错误类别。

use xiao_diagnostics::Diagnostic;

/// 配置解析结果中的诊断类型。
pub type ConfigDiagnostic = Diagnostic;

/// 配置诊断列表的稳定别名。
pub type ConfigDiagnostics = Vec<ConfigDiagnostic>;

/// 配置文件顶层出现非法结构时使用的编号。
pub const CONFIG_BOUNDARY_CODE: &str = "X05-CONFIG-001";
/// 配置中出现函数、调用、控制流、导入或表达式时使用的编号。
pub const UNSUPPORTED_CONSTRUCT_CODE: &str = "X05-CONFIG-002";
/// 同一个表中出现重复键时使用的编号。
pub const DUPLICATE_KEY_CODE: &str = "X05-CONFIG-003";
/// 配置字段值类型不符合模式时使用的编号。
pub const CONFIG_TYPE_MISMATCH_CODE: &str = "X05-CONFIG-004";
/// 已知表中出现未登记字段时使用的编号。
pub const UNKNOWN_FIELD_CODE: &str = "X05-CONFIG-005";
/// 顶层表名不是当前配置模式允许的表时使用的编号。
pub const UNKNOWN_TABLE_CODE: &str = "X05-CONFIG-006";
/// 必填配置字段或表缺失时使用的编号。
pub const MISSING_REQUIRED_CODE: &str = "X05-CONFIG-007";
/// 导出模块路径不符合项目相对 `.xiao` 规则时使用的编号。
pub const INVALID_EXPORT_PATH_CODE: &str = "X05-CONFIG-008";
/// 表头形状非法时使用的编号。
pub const CONFIG_INVALID_TABLE_HEADER_CODE: &str = "X05-CONFIG-009";
/// 字面量、数组或字典结构非法时使用的编号。
pub const CONFIG_INVALID_VALUE_CODE: &str = "X05-CONFIG-010";
/// 键值或容器元素分隔符缺失时使用的编号。
pub const MISSING_SEPARATOR_CODE: &str = "X05-CONFIG-011";
/// 同一个配置文档中重复出现表头时使用的编号。
pub const DUPLICATE_TABLE_CODE: &str = "X05-CONFIG-012";
/// 本地路径依赖不是项目根相对路径时使用的编号。
pub const INVALID_DEPENDENCY_PATH_CODE: &str = "X05-CONFIG-013";
/// 依赖版本约束为空或包含控制字符时使用的编号。
pub const INVALID_DEPENDENCY_CONSTRAINT_CODE: &str = "X05-CONFIG-014";
/// 依赖来源引用为空或包含控制字符时使用的编号。
pub const INVALID_DEPENDENCY_SOURCE_CODE: &str = "X05-CONFIG-015";
/// Git 依赖引用、互斥字段或仓库地址不合法。
pub const INVALID_DEPENDENCY_GIT_CODE: &str = "X05-CONFIG-016";

/// 判断诊断列表是否包含错误级别项目。
#[must_use]
pub fn has_errors(diagnostics: &[ConfigDiagnostic]) -> bool {
    diagnostics.iter().any(Diagnostic::is_error)
}
