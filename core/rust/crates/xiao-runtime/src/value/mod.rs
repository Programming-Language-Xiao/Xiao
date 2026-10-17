//! Runtime 标量、字符串和统一值枚举。
//!
//! 静态标量尽量保持内联；只有 `str` 使用不透明堆对象。表句柄由 `tables`
//! 模块提供并作为统一值的一种可拥有变体接入。本模块只定义值的表示和读取，
//! 算术、比较、相等和显式转换统一放在子模块 `ops`，避免算子矩阵散落。

/// Runtime 唯一的算子表：算术、比较、相等和显式转换。
mod ops;

use std::any::Any;
use std::cell::Cell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use xiao_syntax::ScalarType;

use crate::errors::{RuntimeError, RuntimeResult, XiaoError};
use crate::memory::{
    ObjectLayout, ObjectPayload, RuntimeTypeTag, StrongHandle, WeakHandle, allocate_payload,
};
use crate::resources::io_error as resource_io_error;

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

    /// 计入字符串拥有的 UTF-8 字节缓冲区。
    fn owned_bytes(&self) -> usize {
        self.value.capacity()
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
    /// 从 Runtime 内部强句柄恢复字符串包装；调用方必须已验证类型标签。
    pub(crate) fn from_strong_handle(inner: StrongHandle) -> RuntimeResult<Self> {
        if inner.type_tag() != RuntimeTypeTag::String {
            return Err(crate::errors::RuntimeError::invalid_handle(
                "句柄不是字符串",
            ));
        }
        Ok(Self { inner })
    }

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

/// 首批外部资源的稳定种类标签。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExternalResourceKind {
    /// 文件资源。
    File,
    /// 目录资源。
    Directory,
}

impl ExternalResourceKind {
    /// 返回语言可见的稳定种类名。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
        }
    }

    /// 返回资源在类型名和显示值中共用的稳定文本。
    #[must_use]
    pub fn display_name(self) -> String {
        format!("resource<{}>", self.as_str())
    }
}

pub(crate) const EXTERNAL_RESOURCE_RUNTIME_TAG: RuntimeTypeTag = RuntimeTypeTag::Custom(17);

/// 外部资源的宿主载荷；目录首版只保存已验证路径，不持有平台目录句柄。
#[derive(Debug)]
enum ExternalResourcePayload {
    /// 尚未绑定宿主对象的承载（仅供测试和 ABI 形状使用）。
    Empty,
    /// 已打开的文件。
    File(File),
    /// 已验证的目录路径。
    Directory(PathBuf),
}

/// 外部资源的内部载荷。
#[derive(Debug)]
struct ExternalResourceObject {
    kind: ExternalResourceKind,
    closed: Cell<bool>,
    payload: Option<ExternalResourcePayload>,
}

impl crate::memory::ObjectPayload for ExternalResourceObject {
    /// 返回外部资源的内部 Runtime 标签。
    fn type_tag(&self) -> RuntimeTypeTag {
        EXTERNAL_RESOURCE_RUNTIME_TAG
    }

    /// 返回载荷布局摘要。
    fn layout(&self) -> crate::memory::ObjectLayout {
        crate::memory::ObjectLayout::for_type::<Self>(EXTERNAL_RESOURCE_RUNTIME_TAG)
    }

    /// 目录保存的路径缓冲区也属于资源载荷的内存占用。
    fn owned_bytes(&self) -> usize {
        match self.payload.as_ref() {
            Some(ExternalResourcePayload::Directory(path)) => path.capacity(),
            _ => 0,
        }
    }

    /// 释放资源宿主载荷并把状态置为关闭；重复释放不产生第二次动作。
    fn on_drop(&mut self) -> RuntimeResult<()> {
        self.closed.set(true);
        let _ = self.payload.take();
        Ok(())
    }

    /// 暴露只读载荷视图。
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// 暴露可变载荷视图。
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 统一外部资源的强句柄；首版保持单线程、不实现 `Send`/`Sync`。
pub struct ExternalResourceHandle {
    inner: StrongHandle,
}

impl std::fmt::Debug for ExternalResourceHandle {
    /// 输出资源种类和引用计数，不泄漏宿主句柄。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExternalResourceHandle")
            .field("kind", &self.kind().map(ExternalResourceKind::as_str))
            .field("strong_count", &self.strong_count())
            .finish()
    }
}

