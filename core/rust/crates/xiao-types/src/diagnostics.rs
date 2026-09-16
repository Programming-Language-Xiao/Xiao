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
/// 选择器作用于不支持高级选择的容器。
pub const SELECTOR_UNORDERED_CONTAINER_CODE: &str = "X03-TYPE-008";
/// 选择器步长不是合法的非零整数。
pub const SELECTOR_INVALID_STEP_CODE: &str = "X03-TYPE-009";
/// 随机选择数量不是合法的非负整数。
pub const SELECTOR_INVALID_RANDOM_COUNT_CODE: &str = "X03-TYPE-010";
/// 无放回随机选择数量超过候选元素数。
pub const SELECTOR_RANDOM_EXHAUSTED_CODE: &str = "X03-TYPE-011";
/// 选择器左值不满足标量广播赋值规则。
pub const SELECTOR_ASSIGNMENT_CODE: &str = "X03-TYPE-012";
/// `random.seed` 参数不满足非负 `lint` 规则。
pub const RANDOM_SEED_CODE: &str = "X03-TYPE-013";
/// `random.seed` 参数数量错误。
pub const RANDOM_SEED_ARITY_CODE: &str = "X03-TYPE-014";
/// 集合元素与已锁定的单一元素类型不匹配。
pub const SET_ELEMENT_TYPE_MISMATCH_CODE: &str = "X03-TYPE-015";
/// 集合元素不是 C2-A 可接受的可哈希类型。
pub const SET_UNHASHABLE_ELEMENT_CODE: &str = "X03-TYPE-016";
/// 集合字面量包含重复的静态元素。
pub const SET_DUPLICATE_ELEMENT_CODE: &str = "X03-TYPE-017";
/// `set()` 构造式接收了错误数量的参数。
pub const SET_CONSTRUCTOR_ARITY_CODE: &str = "X03-TYPE-018";
/// 集合成员判断的左右类型不满足集合元素约束。
pub const SET_MEMBERSHIP_TYPE_CODE: &str = "X03-TYPE-019";
/// 集合不支持数字、键名或高级选择器索引。
pub const SET_INDEX_UNSUPPORTED_CODE: &str = "X03-TYPE-020";
/// 集合代数操作数不是两个集合。
pub const SET_OPERATION_TYPE_CODE: &str = "X03-TYPE-021";
/// 集合比较操作数不是两个集合。
pub const SET_COMPARISON_TYPE_CODE: &str = "X03-TYPE-022";

/// 函数名称重复或函数声明无法登记。
pub const FUNCTION_DECLARATION_CODE: &str = "X04-TYPE-001";
/// 函数调用参数与签名不匹配。
pub const FUNCTION_CALL_CODE: &str = "X04-TYPE-002";
/// 返回值与函数返回类型不匹配。
pub const FUNCTION_RETURN_CODE: &str = "X04-TYPE-003";
/// 参数或返回类型无法从静态约束中推断。
pub const FUNCTION_INFERENCE_CODE: &str = "X04-TYPE-004";
/// `if`/`while` 条件不是布尔类型。
pub const CONDITION_TYPE_CODE: &str = "X04-TYPE-005";
/// `for in` 右侧不是已知可迭代容器。
pub const ITERABLE_TYPE_CODE: &str = "X04-TYPE-006";
/// `break`/`continue` 出现在循环外。
pub const LOOP_CONTROL_CODE: &str = "X04-TYPE-007";
/// 程序入口声明与静态规则冲突。
pub const ENTRY_RULE_CODE: &str = "X04-TYPE-008";

/// `raise` 操作数不是可恢复错误值时使用的稳定诊断编号。
pub const RAISE_TYPE_CODE: &str = "X07-TYPE-001";
/// `catch` 的错误类型名称无效时使用的稳定诊断编号。
pub const CATCH_TYPE_CODE: &str = "X07-TYPE-002";
/// 普通 `catch` 尝试捕获不可恢复 `FatalError` 时使用的稳定诊断编号。
pub const CATCH_FATAL_CODE: &str = "X07-TYPE-003";
/// `catch` 处理器顺序从宽到窄时使用的稳定诊断编号。
pub const CATCH_ORDER_CODE: &str = "X07-TYPE-004";

/// 表声明名称重复或表签名无法登记。
pub const TABLE_DECLARATION_CODE: &str = "X05-TYPE-001";
/// 表成员重复、缺失或成员访问形状不合法。
pub const TABLE_MEMBER_CODE: &str = "X05-TYPE-002";
/// 表成员在当前访问位置不可见。
pub const TABLE_VISIBILITY_CODE: &str = "X05-TYPE-003";
/// `new` 目标不是可实例化表，或构造参数不匹配。
pub const TABLE_CONSTRUCTOR_CODE: &str = "X05-TYPE-004";
/// `init`/`drop` 生命周期方法签名不满足静态契约。
pub const TABLE_LIFECYCLE_CODE: &str = "X05-TYPE-005";
/// 表字段初始化器不是允许的静态纯表达式。
pub const TABLE_INITIALIZER_CODE: &str = "X05-TYPE-006";
