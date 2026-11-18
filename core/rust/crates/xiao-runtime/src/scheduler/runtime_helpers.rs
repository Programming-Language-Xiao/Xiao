//! Runtime TLS、根 Runtime、取消辅助和可观测线程等待。

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::{
    RuntimeCancellation, RuntimeExecError, RuntimeHandle, RuntimeInner, ThreadHandle,
    current_cancellation_context, current_runtime_context, detached_errors,
};

thread_local! {
    static ROOT_RUNTIME: RefCell<Option<RuntimeHandle>> = const { RefCell::new(None) };
}

/// 所有 live Runtime 的句柄索引；值只保留弱引用，不延长 Runtime 生命周期。
fn runtime_handles() -> &'static Mutex<HashMap<u64, Weak<RuntimeInner>>> {
    /// 弱引用索引不延长任何 Runtime 的生命周期。
    static HANDLES: OnceLock<Mutex<HashMap<u64, Weak<RuntimeInner>>>> = OnceLock::new();
    HANDLES.get_or_init(Default::default)
}

/// 把新 Runtime 放进当前进程的弱索引。
pub(super) fn register_runtime(id: u64, inner: &Arc<RuntimeInner>) {
    if let Ok(mut handles) = runtime_handles().lock() {
        handles.insert(id, Arc::downgrade(inner));
        handles.retain(|_, inner| inner.strong_count() > 0);
    }
}

/// 返回当前线程上下文 Runtime；没有显式上下文时 lazily 创建根 Runtime。
#[must_use]
pub(super) fn current_runtime_handle() -> RuntimeHandle {
    if let Some(id) = current_runtime_context()
        && let Ok(handles) = runtime_handles().lock()
        && let Some(inner) = handles.get(&id).and_then(Weak::upgrade)
    {
        return RuntimeHandle {
            id,
            owner_runtime_id: inner.owner_runtime_id,
            inner,
        };
    }
    ROOT_RUNTIME.with(|root| {
        let mut root = root.borrow_mut();
        root.get_or_insert_with(RuntimeHandle::create).clone()
    })
}

/// 返回进程启动以来 detached 队列深度峰值。
#[must_use]
pub fn detached_reaper_peak_depth() -> usize {
    super::DETACHED_REAPER_PEAK.load(Ordering::Acquire)
}

/// 返回 detached 回收队列中尚未开始执行的闭包数量。
#[must_use]
pub fn detached_reaper_queue_depth() -> usize {
    super::detached_queue_depth_counter().load(Ordering::Acquire)
}

/// 取走进程级 detached 失败记录；记录采用 X06-RUNTIME-028。
#[must_use]
pub fn take_detached_errors() -> Vec<RuntimeExecError> {
    super::detached_errors()
        .lock()
        .map(|mut errors| std::mem::take(&mut *errors))
        .unwrap_or_default()
}

/// 消费当前执行单元的取消安全点；无并发执行上下文时视为未取消。
pub fn cancellation_safe_point() -> Result<(), RuntimeExecError> {
    if current_cancellation_context().is_some_and(|token| token.is_cancelled()) {
        Err(RuntimeExecError::Cancelled)
    } else {
        Ok(())
    }
}

/// 克隆当前执行单元可继承的取消令牌；根线程没有令牌时返回未取消令牌。
#[must_use]
pub fn current_cancellation_token() -> RuntimeCancellation {
    current_cancellation_context().unwrap_or_default()
}

/// 把原生 trampoline 或嵌入式执行入口观察到的 detached 失败写入进程级通道。
pub fn record_detached_error(error: RuntimeExecError) {
    detached_errors()
        .lock()
        .expect("detached 错误通道锁未中毒")
        .push(error);
}

/// 允许 Runtime ABI 将调用者取消安全点映射到线程回收回调。
pub(crate) trait ThreadHandleReportExt<T> {
    /// 等待期间若调用者中断，取消线程并把最终结果交给指定 detached 接收器。
    fn join_interruptible_with_report(
        self,
        interrupted: impl Fn() -> bool,
        report: impl FnOnce(Result<T, RuntimeExecError>) + Send + 'static,
    ) -> Result<T, RuntimeExecError>
    where
        T: Send + 'static;
}

impl<T: 'static> ThreadHandleReportExt<T> for ThreadHandle<T> {
    /// 中断等待时取消任务并把最终执行结果送至 detached 回调。
    fn join_interruptible_with_report(
        mut self,
        interrupted: impl Fn() -> bool,
        report: impl FnOnce(Result<T, RuntimeExecError>) + Send + 'static,
    ) -> Result<T, RuntimeExecError>
    where
        T: Send + 'static,
    {
        while !self.join.as_ref().is_none_or(JoinHandle::is_finished) {
            if interrupted() {
                self.cancel();
                self.detach_with(report);
                return Err(RuntimeExecError::Cancelled);
            }
            thread::park_timeout(Duration::from_millis(2));
        }
        if interrupted() {
            self.cancel();
            self.detach_with(report);
            return Err(RuntimeExecError::Cancelled);
        }
        self.join
            .take()
            .ok_or(RuntimeExecError::JoinFailed)?
            .join()
            .map_err(|_| RuntimeExecError::JoinFailed)?
    }
}

#[cfg(test)]
/// 22D 内部并发适配实现。
mod tests {
    use super::*;

    #[test]
    /// 验证 root Runtime 复用和显式子 Runtime 上下文恢复。
    fn current_runtime_reuses_root_and_registered_context() {
        let root = RuntimeHandle::current();
        assert_eq!(RuntimeHandle::current().id(), root.id());
        let child = root
            .in_context(RuntimeHandle::create)
            .expect("child Runtime creation");
        assert_eq!(
            child.in_context(|| RuntimeHandle::current().id()),
            Ok(child.id())
        );
        assert_eq!(root.in_context(|| child.try_close()), Ok(Ok(())));
    }
}
