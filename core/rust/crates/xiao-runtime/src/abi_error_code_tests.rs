//! 指定错误读取的 C 状态、借用视图、挂起哨兵与复制失败。

use super::*;
use crate::allocation_testing::StringAllocationFailure;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static HEAP_CALLS: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
}

/// 只在本测试线程显式开启的区间统计 System 分配/释放，不影响生产静态库。
struct CountingAllocator;

#[global_allocator]
/// Runtime 单测的计数适配器；未开启线程局部区间时仅透传 System。
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// 只累加当前线程的活跃区间，TLS 析构期间不访问已经销毁的计数状态。
fn record_heap_call(allocation: bool) {
    let _ = HEAP_CALLS.try_with(|state| {
        if let Some((allocations, releases)) = state.get() {
            state.set(Some((
                allocations + usize::from(allocation),
                releases + usize::from(!allocation),
            )));
        }
    });
}

unsafe impl GlobalAlloc for CountingAllocator {
    /// 记录分配次数后完整透传调用方布局。
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_heap_call(true);
        unsafe { System.alloc(layout) }
    }
    /// 零初始化分配也计入禁止分配的范围。
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_heap_call(true);
        unsafe { System.alloc_zeroed(layout) }
    }
    /// 扩容可能移动或分配内存，作为一次分配观察。
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_heap_call(true);
        unsafe { System.realloc(ptr, layout, size) }
    }
    /// 记录释放次数，防止 getter 通过销毁并重建对象伪装只读。
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record_heap_call(false);
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// 包括断言展开在内恢复计数作用域，避免其它测试分配被混进 getter 的测量。
struct HeapCallScope(Option<(usize, usize)>);

impl HeapCallScope {
    /// 打开从零计数的区间并保存外层状态。
    fn new() -> Self {
        HEAP_CALLS.with(|state| Self(state.replace(Some((0, 0)))))
    }
    /// 在不分配文本的前提下读取当前观察值。
    fn counts(&self) -> (usize, usize) {
        HEAP_CALLS.with(|state| state.get().expect("计数区间"))
    }
}

impl Drop for HeapCallScope {
    /// 无论断言是否展开都恢复外层计数状态。
    fn drop(&mut self) {
        HEAP_CALLS.with(|state| state.set(self.0));
    }
}

/// 返回仍由调用方持有的 Error 值，避免通过挂起槽模拟构造。
fn error_value(code: &str) -> XiaoValue {
    let mut error = RuntimeError::new(
        crate::RuntimeErrorKind::Other,
        code,
        "runtime.user_error",
        "original",
    )
    .with_location(SourceSpan::new(4, 17).unwrap())
    .with_context(
        "origin",
        crate::DiagnosticParam::Text("explicit object".to_owned()),
    )
    .with_cause(RuntimeError::invalid_value("cause"));
    error.push_suppressed(RuntimeError::invalid_value("suppressed"));
    value_from_owned_error(error)
}

/// 测试可观测挂起槽；产品 getter 不得调用此方法或 snapshot。
fn pending_projection() -> (u64, String) {
    PENDING_ERROR.with(|pending| {
        let pending = pending.borrow();
        match pending.as_ref().expect("挂起哨兵") {
            PendingError::Recoverable(error) => (error.error_id(), format!("{error:?}")),
            PendingError::Fatal(error) => panic!("不应替换成 Fatal：{error:?}"),
        }
    })
}

/// ptr/len 的初始哨兵不是可解引用地址；getter 失败时必须原样保留。
fn sentinel() -> XiaoAbiBytes {
    XiaoAbiBytes {
        ptr: std::ptr::dangling::<u8>(),
        len: 123,
    }
}

#[test]
/// 成功类型检查及 getter 均零分配/释放，并保留完整输入和挂起哨兵。
fn success_borrows_exact_bytes_without_allocating_or_touching_pending() {
    let long = "MiXeD_雪😀".repeat(2048);
    for code in ["", "Case", "雪😀", long.as_str()] {
        let mut value = error_value(code);
        let handle = unsafe { value.payload.handle };
        let before = unsafe { error_ref(handle) }
            .expect("有效句柄")
            .inners
            .borrow()
            .len();
        set_pending_runtime_error(RuntimeError::invalid_value("pending sentinel"));
        let pending = pending_projection();
        let fail = StringAllocationFailure::after(0);
        let mut view = sentinel();
        let original = format!("{:?}", unsafe { clone_error(handle) }.unwrap());
        let heap = HeapCallScope::new();
        let checked = abi_dynamic::xiao_runtime_dynamic_check(
            XiaoAbiBytes {
                ptr: b"error_value".as_ptr(),
                len: b"error_value".len(),
            },
            &value,
        );
        let status = xiao_runtime_error_code(&value, &mut view);
        let counts = heap.counts();
        drop(heap);
        assert_eq!(checked, 0);
        assert_eq!(status, 0);
        assert_eq!(counts, (0, 0), "成功类型检查和 getter 不得发生堆分配或释放");
        assert!(!fail.was_triggered(), "裸 getter 不应分配字符串");
        assert_eq!(view.len, code.len());
        if code.is_empty() {
            assert!(view.ptr.is_null());
        } else {
            assert_eq!(
                unsafe { std::slice::from_raw_parts(view.ptr, view.len) },
                code.as_bytes()
            );
        }
        assert_eq!(
            unsafe { error_ref(handle) }.unwrap().inners.borrow().len(),
            before
        );
        assert_eq!(pending_projection(), pending);
        assert_eq!(
            format!("{:?}", unsafe { clone_error(handle) }.unwrap()),
            original
        );
        drop(fail);
        xiao_runtime_value_release(&mut value);
        xiao_runtime_error_clear();
    }
}

