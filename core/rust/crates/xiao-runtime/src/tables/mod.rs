//! 由静态签名驱动的表对象和生命周期状态机。
//!
//! 运行时不允许动态增加成员或改变成员类型。`TableDefinition` 直接携带
//! 05-C 生成的 `TableSignature`，因此表的字段、方法和可见性只有一个来源。

use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};

use xiao_syntax::TableKind;
use xiao_types::{TableMemberKind, TableSignature, TableValueKind, Type};

use crate::errors::{RuntimeError, RuntimeResult};
use crate::memory::{
    ObjectLayout, ObjectPayload, RuntimeTypeTag, StrongHandle, WeakHandle, allocate_payload,
};
use crate::value::RuntimeValue;

/// 表初始化钩子；执行期间可以通过 `TableInstance` 写入已声明字段。
pub type TableInitHook = fn(&mut TableInstance) -> RuntimeResult<()>;

/// 表释放钩子；执行期间只能观察表对象，不得重新拥有其句柄。
pub type TableDropHook = fn(&TableObject) -> RuntimeResult<()>;

/// 表生命周期钩子集合。
#[derive(Clone, Copy, Debug, Default)]
pub struct TableHooks {
    init: Option<TableInitHook>,
    drop: Option<TableDropHook>,
}

impl TableHooks {
    /// 创建一组没有钩子的默认配置。
    #[must_use]
    pub const fn none() -> Self {
        Self {
            init: None,
            drop: None,
        }
    }

    /// 创建带 `init` 和 `drop` 钩子的配置。
    #[must_use]
    pub const fn new(init: Option<TableInitHook>, drop: Option<TableDropHook>) -> Self {
        Self { init, drop }
    }

    /// 返回可选初始化钩子。
    #[must_use]
    pub const fn init(self) -> Option<TableInitHook> {
        self.init
    }

    /// 返回可选释放钩子。
    #[must_use]
    pub const fn drop(self) -> Option<TableDropHook> {
        self.drop
    }
}

/// 静态表签名与运行时钩子的组合。
#[derive(Clone, Debug)]
pub struct TableDefinition {
    signature: TableSignature,
    hooks: TableHooks,
}

impl TableDefinition {
    /// 使用没有生命周期钩子的静态签名创建定义。
    #[must_use]
    pub fn new(signature: TableSignature) -> Self {
        Self {
            signature,
            hooks: TableHooks::none(),
        }
    }

    /// 使用指定生命周期钩子创建定义。
    #[must_use]
    pub fn with_hooks(signature: TableSignature, hooks: TableHooks) -> Self {
        Self { signature, hooks }
    }

    /// 返回静态表签名。
    #[must_use]
    pub const fn signature(&self) -> &TableSignature {
        &self.signature
    }

    /// 返回表生命周期钩子。
    #[must_use]
    pub const fn hooks(&self) -> TableHooks {
        self.hooks
    }

    /// 返回表名称。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.signature.name
    }

    /// 判断定义是否为 `[[Table]]` 可实例化表。
    #[must_use]
    pub fn is_instantiable(&self) -> bool {
        self.signature.declaration_kind == TableKind::Instance
    }
}

/// 表对象的显式构造状态。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TableState {
    /// 已分配对象头，尚未开始字段初始化。
    Allocated,
    /// 正在运行 `init` 或装配构造参数。
    FieldsInitializing,
    /// `init` 已成功返回，等待转为可用状态。
    InitCompleted,
    /// 可以由用户代码访问。
    Usable,
    /// 正在执行 `drop`。
    Dropping,
    /// 字段与钩子均已处理完毕。
    Released,
}

impl TableState {
    /// 返回稳定状态名称。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allocated => "allocated",
            Self::FieldsInitializing => "fields_initializing",
            Self::InitCompleted => "init_completed",
            Self::Usable => "usable",
            Self::Dropping => "dropping",
            Self::Released => "released",
        }
    }
}

/// 表对象的可变运行时状态和字段存储。
#[derive(Debug)]
struct TableData {
    state: TableState,
    fields: BTreeMap<String, RuntimeValue>,
}

/// 表载荷；成员集合由静态签名约束。
pub struct TableObject {
    definition: TableDefinition,
    data: RefCell<TableData>,
}

impl Debug for TableObject {
    /// 输出表身份和状态，不展开所有字段内容。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TableObject")
            .field("name", &self.definition.name())
            .field("state", &self.state())
            .finish()
    }
}

