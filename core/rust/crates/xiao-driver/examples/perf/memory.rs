//! 测量进程的当前驻留内存和历史峰值；只用于显式性能工具。

#[cfg(target_os = "linux")]
/// Linux /proc 的 VmRSS/VmHWM，单位 KiB 转字节。
pub(super) fn resident() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").expect("读取进程内存");
    let read = |name: &str| -> u64 {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            * 1024
    };
    (read("VmRSS:"), read("VmHWM:"))
}

#[cfg(target_os = "windows")]
/// Windows 的 WorkingSetSize/PeakWorkingSetSize，单位字节。
pub(super) fn resident() -> (u64, u64) {
    #[repr(C)]
    /// PROCESS_MEMORY_COUNTERS 的原生布局。
    struct Counters {
        cb: u32,
        page_faults: u32,
        peak_working_set: usize,
        working_set: usize,
        other: [usize; 6],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut std::ffi::c_void,
            counters: *mut Counters,
            size: u32,
        ) -> i32;
    }
    let mut counters = Counters {
        cb: std::mem::size_of::<Counters>() as u32,
        page_faults: 0,
        peak_working_set: 0,
        working_set: 0,
        other: [0; 6],
    };
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<Counters>() as u32,
        )
    };
    assert_ne!(ok, 0, "读取进程内存失败");
    (
        counters.working_set as u64,
        counters.peak_working_set as u64,
    )
}

#[cfg(target_os = "macos")]
/// macOS MACH_TASK_BASIC_INFO 的 resident_size/resident_size_max，单位字节。
pub(super) fn resident() -> (u64, u64) {
    #[repr(C)]
    /// mach_task_basic_info 的原生布局。
    struct BasicInfo {
        virtual_size: u64,
        resident_size: u64,
        resident_size_max: u64,
        times: [i32; 4],
        policy: i32,
        suspend_count: i32,
    }
    unsafe extern "C" {
        static mach_task_self_: u32;
        fn task_info(task: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
    }
    let mut info = BasicInfo {
        virtual_size: 0,
        resident_size: 0,
        resident_size_max: 0,
        times: [0; 4],
        policy: 0,
        suspend_count: 0,
    };
    let mut count = (std::mem::size_of::<BasicInfo>() / std::mem::size_of::<i32>()) as u32;
    let ok = unsafe {
        task_info(
            mach_task_self_,
            20,
            (&mut info as *mut BasicInfo).cast(),
            &mut count,
        )
    };
    assert_eq!(ok, 0, "读取进程内存失败");
    (info.resident_size, info.resident_size_max)
}
