//! 06-B 规格测试使用的 Runtime 释放计划驱动器。
//!
//! 驱动器不属于用户程序 API，也不接入 VM；它把 `xiao-lifetime` 的静态动作
//! 映射到真实句柄，便于在后端出现前验证释放顺序、转移值和计数平衡。

use std::collections::BTreeMap;

use xiao_lifetime::{ExitKind, ReleaseActionKind, ReleasePlan, ValueId};

use crate::errors::{ErrorAccumulator, RuntimeError, RuntimeResult};
use crate::memory::{StrongHandle, WeakHandle};

/// 释放计划中一个值的测试绑定。
#[derive(Debug)]
pub enum RuntimeBinding {
    /// 强拥有句柄。
    Strong(StrongHandle),
    /// 弱引用句柄。
    Weak(WeakHandle),
}

/// 驱动器记录的一条释放事件。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseEvent {
    /// 对应的静态值身份。
    pub value: ValueId,
    /// 执行的动作种类。
    pub kind: ReleaseActionKind,
}

/// 一次释放计划执行的可审计结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseExecution {
    /// 实际执行顺序。
    pub events: Vec<ReleaseEvent>,
    /// 计划声明转移给外层的值。
    pub transferred: Vec<ValueId>,
    /// 触发计划的退出边。
    pub exit: ExitKind,
}

/// 一次完整作用域展开的结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnwindExecution {
    /// 释放计划执行记录。
    pub release: ReleaseExecution,
    /// 展开结束后需要交给 `catch` 或程序边界的主错误。
    pub error: Option<RuntimeError>,
}

impl UnwindExecution {
    /// 借用展开后的主错误。
    #[must_use]
    pub fn error(&self) -> Option<&RuntimeError> {
        self.error.as_ref()
    }

    /// 消耗结果并取出主错误。
    #[must_use]
    pub fn into_error(self) -> Option<RuntimeError> {
        self.error
    }
}

/// 只在测试中使用的 Runtime 执行驱动器。
#[derive(Clone, Debug, Default)]
pub struct RuntimeDriver;

impl RuntimeDriver {
    /// 创建一个空驱动器。
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// 按静态释放计划执行绑定并返回确定性事件序列。
    pub fn execute_plan(
        &self,
        plan: &ReleasePlan,
        bindings: &mut BTreeMap<ValueId, RuntimeBinding>,
    ) -> RuntimeResult<ReleaseExecution> {
        let mut errors = ErrorAccumulator::new(None);
        let execution = self.execute_plan_collect(plan, bindings, &mut errors);
        if let Some(error) = errors.finish() {
            Err(error)
        } else {
            Ok(execution)
        }
    }

    /// 按 `finally -> drop -> catch/传播` 顺序执行一次作用域展开。
    ///
    /// `finally_hooks` 只模拟已经由前端/IR 确定的清理动作；它们不执行
    /// Xiao 源码。所有清理错误都会按统一规则并入 `error.suppressed`。
    pub fn unwind(
        &self,
        plan: &ReleasePlan,
        bindings: &mut BTreeMap<ValueId, RuntimeBinding>,
        primary: Option<RuntimeError>,
        finally_hooks: &[fn() -> RuntimeResult<()>],
    ) -> UnwindExecution {
        let mut errors = ErrorAccumulator::new(primary);
        for hook in finally_hooks {
            if let Err(error) = hook() {
                errors.record(error);
            }
        }
        let release = self.execute_plan_collect(plan, bindings, &mut errors);
        UnwindExecution {
            release,
            error: errors.finish(),
        }
    }

    /// 执行所有释放动作并持续收集清理错误，避免首个错误导致剩余绑定泄漏。
    fn execute_plan_collect(
        &self,
        plan: &ReleasePlan,
        bindings: &mut BTreeMap<ValueId, RuntimeBinding>,
        errors: &mut ErrorAccumulator,
    ) -> ReleaseExecution {
        let mut events = Vec::with_capacity(plan.actions.len());
        for action in &plan.actions {
            let Some(binding) = bindings.remove(&action.value) else {
                errors.record(RuntimeError::invalid_handle(format!(
                    "释放计划缺少值 {}",
                    action.value.get()
                )));
                events.push(ReleaseEvent {
                    value: action.value,
                    kind: action.kind,
                });
                continue;
            };
            let action_error = match (action.kind, binding) {
                (ReleaseActionKind::Strong, RuntimeBinding::Strong(handle)) => {
                    handle.try_release().err()
                }
                (ReleaseActionKind::Weak, RuntimeBinding::Weak(handle)) => {
                    drop(handle);
                    None
                }
                (ReleaseActionKind::Strong, RuntimeBinding::Weak(_)) => Some(
                    RuntimeError::invalid_value("释放计划要求强句柄，但绑定是弱句柄"),
                ),
                (ReleaseActionKind::Weak, RuntimeBinding::Strong(_)) => Some(
                    RuntimeError::invalid_value("释放计划要求弱句柄，但绑定是强句柄"),
                ),
            };
            if let Some(error) = action_error {
                errors.record(error);
            }
            events.push(ReleaseEvent {
                value: action.value,
                kind: action.kind,
            });
        }
        ReleaseExecution {
            events,
            transferred: plan.transferred.clone(),
            exit: plan.exit,
        }
    }
}