impl TableObject {
    /// 根据静态定义创建尚未初始化的表载荷。
    fn new(definition: TableDefinition) -> Self {
        Self {
            definition,
            data: RefCell::new(TableData {
                state: TableState::Allocated,
                fields: BTreeMap::new(),
            }),
        }
    }

    /// 返回表的静态定义。
    #[must_use]
    pub fn definition(&self) -> &TableDefinition {
        &self.definition
    }

    /// 返回表名称。
    #[must_use]
    pub fn name(&self) -> &str {
        self.definition.name()
    }

    /// 返回当前构造状态。
    #[must_use]
    pub fn state(&self) -> TableState {
        self.data.borrow().state
    }

    /// 读取一个已初始化字段的副本。
    pub fn field(&self, name: &str) -> RuntimeResult<Option<RuntimeValue>> {
        self.ensure_readable()?;
        Ok(self.data.borrow().fields.get(name).cloned())
    }

    /// 在 Runtime 内部写入字段；调用方必须已经通过签名检查。
    pub(crate) fn set_internal(&self, name: &str, value: RuntimeValue) -> RuntimeResult<()> {
        self.ensure_writable()?;
        let member = self.definition.signature.member(name).ok_or_else(|| {
            RuntimeError::invalid_value(format!("表 {} 没有字段 {name}", self.name()))
        })?;
        if member.kind != TableMemberKind::Field {
            return Err(RuntimeError::invalid_value(format!(
                "{} 是方法而不是字段",
                name
            )));
        }
        if !runtime_value_matches(&value, &member.ty) {
            return Err(RuntimeError::type_mismatch(
                member.ty.to_string(),
                value.type_name(),
            ));
        }
        self.data.borrow_mut().fields.insert(name.to_owned(), value);
        Ok(())
    }

    /// 检查当前状态是否允许读取字段。
    fn ensure_readable(&self) -> RuntimeResult<()> {
        let state = self.state();
        if matches!(
            state,
            TableState::InitCompleted | TableState::Usable | TableState::Dropping
        ) {
            Ok(())
        } else {
            Err(RuntimeError::table_state("usable", state.as_str()))
        }
    }

    /// 检查当前状态是否允许写入字段。
    fn ensure_writable(&self) -> RuntimeResult<()> {
        let state = self.state();
        if matches!(
            state,
            TableState::FieldsInitializing | TableState::InitCompleted | TableState::Usable
        ) {
            Ok(())
        } else {
            Err(RuntimeError::table_state("writable", state.as_str()))
        }
    }
}

impl ObjectPayload for TableObject {
    /// 返回表对象标签。
    fn type_tag(&self) -> RuntimeTypeTag {
        RuntimeTypeTag::Table
    }

    /// 返回表载荷布局摘要。
    fn layout(&self) -> ObjectLayout {
        ObjectLayout::for_type::<Self>(RuntimeTypeTag::Table)
    }

