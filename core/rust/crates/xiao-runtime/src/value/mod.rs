//! Runtime 标量、字符串和统一值枚举。
//!
//! 静态标量尽量保持内联；只有 `str` 使用不透明堆对象。表句柄由 `tables`
//! 模块提供并作为统一值的一种可拥有变体接入。

use std::any::Any;

use xiao_syntax::ScalarType;

use crate::errors::{RuntimeError, RuntimeResult};
use crate::memory::{
    ObjectLayout, ObjectPayload, RuntimeTypeTag, StrongHandle, WeakHandle, allocate_payload,
};

/// Xiao `str` 的堆载荷。
#[derive(Debug)]
pub(crate) struct StringObject {
    /// UTF-8 字符串内容。
    pub value: String,
}

impl ObjectPayload for StringObject {
    /// 返回字符串标签。
    fn type_tag(&self) -> RuntimeTypeTag {
        RuntimeTypeTag::String
    }

    /// 返回字符串载荷布局。
    fn layout(&self) -> ObjectLayout {
        ObjectLayout::for_type::<Self>(RuntimeTypeTag::String)
    }

    /// 字符串没有用户 `drop` 钩子。
    fn on_drop(&mut self) -> RuntimeResult<()> {
        Ok(())
    }

    /// 暴露只读 `Any` 视图。
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// 暴露可变 `Any` 视图。
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 不透明字符串强句柄。
pub struct StringHandle {
    inner: StrongHandle,
}

impl std::fmt::Debug for StringHandle {
    /// 输出字符串句柄摘要，不泄漏文本内容。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StringHandle")
            .field("length", &self.len())
            .finish()
    }
}

impl Clone for StringHandle {
    /// 克隆字符串强句柄。
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl StringHandle {
    /// 分配一个 UTF-8 字符串对象。
    pub fn new(value: impl Into<String>) -> RuntimeResult<Self> {
        let inner = allocate_payload(Box::new(StringObject {
            value: value.into(),
        }))?;
        Ok(Self { inner })
    }

    /// 返回字符串内容的拥有副本。
    pub fn to_string(&self) -> RuntimeResult<String> {
        self.inner
            .with_payload(RuntimeTypeTag::String, |object: &StringObject| {
                object.value.clone()
            })
    }

    /// 返回 Unicode 标量数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .with_payload(RuntimeTypeTag::String, |object: &StringObject| {
                object.value.chars().count()
            })
            .unwrap_or(0)
    }

    /// 判断字符串是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 以只读闭包访问字符串，避免把对象头引用泄漏到 Runtime 外部。
    pub fn with_str<R>(&self, callback: impl FnOnce(&str) -> R) -> RuntimeResult<R> {
        self.inner
            .with_payload(RuntimeTypeTag::String, |object: &StringObject| {
                callback(&object.value)
            })
    }

    /// 创建不拥有字符串生命周期的弱句柄。
    #[must_use]
    pub fn downgrade(&self) -> WeakHandle {
        self.inner.downgrade()
    }

    /// 返回当前强引用计数。
    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.inner.strong_count()
    }

    /// 返回底层对象类型标签。
    #[must_use]
    pub fn type_tag(&self) -> RuntimeTypeTag {
        self.inner.type_tag()
    }

    /// 显式释放字符串并返回最后一个 `drop` 错误。
    pub fn try_release(self) -> RuntimeResult<()> {
        self.inner.try_release()
    }

    /// 消耗字符串包装并取出底层强句柄，供 Runtime/后端适配层使用。
    pub fn into_strong_handle(self) -> StrongHandle {
        self.inner
    }
}

/// Xiao Runtime 中可传递的最小值集合。
#[derive(Clone, Debug)]
pub enum RuntimeValue {
    /// 默认 64 位整数。
    Int(i64),
    /// 32 位整数。
    Sint(i32),
    /// 可扩展宽度整数的规范十进制文本。
    Lint(String),
    /// 默认 64 位浮点。
    Float(f64),
    /// 32 位浮点。
    Sfloat(f32),
    /// 可扩展精度浮点的规范文本。
    Lfloat(String),
    /// 布尔值。
    Bool(bool),
    /// 堆字符串。
    Str(StringHandle),
    /// 表对象；实际类型在 `TableInstance` 中校验。
    Table(crate::tables::TableInstance),
    /// 空值。
    None,
}

impl PartialEq for RuntimeValue {
    /// 按 Xiao 值语义比较已实现的标量、字符串和表身份。
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Sint(left), Self::Sint(right)) => left == right,
            (Self::Lint(left), Self::Lint(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left.to_bits() == right.to_bits(),
            (Self::Sfloat(left), Self::Sfloat(right)) => left.to_bits() == right.to_bits(),
            (Self::Lfloat(left), Self::Lfloat(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Str(left), Self::Str(right)) => left.to_string().ok() == right.to_string().ok(),
            (Self::Table(left), Self::Table(right)) => left.same_object(right),
            (Self::None, Self::None) => true,
            _ => false,
        }
    }
}

