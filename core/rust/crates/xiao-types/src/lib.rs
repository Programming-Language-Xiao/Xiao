//! Xiao P2 静态类型与值规则的稳定门面。
//!
//! 实现按职责拆分为类型表示、统一器、作用域环境、转换矩阵、数值检查和
//! AST 类型检查器。门面只负责装配与公开重导出，不承载语义实现细节。

/// AST 类型检查器和运行时检查标记。
mod checker;
/// C0 容器类型、路径约束和物化计划。
mod containers;
/// 标量转换矩阵与赋值兼容规则。
mod conversion;
/// P2 类型诊断编号。
mod diagnostics;
/// 作用域和绑定状态环境。
mod environment;
/// C0 空容器形状计划装配。
mod materialization;
/// 数值提升、范围和常量辅助。
mod numeric;
/// C0 精确路径转换和静态解析。
mod path_constraints;
/// 类型与 HM 类型方案表示。
mod types;
/// 统一、occurs-check、泛化和实例化算法。
mod unify;

/// 重新导出类型检查器及其结果结构。
pub use checker::{RuntimeCheck, RuntimeCheckKind, TypeCheckResult, TypeChecker, TypedNode, check};
/// 重新导出容器形状、路径约束和默认值计划。
pub use containers::{
    ArrayType, ContainerMaterializationPlan, ContainerPathSegment, DictEntryType, DictType,
    PathConstraint, PathConstraintTree,
};
/// 重新导出转换分类和数值提升辅助。
pub use conversion::{
    Conversion, ConversionError, ConversionKind, can_assign, classify_conversion, is_float,
    is_integer, is_numeric, numeric_rank, promote_numeric_scalars,
};
/// 重新导出稳定的 P2 诊断编号。
pub use diagnostics::*;
/// 重新导出作用域环境和绑定结构。
pub use environment::{Binding, EnvironmentError, TypeEnvironment};
/// 重新导出静态物化计划装配辅助。
pub use materialization::{build_plan, merge_constraints};
/// 重新导出常量、数值检查和二元分析函数。
pub use numeric::{
    ConstantValue, NumericError, NumericOperation, analyze_binary, binary_scalar_type,
    boolean_integer_adjust, check_float_range, check_float_to_integer_range, check_integer_range,
    is_decimal_integer, parse_float_literal, parse_integer_literal,
};
/// 重新导出容器路径转换和解析错误。
pub use path_constraints::{
    PathConversionError, PathConversionErrorKind, PathResolutionError, PathResolutionErrorKind,
    lower_index_path, resolve_exact_path,
};
/// 重新导出类型表示和方案别名。
pub use types::{Scheme, Type, TypeScheme, TypeVarId};
/// 重新导出 HM 统一上下文和替换结构。
pub use unify::{Substitution, TypeContext, UnifyError};

/// 与语法层共享的标量类型别名，避免调用方同时依赖两个名字空间。
pub use xiao_syntax::ScalarType;
