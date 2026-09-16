//! Xiao 词法与解析阶段的稳定诊断编号。
//!
//! 诊断身份由语法 crate 统一维护，具体渲染交给上层国际化和 CLI；本模块不依赖
//! 解析器状态，也不执行用户代码。

/// 非法 UTF-8 源文件的稳定诊断编号（从源码层重新导出）。
pub use xiao_source::INVALID_UTF8_CODE;

/// L0 非法字符的稳定诊断编号。
pub const INVALID_CHARACTER_CODE: &str = "X01-LEX-001";

/// 字符串字面量没有闭合时使用的稳定诊断编号。
pub const UNTERMINATED_STRING_CODE: &str = "X01-LEX-002";

/// 字符串中出现不支持的转义序列时使用的稳定诊断编号。
pub const INVALID_ESCAPE_CODE: &str = "X01-LEX-003";

/// 数字字面量结构不完整时使用的稳定诊断编号。
pub const INVALID_NUMBER_CODE: &str = "X01-LEX-004";

/// 反引号名称没有闭合时使用的稳定诊断编号。
pub const UNTERMINATED_BACKTICK_CODE: &str = "X01-LEX-005";

/// 文档注释没有闭合时使用的稳定诊断编号。
pub const UNTERMINATED_DOC_COMMENT_CODE: &str = "X01-LEX-006";

/// 缩进宽度无法对应四空格层级时使用的稳定诊断编号。
pub const INCONSISTENT_INDENT_CODE: &str = "X01-LEX-007";

/// 反引号名称中的转义序列不受支持时使用的稳定诊断编号。
pub const INVALID_BACKTICK_ESCAPE_CODE: &str = "X01-LEX-008";

/// 关闭分隔符与最近打开分隔符不匹配时使用的稳定诊断编号。
pub const UNMATCHED_DELIMITER_CODE: &str = "X01-LEX-009";

/// 文件结束时仍有未闭合分隔符时使用的稳定诊断编号。
pub const UNTERMINATED_DELIMITER_CODE: &str = "X01-LEX-010";

/// P0 中无法作为表达式起点的 Token 使用的稳定诊断编号。
pub const INVALID_EXPRESSION_CODE: &str = "X01-PARSE-001";

/// P0 中赋值左侧不是名称时使用的稳定诊断编号。
pub const INVALID_ASSIGNMENT_TARGET_CODE: &str = "X01-PARSE-002";

/// P0 遇到缩进代码块时使用的稳定诊断编号。
pub const UNSUPPORTED_BLOCK_CODE: &str = "X01-PARSE-003";

/// P0 赋值缺少右侧表达式时使用的稳定诊断编号。
pub const MISSING_ASSIGNMENT_VALUE_CODE: &str = "X01-PARSE-004";

/// P0 遇到运算、调用或其他复杂表达式尾部时使用的稳定诊断编号。
pub const UNSUPPORTED_EXPRESSION_CODE: &str = "X01-PARSE-005";

/// P1 选择器或路径结构非法时使用的稳定诊断编号。
pub const INVALID_SELECTOR_CODE: &str = "X01-PARSE-006";

/// P1 缺少配对分隔符时使用的稳定诊断编号。
pub const MISSING_DELIMITER_CODE: &str = "X01-PARSE-007";

/// P1 `as` 目标不是受支持标量类型时使用的稳定诊断编号。
pub const INVALID_CAST_TARGET_CODE: &str = "X01-PARSE-008";

/// P1 表达式需要值但当前 Token 不能提供值时使用的稳定诊断编号。
pub const MISSING_EXPRESSION_CODE: &str = "X01-PARSE-009";

/// P1 赋值运算符后出现非法表达式时使用的稳定诊断编号。
pub const INVALID_ASSIGNMENT_CODE: &str = "X01-PARSE-010";

/// P2 声明缺少目标名称或声明结构不完整时使用的稳定诊断编号。
pub const INVALID_DECLARATION_CODE: &str = "X02-PARSE-001";

/// `const` 声明缺少初始化表达式时使用的稳定诊断编号。
pub const MISSING_CONST_VALUE_CODE: &str = "X02-PARSE-002";