impl RuntimeValue {
    /// 返回对应的静态标量类型；表和空值返回 `None`。
    #[must_use]
    pub const fn scalar_type(&self) -> Option<ScalarType> {
        Some(match self {
            Self::Int(_) => ScalarType::Int,
            Self::Sint(_) => ScalarType::Sint,
            Self::Lint(_) => ScalarType::Lint,
            Self::Float(_) => ScalarType::Float,
            Self::Sfloat(_) => ScalarType::Sfloat,
            Self::Lfloat(_) => ScalarType::Lfloat,
            Self::Bool(_) => ScalarType::Bool,
            Self::Str(_) => ScalarType::Str,
            Self::Table(_) | Self::None => return None,
        })
    }

    /// 返回稳定的 Runtime 类型名称。
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::Table(instance) => format!("table {}", instance.name()),
            Self::None => "none".to_owned(),
            _ => self
                .scalar_type()
                .map_or_else(|| "dynamic".to_owned(), |ty| ty.as_str().to_owned()),
        }
    }

    /// 读取布尔值；其他类型返回 `None`。
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// 对布尔值执行严格的 `bool ± 整数` 运算。
    pub fn bool_adjust(&self, amount: i128) -> RuntimeResult<Self> {
        let Self::Bool(value) = self else {
            return Err(RuntimeError::type_mismatch("bool", self.type_name()));
        };
        let parity = amount.unsigned_abs() % 2 == 1;
        Ok(Self::Bool(if parity { !value } else { *value }))
    }

    /// 执行首版已定义的加法，包括字符串拼接和布尔整数调整。
    pub fn add(&self, other: &Self) -> RuntimeResult<Self> {
        self.numeric_binary(other, false)
    }

    /// 执行首版已定义的减法，包括布尔整数调整。
    pub fn subtract(&self, other: &Self) -> RuntimeResult<Self> {
        self.numeric_binary(other, true)
    }

    /// 执行标量二元运算并统一处理布尔奇偶规则与溢出。
    fn numeric_binary(&self, other: &Self, subtract: bool) -> RuntimeResult<Self> {
        if let Self::Bool(value) = self {
            let amount = integer_value(other)
                .ok_or_else(|| RuntimeError::type_mismatch("int", other.type_name()))?;
            let parity = amount.unsigned_abs() % 2 == 1;
            return Ok(Self::Bool(if parity { !value } else { *value }));
        }
        match (self, other) {
            (Self::Int(left), Self::Int(right)) => {
                let result = if subtract {
                    left.checked_sub(*right)
                } else {
                    left.checked_add(*right)
                };
                result
                    .map(Self::Int)
                    .ok_or_else(|| RuntimeError::numeric_overflow("int 运算溢出"))
            }
            (Self::Sint(left), Self::Sint(right)) => {
                let result = if subtract {
                    left.checked_sub(*right)
                } else {
                    left.checked_add(*right)
                };
                result
                    .map(Self::Sint)
                    .ok_or_else(|| RuntimeError::numeric_overflow("sint 运算溢出"))
            }
            (Self::Float(left), Self::Float(right)) => {
                let result = if subtract {
                    *left - *right
                } else {
                    *left + *right
                };
                result
                    .is_finite()
                    .then_some(Self::Float(result))
                    .ok_or_else(|| RuntimeError::numeric_overflow("float 运算产生非有限值"))
            }
            (Self::Sfloat(left), Self::Sfloat(right)) => {
                let result = if subtract {
                    *left - *right
                } else {
                    *left + *right
                };
                result
                    .is_finite()
                    .then_some(Self::Sfloat(result))
                    .ok_or_else(|| RuntimeError::numeric_overflow("sfloat 运算产生非有限值"))
            }
            (Self::Str(left), Self::Str(right)) if !subtract => {
                let mut value = left.to_string()?;
                value.push_str(&right.to_string()?);
                Self::new_string(value)
            }
            _ => Err(RuntimeError::invalid_value("当前 Runtime 不支持这组运算")),
        }
    }

    /// 从字符串创建 `RuntimeValue::Str`。
    pub fn new_string(value: impl Into<String>) -> RuntimeResult<Self> {
        Ok(Self::Str(StringHandle::new(value)?))
    }
}

/// 提取可参与布尔调整的整数值。
fn integer_value(value: &RuntimeValue) -> Option<i128> {
    match value {
        RuntimeValue::Int(value) => Some(i128::from(*value)),
        RuntimeValue::Sint(value) => Some(i128::from(*value)),
        RuntimeValue::Lint(value) => value.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
/// 标量运算和 UTF-8 字符串句柄的回归测试。
mod tests {
    use super::{RuntimeValue, StringHandle};

    #[test]
    /// 验证布尔值按整数奇偶翻转。
    fn supports_bool_parity_adjustment() {
        assert_eq!(
            RuntimeValue::Bool(false).add(&RuntimeValue::Int(1)),
            Ok(RuntimeValue::Bool(true))
        );
        assert_eq!(
            RuntimeValue::Bool(true).subtract(&RuntimeValue::Sint(2)),
            Ok(RuntimeValue::Bool(true))
        );
    }

    #[test]
    /// 验证字符串载荷保留 UTF-8 内容。
    fn stores_and_reads_utf8_string() {
        let value = StringHandle::new("小雪").expect("字符串应分配");
        assert_eq!(value.len(), 2);
        assert_eq!(value.to_string().expect("应读取"), "小雪");
    }
}