impl Clone for ExternalResourceHandle {
    /// 复制同一个逻辑资源的强引用。
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl ExternalResourceHandle {
    /// 创建一个首批种类的空资源承载对象。
    pub fn new(kind: ExternalResourceKind) -> RuntimeResult<Self> {
        let inner = allocate_payload(Box::new(ExternalResourceObject {
            kind,
            closed: Cell::new(false),
            payload: Some(ExternalResourcePayload::Empty),
        }))?;
        Ok(Self { inner })
    }

    /// 从已打开的文件创建资源承载。
    pub fn from_file(file: File) -> RuntimeResult<Self> {
        let inner = allocate_payload(Box::new(ExternalResourceObject {
            kind: ExternalResourceKind::File,
            closed: Cell::new(false),
            payload: Some(ExternalResourcePayload::File(file)),
        }))?;
        Ok(Self { inner })
    }

    /// 从已验证的目录路径创建资源承载。
    pub fn from_directory(path: PathBuf) -> RuntimeResult<Self> {
        let inner = allocate_payload(Box::new(ExternalResourceObject {
            kind: ExternalResourceKind::Directory,
            closed: Cell::new(false),
            payload: Some(ExternalResourcePayload::Directory(path)),
        }))?;
        Ok(Self { inner })
    }

    /// 从已验证的强句柄恢复资源包装。
    pub(crate) fn from_strong_handle(inner: StrongHandle) -> RuntimeResult<Self> {
        if inner.type_tag() != EXTERNAL_RESOURCE_RUNTIME_TAG {
            return Err(crate::errors::RuntimeError::invalid_handle(
                "句柄不是外部资源",
            ));
        }
        Ok(Self { inner })
    }

    /// 返回资源种类。
    pub fn kind(&self) -> RuntimeResult<ExternalResourceKind> {
        self.inner.with_payload(
            EXTERNAL_RESOURCE_RUNTIME_TAG,
            |object: &ExternalResourceObject| object.kind,
        )
    }

    /// 判断资源是否已经关闭。
    pub fn is_closed(&self) -> RuntimeResult<bool> {
        self.inner.with_payload(
            EXTERNAL_RESOURCE_RUNTIME_TAG,
            |object: &ExternalResourceObject| object.closed.get(),
        )
    }

    /// 显式关闭资源；重复关闭幂等。
    pub fn close(&self) -> RuntimeResult<()> {
        self.inner.with_payload_mut(
            EXTERNAL_RESOURCE_RUNTIME_TAG,
            |object: &mut ExternalResourceObject| {
                object.closed.set(true);
                let _ = object.payload.take();
            },
        )
    }

    /// 要求资源仍然可用；关闭或失效时返回稳定生命周期错误。
    pub fn ensure_open(&self) -> RuntimeResult<()> {
        if self.is_closed()? {
            Err(crate::errors::RuntimeError::resource_closed())
        } else {
            Ok(())
        }
    }

    /// 读取文件的完整 UTF-8 文本；目录或空承载会返回稳定 Runtime 错误。
    pub fn read_text(&self) -> RuntimeResult<String> {
        self.ensure_open()?;
        self.inner.with_payload_mut(
            EXTERNAL_RESOURCE_RUNTIME_TAG,
            |object: &mut ExternalResourceObject| {
                let Some(payload) = object.payload.as_mut() else {
                    return Err(RuntimeError::invalid_value("资源没有可读取的宿主载荷"));
                };
                let ExternalResourcePayload::File(file) = payload else {
                    return Err(RuntimeError::invalid_value("directory 资源不能读取文本"));
                };
                file.seek(SeekFrom::Start(0))
                    .map_err(|error| resource_io_error("定位文件", error))?;
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)
                    .map_err(|error| resource_io_error("读取文件", error))?;
                String::from_utf8(bytes)
                    .map_err(|error| resource_io_error("解码文件", error.utf8_error()))
            },
        )?
    }

    /// 返回强引用计数。
    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.inner.strong_count()
    }

    /// 判断两个包装是否指向同一逻辑资源。
    #[must_use]
    pub fn same_resource(&self, other: &Self) -> bool {
        self.inner.same_object(&other.inner)
    }

    /// 消耗包装并显式释放强引用。
    pub fn try_release(self) -> RuntimeResult<()> {
        self.inner.try_release()
    }

    /// 消耗包装并取得 ABI 不透明强句柄。
    pub(crate) fn into_strong_handle(self) -> StrongHandle {
        self.inner
    }
}