#[cfg(test)]
/// 释放计划驱动器的顺序、展开和错误聚合回归测试。
mod tests {
    use super::{ReleaseExecution, RuntimeBinding, RuntimeDriver};
    use crate::memory::{ObjectLayout, ObjectPayload, RuntimeTypeTag, allocate_payload};
    use std::any::Any;
    use std::cell::Cell;
    use std::rc::Rc;
    use xiao_lifetime::{
        ExitKind, ReleaseAction, ReleaseActionKind, ReleasePlan, ScopeId, ValueId,
    };

    /// 用于观察释放次数的测试载荷。
    struct Probe(Rc<Cell<u32>>);

    impl ObjectPayload for Probe {
        /// 返回测试载荷标签。
        fn type_tag(&self) -> RuntimeTypeTag {
            RuntimeTypeTag::Custom(7)
        }

        /// 返回测试载荷布局。
        fn layout(&self) -> ObjectLayout {
            ObjectLayout::for_type::<Self>(RuntimeTypeTag::Custom(7))
        }

        /// 记录测试载荷释放。
        fn on_drop(&mut self) -> crate::RuntimeResult<()> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }

        /// 暴露只读测试载荷视图。
        fn as_any(&self) -> &dyn Any {
            self
        }

        /// 暴露可变测试载荷视图。
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    /// 按静态计划顺序释放绑定。
    fn executes_static_plan_in_order() {
        let dropped = Rc::new(Cell::new(0));
        let handle = allocate_payload(Box::new(Probe(dropped.clone()))).expect("应分配");
        let value = ValueId::new(1);
        let plan = ReleasePlan {
            scope: ScopeId::new(0),
            exit: ExitKind::Normal,
            actions: vec![ReleaseAction {
                value,
                order: 0,
                kind: ReleaseActionKind::Strong,
            }],
            transferred: Vec::new(),
        };
        let mut bindings = [(value, RuntimeBinding::Strong(handle))]
            .into_iter()
            .collect();
        let execution: ReleaseExecution = RuntimeDriver::new()
            .execute_plan(&plan, &mut bindings)
            .expect("计划应执行");
        assert_eq!(execution.events.len(), 1);
        assert_eq!(dropped.get(), 1);
        assert!(bindings.is_empty());
    }

    /// 返回固定错误以覆盖 `finally` 失败路径。
    fn failing_finally() -> crate::RuntimeResult<()> {
        Err(crate::RuntimeError::invalid_value("测试 finally 失败"))
    }

    #[test]
    /// 主错误优先，清理错误进入 suppressed。
    fn unwind_keeps_primary_and_suppresses_cleanup_errors() {
        let value_id = ValueId::new(2);
        let handle = allocate_payload(Box::new(Probe(Rc::new(Cell::new(0))))).expect("应分配");
        let plan = ReleasePlan {
            scope: ScopeId::new(0),
            exit: ExitKind::Error,
            actions: vec![ReleaseAction {
                value: value_id,
                order: 0,
                kind: ReleaseActionKind::Strong,
            }],
            transferred: Vec::new(),
        };
        let mut bindings = [(value_id, RuntimeBinding::Strong(handle))]
            .into_iter()
            .collect();
        let execution = RuntimeDriver::new().unwind(
            &plan,
            &mut bindings,
            Some(crate::RuntimeError::invalid_handle("主错误")),
            &[failing_finally],
        );
        let error = execution.error().expect("应有主错误");
        assert_eq!(error.code(), crate::INVALID_HANDLE_CODE);
        assert_eq!(error.suppressed().len(), 1);
        assert_eq!(execution.release.events.len(), 1);
        assert!(bindings.is_empty());
    }
}
