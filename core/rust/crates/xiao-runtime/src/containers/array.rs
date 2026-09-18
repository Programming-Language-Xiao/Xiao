//! 数组对象。
//!
//! 数组是有序、可重复、按位置索引的容器。本模块只保存元素与读取，不含任何
//! 选择器语义。

use std::any::Any;

use crate::errors::RuntimeResult;
use crate::memory::{
    ObjectLayout, ObjectPayload, RuntimeTypeTag, StrongHandle, WeakHandle, allocate_payload,
};
use crate::value::RuntimeValue;

/// 数组载荷。
pub(crate) struct ArrayObject {
    /// 按位置保存的元素。
    elements: Vec<RuntimeValue>,
}

impl ObjectPayload for ArrayObject {
    /// 返回数组标签。
    fn type_tag(&self) -> RuntimeTypeTag {
        RuntimeTypeTag::Array
    }

    /// 返回数组载荷布局。
    fn layout(&self) -> ObjectLayout {
        ObjectLayout::for_type::<Self>(RuntimeTypeTag::Array)
    }

    /// 数组自身没有用户钩子；元素由引用计数逐个释放。
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

/// 不透明数组强句柄。
pub struct ArrayHandle {
    inner: StrongHandle,
}

impl std::fmt::Debug for ArrayHandle {
    /// 输出数组长度摘要，不展开全部元素。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArrayHandle")
            .field("length", &self.len())
            .finish()
    }
}

impl Clone for ArrayHandle {
    /// 克隆数组强句柄。
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl ArrayHandle {
    /// 分配一个数组对象。
    pub fn new(elements: Vec<RuntimeValue>) -> RuntimeResult<Self> {
        let inner = allocate_payload(Box::new(ArrayObject { elements }))?;
        Ok(Self { inner })
    }

    /// 返回元素数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .with_payload(RuntimeTypeTag::Array, |object: &ArrayObject| {
                object.elements.len()
            })
            .unwrap_or(0)
    }

    /// 判断数组是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 按零基位置读取元素副本；越界返回 `None`。
    ///
    /// 返回副本会持有一次强引用，调用方负责其生命周期；位置归一化与越界
    /// 诊断由调用方决定，本方法只做位置访问。
    pub fn element(&self, index: usize) -> RuntimeResult<Option<RuntimeValue>> {
        self.inner
            .with_payload(RuntimeTypeTag::Array, |object: &ArrayObject| {
                object.elements.get(index).cloned()
            })
    }

    /// 以只读闭包访问全部元素，避免载荷引用逃逸。
    pub fn with_elements<R>(
        &self,
        callback: impl FnOnce(&[RuntimeValue]) -> R,
    ) -> RuntimeResult<R> {
        self.inner
            .with_payload(RuntimeTypeTag::Array, |object: &ArrayObject| {
                callback(&object.elements)
            })
    }

    /// 以可变闭包访问全部元素。
    pub fn with_elements_mut<R>(
        &self,
        callback: impl FnOnce(&mut Vec<RuntimeValue>) -> R,
    ) -> RuntimeResult<R> {
        self.inner
            .with_payload_mut(RuntimeTypeTag::Array, |object: &mut ArrayObject| {
                callback(&mut object.elements)
            })
    }

    /// 创建一个不拥有数组生命周期的弱句柄。
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

    /// 显式释放数组并返回最后一个 `drop` 错误。
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
