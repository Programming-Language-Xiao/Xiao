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

/// 容器声明或元素赋值的静态类型不匹配。
pub const CONTAINER_TYPE_MISMATCH_CODE: &str = "X03-TYPE-001";
/// 字典表或字典列出现重复键。
pub const DUPLICATE_CONTAINER_KEY_CODE: &str = "X03-TYPE-002";
/// 容器路径段的种类与当前容器不匹配。
pub const INVALID_CONTAINER_PATH_CODE: &str = "X03-TYPE-003";
/// 静态可知的数组、元组或字典列数字索引越界。
pub const CONTAINER_INDEX_OUT_OF_BOUNDS_CODE: &str = "X03-TYPE-004";
/// 静态可知的字典键不存在。
pub const CONTAINER_KEY_NOT_FOUND_CODE: &str = "X03-TYPE-005";
/// 声明路径无法转换为受支持的非负整数/键路径。
pub const INVALID_DECLARATION_PATH_CODE: &str = "X03-TYPE-006";
/// C0 选择器包含范围、多选、步长或随机项。
pub const UNSUPPORTED_CONTAINER_SELECTOR_CODE: &str = "X03-TYPE-007";
