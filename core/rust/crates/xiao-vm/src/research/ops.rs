//! 值运算的薄包装。
//!
//! 语义核经这里调用 Runtime 的算子表，不散落裸 `RuntimeValue` 调用。这样算子
//! 矩阵只有一处引用点：将来 Runtime 换实现、或某个机型需要不同的快速路径，
//! 都只改这一个文件。这里不做任何隐式宽度提升——那由后端在降低时插入显式转换。

use xiao_bytecode::research::{ArithOp, CompareOp};
use xiao_runtime::{RuntimeResult, RuntimeValue};

/// 执行一条三地址算术指令。
pub fn apply_arith(
    op: ArithOp,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    match op {
        ArithOp::Add => left.add(right),
        ArithOp::Subtract => left.subtract(right),
        ArithOp::Multiply => left.multiply(right),
        ArithOp::Divide => left.divide(right),
        ArithOp::FloorDivide => left.floor_divide(right),
        ArithOp::Remainder => left.remainder(right),
        ArithOp::Power => left.power(right),
    }
}

/// 执行一条三地址比较指令，结果恒为布尔。
pub fn apply_compare(
    op: CompareOp,
    left: &RuntimeValue,
    right: &RuntimeValue,
) -> RuntimeResult<RuntimeValue> {
    let result = match op {
        CompareOp::Less => left.less(right)?,
        CompareOp::LessEqual => left.less_equal(right)?,
        CompareOp::Greater => left.greater(right)?,
        CompareOp::GreaterEqual => left.greater_equal(right)?,
        CompareOp::Equal => left.equals(right),
        CompareOp::NotEqual => left.not_equals(right),
    };
    Ok(RuntimeValue::Bool(result))
}
