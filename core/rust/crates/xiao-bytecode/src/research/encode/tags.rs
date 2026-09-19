//! opcode 与外部枚举的稳定标签映射。

use super::{
    ArgKind, ArithOp, CompareOp, EncodeError, ParamKind, RegisterClass, SetCompareOp, SetOpKind,
    TacOp,
};
use xiao_lifetime::ReleaseActionKind;
use xiao_syntax::ScalarType;

/// `TacOp` 到稳定 opcode 的映射，0–35 连续。
///
/// 这张表是格式的核心契约：**只能追加，不得重排**。调换两个编号会让旧字节被读
/// 成另一种指令，而这种错误在往返测试里是看不出来的（编码器和解码器用的是同一
/// 张表）。`all_ops_program` 里断言了 `ops` 的顺序恰好产生 `0..36`，新增变体插在
/// 中间会立刻失败。
pub(super) fn opcode(op: &TacOp) -> u8 {
    match op {
        TacOp::LoadConst(_) => 0,
        TacOp::LoadNone => 1,
        TacOp::LoadFunc(_) => 2,
        TacOp::Move(_) => 3,
        TacOp::Copy(_) => 4,
        TacOp::Box(_) => 5,
        TacOp::Unbox(_) => 6,
        TacOp::Cast { .. } => 7,
        TacOp::Arith { .. } => 8,
        TacOp::Compare { .. } => 9,
        TacOp::NewArray { .. } => 10,
        TacOp::NewTuple { .. } => 11,
        TacOp::NewDictTable { .. } => 12,
        TacOp::NewDictColumn { .. } => 13,
        TacOp::NewSet { .. } => 14,
        TacOp::IndexGet { .. } => 15,
        TacOp::Jump(_) => 16,
        TacOp::BranchIf { .. } => 17,
        TacOp::Call { .. } => 18,
        TacOp::CallDynamic { .. } => 19,
        TacOp::Return { .. } => 20,
        TacOp::Raise { .. } => 21,
        TacOp::MakeError { .. } => 22,
        TacOp::CallSub { .. } => 23,
        TacOp::RetFromSub => 24,
        TacOp::Check { .. } => 25,
        TacOp::Release { .. } => 26,
        TacOp::Transfer { .. } => 27,
        TacOp::RunReleasePlan { .. } => 28,
        TacOp::EnterScope(_) => 29,
        TacOp::ExitScope { .. } => 30,
        TacOp::SelectorApply { .. } => 31,
        TacOp::BroadcastAssign { .. } => 32,
        TacOp::RandomSeed { .. } => 33,
        TacOp::SetOp { .. } => 34,
        TacOp::SetCompare { .. } => 35,
    }
}

/// `SetOpKind` 与稳定标签的双向表。
pub(super) fn set_op_tag(value: SetOpKind) -> u8 {
    match value {
        SetOpKind::Union => 0,
        SetOpKind::Intersection => 1,
        SetOpKind::Difference => 2,
        SetOpKind::SymmetricDifference => 3,
    }
}

/// 解析 `SetOpKind` 标签；未知标签显式报错，不退回默认值。
pub(super) fn set_op_from_tag(tag: u8) -> Result<SetOpKind, EncodeError> {
    Ok(match tag {
        0 => SetOpKind::Union,
        1 => SetOpKind::Intersection,
        2 => SetOpKind::Difference,
        3 => SetOpKind::SymmetricDifference,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "SetOpKind".to_owned(),
                value: value as u64,
            });
        }
    })
}

/// `SetCompareOp` 与稳定标签的双向表。
pub(super) fn set_compare_tag(value: SetCompareOp) -> u8 {
    match value {
        SetCompareOp::Equal => 0,
        SetCompareOp::NotEqual => 1,
        SetCompareOp::ProperSubset => 2,
        SetCompareOp::Subset => 3,
        SetCompareOp::ProperSuperset => 4,
        SetCompareOp::Superset => 5,
        SetCompareOp::Member => 6,
        SetCompareOp::NotMember => 7,
    }
}

/// 解析 `SetCompareOp` 标签；未知标签显式报错，不退回默认值。
pub(super) fn set_compare_from_tag(tag: u8) -> Result<SetCompareOp, EncodeError> {
    Ok(match tag {
        0 => SetCompareOp::Equal,
        1 => SetCompareOp::NotEqual,
        2 => SetCompareOp::ProperSubset,
        3 => SetCompareOp::Subset,
        4 => SetCompareOp::ProperSuperset,
        5 => SetCompareOp::Superset,
        6 => SetCompareOp::Member,
        7 => SetCompareOp::NotMember,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "SetCompareOp".to_owned(),
                value: value as u64,
            });
        }
    })
}

/// `ScalarType` 与稳定标签的双向表。
///
/// 用数组而不是两段 `match`，是为了让正反两个方向共用同一份数据：写成两段
/// `match` 时「正向写 3、反向读回 4」这种错配编译得过去，测试也未必覆盖到。
/// 顺序即标签，改动等于改格式。
pub(super) const SCALAR_TAGS: [(ScalarType, u8); 8] = [
    (ScalarType::Int, 0),
    (ScalarType::Sint, 1),
    (ScalarType::Lint, 2),
    (ScalarType::Float, 3),
    (ScalarType::Sfloat, 4),
    (ScalarType::Lfloat, 5),
    (ScalarType::Str, 6),
    (ScalarType::Bool, 7),
];