/// Xiao Runtime 中可传递的最小值集合。
#[derive(Clone, Debug)]
pub enum RuntimeValue {
    /// 尚未执行或已经缓存的模块命名空间身份。
    Module(String),
    /// 已初始化模块中的函数引用与函数表索引。
    ModuleFunction(String, u32),
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
    /// 析构回调中的弱只读接收者，不拥有原表对象。
    TableDropView(crate::tables::TableDropView),
    /// 数组对象。
    Array(crate::containers::ArrayHandle),
    /// 元组对象。
    Tuple(crate::containers::TupleHandle),
    /// 无序字典表对象。
    DictTable(crate::containers::DictHandle),
    /// 顺序稳定的字典列对象。
    DictColumn(crate::containers::DictHandle),
    /// 集合对象。
    Set(crate::containers::SetHandle),
    /// 外部资源对象；首批种类由 `ExternalResourceKind` 区分。
    ExternalResource(ExternalResourceHandle),
    /// 可恢复错误对象；错误身份由 `XiaoError::error_id` 保持。
    Error(Box<XiaoError>),
    /// 空值。
    None,
}

impl PartialEq for RuntimeValue {
    /// 按 Xiao 值语义比较已实现的标量、字符串和表身份。
    ///
    /// `str` 走句柄的只读读取路径逐个比较内容，不产生副本；任一侧已经释放时
    /// 保守判为不相等，而不是把两个「读不到内容」的句柄当成相等。
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Module(left), Self::Module(right)) => left == right,
            (
                Self::ModuleFunction(left_name, left_id),
                Self::ModuleFunction(right_name, right_id),
            ) => left_name == right_name && left_id == right_id,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Sint(left), Self::Sint(right)) => left == right,
            (Self::Lint(left), Self::Lint(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left.to_bits() == right.to_bits(),
            (Self::Sfloat(left), Self::Sfloat(right)) => left.to_bits() == right.to_bits(),
            (Self::Lfloat(left), Self::Lfloat(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Str(left), Self::Str(right)) => left
                .with_str(|left| right.with_str(|right| left == right))
                .and_then(|equal| equal)
                .unwrap_or(false),
            (Self::Table(left), Self::Table(right)) => left.same_object(right),
            (Self::TableDropView(left), Self::TableDropView(right)) => left.same_object(right),
            (Self::Array(left), Self::Array(right)) => left.same_object(right),
            (Self::Tuple(left), Self::Tuple(right)) => left.same_object(right),
            (Self::DictTable(left), Self::DictTable(right)) => left.same_object(right),
            (Self::DictColumn(left), Self::DictColumn(right)) => left.same_object(right),
            (Self::Set(left), Self::Set(right)) => left.same_object(right),
            (Self::ExternalResource(left), Self::ExternalResource(right)) => {
                left.kind() == right.kind() && left.same_resource(right)
            }
            (Self::Error(left), Self::Error(right)) => left == right,
            (Self::None, Self::None) => true,
            _ => false,
        }
    }
}

impl Eq for RuntimeValue {}

impl std::hash::Hash for RuntimeValue {
    /// 按与 [`PartialEq`] 自洽的口径哈希。
    ///
    /// 浮点走 `to_bits()`，与相等比较完全一致；`str` 按内容哈希，与内容相等
    /// 一致。表与容器按**对象身份**相等，但对象头地址不可得，因此这里只哈希
    /// 判别式——不相等的值允许哈希相同，符合 `Hash`/`Eq` 契约。它们本来就被
    /// 判为不可哈希，不会成为集合元素或字典键。
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Module(name) => name.hash(state),
            Self::ModuleFunction(name, index) => {
                name.hash(state);
                index.hash(state);
            }
            Self::Int(value) => value.hash(state),
            Self::Sint(value) => value.hash(state),
            Self::Lint(value) => value.hash(state),
            Self::Float(value) => value.to_bits().hash(state),
            Self::Sfloat(value) => value.to_bits().hash(state),
            Self::Lfloat(value) => value.hash(state),
            Self::Bool(value) => value.hash(state),
            Self::Str(handle) => {
                let _ = handle.with_str(|text| text.hash(state));
            }
            Self::Table(_)
            | Self::TableDropView(_)
            | Self::Array(_)
            | Self::Tuple(_)
            | Self::DictTable(_)
            | Self::DictColumn(_)
            | Self::Set(_)
            | Self::ExternalResource(_)
            | Self::Error(_)
            | Self::None => {}
        }
    }
}

