//! 字典表与字典列对象。
//!
//! 两者共用同一份载荷与句柄，只在 `kind` 上区分：
//!
//! - **字典表**对顺序**没有承诺**，键查找是主要访问方式。存储保持有序只是为了
//!   让构造与释放可复现，不构成语义顺序。
//! - **字典列**顺序稳定，源码书写顺序就是它的顺序。

use std::any::Any;

use crate::errors::RuntimeResult;
use crate::memory::{
    ObjectLayout, ObjectPayload, RuntimeTypeTag, StrongHandle, WeakHandle, allocate_payload,
};
use crate::value::RuntimeValue;

/// 字典对象的两种形态。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DictKind {
    /// 无序字典表。
    Table,
    /// 顺序稳定的字典列。
    Column,
}

impl DictKind {
    /// 返回稳定名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Table => "dict_table",
            Self::Column => "dict_column",
        }
    }

    /// 返回对应的对象类型标签。
    #[must_use]
    pub const fn type_tag(self) -> RuntimeTypeTag {
        match self {
            Self::Table => RuntimeTypeTag::DictTable,
            Self::Column => RuntimeTypeTag::DictColumn,
        }
    }
}

/// 字典载荷。
pub(crate) struct DictObject {
    /// 形态。
    kind: DictKind,
    /// 按键值对保存；顺序只用于可复现构造，字典表不把它当作语义。
    entries: Vec<(String, RuntimeValue)>,
}

impl DictObject {
    /// 按键查找值副本。
    fn value(&self, key: &str) -> Option<RuntimeValue> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
    }
}

impl ObjectPayload for DictObject {
    /// 返回字典标签；字典表与字典列各有独立标签。
    fn type_tag(&self) -> RuntimeTypeTag {
        self.kind.type_tag()
    }

    /// 返回字典载荷布局。
    fn layout(&self) -> ObjectLayout {
        ObjectLayout::for_type::<Self>(self.kind.type_tag())
    }

    /// 字典自身没有用户钩子；值由引用计数逐个释放。
    fn on_drop(&mut self) -> RuntimeResult<()> {
        self.entries.clear();
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

/// 不透明字典强句柄。
pub struct DictHandle {
    inner: StrongHandle,
}

impl std::fmt::Debug for DictHandle {
    /// 输出字典形态与条目数摘要，不展开全部值。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DictHandle")
            .field("kind", &self.kind().as_str())
            .field("length", &self.len())
            .finish()
    }
}

impl Clone for DictHandle {
    /// 克隆字典强句柄。
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl DictHandle {
    /// 从 Runtime 内部强句柄恢复字典包装；调用方必须已验证字典标签。
    pub(crate) fn from_strong_handle(inner: StrongHandle) -> RuntimeResult<Self> {
        if !matches!(
            inner.type_tag(),
            RuntimeTypeTag::DictTable | RuntimeTypeTag::DictColumn
        ) {
            return Err(crate::errors::RuntimeError::invalid_handle("句柄不是字典"));
        }
        Ok(Self { inner })
    }

    /// 分配一个字典对象。
    pub fn new(kind: DictKind, entries: Vec<(String, RuntimeValue)>) -> RuntimeResult<Self> {
        let inner = allocate_payload(Box::new(DictObject { kind, entries }))?;
        Ok(Self { inner })
    }

    /// 返回字典形态；载荷不可读时保守返回字典表。
    #[must_use]
    pub fn kind(&self) -> DictKind {
        self.inner
            .with_payload(RuntimeTypeTag::DictTable, |object: &DictObject| object.kind)
            .or_else(|_| {
                self.inner
                    .with_payload(RuntimeTypeTag::DictColumn, |object: &DictObject| {
                        object.kind
                    })
            })
            .unwrap_or(DictKind::Table)
    }

    /// 返回条目数量。
    #[must_use]
    pub fn len(&self) -> usize {
        self.with_entries(|entries| entries.len()).unwrap_or(0)
    }

    /// 判断字典是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 按键读取值副本；键不存在返回 `None`。
    pub fn value(&self, key: &str) -> RuntimeResult<Option<RuntimeValue>> {
        let kind = self.kind();
        self.inner
            .with_payload(kind.type_tag(), |object: &DictObject| object.value(key))
    }

    /// 按键判断条目是否存在。
    pub fn contains_key(&self, key: &str) -> RuntimeResult<bool> {
        let kind = self.kind();
        self.inner
            .with_payload(kind.type_tag(), |object: &DictObject| {
                object.entries.iter().any(|(name, _)| name == key)
            })
    }

    /// 以只读闭包访问全部条目，避免载荷引用逃逸。
    ///
    /// 字典表不承诺顺序，调用方不得把这里的迭代顺序当作语义。
    pub fn with_entries<R>(
        &self,
        callback: impl FnOnce(&[(String, RuntimeValue)]) -> R,
    ) -> RuntimeResult<R> {
        let kind = self.kind();
        self.inner
            .with_payload(kind.type_tag(), |object: &DictObject| {
                callback(&object.entries)
            })
    }

    /// 以可变闭包访问全部条目。
    pub fn with_entries_mut<R>(
        &self,
        callback: impl FnOnce(&mut Vec<(String, RuntimeValue)>) -> R,
    ) -> RuntimeResult<R> {
        let kind = self.kind();
        self.inner
            .with_payload_mut(kind.type_tag(), |object: &mut DictObject| {
                callback(&mut object.entries)
            })
    }

    /// 创建一个不拥有字典生命周期的弱句柄。
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

    /// 显式释放字典并返回最后一个 `drop` 错误。
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