    /// 运行 `drop` 钩子并把对象转为已释放状态。
    fn on_drop(&mut self) -> RuntimeResult<()> {
        let hook = {
            let mut data = self.data.borrow_mut();
            if data.state == TableState::Released {
                return Err(RuntimeError::refcount_invariant("表对象被重复释放"));
            }
            data.state = TableState::Dropping;
            self.definition.hooks.drop
        };
        let result = hook.map_or(Ok(()), |callback| callback(self));
        self.data.borrow_mut().state = TableState::Released;
        result.map_err(|error| RuntimeError::table_drop("表 drop 钩子失败").with_cause(error))
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

/// 表实例的安全强句柄封装。
pub struct TableInstance {
    handle: StrongHandle,
}

impl Debug for TableInstance {
    /// 输出表身份、状态和计数摘要。
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TableInstance")
            .field("name", &self.name())
            .field("state", &self.state())
            .field("strong_count", &self.strong_count())
            .finish()
    }
}

impl Clone for TableInstance {
    /// 克隆表实例强句柄。
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl TableInstance {
    /// 按 `[[Table]]` 定义分配并初始化一个实例。
    pub fn new(definition: TableDefinition) -> RuntimeResult<Self> {
        if !definition.is_instantiable() {
            return Err(RuntimeError::invalid_value(format!(
                "表 {} 不是可实例化构造目标",
                definition.name()
            )));
        }
        Self::allocate_and_initialize(definition)
    }

    /// 按 `[Table]` 定义创建一个单例表值。
    pub fn singleton(definition: TableDefinition) -> RuntimeResult<Self> {
        if definition.signature.declaration_kind != TableKind::Singleton {
            return Err(RuntimeError::invalid_value(format!(
                "表 {} 不是单例表定义",
                definition.name()
            )));
        }
        Self::allocate_and_initialize(definition)
    }

    /// 分配表载荷并执行完整初始化状态机。
    fn allocate_and_initialize(definition: TableDefinition) -> RuntimeResult<Self> {
        let handle = allocate_payload(Box::new(TableObject::new(definition)))?;
        let instance = Self { handle };
        instance.initialize()
    }

    /// 执行字段初始化、`init` 钩子和失败回滚。
    fn initialize(self) -> RuntimeResult<Self> {
        let hook = self
            .handle
            .with_payload(RuntimeTypeTag::Table, |object: &TableObject| {
                object.definition.hooks.init
            })?;
        self.set_state(TableState::FieldsInitializing)?;
        let Some(callback) = hook else {
            self.set_state(TableState::InitCompleted)?;
            self.set_state(TableState::Usable)?;
            return Ok(self);
        };
        let mut callback_view = self.clone();
        let result = callback(&mut callback_view);
        drop(callback_view);
        match result {
            Ok(()) => {
                self.set_state(TableState::InitCompleted)?;
                self.set_state(TableState::Usable)?;
                Ok(self)
            }
            Err(cause) => {
                let mut error = RuntimeError::table_init("表 init 钩子失败").with_cause(cause);
                if let Err(drop_error) = self.handle.try_release() {
                    error.push_suppressed(drop_error);
                }
                Err(error)
            }
        }
    }

    /// 返回表名称。
    #[must_use]
    pub fn name(&self) -> String {
        self.handle
            .with_payload(RuntimeTypeTag::Table, |object: &TableObject| {
                object.name().to_owned()
            })
            .unwrap_or_else(|_| "<released>".to_owned())
    }

    /// 返回表当前状态。
    #[must_use]
    pub fn state(&self) -> TableState {
        self.handle
            .with_payload(RuntimeTypeTag::Table, TableObject::state)
            .unwrap_or(TableState::Released)
    }

    /// 返回静态表定义。
    pub fn definition(&self) -> RuntimeResult<TableDefinition> {
        self.handle
            .with_payload(RuntimeTypeTag::Table, |object: &TableObject| {
                object.definition.clone()
            })
    }

    /// 读取公开字段。
    pub fn get(&self, name: &str) -> RuntimeResult<Option<RuntimeValue>> {
        self.handle
            .with_payload(RuntimeTypeTag::Table, |object: &TableObject| {
                let member = object.definition.signature.member(name).ok_or_else(|| {
                    RuntimeError::invalid_value(format!("表 {} 没有成员 {name}", object.name()))
                })?;
                if !member.visibility.is_public() {
                    return Err(RuntimeError::invalid_value(format!(
                        "表成员 {name} 不可从表外访问"
                    )));
                }
                if member.kind != TableMemberKind::Field {
                    return Err(RuntimeError::invalid_value(format!(
                        "{} 是方法而不是字段",
                        name
                    )));
                }
                object.field(name)
            })?
    }

    /// 写入公开字段并执行静态类型对应的 Runtime 校验。
    pub fn set(&self, name: &str, value: RuntimeValue) -> RuntimeResult<()> {
        self.handle
            .with_payload(RuntimeTypeTag::Table, |object: &TableObject| {
                let member = object.definition.signature.member(name).ok_or_else(|| {
                    RuntimeError::invalid_value(format!("表 {} 没有成员 {name}", object.name()))
                })?;
                if !member.visibility.is_public() {
                    return Err(RuntimeError::invalid_value(format!(
                        "表成员 {name} 不可从表外访问"
                    )));
                }
                object.set_internal(name, value)
            })?
    }

    /// 返回强引用计数。
    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.handle.strong_count()
    }

    /// 创建弱表引用。
    #[must_use]
    pub fn downgrade(&self) -> WeakHandle {
        self.handle.downgrade()
    }

    /// 显式执行一次强句柄释放。
    pub fn try_release(self) -> RuntimeResult<()> {
        self.handle.try_release()
    }

    /// 消耗表包装并取出底层强句柄，供 Runtime/后端适配层使用。
    pub fn into_strong_handle(self) -> StrongHandle {
        self.handle
    }