/// 类型前缀后出现的目标不是名称时使用的稳定诊断编号。
pub const INVALID_DECLARATION_TARGET_CODE: &str = "X02-PARSE-003";

/// C0 容器字面量结构非法时使用的稳定诊断编号。
pub const INVALID_CONTAINER_CODE: &str = "X03-PARSE-001";

/// C0/C2-A 容器条目缺少值、键值分隔符或集合元素分隔符时使用的稳定诊断编号。
pub const INVALID_CONTAINER_ENTRY_CODE: &str = "X03-PARSE-002";

/// C0 当前只允许精确索引时使用的稳定诊断编号。
pub const UNSUPPORTED_CONTAINER_SELECTOR_CODE: &str = "X03-PARSE-003";

/// 尚未开放 `const name[path]` 运行时锁定语义时使用的稳定诊断编号。
pub const UNSUPPORTED_CONST_PATH_CODE: &str = "X03-PARSE-004";

/// 集合类型并集注解结构非法时使用的稳定诊断编号。
pub const INVALID_SET_TYPE_ANNOTATION_CODE: &str = "X03-PARSE-005";

/// 集合类型并集注解出现在路径约束上时使用的稳定诊断编号。
pub const UNSUPPORTED_SET_TYPE_PATH_CODE: &str = "X03-PARSE-006";

/// `const` 声明尝试使用集合类型注解时使用的稳定诊断编号。
pub const UNSUPPORTED_CONST_SET_TYPE_CODE: &str = "X03-PARSE-007";

/// 函数或控制流语句结构非法时使用的稳定诊断编号。
pub const INVALID_FUNCTION_CODE: &str = "X04-PARSE-001";

/// 函数参数列表结构非法时使用的稳定诊断编号。
pub const INVALID_PARAMETER_CODE: &str = "X04-PARSE-002";

/// 代码块缺少缩进体时使用的稳定诊断编号。
pub const MISSING_BLOCK_CODE: &str = "X04-PARSE-003";

/// 控制流头部或返回语句结构非法时使用的稳定诊断编号。
pub const INVALID_CONTROL_FLOW_CODE: &str = "X04-PARSE-004";

/// 函数返回类型注解无法解析时使用的稳定诊断编号。
pub const INVALID_FUNCTION_TYPE_CODE: &str = "X04-PARSE-005";

/// `[main]` 入口头部结构非法时使用的稳定诊断编号。
pub const INVALID_ENTRY_CODE: &str = "X04-PARSE-006";

/// 错误控制流头部结构非法时使用的稳定诊断编号。
pub const INVALID_ERROR_CONTROL_FLOW_CODE: &str = "X07-PARSE-001";

/// `raise` 缺少错误表达式时使用的稳定诊断编号。
pub const MISSING_RAISE_VALUE_CODE: &str = "X07-PARSE-002";

/// `try` 没有 `catch` 或 `finally` 时使用的稳定诊断编号。
pub const MISSING_ERROR_HANDLER_CODE: &str = "X07-PARSE-003";

/// 绝对模块路径结构非法时使用的稳定诊断编号。
pub const INVALID_IMPORT_PATH_CODE: &str = "X05-PARSE-001";

/// 导入列表或目标缺失时使用的稳定诊断编号。
pub const INVALID_IMPORT_TARGET_CODE: &str = "X05-PARSE-002";

/// 导入别名结构非法时使用的稳定诊断编号。
pub const INVALID_IMPORT_ALIAS_CODE: &str = "X05-PARSE-003";

/// 当前阶段不支持的导入形式使用的稳定诊断编号。
pub const UNSUPPORTED_IMPORT_FORM_CODE: &str = "X05-PARSE-004";

/// 表头结构非法时使用的稳定诊断编号。
pub const INVALID_TABLE_HEADER_CODE: &str = "X05-PARSE-005";

/// 表体出现不允许的成员语句时使用的稳定诊断编号。
pub const INVALID_TABLE_MEMBER_CODE: &str = "X05-PARSE-006";

/// 表声明缺少有效缩进体时使用的稳定诊断编号。
pub const MISSING_TABLE_BODY_CODE: &str = "X05-PARSE-007";
