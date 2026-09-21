//! 元组对象。
//!
//! 元组是有序、可重复、位置固定的容器。**解释器把全栈元组也物化成堆对象**，
//! 而静态存储类别说它是栈值；这条差异已在 09R 交接文档登记，不是缺陷。

use std::any::Any;

use crate::errors::RuntimeResult;
use crate::memory::{
    ObjectLayout, ObjectPayload, RuntimeTypeTag, StrongHandle, WeakHandle, allocate_payload,
};
use crate::value::RuntimeValue;

/// 元组载荷。
pub(crate) struct TupleObject {
    /// 按位置保存的元素。
    elements: Vec<RuntimeValue>,
}

impl ObjectPayload for TupleObject {
    /// 返回元组标签。
    fn type_tag(&self) -> RuntimeTypeTag {
        RuntimeTypeTag::Tuple
    }

    /// 返回元组载荷布局。
    fn layout(&self) -> ObjectLayout {
        ObjectLayout::for_type::<Self>(RuntimeTypeTag::Tuple)
    }

    /// 元组自身没有用户钩子；元素由引用计数逐个释放。
    fn on_drop(&mut self) -> RuntimeResult<()> {
        self.elements.clear();
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

/// 不透明元组强句柄。
pub struct TupleHandle {
    inner: StrongHandle,
}

impl std::fmt::Debug for TupleHandle {
    /// 输出元组长度摘要，不展开全部元素。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TupleHandle")
            .field("length", &self.len())
            .finish()
    }
}

impl Clone for TupleHandle {
    /// 克隆元组强句柄。
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl TupleHandle {
    /// 从 Runtime 内部强句柄恢复元组包装；调用方必须已验证类型标签。
    pub(crate) fn from_strong_handle(inner: StrongHandle) -> RuntimeResult<Self> {
        if inner.type_tag() != RuntimeTypeTag::Tuple {
            return Err(crate::errors::RuntimeError::invalid_handle("句柄不是元组"));
        }
        Ok(Self { inner })
    }

    /// 分配一个元组对象。
    pub fn new(elements: Vec<RuntimeValue>) -> RuntimeResult<Self> {
        let inner = allocate_payload(Box::new(TupleObject { elements }))?;
        Ok(Self { inner })
    }

    /// 返回元素数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .with_payload(RuntimeTypeTag::Tuple, |object: &TupleObject| {
                object.elements.len()
            })
            .unwrap_or(0)
    }

    /// 判断元组是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 按零基位置读取元素副本；越界返回 `None`。
    pub fn element(&self, index: usize) -> RuntimeResult<Option<RuntimeValue>> {
        self.inner
            .with_payload(RuntimeTypeTag::Tuple, |object: &TupleObject| {
                object.elements.get(index).cloned()
            })
    }

    /// 以只读闭包访问全部元素，避免载荷引用逃逸。
    pub fn with_elements<R>(
        &self,
        callback: impl FnOnce(&[RuntimeValue]) -> R,
    ) -> RuntimeResult<R> {
        self.inner
            .with_payload(RuntimeTypeTag::Tuple, |object: &TupleObject| {
                callback(&object.elements)
            })
    }

    /// 以可变闭包访问全部元素。
    pub fn with_elements_mut<R>(
        &self,
        callback: impl FnOnce(&mut Vec<RuntimeValue>) -> R,
    ) -> RuntimeResult<R> {
        self.inner
            .with_payload_mut(RuntimeTypeTag::Tuple, |object: &mut TupleObject| {
                callback(&mut object.elements)
            })
    }

    /// 创建一个不拥有元组生命周期的弱句柄。
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

    /// 显式释放元组并返回最后一个 `drop` 错误。
    pub fn try_release(self) -> RuntimeResult<()> {
        self.inner.try_release()
    }

    /// 消耗包装并取出底层强句柄，供 Runtime/后端适配层使用。
    pub fn into_strong_handle(self) -> StrongHandle {
        self.inner
    }

    /// 判断两个句柄是否引用同一个对象头。
    #[must_use]
    pub fn same_object(&self, other: &Self) -> bool {
        self.inner.same_object(&other.inner)
    }
}
