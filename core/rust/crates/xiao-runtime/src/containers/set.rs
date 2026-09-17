//! 集合对象。
//!
//! 集合的元素唯一、无序。本模块用**有序 `Vec` 加线性去重**而不是哈希表：
//! 去重结果必须确定，哈希序会污染语义向量里的释放序列，而当前集合规模很小。
//! 元素必须是可哈希值，判定见 [`super::is_hashable`]。

use std::any::Any;

use crate::containers::{deduplicate, is_hashable};
use crate::errors::{RuntimeError, RuntimeResult};
use crate::memory::{
    ObjectLayout, ObjectPayload, RuntimeTypeTag, StrongHandle, WeakHandle, allocate_payload,
};
use crate::value::RuntimeValue;

/// 集合载荷。
pub(crate) struct SetObject {
    /// 去重后按首次出现顺序保存的元素。
    elements: Vec<RuntimeValue>,
}

impl ObjectPayload for SetObject {
    /// 返回集合标签。
    fn type_tag(&self) -> RuntimeTypeTag {
        RuntimeTypeTag::Set
    }

    /// 返回集合载荷布局。
    fn layout(&self) -> ObjectLayout {
        ObjectLayout::for_type::<Self>(RuntimeTypeTag::Set)
    }

    /// 集合自身没有用户钩子；元素由引用计数逐个释放。
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

/// 不透明集合强句柄。
pub struct SetHandle {
    inner: StrongHandle,
}

impl std::fmt::Debug for SetHandle {
    /// 输出集合大小摘要，不展开全部元素。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SetHandle")
            .field("length", &self.len())
            .finish()
    }
}

impl Clone for SetHandle {
    /// 克隆集合强句柄。
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl SetHandle {
    /// 分配一个集合对象；不可哈希元素被拒绝。
    pub fn new(elements: Vec<RuntimeValue>) -> RuntimeResult<Self> {
        if let Some(offender) = elements.iter().find(|value| !is_hashable(value)) {
            return Err(RuntimeError::unhashable_element(offender.type_name()));
        }
        let inner = allocate_payload(Box::new(SetObject {
            elements: deduplicate(elements),
        }))?;
        Ok(Self { inner })
    }

    /// 返回去重后的元素数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .with_payload(RuntimeTypeTag::Set, |object: &SetObject| {
                object.elements.len()
            })
            .unwrap_or(0)
    }

    /// 判断集合是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 判断集合是否包含某个值。
    pub fn contains(&self, value: &RuntimeValue) -> RuntimeResult<bool> {
        self.inner
            .with_payload(RuntimeTypeTag::Set, |object: &SetObject| {
                object.elements.contains(value)
            })
    }

    /// 以只读闭包访问全部元素，避免载荷引用逃逸。
    ///
    /// 集合无序；这里的迭代顺序只是构造顺序，调用方不得当作语义。
    pub fn with_elements<R>(
        &self,
        callback: impl FnOnce(&[RuntimeValue]) -> R,
    ) -> RuntimeResult<R> {
        self.inner
            .with_payload(RuntimeTypeTag::Set, |object: &SetObject| {
                callback(&object.elements)
            })
    }

    /// 创建一个不拥有集合生命周期的弱句柄。
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

    /// 显式释放集合并返回最后一个 `drop` 错误。
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