impl RuntimeValue {
    /// 返回 intrinsic 输出使用的稳定文本表示。
    pub fn display_text(&self) -> RuntimeResult<String> {
        Ok(match self {
            Self::Module(_) | Self::ModuleFunction(_, _) => self.type_name(),
            Self::Int(value) => value.to_string(),
            Self::Sint(value) => value.to_string(),
            Self::Lint(value) | Self::Lfloat(value) => value.clone(),
            Self::Float(value) => value.to_string(),
            Self::Sfloat(value) => value.to_string(),
            Self::Bool(value) => value.to_string(),
            Self::Str(handle) => return handle.to_string(),
            Self::None => "none".to_owned(),
            Self::ExternalResource(resource) => {
                let kind = resource.kind()?;
                kind.display_name()
            }
            Self::Error(_) => "error".to_owned(),
            Self::Table(_)
            | Self::TableDropView(_)
            | Self::Array(_)
            | Self::Tuple(_)
            | Self::DictTable(_)
            | Self::DictColumn(_)
            | Self::Set(_) => format!("{self:?}"),
        })
    }

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
            Self::Table(_)
            | Self::Module(_)
            | Self::ModuleFunction(_, _)
            | Self::TableDropView(_)
            | Self::Array(_)
            | Self::Tuple(_)
            | Self::DictTable(_)
            | Self::DictColumn(_)
            | Self::Set(_)
            | Self::ExternalResource(_)
            | Self::Error(_)
            | Self::None => return None,
        })
    }

    /// 返回稳定的 Runtime 类型名称。
    ///
    /// 每个容器变体都必须显式列出：`scalar_type()` 对它们返回 `None`，靠 `_`
    /// 兜底会让容器类型名静默变成 `dynamic`，而 `type_mismatch` 的参数正是它。
    #[must_use]
    pub fn type_name(&self) -> String {
        match self {
            Self::Module(_) => "module".to_owned(),
            Self::ModuleFunction(_, _) => "function".to_owned(),
            Self::Table(instance) => format!("table {}", instance.name()),
            Self::TableDropView(view) => format!("table {}", view.signature().name),
            Self::Array(_) => RuntimeTypeTag::Array.as_str().to_owned(),
            Self::Tuple(_) => RuntimeTypeTag::Tuple.as_str().to_owned(),
            Self::DictTable(_) => RuntimeTypeTag::DictTable.as_str().to_owned(),
            Self::DictColumn(_) => RuntimeTypeTag::DictColumn.as_str().to_owned(),
            Self::Set(_) => RuntimeTypeTag::Set.as_str().to_owned(),
            Self::ExternalResource(resource) => resource
                .kind()
                .map(ExternalResourceKind::display_name)
                .unwrap_or_else(|_| "resource<invalid>".to_owned()),
            Self::Error(_) => "error".to_owned(),
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

    /// 从字符串创建 `RuntimeValue::Str`。
    pub fn new_string(value: impl Into<String>) -> RuntimeResult<Self> {
        Ok(Self::Str(StringHandle::new(value)?))
    }

    /// 从可恢复错误创建统一运行时值。
    #[must_use]
    pub fn error(error: XiaoError) -> Self {
        Self::Error(Box::new(error))
    }

    /// 读取错误对象；其他值返回 `None`。
    #[must_use]
    pub fn as_error(&self) -> Option<&XiaoError> {
        match self {
            Self::Error(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
/// 标量运算和 UTF-8 字符串句柄的回归测试。
mod tests {
    use super::{ExternalResourceHandle, ExternalResourceKind, RuntimeValue, StringHandle};

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

    #[test]
    /// 错误对象复制保留身份相等，独立构造的同内容错误仍不相等。
    fn error_values_keep_identity_semantics() {
        let original = RuntimeValue::error(super::XiaoError::invalid_value("x"));
        assert_eq!(original, original.clone());
        assert_ne!(
            original,
            RuntimeValue::error(super::XiaoError::invalid_value("x"))
        );
    }

    #[test]
    /// 错误对象的类型名必须稳定为 `error`，不能落入 dynamic 兜底。
    fn error_type_name_is_explicit() {
        let value = RuntimeValue::error(super::XiaoError::invalid_value("x"));
        assert_eq!(value.type_name(), "error");
        assert!(!crate::containers::is_hashable(&value));
    }

    #[test]
    /// 资源别名共享逻辑身份，关闭幂等且关闭后返回稳定生命周期错误。
    fn external_resource_aliases_share_identity_and_close_idempotently() {
        let resource = ExternalResourceHandle::new(ExternalResourceKind::File).unwrap();
        let alias = resource.clone();
        let left = RuntimeValue::ExternalResource(resource);
        let right = RuntimeValue::ExternalResource(alias.clone());
        assert_eq!(left, right);
        assert_eq!(left.type_name(), "resource<file>");
        assert_eq!(left.display_text().unwrap(), "resource<file>");
        alias.close().unwrap();
        alias.close().unwrap();
        assert_eq!(alias.ensure_open().unwrap_err().code(), "X06-RUNTIME-025");
    }
}
