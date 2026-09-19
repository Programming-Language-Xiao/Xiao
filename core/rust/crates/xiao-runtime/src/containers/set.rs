//! 集合对象。
//!
//! 集合的元素唯一、无序。本模块用**有序 `Vec` 加线性去重**而不是哈希表：
//! 去重结果必须确定，哈希序会污染语义向量里的释放序列，而当前集合规模很小。
//! 元素必须是可哈希值，判定见 [`super::is_hashable`]。
//!
//! # 成员判定与去重
//!
//! 一律走 [`RuntimeValue`] 的 `PartialEq` + 有序 `Vec`，**不得为实现集合代数而引入
//! 哈希索引**。哈希索引会把元素顺序变成实现细节，而顺序是共享向量的可观察期望值；
//! 上面「哈希序会污染释放序列」正是同一条理由。
//!
//! # 运算结果顺序
//!
//! 代数运算的结果顺序**只依赖操作数，不依赖哈希、不依赖排序**：
//!
//! | 运算 | 顺序 |
//! | --- | --- |
//! | 并集 | 左侧原序，随后右侧中不在左侧者按右侧原序 |
//! | 交集 / 差集 | 左侧原序过滤 |
//! | 对称差 | 左侧独有按左侧序，随后右侧独有按右侧序 |
//! | 六种比较 | 与顺序无关 |
//!
//! `RuntimeValue` 没有全序，引入排序等于新增一个待冻结契约，因此这里不排序。
//! **后续若把集合接进 `for` 之类的可观察路径，不得为了让输出好看而改这里的顺序**
//! ——改了要连带改共享向量，那是掩盖而不是修复。

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

    /// 取出元素快照，供两个句柄之间的运算使用。
    ///
    /// 快照而不是嵌套借用：两个句柄的载荷借用关系在编译期无从表达，
    /// 而集合规模按模块文档约定很小，克隆的代价可忽略。
    fn elements(&self) -> RuntimeResult<Vec<RuntimeValue>> {
        self.with_elements(<[RuntimeValue]>::to_vec)
    }

    /// 返回两个集合的并集。
    ///
    /// 顺序见模块文档的运算结果顺序：左侧原序，随后右侧中不在左侧者按右侧原序。
    pub fn union(&self, other: &Self) -> RuntimeResult<Self> {
        let left = self.elements()?;
        let right = other.elements()?;
        let mut merged = left.clone();
        merged.extend(right.iter().filter(|value| !left.contains(value)).cloned());
        Self::new(merged)
    }

    /// 返回两个集合的交集，按左侧原序。
    pub fn intersection(&self, other: &Self) -> RuntimeResult<Self> {
        let left = self.elements()?;
        let right = other.elements()?;
        Self::new(
            left.iter()
                .filter(|value| right.contains(value))
                .cloned()
                .collect(),
        )
    }

    /// 返回差集（左侧独有），按左侧原序。
    pub fn difference(&self, other: &Self) -> RuntimeResult<Self> {
        let left = self.elements()?;
        let right = other.elements()?;
        Self::new(
            left.iter()
                .filter(|value| !right.contains(value))
                .cloned()
                .collect(),
        )
    }

    /// 返回对称差，按左侧独有、随后右侧独有的顺序。
    pub fn symmetric_difference(&self, other: &Self) -> RuntimeResult<Self> {
        let left = self.elements()?;
        let right = other.elements()?;
        let mut merged: Vec<RuntimeValue> = left
            .iter()
            .filter(|value| !right.contains(value))
            .cloned()
            .collect();
        merged.extend(right.iter().filter(|value| !left.contains(value)).cloned());
        Self::new(merged)
    }

    /// 判断两个集合是否相等。
    ///
    /// 相等是**无序双向包含**，既不是句柄身份，也不是元素序列逐位相等：
    /// 集合的物理表示是有序 `Vec`，逐位比较会把 `{1, 2}` 与 `{2, 1}` 判成不等。
    pub fn equals(&self, other: &Self) -> RuntimeResult<bool> {
        let left = self.elements()?;
        let right = other.elements()?;
        Ok(left.len() == right.len() && left.iter().all(|value| right.contains(value)))
    }

    /// 判断 `self` 是否为 `other` 的子集（允许两者相等）。
    pub fn is_subset(&self, other: &Self) -> RuntimeResult<bool> {
        let left = self.elements()?;
        let right = other.elements()?;
        Ok(left.iter().all(|value| right.contains(value)))
    }

    /// 判断 `self` 是否为 `other` 的真子集。
    ///
    /// 真子集要求包含**且**不相等；与 [`Self::is_subset`] 共用同一个判断会让
    /// `{1, 2} < {1, 2}` 判成真。先比长度即可短路，也顺带保证了不相等。
    pub fn is_proper_subset(&self, other: &Self) -> RuntimeResult<bool> {
        let left = self.elements()?;
        let right = other.elements()?;
        Ok(left.len() < right.len() && left.iter().all(|value| right.contains(value)))
    }

    /// 判断 `self` 是否为 `other` 的超集（允许两者相等）。
    pub fn is_superset(&self, other: &Self) -> RuntimeResult<bool> {
        other.is_subset(self)
    }

    /// 判断 `self` 是否为 `other` 的真超集。
    pub fn is_proper_superset(&self, other: &Self) -> RuntimeResult<bool> {
        other.is_proper_subset(self)
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
