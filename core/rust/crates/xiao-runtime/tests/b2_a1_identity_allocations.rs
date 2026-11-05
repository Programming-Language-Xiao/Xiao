//! A1真实分配器证据：构造完成后，Eq/Hash对最大合法文本也不分配或释放。

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use xiao_runtime::RuntimeValue;

thread_local! {
    /// 只统计当前测试线程打开的区间，避免libtest并行产生假阳性。
    static COUNTS: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
}

/// 透传系统分配器，只旁观当前线程的分配和释放次数。
struct ObservedAllocator;

#[global_allocator]
/// 本独立集成测试二进制的计数器，不修改产品分配策略。
static ALLOCATOR: ObservedAllocator = ObservedAllocator;

/// 记录一次堆操作，不在TLS析构时重新建立状态。
fn record(allocation: bool) {
    let _ = COUNTS.try_with(|state| {
        if let Some((allocations, releases)) = state.get() {
            state.set(Some((
                allocations + usize::from(allocation),
                releases + usize::from(!allocation),
            )));
        }
    });
}

unsafe impl GlobalAlloc for ObservedAllocator {
    /// 保留原布局并记录普通分配。
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(true);
        unsafe { System.alloc(layout) }
    }
    /// 零初始化也属于分配。
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(true);
        unsafe { System.alloc_zeroed(layout) }
    }
    /// 扩容属于潜在分配，不得藏在规范视图中。
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(true);
        unsafe { System.realloc(ptr, layout, size) }
    }
    /// Eq/Hash也不得销毁输入载荷。
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(false);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[test]
/// 使用真实系统分配器核对相等与哈希，而不是以执行耗时推断无分配。
fn valid_numeric_eq_and_hash_do_not_allocate_or_release() {
    let maximum = "9".to_owned() + &"0".repeat(32_767);
    let pairs = [
        (
            RuntimeValue::new_lfloat("1.50").unwrap(),
            RuntimeValue::new_lfloat("15e-1").unwrap(),
        ),
        (
            RuntimeValue::new_lfloat("-0.00").unwrap(),
            RuntimeValue::new_lfloat("0e100000").unwrap(),
        ),
        (
            RuntimeValue::new_lfloat("1e100000").unwrap(),
            RuntimeValue::new_lfloat("10e99999").unwrap(),
        ),
        (
            RuntimeValue::new_lfloat(&maximum).unwrap(),
            RuntimeValue::new_lfloat("9e32767").unwrap(),
        ),
        (
            RuntimeValue::new_lint(&maximum).unwrap(),
            RuntimeValue::new_lint(&("+000".to_owned() + &maximum)).unwrap(),
        ),
    ];
    let mut equal = true;
    COUNTS.with(|state| state.set(Some((0, 0))));
    for _ in 0..4 {
        for (left, right) in std::hint::black_box(&pairs) {
            equal &= std::hint::black_box(left == right);
            let mut first = DefaultHasher::new();
            let mut second = DefaultHasher::new();
            left.hash(&mut first);
            right.hash(&mut second);
            equal &= std::hint::black_box(first.finish() == second.finish());
        }
    }
    let counts = COUNTS.with(|state| state.replace(None)).unwrap();
    assert!(equal);
    assert_eq!(counts, (0, 0), "Eq/Hash应当不分配也不释放");
    assert_eq!(pairs[0].0.display_text().unwrap(), "1.50");
}
