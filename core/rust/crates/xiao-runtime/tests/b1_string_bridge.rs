//! B1 F10：读取 ABI 字符串不得分配与完整来源等大的临时文本。

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::ptr;

use xiao_runtime as _;
use xiao_runtime_abi::{XiaoAbiBytes, XiaoValue, XiaoValuePayload, XiaoValueTag};
use xiao_runtime_abi::{
    xiao_runtime_string_new, xiao_runtime_value_int, xiao_runtime_value_iter_get,
    xiao_runtime_value_iter_len, xiao_runtime_value_release_strong, xiao_runtime_value_str_owned,
};

unsafe extern "C" {
    /// 声明现有动态下标符号；布局与生产调用相同，不新增 Runtime 入口。
    fn xiao_runtime_value_index_get_dynamic(
        source: *const XiaoValue,
        index: *const XiaoValue,
        out: *mut XiaoValue,
    ) -> i32;
}

thread_local! {
    /// 只统计本测试线程的窗口，避免测试框架其它线程干扰。
    static LARGEST: Cell<Option<usize>> = const { Cell::new(None) };
}

/// 使用系统分配器，只观察真实申请大小，不注入失败或修改 Runtime。
struct Observer;

/// 分配完成后无分配地更新线程局部最大值，TLS 已销毁时直接忽略。
fn observe(size: usize) {
    let _ = LARGEST.try_with(|largest| {
        if let Some(previous) = largest.get() {
            largest.set(Some(previous.max(size)));
        }
    });
}

unsafe impl GlobalAlloc for Observer {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: 参数原样交给系统分配器，统计不拥有或解引用分配结果。
        let result = unsafe { System.alloc(layout) };
        if !result.is_null() {
            observe(layout.size());
        }
        result
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: 与 alloc 相同，保持系统分配器的布局和零初始化契约。
        let result = unsafe { System.alloc_zeroed(layout) };
        if !result.is_null() {
            observe(layout.size());
        }
        result
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: 调用方提供的存活指针和原布局直接归还系统分配器。
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: 不改变重分配的指针、布局或大小，不额外读取旧指针。
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            observe(size);
        }
        result
    }
}

#[global_allocator]
static ALLOCATOR: Observer = Observer;

/// 保证断言/展开后关闭观察窗口。
struct Window;
impl Drop for Window {
    fn drop(&mut self) {
        LARGEST.with(|largest| largest.set(None));
    }
}

#[test]
/// 大 ASCII 和 Unicode 的长度、迭代及动态下标只借读来源，保留原有拥有/释放契约。
fn abi_string_reads_do_not_copy_the_entire_source() {
    for text in ["x".repeat(131_072), "雪😀".repeat(32_768)] {
        // SAFETY: 输入缓冲在构造期间存活，所有输出槽已初始化，归还前始终持有有效句柄。
        unsafe {
            let mut handle = ptr::null_mut();
            assert_eq!(
                xiao_runtime_string_new(
                    XiaoAbiBytes {
                        ptr: text.as_ptr(),
                        len: text.len()
                    },
                    &mut handle
                ),
                0
            );
            let mut source = xiao_runtime_value_str_owned(handle);
            assert_eq!(source.tag, XiaoValueTag::Str);
            let mut length = 0;
            assert_eq!(xiao_runtime_value_iter_len(&source, &mut length), 0);
            assert_eq!(length, text.chars().count());
            let mut out = XiaoValue {
                tag: XiaoValueTag::None,
                payload: XiaoValuePayload { i64_value: 0 },
            };
            let mut index = xiao_runtime_value_int(-1);
            LARGEST.with(|largest| largest.set(Some(0)));
            let window = Window;
            let length_status = xiao_runtime_value_iter_len(&source, &mut length);
            let iter_status = xiao_runtime_value_iter_get(&source, length as i64 - 1, &mut out);
            let index_status = xiao_runtime_value_index_get_dynamic(&source, &index, &mut out);
            let largest = LARGEST.with(|largest| largest.get().unwrap());
            drop(window);
            assert_eq!((length_status, iter_status, index_status), (0, 0, 0));
            assert_eq!(out.tag, XiaoValueTag::Str);
            assert!(
                largest < 4096,
                "只读 ABI 读取分配了 {largest} 字节，来源大小 {}",
                text.len()
            );
            xiao_runtime_value_release_strong(&mut out);
            xiao_runtime_value_release_strong(&mut index);
            xiao_runtime_value_release_strong(&mut source);
        }
    }
}
