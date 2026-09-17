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
/// 04 函数签名和参数匹配模型。
mod functions;
/// C0 空容器形状计划装配。
mod materialization;
/// 数值提升、范围和常量辅助。
mod numeric;
/// C0 精确路径转换和静态解析。
mod path_constraints;
/// C1 有序容器选择的规范化计划模型。
mod selection_model;
/// C1 随机源和抽样算法。
mod selection_random;
/// C1 选择结果的静态形状重建。
mod selection_shape;
/// C2-A 集合元素类型与可哈希能力。
mod set_types;
/// 05-C 表类型、成员签名和生命周期静态契约。
mod tables;
/// 类型与 HM 类型方案表示。
mod types;
/// 统一、occurs-check、泛化和实例化算法。
mod unify;

/// 重导出字符串字面量的唯一解码实现，供 IR 降低复用。
pub use checker::container_checker::decode_string_literal;
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
/// 重新导出函数签名旁路结构。
pub use functions::{FunctionParameterSignature, FunctionSignature};
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
/// 重新导出选择计划、广播计划和随机种子记录。
pub use selection_model::{
    BroadcastAssignmentPlan, RandomSeedPlan, SelectionItemPlan, SelectionPath,
    SelectionPathSegment, SelectionPlan, StepPlan, normalize_index,
};
/// 重新导出可注入随机源和抽样辅助。
pub use selection_random::{RandomSelectionError, RandomSource, SeededRandom, sample_indices};
/// 重新导出选择结果形状辅助。
pub use selection_shape::{
    direct_selection_children, empty_selection_type, project_selection_type,
};
/// 重新导出集合类型和可哈希判定。
pub use set_types::{Hashability, SetType, can_assign_set, hashability};
/// 重新导出表值类型、成员签名和可见性模型。
pub use tables::{
    TableMemberKind, TableMemberSignature, TableSignature, TableType, TableValueKind, Visibility,
};
/// 重新导出类型表示和方案别名。
pub use types::{Scheme, Type, TypeScheme, TypeVarId};
/// 重新导出 HM 统一上下文和替换结构。
pub use unify::{Substitution, TypeContext, UnifyError};

/// 与语法层共享的标量类型别名，避免调用方同时依赖两个名字空间。
pub use xiao_syntax::ScalarType;
