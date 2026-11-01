//! 测试专用的一次字符串分配失败注入；默认生产构建不编译此模块。

use std::cell::Cell;

use crate::{ALLOCATION_CODE, RuntimeError, RuntimeErrorKind, RuntimeResult};

#[derive(Clone, Copy)]
/// 当前线程的剩余成功次数和已注入失败计数，用于嵌套作用域恢复。
struct State {
    remaining: Option<usize>,
    failures: usize,
}

thread_local! {
    static STATE: Cell<State> = const { Cell::new(State { remaining: None, failures: 0 }) };
}

/// 在当前线程的指定次数成功字符串分配后，仅拒绝下一次；离开作用域恢复原状态。
///
/// 只用于 Rust 测试，不暴露环境变量、intrinsic 或原生生产进程的注入协议。
pub struct StringAllocationFailure {
    previous: State,
    initial_failures: usize,
}

impl StringAllocationFailure {
    /// 允许 successful 次分配，然后一次性返回既有 ResourceError / 011。
    pub fn after(successful: usize) -> Self {
        STATE.with(|state| {
            let previous = state.get();
            state.set(State {
                remaining: Some(successful),
                failures: previous.failures,
            });
            Self {
                previous,
                initial_failures: previous.failures,
            }
        })
    }

    /// 注入是否在本作用域被真正消费，防止未走目标路径的测试误报通过。
    pub fn was_triggered(&self) -> bool {
        STATE.with(|state| state.get().failures > self.initial_failures)
    }
}

impl Drop for StringAllocationFailure {
    /// 包括 panic 展开在内都恢复当前线程的先前注入状态。
    fn drop(&mut self) {
        STATE.with(|state| state.set(self.previous));
    }
}

/// 在字符串分配入口消费一次性注入；正常生产构建不会编译此检查。
pub(crate) fn check_string_allocation() -> RuntimeResult<()> {
    STATE.with(|state| {
        let mut current = state.get();
        match current.remaining {
            Some(0) => {
                current.remaining = None;
                current.failures += 1;
                state.set(current);
                Err(RuntimeError::new(
                    RuntimeErrorKind::Resource,
                    ALLOCATION_CODE,
                    "runtime.allocation",
                    "测试注入的字符串分配失败",
                ))
            }
            Some(remaining) => {
                current.remaining = Some(remaining - 1);
                state.set(current);
                Ok(())
            }
            None => Ok(()),
        }
    })
}