/// 查 `ScalarType` 的正向标签。
///
/// **穷尽匹配，没有兜底分支**：`ScalarType` 新增变体而没同步本函数时，这里会
/// **编译失败**，强制作者回来处理。
///
/// 曾经写成「查 [`SCALAR_TAGS`]，查不到退回 0」——那样确实不会崩，但会**静默写出
/// 一个错误标签**，产出一份「能解码、标量类型却被悄悄改写」的字节。错误推迟到
/// 计算结果不对时才暴露，而且没有任何一层能指出是编码器写错了。编译失败比这
/// 危险得多地便宜。
///
/// 取值必须与 [`SCALAR_TAGS`] 一致；两者之间的漂移由
/// `scalar_and_release_tags_have_one_bidirectional_mapping` 逐条钉住。
pub(super) const fn scalar_tag(value: ScalarType) -> u8 {
    match value {
        ScalarType::Int => 0,
        ScalarType::Sint => 1,
        ScalarType::Lint => 2,
        ScalarType::Float => 3,
        ScalarType::Sfloat => 4,
        ScalarType::Lfloat => 5,
        ScalarType::Str => 6,
        ScalarType::Bool => 7,
    }
}

/// 按标签还原标量类型；未知标签报 [`EncodeError::InvalidEnum`]。
///
/// 不退回默认标量：静默退回会让一份损坏字节被当成合法程序继续往执行器走，
/// 错误会推迟到计算结果不对的时候才暴露。
pub(super) fn scalar_from_tag(tag: u8) -> Result<ScalarType, EncodeError> {
    SCALAR_TAGS
        .iter()
        .find(|(_, item)| *item == tag)
        .map(|(value, _)| *value)
        .ok_or_else(|| EncodeError::InvalidEnum {
            field: "ScalarType".to_owned(),
            value: tag as u64,
        })
}

/// `ReleaseActionKind` 到标签的映射，直接取 `ALL` 数组下标。
///
/// 标签顺序就是 `xiao-lifetime` 冻结的枚举顺序，所以这条映射是与上游的接口
/// 契约而不是本地编号：上游调整 `ALL` 的次序就等于改格式。查不到时报
/// [`EncodeError::InvalidEnum`]，`value` 填 `u64::MAX`——出错的是「这个值不在
/// `ALL` 里」，没有可报的输入标签。
pub(super) fn release_tag(value: ReleaseActionKind) -> Result<u8, EncodeError> {
    ReleaseActionKind::ALL
        .iter()
        .position(|item| *item == value)
        .map(|tag| tag as u8)
        .ok_or_else(|| EncodeError::InvalidEnum {
            field: "ReleaseActionKind".to_owned(),
            value: u64::MAX,
        })
}

/// 按 `ALL` 下标还原释放动作类别；越界报 [`EncodeError::InvalidEnum`]。
///
/// 用 `ALL.get` 而不是下标索引，是因为 `tag` 直接来自输入字节：索引越界会 panic，
/// 而这里需要的是一条结构化错误。强释放与弱释放的区分决定引用计数是否递减，
/// 读错标签会静默改变释放语义。
pub(super) fn release_from_tag(tag: u8) -> Result<ReleaseActionKind, EncodeError> {
    ReleaseActionKind::ALL
        .get(tag as usize)
        .copied()
        .ok_or_else(|| EncodeError::InvalidEnum {
            field: "ReleaseActionKind".to_owned(),
            value: tag as u64,
        })
}

/// 写 `ArithOp` 的稳定标签（加 0 到幂 6）。
///
/// 和 opcode 表一样只能追加：重排会让旧字节被当成另一种运算执行。算术错误不会
/// 被结构校验发现，只会在结果里体现出来。
pub(super) fn arith_tag(value: ArithOp) -> u8 {
    match value {
        ArithOp::Add => 0,
        ArithOp::Subtract => 1,
        ArithOp::Multiply => 2,
        ArithOp::Divide => 3,
        ArithOp::FloorDivide => 4,
        ArithOp::Remainder => 5,
        ArithOp::Power => 6,
    }
}

/// 按标签还原算术运算；未知标签报 [`EncodeError::InvalidEnum`]。
///
/// 这里必须报错而不是挑一个默认运算：除法与取模的编号相邻，静默兜底会把一份
/// 损坏字节变成一次「合法的」错运算。
pub(super) fn arith_from_tag(tag: u8) -> Result<ArithOp, EncodeError> {
    Ok(match tag {
        0 => ArithOp::Add,
        1 => ArithOp::Subtract,
        2 => ArithOp::Multiply,
        3 => ArithOp::Divide,
        4 => ArithOp::FloorDivide,
        5 => ArithOp::Remainder,
        6 => ArithOp::Power,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "ArithOp".to_owned(),
                value: value as u64,
            });
        }
    })
}