    /// 判断两个实例是否引用同一个对象头。
    #[must_use]
    pub fn same_object(&self, other: &Self) -> bool {
        self.handle.same_object(&other.handle)
    }

    /// 按允许的状态转移边更新表状态。
    fn set_state(&self, state: TableState) -> RuntimeResult<()> {
        self.handle
            .with_payload_mut(RuntimeTypeTag::Table, |object: &mut TableObject| {
                let current = object.data.borrow().state;
                let valid = matches!(
                    (current, state),
                    (TableState::Allocated, TableState::FieldsInitializing)
                        | (TableState::FieldsInitializing, TableState::InitCompleted)
                        | (TableState::FieldsInitializing, TableState::Dropping)
                        | (TableState::InitCompleted, TableState::Usable)
                        | (TableState::Usable, TableState::Dropping)
                        | (TableState::Dropping, TableState::Released)
                );
                if !valid {
                    return Err(RuntimeError::table_state(state.as_str(), current.as_str()));
                }
                object.data.borrow_mut().state = state;
                Ok(())
            })?
    }
}

/// 检查 Runtime 值是否符合静态字段类型。
fn runtime_value_matches(value: &RuntimeValue, expected: &Type) -> bool {
    match expected {
        Type::Dynamic => true,
        Type::None => matches!(value, RuntimeValue::None),
        Type::Scalar(scalar) => value.scalar_type() == Some(*scalar),
        Type::Table(table) => match value {
            RuntimeValue::Table(instance) => instance
                .definition()
                .map(|definition| {
                    definition.name() == table.name
                        && match table.kind {
                            TableValueKind::Singleton => !definition.is_instantiable(),
                            TableValueKind::Constructor => false,
                            TableValueKind::Instance => definition.is_instantiable(),
                        }
                })
                .unwrap_or(false),
            _ => false,
        },
        Type::Variable(_)
        | Type::Function { .. }
        | Type::Array(_)
        | Type::Tuple(_)
        | Type::DictTable(_)
        | Type::DictColumn(_)
        | Type::Set(_) => false,
    }
}

#[cfg(test)]
/// 表状态机、字段访问和初始化失败回滚的回归测试。
mod tests {
    use super::{TableDefinition, TableHooks, TableInstance, TableState};
    use crate::errors::RuntimeResult;
    use crate::value::RuntimeValue;
    use xiao_source::SourceSpan;
    use xiao_syntax::{ScalarType, TableKind};
    use xiao_types::{TableMemberSignature, TableSignature, Type};

    /// 返回测试用的有效源码区间。
    fn span() -> SourceSpan {
        SourceSpan::new(0, 1).expect("测试区间有效")
    }

    /// 构造带整数字段的实例表定义。
    fn definition() -> TableDefinition {
        let mut signature = TableSignature::new("User", TableKind::Instance, span());
        signature.members.insert(
            "id".to_owned(),
            TableMemberSignature::field("id", Type::scalar(ScalarType::Int), span()),
        );
        TableDefinition::with_hooks(signature, TableHooks::none())
    }

    #[test]
    /// 验证构造状态机和字段类型检查。
    fn follows_construct_state_machine_and_checks_fields() {
        let instance = TableInstance::new(definition()).expect("实例应构造");
        assert_eq!(instance.state(), TableState::Usable);
        instance
            .set("id", RuntimeValue::Int(7))
            .expect("字段应可写");
        assert_eq!(
            instance.get("id").expect("读取应成功"),
            Some(RuntimeValue::Int(7))
        );
        assert!(instance.set("id", RuntimeValue::Bool(true)).is_err());
    }

    /// 返回固定错误以覆盖初始化失败路径。
    fn failing_init(_: &mut TableInstance) -> RuntimeResult<()> {
        Err(crate::errors::RuntimeError::invalid_value("测试 init 失败"))
    }

    #[test]
    /// 初始化失败后报告错误并释放对象。
    fn init_failure_is_reported_and_object_is_released() {
        let mut signature = TableSignature::new("Broken", TableKind::Instance, span());
        signature.members.insert(
            "id".to_owned(),
            TableMemberSignature::field("id", Type::scalar(ScalarType::Int), span()),
        );
        let definition =
            TableDefinition::with_hooks(signature, TableHooks::new(Some(failing_init), None));
        let error = TableInstance::new(definition).expect_err("init 应失败");
        assert_eq!(error.code(), "X06-RUNTIME-007");
        assert!(error.cause().is_some());
    }
}