#[test]
/// 入口空指针、标签和有效内存中的无效句柄分别返回 1/2/3，失败不改 out/挂起槽。
fn failure_statuses_preserve_both_output_fields_and_pending() {
    let mut error = error_value("explicit");
    let wrong_kind = xiao_runtime_value_int(7);
    let empty_payload = XiaoValue {
        tag: XiaoValueTag::Error,
        payload: XiaoValuePayload {
            handle: std::ptr::null_mut(),
        },
    };
    let mut empty_box = Box::new(AbiError {
        magic: ABI_HANDLE_MAGIC,
        kind: ABI_KIND_ERROR,
        _reserved: 0,
        inners: RefCell::new(Vec::new()),
    });
    let empty_handle = XiaoValue {
        tag: XiaoValueTag::Error,
        payload: XiaoValuePayload {
            handle: (&mut *empty_box as *mut AbiError).cast(),
        },
    };
    // 使用完整、存活的 ABI 前缀内存检查错误种类，绝不伪造短内存或悬空指针。
    let mut wrong_kind_box = Box::new(AbiError {
        magic: ABI_HANDLE_MAGIC,
        kind: ABI_KIND_STRONG,
        _reserved: 0,
        inners: RefCell::new(Vec::new()),
    });
    let wrong_box = XiaoValue {
        tag: XiaoValueTag::Error,
        payload: XiaoValuePayload {
            handle: (&mut *wrong_kind_box as *mut AbiError).cast(),
        },
    };
    for (value, expected) in [
        (std::ptr::null(), 1),
        (&wrong_kind as *const XiaoValue, 2),
        (&empty_payload as *const XiaoValue, 3),
        (&empty_handle as *const XiaoValue, 3),
        (&wrong_box as *const XiaoValue, 3),
    ] {
        set_pending_runtime_error(RuntimeError::invalid_value("unchanged pending"));
        let pending = pending_projection();
        let mut out = sentinel();
        let initial = out;
        assert_eq!(xiao_runtime_error_code(value, &mut out), expected);
        assert_eq!(out, initial);
        assert_eq!(pending_projection(), pending);
        xiao_runtime_error_clear();
    }
    set_pending_runtime_error(RuntimeError::invalid_value("null output"));
    let pending = pending_projection();
    assert_eq!(xiao_runtime_error_code(&error, std::ptr::null_mut()), 1);
    assert_eq!(pending_projection(), pending);
    xiao_runtime_value_release(&mut error);
    xiao_runtime_error_clear();
}

#[test]
/// 先安全比较借用地址，验证释放另一别名不会提前销毁源 code 的存储。
fn borrowed_view_survives_releasing_a_different_alias() {
    let mut source = error_value("stable-view-雪");
    let mut alias = XiaoValue::none();
    assert_eq!(xiao_runtime_value_copy(&source, &mut alias), 0);
    let mut before = sentinel();
    assert_eq!(xiao_runtime_error_code(&source, &mut before), 0);
    xiao_runtime_value_release(&mut alias);
    let mut after = sentinel();
    assert_eq!(xiao_runtime_error_code(&source, &mut after), 0);
    // 只比较地址，不在旧实现失败时读取可能已经悬空的 before.ptr。
    assert_eq!(
        before, after,
        "仍持有 source 时，释放其他别名不能使借用视图失效"
    );
    let mut handle = std::ptr::null_mut();
    assert_eq!(xiao_runtime_string_new(before, &mut handle), 0);
    let mut result = xiao_runtime_value_str_owned(handle);
    xiao_runtime_value_release(&mut source);
    xiao_runtime_value_release(&mut result);
}

#[test]
/// 受控复制失败保持源及别名，成功拥有结果在源销毁和后续分配后仍正确。
fn copied_result_survives_source_release_and_copy_failure_keeps_alias() {
    xiao_runtime_error_clear();
    let mut source = error_value("owned-after-release_雪");
    let mut alias = XiaoValue::none();
    assert_eq!(xiao_runtime_value_copy(&source, &mut alias), 0);
    let mut view = sentinel();
    assert_eq!(xiao_runtime_error_code(&source, &mut view), 0);
    let fail = StringAllocationFailure::after(0);
    let mut failed_handle = std::ptr::null_mut();
    assert_eq!(
        xiao_runtime_string_new(view, &mut failed_handle),
        XiaoAbiStatus::RuntimeError.code()
    );
    assert!(fail.was_triggered());
    assert!(failed_handle.is_null());
    PENDING_ERROR.with(|pending| {
        let pending = pending.borrow();
        let Some(PendingError::Recoverable(error)) = pending.as_ref() else {
            panic!("分配错误");
        };
        assert_eq!(error.code(), "X06-RUNTIME-011");
        assert_eq!(error.kind(), crate::RuntimeErrorKind::Resource);
    });
    drop(fail);
    xiao_runtime_error_clear();
    let mut alias_view = sentinel();
    assert_eq!(xiao_runtime_error_code(&alias, &mut alias_view), 0);
    assert_eq!(alias_view, view);
    let mut handle = std::ptr::null_mut();
    assert_eq!(xiao_runtime_string_new(view, &mut handle), 0);
    let mut result = xiao_runtime_value_str_owned(handle);
    xiao_runtime_value_release(&mut source);
    xiao_runtime_value_release(&mut alias);
    for _ in 0..64 {
        drop(crate::StringHandle::new("replacement allocation").unwrap());
    }
    let RuntimeValue::Str(text) = (unsafe { value_to_runtime(&result) }).unwrap() else {
        panic!("拥有的 str");
    };
    assert_eq!(text.to_string().unwrap(), "owned-after-release_雪");
    xiao_runtime_value_release(&mut result);
}
