//! P2 类型检查稳定诊断编号。
//!
//! 消息文本由上层国际化层渲染；这里仅提供机器可识别的错误身份，避免
//! `xiao-types` 与 CLI 的语言目录形成反向依赖。

/// 读取未定义名称。
pub const UNDEFINED_NAME_CODE: &str = "X02-TYPE-001";
/// 当前作用域重复声明名称。
pub const DUPLICATE_DECLARATION_CODE: &str = "X02-TYPE-002";
/// 读取尚未初始化的名称。
pub const UNINITIALIZED_READ_CODE: &str = "X02-TYPE-003";
/// 赋值类型与锁定类型不兼容。
pub const ASSIGNMENT_TYPE_MISMATCH_CODE: &str = "X02-TYPE-004";
/// 运算符操作数类型不合法。
pub const INVALID_OPERANDS_CODE: &str = "X02-TYPE-005";
/// 显式转换不在支持矩阵内。
pub const INVALID_CONVERSION_CODE: &str = "X02-TYPE-006";
/// 可静态证明的溢出、除零或非法算术。
pub const ARITHMETIC_ERROR_CODE: &str = "X02-TYPE-007";
/// `const` 初始化不能在编译期求值。
pub const NON_CONSTANT_CODE: &str = "X02-TYPE-008";
/// HM 类型统一或 occurs-check 失败。
pub const UNIFICATION_ERROR_CODE: &str = "X02-TYPE-009";
