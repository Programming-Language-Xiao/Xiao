//! 06-B Runtime 对象、表生命周期和释放计划的公开接口测试。

use std::sync::atomic::{AtomicUsize, Ordering};

use xiao_lifetime::{ExitKind, ReleaseAction, ReleaseActionKind, ReleasePlan, ScopeId, ValueId};
use xiao_runtime::testing::{RuntimeBinding, RuntimeDriver};
use xiao_runtime::{
    RuntimeError, RuntimeResult, RuntimeValue, StringHandle, TableDefinition, TableDropHook,
    TableHooks, TableInitHook, TableInstance, TableObject, TableState,
};
use xiao_source::SourceSpan;
use xiao_syntax::{ScalarType, TableKind};
use xiao_types::{TableMemberSignature, TableSignature, Type};

/// 返回集成测试使用的有效源码区间。
fn span() -> SourceSpan {
    SourceSpan::new(0, 1).expect("测试区间有效")
}

/// 构造指定种类并带有整数字段的表签名。
fn signature(kind: TableKind) -> TableSignature {
    let mut signature = TableSignature::new("Resource", kind, span());
    signature.members.insert(
        "id".to_owned(),
        TableMemberSignature::field("id", Type::scalar(ScalarType::Int), span()),
    );
    signature
}

/// 在初始化钩子中写入固定字段值。
fn init_id(instance: &mut TableInstance) -> RuntimeResult<()> {
    instance.set("id", RuntimeValue::Int(42))
}

/// 集成测试记录释放钩子触发次数。
static DROP_CALLS: AtomicUsize = AtomicUsize::new(0);

/// 观察表处于 Dropping 状态并递增计数。
fn observe_drop(object: &TableObject) -> RuntimeResult<()> {
    assert_eq!(object.state(), TableState::Dropping);
    assert_eq!(
        object.field("id")?,
        Some(RuntimeValue::Int(42)),
        "drop 钩子应能观察已初始化字段"
    );
    DROP_CALLS.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

/// 返回固定错误以覆盖表释放钩子失败。
fn failing_drop(_: &TableObject) -> RuntimeResult<()> {
    Err(RuntimeError::invalid_value("测试 drop 失败"))
}

#[test]
/// 验证静态表签名、初始化钩子和释放钩子协作。
fn runs_init_and_drop_with_static_signature() {
    DROP_CALLS.store(0, Ordering::SeqCst);
    let definition = TableDefinition::with_hooks(
        signature(TableKind::Instance),
        TableHooks::new(
            Some(init_id as TableInitHook),
            Some(observe_drop as TableDropHook),
        ),
    );
    let instance = TableInstance::new(definition).expect("实例应构造");
    assert_eq!(instance.state(), TableState::Usable);
    assert_eq!(
        instance.get("id").expect("读取成功"),
        Some(RuntimeValue::Int(42))
    );
    instance.try_release().expect("drop 应成功");
    assert_eq!(DROP_CALLS.load(Ordering::SeqCst), 1);
}

#[test]
/// 验证释放钩子错误保留结构化原因身份。
fn drop_failure_keeps_structured_identity() {
    let definition = TableDefinition::with_hooks(
        signature(TableKind::Instance),
        TableHooks::new(None, Some(failing_drop as TableDropHook)),
    );
    let error = TableInstance::new(definition)
        .expect("构造应成功")
        .try_release()
        .expect_err("drop 应返回错误");
    assert_eq!(error.code(), "X06-RUNTIME-008");
    assert_eq!(
        error.cause().map(RuntimeError::code),
        Some("X06-RUNTIME-012")
    );
}

#[test]
/// 验证强引用释放后弱句柄仍可观察对象已失活。
fn weak_handle_survives_strong_release_but_cannot_upgrade() {
    let string = StringHandle::new("x").expect("字符串应创建");
    let weak = string.downgrade();
    assert_eq!(weak.strong_count(), 1);
    string.try_release().expect("释放应成功");
    assert!(!weak.is_alive());
    assert_eq!(
        weak.upgrade().expect_err("升级应失败").code(),
        "X06-RUNTIME-005"
    );
}

#[test]
/// 验证生命周期释放计划能被 Runtime 驱动器消费。
fn driver_consumes_lifetime_release_plan() {
    let value_id = ValueId::new(11);
    let string = StringHandle::new("plan").expect("字符串应创建");
    let plan = ReleasePlan {
        scope: ScopeId::new(0),
        exit: ExitKind::Return,
        actions: vec![ReleaseAction {
            value: value_id,
            order: 0,
            kind: ReleaseActionKind::Strong,
        }],
        transferred: vec![ValueId::new(12)],
    };
    let mut bindings = [(
        value_id,
        RuntimeBinding::Strong(string.into_strong_handle()),
    )]
    .into_iter()
    .collect();
    let result = RuntimeDriver::new()
        .execute_plan(&plan, &mut bindings)
        .expect("释放计划应执行");
    assert_eq!(result.exit, ExitKind::Return);
    assert_eq!(result.events[0].value, value_id);
    assert_eq!(result.transferred, vec![ValueId::new(12)]);
    assert!(bindings.is_empty());
}

#[test]
/// 验证布尔整数运算严格按奇偶翻转并报告溢出。
fn bool_arithmetic_is_strict_and_overflow_is_reported() {
    assert_eq!(
        RuntimeValue::Bool(false)
            .add(&RuntimeValue::Int(3))
            .expect("奇数应翻转")
            .as_bool(),
        Some(true)
    );
    let overflow = RuntimeValue::Int(i64::MAX)
        .add(&RuntimeValue::Int(1))
        .expect_err("应溢出");
    assert_eq!(overflow.code(), "X06-RUNTIME-009");
    assert!(
        RuntimeValue::Bool(true)
            .add(&RuntimeValue::Float(1.0))
            .is_err()
    );
}

#[test]
/// 验证单例表与可实例化表不能混用。
fn singleton_and_instance_kinds_are_not_interchangeable() {
    let singleton = TableDefinition::new(signature(TableKind::Singleton));
    assert!(TableInstance::new(singleton.clone()).is_err());
    let instance = TableInstance::singleton(singleton).expect("单例应创建");
    assert_eq!(instance.state(), TableState::Usable);
}