/// 写 `CompareOp` 的稳定标签（`<` 0 到 `!=` 5）。
///
/// 顺序即标签，只可追加。比较结果恒为布尔，所以标签错位不会被类别检查发现——
/// 仍然是同一类值，只是比较的语义变了。
pub(super) fn compare_tag(value: CompareOp) -> u8 {
    match value {
        CompareOp::Less => 0,
        CompareOp::LessEqual => 1,
        CompareOp::Greater => 2,
        CompareOp::GreaterEqual => 3,
        CompareOp::Equal => 4,
        CompareOp::NotEqual => 5,
    }
}

/// 按标签还原比较运算；未知标签报 [`EncodeError::InvalidEnum`]。
pub(super) fn compare_from_tag(tag: u8) -> Result<CompareOp, EncodeError> {
    Ok(match tag {
        0 => CompareOp::Less,
        1 => CompareOp::LessEqual,
        2 => CompareOp::Greater,
        3 => CompareOp::GreaterEqual,
        4 => CompareOp::Equal,
        5 => CompareOp::NotEqual,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "CompareOp".to_owned(),
                value: value as u64,
            });
        }
    })
}

/// 写实参类别标签（位置 0、关键字 1、`*` 2、`**` 3）。
///
/// 类别决定被调方怎么绑定形参：位置实参按顺序占槽，关键字实参按名字找，
/// `*`/`**` 展开成变参。读错标签不会越界，只会把值绑到别的形参上。
pub(super) fn arg_kind_tag(value: ArgKind) -> u8 {
    match value {
        ArgKind::Positional => 0,
        ArgKind::Keyword => 1,
        ArgKind::VarArgs => 2,
        ArgKind::KwArgs => 3,
    }
}

/// 按标签还原实参类别；未知标签报 [`EncodeError::InvalidEnum`]。
pub(super) fn arg_kind_from_tag(tag: u8) -> Result<ArgKind, EncodeError> {
    Ok(match tag {
        0 => ArgKind::Positional,
        1 => ArgKind::Keyword,
        2 => ArgKind::VarArgs,
        3 => ArgKind::KwArgs,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "ArgKind".to_owned(),
                value: value as u64,
            });
        }
    })
}

/// 写形参类别标签（位置或关键字 0、位置专用 1、关键字专用 2、`*args` 3、
/// `**kwargs` 4）。
///
/// 标签顺序与 [`ParamKind`] 的变体声明顺序一致，但**它本身是格式**：变体顺序
/// 变了就得同步改这里，不能靠「声明顺序即标签」的默契。
pub(super) fn param_kind_tag(value: ParamKind) -> u8 {
    match value {
        ParamKind::PositionalOrKeyword => 0,
        ParamKind::PositionalOnly => 1,
        ParamKind::KeywordOnly => 2,
        ParamKind::VarArgs => 3,
        ParamKind::VarKeywords => 4,
    }
}

/// 按标签还原形参类别；未知标签报 [`EncodeError::InvalidEnum`]。
pub(super) fn param_kind_from_tag(tag: u8) -> Result<ParamKind, EncodeError> {
    Ok(match tag {
        0 => ParamKind::PositionalOrKeyword,
        1 => ParamKind::PositionalOnly,
        2 => ParamKind::KeywordOnly,
        3 => ParamKind::VarArgs,
        4 => ParamKind::VarKeywords,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "ParamKind".to_owned(),
                value: value as u64,
            });
        }
    })
}

/// 写寄存器类别标签（整数 0、浮点 1、布尔 2、对象句柄 3、动态 4、无 5、合流 6）。
///
/// 类别是物理分配的依据：它决定值落在寄存器文件还是帧槽、要不要带运行时类型标
/// 签。标签错位不会让编码失败，只会让分配器把对象句柄当整数处理。
pub(super) fn register_class_tag(value: RegisterClass) -> u8 {
    match value {
        RegisterClass::Int => 0,
        RegisterClass::Float => 1,
        RegisterClass::Bool => 2,
        RegisterClass::ObjHandle => 3,
        RegisterClass::Dynamic => 4,
        RegisterClass::None => 5,
        RegisterClass::Poly => 6,
    }
}

/// 按标签还原寄存器类别；未知标签报 [`EncodeError::InvalidEnum`]。
///
/// 不退回 [`RegisterClass::Poly`]：`Poly` 是「合流点无法收敛」这一具体事实的
/// 表示，用它兜底会把「类别未知」和「类别确实退化」混成一种。
pub(super) fn register_class_from_tag(tag: u8) -> Result<RegisterClass, EncodeError> {
    Ok(match tag {
        0 => RegisterClass::Int,
        1 => RegisterClass::Float,
        2 => RegisterClass::Bool,
        3 => RegisterClass::ObjHandle,
        4 => RegisterClass::Dynamic,
        5 => RegisterClass::None,
        6 => RegisterClass::Poly,
        value => {
            return Err(EncodeError::InvalidEnum {
                field: "RegisterClass".to_owned(),
                value: value as u64,
            });
        }
    })
}
